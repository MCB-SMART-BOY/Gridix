//! Shared test utilities for external integration tests.
//!
//! Usage from `tests/*.rs`:
//! ```ignore
//! mod common;
//! use common::{begin_key_pass, focus_text_input};
//! ```

// 每个测试二进制只使用本模块的一个子集,未使用的辅助函数不应产生警告。
#![allow(dead_code)]

use std::time::Duration;

use gridix::data::{ConnectionConfig, DatabaseType, MySqlSslMode, PostgresSslMode, execute_typed};
use gridix::domain::execution::StatementOutcome;
use gridix::domain::value::DbValue;

/// Read an optional PostgreSQL integration URL and apply TLS overrides.
pub fn pg_config_from_env() -> Option<ConnectionConfig> {
    let url = std::env::var("GRIDIX_TEST_PG_URL").ok()?;
    Some(pg_config_from_url(&url))
}

/// Parse a PostgreSQL integration URL and apply the configured TLS contract.
pub fn pg_config_from_url(url: &str) -> ConnectionConfig {
    let rest = url
        .strip_prefix("postgresql://")
        .or_else(|| url.strip_prefix("postgres://"))
        .unwrap_or(url);
    let (authority, database) = rest
        .split_once('/')
        .map_or((rest, String::new()), |(auth, db)| (auth, db.to_string()));
    let (userinfo, hostport) = authority
        .split_once('@')
        .map_or((None, authority), |(ui, hp)| (Some(ui), hp));
    let (username, password) = userinfo.map_or_else(
        || (String::new(), String::new()),
        |ui| match ui.split_once(':') {
            Some((user, password)) => (user.to_string(), password.to_string()),
            None => (ui.to_string(), String::new()),
        },
    );
    let (host, port) =
        hostport
            .split_once(':')
            .map_or((hostport.to_string(), 5432), |(host, port)| {
                (
                    host.to_string(),
                    port.parse::<u16>().unwrap_or_else(|_| {
                        panic!("invalid PostgreSQL port in GRIDIX_TEST_PG_URL")
                    }),
                )
            });
    let mut config = ConnectionConfig {
        db_type: DatabaseType::PostgreSQL,
        host,
        port,
        username,
        password,
        database,
        ..Default::default()
    };
    config.postgres_ssl_mode = postgres_ssl_mode_from_env();
    config.ssl_ca_cert = std::env::var("GRIDIX_TEST_PG_CA_CERT").unwrap_or_default();
    config.tls_server_name = optional_env("GRIDIX_TEST_PG_TLS_SERVER_NAME");
    config
}

/// Read an optional MySQL integration URL and apply TLS overrides.
pub fn mysql_config_from_env() -> Option<ConnectionConfig> {
    let url = std::env::var("GRIDIX_TEST_MYSQL_URL").ok()?;
    Some(mysql_config_from_url(&url))
}

/// Parse a MySQL integration URL and apply the configured TLS contract.
pub fn mysql_config_from_url(url: &str) -> ConnectionConfig {
    let rest = url
        .strip_prefix("mysql://")
        .unwrap_or_else(|| panic!("GRIDIX_TEST_MYSQL_URL must start with mysql://"));
    let (userinfo, rest) = rest
        .split_once('@')
        .unwrap_or_else(|| panic!("GRIDIX_TEST_MYSQL_URL must include credentials"));
    let (username, password) = userinfo.split_once(':').unwrap_or((userinfo, ""));
    let (hostport, database) = rest
        .split_once('/')
        .unwrap_or_else(|| panic!("GRIDIX_TEST_MYSQL_URL must include a database"));
    let (host, port) =
        hostport
            .split_once(':')
            .map_or((hostport.to_string(), 3306), |(host, port)| {
                (
                    host.to_string(),
                    port.parse::<u16>()
                        .unwrap_or_else(|_| panic!("invalid MySQL port in GRIDIX_TEST_MYSQL_URL")),
                )
            });
    let mut config = ConnectionConfig {
        db_type: DatabaseType::MySQL,
        host,
        port,
        username: username.to_string(),
        password: password.to_string(),
        database: database.to_string(),
        ..Default::default()
    };
    config.mysql_ssl_mode = mysql_ssl_mode_from_env();
    config.ssl_ca_cert = std::env::var("GRIDIX_TEST_MYSQL_CA_CERT").unwrap_or_default();
    config.tls_server_name = optional_env("GRIDIX_TEST_MYSQL_TLS_SERVER_NAME");
    config
}

/// Require an environment variable in acceptance-only tests.
pub fn required_env(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| panic!("{name} must be set for acceptance"))
}

/// Whether an acceptance-only suite was explicitly requested.
///
/// Acceptance suites need external fixtures that the generic `cargo test --workspace` run
/// cannot provide, so they skip with a printed reason unless `GRIDIX_ACCEPTANCE=1` is set.
/// The dedicated acceptance workflows set that flag, which keeps `required_env` a hard failure
/// there: an acceptance run can never report a silent skip as evidence.
pub fn acceptance_requested() -> bool {
    matches!(
        std::env::var("GRIDIX_ACCEPTANCE").as_deref(),
        Ok("1") | Ok("true")
    )
}

fn optional_env(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn postgres_ssl_mode_from_env() -> PostgresSslMode {
    match std::env::var("GRIDIX_TEST_PG_SSL_MODE")
        .unwrap_or_else(|_| "disable".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "disable" | "disabled" => PostgresSslMode::Disable,
        "require" => PostgresSslMode::Require,
        "verify-ca" | "verify_ca" | "verifyca" => PostgresSslMode::VerifyCa,
        "verify-full" | "verify_full" | "verifyfull" => PostgresSslMode::VerifyFull,
        mode => panic!(
            "GRIDIX_TEST_PG_SSL_MODE={mode:?} is invalid; use disable, require, verify-ca, or verify-full"
        ),
    }
}

fn mysql_ssl_mode_from_env() -> MySqlSslMode {
    match std::env::var("GRIDIX_TEST_MYSQL_SSL_MODE")
        .unwrap_or_else(|_| "disabled".to_string())
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "disabled" | "disable" => MySqlSslMode::Disabled,
        "required" | "require" => MySqlSslMode::Required,
        "verify-ca" | "verify_ca" | "verifyca" => MySqlSslMode::VerifyCa,
        "verify-identity" | "verify_identity" | "verifyidentity" => MySqlSslMode::VerifyIdentity,
        mode => panic!(
            "GRIDIX_TEST_MYSQL_SSL_MODE={mode:?} is invalid; use disabled, required, verify-ca, or verify-identity"
        ),
    }
}
use egui::{Event, Key, Modifiers, RawInput};

/// Inject a keypress event into an egui context via `begin_pass`.
pub fn begin_key_pass(ctx: &egui::Context, key: Key) {
    ctx.begin_pass(RawInput {
        events: vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        modifiers: Modifiers::NONE,
        ..Default::default()
    });
}

/// Focus a transient text input so that keyboard conflict tests can detect
/// text-entry priority.
pub fn focus_text_input(ctx: &egui::Context) {
    let mut text = String::new();
    ctx.begin_pass(RawInput::default());
    egui::Window::new("shared test text input").show(ctx, |ui| {
        let response =
            ui.add(egui::TextEdit::singleline(&mut text).id_salt("shared_test_text_input"));
        response.request_focus();
    });
    let _ = ctx.end_pass();
}

// ===== 连接池验收辅助（需要真实服务端；见 tests/mysql_pool_acceptance.rs）=====

/// 单次验收任务的硬超时，避免缺陷以挂起形式出现而不是断言失败。
pub const ACCEPTANCE_TASK_TIMEOUT: Duration = Duration::from_secs(60);

/// MySQL 验收配置：`GRIDIX_ACCEPTANCE` 已设置却缺少 URL 时硬失败，否则跳过。
pub fn mysql_acceptance_config_or_skip() -> Option<ConnectionConfig> {
    if !acceptance_requested() {
        eprintln!("SKIP: GRIDIX_ACCEPTANCE=1 not set");
        return None;
    }
    Some(mysql_config_from_url(&required_env(
        "GRIDIX_TEST_MYSQL_URL",
    )))
}

/// PostgreSQL 验收配置，语义同 [`mysql_acceptance_config_or_skip`]。
pub fn pg_acceptance_config_or_skip() -> Option<ConnectionConfig> {
    if !acceptance_requested() {
        eprintln!("SKIP: GRIDIX_ACCEPTANCE=1 not set");
        return None;
    }
    Some(pg_config_from_url(&required_env("GRIDIX_TEST_PG_URL")))
}

/// 进程级串行守卫。
///
/// 同一测试二进制内的验收用例共享全局 `POOL_MANAGER`，而 `clear_all` 会拆除兄弟
/// 用例正在使用的连接；不依赖 `--test-threads=1` 也必须串行。
pub async fn acceptance_serial_guard() -> tokio::sync::MutexGuard<'static, ()> {
    static SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    SERIAL.lock().await
}

/// 断言一次 `SELECT <marker> AS marker` 返回单行单列，且值为本次调用的 marker。
///
/// 值断言用于发现并发下结果串线：每个调用者都必须拿到自己的结果。
pub async fn assert_select_round_trip(config: &ConnectionConfig, marker: usize) {
    let sql = format!("SELECT {marker} AS marker");
    let outcome = tokio::time::timeout(ACCEPTANCE_TASK_TIMEOUT, execute_typed(config, &sql))
        .await
        .unwrap_or_else(|_| panic!("pool query {marker} must finish before deadline"))
        .unwrap_or_else(|error| panic!("pool query {marker} must succeed: {error}"));

    assert_eq!(
        outcome.statements.len(),
        1,
        "query {marker} must return one statement"
    );
    let StatementOutcome::ResultSet(result_set) = &outcome.statements[0] else {
        panic!("query {marker} must return a result set");
    };
    assert_eq!(
        result_set.row_count, 1,
        "query {marker} must return one row"
    );
    assert_eq!(
        result_set.cell(0, 0),
        &DbValue::Int(marker as i64),
        "query {marker} must return its own value, not another caller's"
    );
}

/// 并发发出 `concurrency` 个查询；每个都必须在超时内完成并返回自己的 marker。
pub async fn assert_concurrent_queries_served(config: ConnectionConfig, concurrency: usize) {
    let mut tasks = Vec::with_capacity(concurrency);
    for marker in 0..concurrency {
        let config = config.clone();
        tasks.push(tokio::spawn(async move {
            assert_select_round_trip(&config, marker).await;
        }));
    }

    for (index, task) in tasks.into_iter().enumerate() {
        tokio::time::timeout(ACCEPTANCE_TASK_TIMEOUT, task)
            .await
            .unwrap_or_else(|_| panic!("concurrent task {index} must finish before deadline"))
            .unwrap_or_else(|error| panic!("concurrent task {index} must not panic: {error}"));
    }
}
