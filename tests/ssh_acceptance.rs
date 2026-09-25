//! Real SSH tunnel acceptance test.
//!
//! This test deliberately refuses to pass without a reachable jump host and database: it runs
//! only when `GRIDIX_ACCEPTANCE=1` is set, and then a missing fixture variable is a hard failure.
//! The manually triggered SSH acceptance workflow sets that flag.

use gridix::data::ssh_tunnel::SSH_TUNNEL_MANAGER;
use gridix::data::{ConnectionConfig, DatabaseType, SshAuthMethod, SshTunnelConfig, execute_typed};
use gridix::domain::execution::{ExecutionOutcome, StatementOutcome};
use gridix::domain::value::DbValue;

mod common;
use common::{acceptance_requested, pg_config_from_url, required_env};

fn required_port(name: &str) -> u16 {
    required_env(name)
        .parse()
        .unwrap_or_else(|_| panic!("{name} must be a valid u16"))
}

fn ssh_config_from_env() -> SshTunnelConfig {
    SshTunnelConfig {
        enabled: true,
        ssh_host: required_env("GRIDIX_TEST_SSH_HOST"),
        ssh_port: required_port("GRIDIX_TEST_SSH_PORT"),
        ssh_username: required_env("GRIDIX_TEST_SSH_USERNAME"),
        ssh_password: required_env("GRIDIX_TEST_SSH_PASSWORD"),
        auth_method: SshAuthMethod::Password,
        remote_host: required_env("GRIDIX_TEST_SSH_REMOTE_HOST"),
        remote_port: required_port("GRIDIX_TEST_SSH_REMOTE_PORT"),
        ..SshTunnelConfig::new()
    }
}

fn result_set(outcome: ExecutionOutcome) -> gridix::domain::result::ResultSet {
    assert_eq!(outcome.statements.len(), 1);
    match &outcome.statements[0] {
        StatementOutcome::ResultSet(result) => result.clone(),
        other => panic!("expected one ResultSet, got {other:?}"),
    }
}

#[tokio::test]
async fn ssh_tunnel_forwards_postgresql_query() {
    if !acceptance_requested() {
        eprintln!("skipping SSH tunnel acceptance: set GRIDIX_ACCEPTANCE=1 and its fixtures");
        return;
    }
    let database_url = required_env("GRIDIX_TEST_SSH_PG_URL");
    let mut config: ConnectionConfig = pg_config_from_url(&database_url);
    config.db_type = DatabaseType::PostgreSQL;
    config.postgres_ssl_mode = gridix::data::PostgresSslMode::Disable;
    config.ssl_ca_cert.clear();
    config.tls_server_name = None;
    config.ssh_config = ssh_config_from_env();

    let result = result_set(
        execute_typed(&config, "SELECT 1 AS tunnel_probe")
            .await
            .expect("PostgreSQL query through SSH tunnel must succeed"),
    );
    assert_eq!(result.row_count, 1);
    assert!(matches!(result.cell(0, 0), DbValue::Int(1)));
    SSH_TUNNEL_MANAGER.stop_all().await;
}
