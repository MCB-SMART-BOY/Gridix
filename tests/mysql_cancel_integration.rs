//! MySQL 查询取消集成测试（需要外部 MySQL 环境）
//!
//! 运行方式（示例）:
//! GRIDIX_IT_MYSQL_HOST=127.0.0.1 \
//! GRIDIX_IT_MYSQL_PORT=3306 \
//! GRIDIX_IT_MYSQL_USER=root \
//! GRIDIX_IT_MYSQL_PASSWORD=secret \
//! GRIDIX_IT_MYSQL_DB=test \
//! cargo test --test mysql_cancel_integration -- --ignored

use gridix::database::{
    ConnectionConfig, DatabaseType, MySqlSslMode, execute_query, execute_query_cancellable,
};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

fn mysql_test_config() -> Option<ConnectionConfig> {
    let host = std::env::var("GRIDIX_IT_MYSQL_HOST").ok()?;
    let username = std::env::var("GRIDIX_IT_MYSQL_USER").ok()?;
    let database = std::env::var("GRIDIX_IT_MYSQL_DB").ok()?;
    let password = std::env::var("GRIDIX_IT_MYSQL_PASSWORD").unwrap_or_default();
    let port = std::env::var("GRIDIX_IT_MYSQL_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(3306);

    Some(ConnectionConfig {
        name: "mysql-it-cancel".to_string(),
        db_type: DatabaseType::MySQL,
        host,
        port,
        username,
        password,
        database,
        mysql_ssl_mode: MySqlSslMode::Disabled,
        ..Default::default()
    })
}

#[tokio::test]
#[ignore = "requires external MySQL and GRIDIX_IT_MYSQL_* env vars"]
async fn mysql_cancel_interrupts_long_running_query() {
    let Some(config) = mysql_test_config() else {
        eprintln!("skip mysql integration test: missing GRIDIX_IT_MYSQL_* env vars");
        return;
    };

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = cancel_tx.send(());
    });

    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        execute_query_cancellable(&config, "SELECT SLEEP(10)", cancel_rx),
    )
    .await;

    let elapsed = started.elapsed();
    assert!(
        elapsed < Duration::from_secs(5),
        "cancelled query should return quickly, elapsed: {:?}",
        elapsed
    );

    let err = result
        .expect("cancellable query future should complete before timeout")
        .expect_err("query should be cancelled");
    let err_text = err.to_string();
    assert!(
        err_text.contains("查询已取消") || err_text.to_ascii_lowercase().contains("cancel"),
        "unexpected cancel error text: {}",
        err_text
    );
}

#[tokio::test]
#[ignore = "requires external MySQL and GRIDIX_IT_MYSQL_* env vars"]
async fn mysql_connection_still_works_after_cancel() {
    let Some(config) = mysql_test_config() else {
        eprintln!("skip mysql integration test: missing GRIDIX_IT_MYSQL_* env vars");
        return;
    };

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = cancel_tx.send(());
    });

    let _ = execute_query_cancellable(&config, "SELECT SLEEP(5)", cancel_rx)
        .await
        .expect_err("query should be cancelled");

    let quick = execute_query(&config, "SELECT 1 AS ping")
        .await
        .expect("connection should remain usable after cancellation");

    assert_eq!(quick.columns, vec!["ping".to_string()]);
    assert_eq!(quick.rows.len(), 1);
}

#[tokio::test]
#[ignore = "requires external MySQL and GRIDIX_IT_MYSQL_* env vars"]
async fn mysql_cancel_still_works_when_pool_near_capacity() {
    let Some(config) = mysql_test_config() else {
        eprintln!("skip mysql integration test: missing GRIDIX_IT_MYSQL_* env vars");
        return;
    };

    // 默认池上限为 10，先占用 9 个连接，再验证取消查询仍可生效。
    let mut blockers = Vec::new();
    for _ in 0..9 {
        let blocker_config = config.clone();
        blockers.push(tokio::spawn(async move {
            let _ = execute_query(&blocker_config, "SELECT SLEEP(5)").await;
        }));
    }

    tokio::time::sleep(Duration::from_millis(300)).await;

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        let _ = cancel_tx.send(());
    });

    let started = Instant::now();
    let result = tokio::time::timeout(
        Duration::from_secs(6),
        execute_query_cancellable(&config, "SELECT SLEEP(10)", cancel_rx),
    )
    .await;
    let elapsed = started.elapsed();

    assert!(
        elapsed < Duration::from_secs(6),
        "cancelled query should return before timeout under high pool usage, elapsed: {:?}",
        elapsed
    );

    let err = result
        .expect("cancellable query future should complete before timeout")
        .expect_err("query should be cancelled");
    let err_text = err.to_string();
    assert!(
        err_text.contains("查询已取消") || err_text.to_ascii_lowercase().contains("cancel"),
        "unexpected cancel error text: {}",
        err_text
    );

    for blocker in blockers {
        let _ = blocker.await;
    }
}
