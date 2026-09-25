//! PostgreSQL 服务端查询取消集成测试。

use std::time::{Duration, Instant};

use gridix::data::{ConnectionConfig, DbError, execute_typed, execute_typed_cancellable};
mod common;
use common::pg_config_from_url;
use gridix::domain::execution::StatementOutcome;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const OBSERVER_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

fn pg_config() -> Result<Option<(String, ConnectionConfig)>, std::env::VarError> {
    match std::env::var("GRIDIX_TEST_PG_URL") {
        Ok(url) => Ok(Some((url.clone(), pg_config_from_url(&url)))),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(error),
    }
}

async fn marker_is_visible(
    observer: &tokio_postgres::Client,
    marker: &str,
) -> Result<bool, tokio_postgres::Error> {
    let row = observer
        .query_opt(
            "SELECT 1 FROM pg_stat_activity WHERE query LIKE $1",
            &[&format!("%{}%", marker)],
        )
        .await?;
    Ok(row.is_some())
}

async fn wait_for_marker(observer: &tokio_postgres::Client, marker: &str, should_be_visible: bool) {
    let deadline = Instant::now() + OBSERVER_TIMEOUT;
    loop {
        let visible = marker_is_visible(observer, marker)
            .await
            .expect("observer pg_stat_activity query must succeed");
        if visible == should_be_visible {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "marker {} did not become visible={}",
            marker,
            should_be_visible
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[tokio::test]
async fn execute_typed_cancellable_server_query_observed_cancelled_and_connection_reusable() {
    let Some((url, config)) = pg_config().expect("GRIDIX_TEST_PG_URL could not be read") else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let (observer, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .expect("observer PostgreSQL connection must succeed");
    tokio::spawn(async move {
        connection
            .await
            .expect("observer PostgreSQL connection must remain healthy");
    });

    let marker = format!("gridix-cancel:{}", Uuid::new_v4());
    let sql = format!("SELECT pg_sleep(30) /* {} */", marker);
    let cancellation = CancellationToken::new();
    let worker_config = config.clone();
    let worker_token = cancellation.clone();
    println!("marker={} milestone=start", marker);
    let worker = tokio::spawn(async move {
        execute_typed_cancellable(&worker_config, &sql, &worker_token).await
    });

    wait_for_marker(&observer, &marker, true).await;
    println!("marker={} milestone=observed", marker);
    cancellation.cancel();

    let result = tokio::time::timeout(OBSERVER_TIMEOUT, worker)
        .await
        .expect("cancelled PostgreSQL query must complete before deadline")
        .expect("PostgreSQL worker must not panic");
    assert!(matches!(result, Err(DbError::Cancelled)));
    println!("marker={} milestone=cancelled", marker);

    wait_for_marker(&observer, &marker, false).await;
    println!("marker={} milestone=disappeared", marker);

    let outcome = execute_typed(&config, "SELECT 1 AS one")
        .await
        .expect("execution connection must remain usable");
    assert_eq!(
        outcome.statements.len(),
        1,
        "SELECT 1 must return one result"
    );
    let StatementOutcome::ResultSet(result_set) = &outcome.statements[0] else {
        panic!("SELECT 1 must return a result set");
    };
    assert_eq!(result_set.row_count, 1, "SELECT 1 must return one row");
    println!("marker={} milestone=select-1", marker);
}

#[tokio::test]
async fn execute_typed_cancellable_pre_cancel_returns_cancelled_without_dispatching_marker() {
    let Some((url, config)) = pg_config().expect("GRIDIX_TEST_PG_URL could not be read") else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let (observer, connection) = tokio_postgres::connect(&url, tokio_postgres::NoTls)
        .await
        .expect("observer PostgreSQL connection must succeed");
    tokio::spawn(async move {
        connection
            .await
            .expect("observer PostgreSQL connection must remain healthy");
    });

    let marker = format!("gridix-pre-cancel:{}", Uuid::new_v4());
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = execute_typed_cancellable(
        &config,
        &format!("SELECT pg_sleep(30) /* {} */", marker),
        &cancellation,
    )
    .await;

    assert!(matches!(result, Err(DbError::Cancelled)));
    assert!(
        !marker_is_visible(&observer, &marker)
            .await
            .expect("observer pg_stat_activity query must succeed"),
        "pre-cancelled query must never be dispatched"
    );
}
