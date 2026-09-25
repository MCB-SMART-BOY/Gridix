//! Real TLS acceptance tests for PostgreSQL and MySQL.
//!
//! These tests refuse to pass without their fixtures: they run only when `GRIDIX_ACCEPTANCE=1`
//! is set, and then a missing fixture variable is a hard failure. The dedicated workflow starts
//! TLS-required database fixtures, so a skipped test cannot be reported as successful TLS
//! evidence.
//!
//! Required variables:
//! - `GRIDIX_TEST_PG_URL`, `GRIDIX_TEST_PG_SSL_MODE=verify-full`, `GRIDIX_TEST_PG_CA_CERT`
//! - `GRIDIX_TEST_MYSQL_URL`, `GRIDIX_TEST_MYSQL_SSL_MODE=verify-identity`,
//!   `GRIDIX_TEST_MYSQL_CA_CERT`, `GRIDIX_TEST_MYSQL_TLS_SERVER_NAME`

use gridix::data::{MySqlSslMode, PostgresSslMode, execute_typed};
use gridix::domain::execution::{ExecutionOutcome, StatementOutcome};
use gridix::domain::value::DbValue;
use std::path::Path;

mod common;
use common::{acceptance_requested, mysql_config_from_url, pg_config_from_url, required_env};

fn result_set(outcome: ExecutionOutcome) -> gridix::domain::result::ResultSet {
    assert_eq!(outcome.statements.len(), 1);
    match &outcome.statements[0] {
        StatementOutcome::ResultSet(result) => result.clone(),
        other => panic!("expected one ResultSet, got {other:?}"),
    }
}

#[tokio::test]
async fn postgres_tls_fixture_requires_ca_and_reports_encrypted_session() {
    if !acceptance_requested() {
        eprintln!("skipping PostgreSQL TLS acceptance: set GRIDIX_ACCEPTANCE=1 and its fixtures");
        return;
    }
    let url = required_env("GRIDIX_TEST_PG_URL");
    let ca_cert = required_env("GRIDIX_TEST_PG_CA_CERT");
    let config = pg_config_from_url(&url);

    assert_eq!(config.postgres_ssl_mode, PostgresSslMode::VerifyFull);
    assert!(
        Path::new(&ca_cert).is_file(),
        "PostgreSQL CA fixture must exist"
    );
    assert_eq!(config.ssl_ca_cert, ca_cert);

    let result = result_set(
        execute_typed(
            &config,
            "SELECT ssl FROM pg_stat_ssl WHERE pid = pg_backend_pid()",
        )
        .await
        .expect("PostgreSQL TLS query must succeed"),
    );
    assert_eq!(result.row_count, 1);
    assert!(matches!(result.cell(0, 0), DbValue::Bool(true)));
}

#[tokio::test]
async fn mysql_tls_fixture_requires_ca_and_reports_cipher() {
    if !acceptance_requested() {
        eprintln!("skipping MySQL TLS acceptance: set GRIDIX_ACCEPTANCE=1 and its fixtures");
        return;
    }
    let url = required_env("GRIDIX_TEST_MYSQL_URL");
    let ca_cert = required_env("GRIDIX_TEST_MYSQL_CA_CERT");
    let server_name = required_env("GRIDIX_TEST_MYSQL_TLS_SERVER_NAME");
    let config = mysql_config_from_url(&url);

    assert_eq!(config.mysql_ssl_mode, MySqlSslMode::VerifyIdentity);
    assert!(Path::new(&ca_cert).is_file(), "MySQL CA fixture must exist");
    assert_eq!(config.ssl_ca_cert, ca_cert);
    assert_eq!(
        config.tls_server_name.as_deref(),
        Some(server_name.as_str())
    );

    let result = result_set(
        execute_typed(&config, "SHOW STATUS LIKE 'Ssl_cipher'")
            .await
            .expect("MySQL TLS query must succeed"),
    );
    assert_eq!(result.row_count, 1);
    let cipher = result.cell(0, 1);
    assert!(!matches!(cipher, DbValue::Null));
    assert!(!cipher.display().is_empty());
    assert_ne!(cipher.display(), "NULL");
}
