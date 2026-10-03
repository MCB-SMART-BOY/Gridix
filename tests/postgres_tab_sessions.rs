//! Real PostgreSQL SQL Tab session isolation and transaction lifetime acceptance.

mod common;

use common::pg_config_from_env;
use gridix::data::{
    ConnectionConfig, DbError, await_pg_tab_query, close_pg_connection_sessions,
    close_pg_tab_session, enqueue_pg_tab_query, execute_typed,
};
use gridix::domain::execution::{ExecutionOutcome, StatementOutcome};
use gridix::domain::ids::{ConnectionId, DocumentId};
use gridix::domain::value::DbValue;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const OBSERVER_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

type TabKey = (ConnectionId, DocumentId);

async fn tab_query(
    config: &ConnectionConfig,
    key: TabKey,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    let cancellation = CancellationToken::new();
    let pending = enqueue_pg_tab_query(
        config,
        sql,
        key.0,
        key.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )?;
    await_pg_tab_query(pending).await
}

async fn scalar(config: &ConnectionConfig, key: TabKey, sql: &str) -> i64 {
    let outcome = tab_query(config, key, sql)
        .await
        .expect("SQL Tab SELECT must succeed");
    scalar_result(outcome)
}

fn scalar_result(outcome: ExecutionOutcome) -> i64 {
    let [StatementOutcome::ResultSet(rows)] = outcome.statements.as_slice() else {
        panic!("expected one typed result set");
    };
    assert_eq!(rows.row_count, 1);
    let [DbValue::Int(value)] = rows.cells.as_slice() else {
        panic!("expected one integer cell");
    };
    *value
}

fn new_key(connection: ConnectionId) -> TabKey {
    (connection, DocumentId(Uuid::new_v4()))
}

async fn create_test_table(config: &ConnectionConfig) -> String {
    let table = format!("gridix_tab_{}", Uuid::new_v4().simple());
    execute_typed(
        config,
        &format!("CREATE TABLE {table} (value INTEGER NOT NULL)"),
    )
    .await
    .expect("fixture table creation must succeed");
    table
}

async fn drop_test_table(config: &ConnectionConfig, table: &str) {
    execute_typed(config, &format!("DROP TABLE {table}"))
        .await
        .expect("fixture table removal must succeed");
}

async fn wait_for_running_marker(config: &ConnectionConfig, marker: &str) {
    let deadline = Instant::now() + OBSERVER_TIMEOUT;
    let sql = format!(
        "SELECT count(*) FROM pg_stat_activity WHERE pid <> pg_backend_pid() AND query LIKE '%{marker}%' AND state = 'active'"
    );
    loop {
        let outcome = execute_typed(config, &sql)
            .await
            .expect("activity observer must succeed");
        if scalar_result(outcome) > 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "query marker never became visible"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

async fn wait_for_commit_lock(config: &ConnectionConfig, backend_pid: i64) {
    let deadline = Instant::now() + OBSERVER_TIMEOUT;
    let sql = format!(
        "SELECT count(*) FROM pg_stat_activity WHERE pid = {backend_pid} AND query = 'COMMIT' AND wait_event_type = 'Lock'"
    );
    loop {
        let outcome = execute_typed(config, &sql)
            .await
            .expect("COMMIT observer must succeed");
        if scalar_result(outcome) > 0 {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "COMMIT never blocked on the row lock"
        );
        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

#[tokio::test]
async fn pg_tab_enqueued_from_sync_thread_runs_on_supplied_runtime() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    let runtime = tokio::runtime::Handle::current();
    let pending = std::thread::spawn(move || {
        enqueue_pg_tab_query(
            &config,
            "SELECT 42",
            tab.0,
            tab.1,
            &CancellationToken::new(),
            &runtime,
        )
        .expect("synchronous enqueue outside Tokio context")
    })
    .join()
    .expect("enqueue thread must not panic");
    assert_eq!(
        scalar_result(await_pg_tab_query(pending).await.expect("queued query")),
        42
    );
    close_pg_connection_sessions(connection);
}

#[tokio::test]
async fn pg_tab_begin_insert_select_rollback_isolated_from_other_tab_and_pool() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab_a = new_key(connection);
    let tab_b = new_key(connection);
    let cancellation = CancellationToken::new();
    let begin = enqueue_pg_tab_query(
        &config,
        "BEGIN",
        tab_a.0,
        tab_a.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue BEGIN");
    let insert = enqueue_pg_tab_query(
        &config,
        &format!("INSERT INTO {table} VALUES (1)"),
        tab_a.0,
        tab_a.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue INSERT before BEGIN finishes");
    await_pg_tab_query(begin).await.expect("BEGIN");
    await_pg_tab_query(insert)
        .await
        .expect("insert within transaction");
    let count = format!("SELECT count(*) FROM {table}");
    assert_eq!(scalar(&config, tab_a, &count).await, 1);
    assert_eq!(scalar(&config, tab_b, &count).await, 0);
    let other_connection = ConnectionId(Uuid::new_v4());
    assert_eq!(
        scalar(&config, (other_connection, tab_a.1), &count).await,
        0
    );
    close_pg_connection_sessions(other_connection);
    assert_eq!(
        scalar_result(
            execute_typed(&config, &count)
                .await
                .expect("pooled metadata query")
        ),
        0
    );
    tab_query(&config, tab_a, "ROLLBACK")
        .await
        .expect("explicit rollback");
    assert_eq!(scalar(&config, tab_a, &count).await, 0);
    assert_eq!(scalar(&config, tab_b, &count).await, 0);
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_tab_local_decode_failure_keeps_transaction_healthy() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    tab_query(&config, tab, "BEGIN").await.expect("BEGIN");

    let error = tab_query(
        &config,
        tab,
        "SELECT '00000000-0000-0000-0000-000000000001'::uuid",
    )
    .await
    .expect_err("unsupported result must fail instead of exporting a placeholder");
    assert!(
        matches!(error, DbError::Query(message) if message.contains("unsupported PostgreSQL type uuid"))
    );
    assert_eq!(scalar(&config, tab, "SELECT 42").await, 42);
    tab_query(&config, tab, "COMMIT")
        .await
        .expect("local decode failure must not mark the transaction aborted");
    close_pg_connection_sessions(connection);
}

#[tokio::test]
async fn pg_tab_numeric_special_values_decode_without_aborting_transaction() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    tab_query(&config, tab, "BEGIN").await.expect("BEGIN");
    let result = tab_query(
        &config,
        tab,
        "SELECT 'NaN'::numeric, 'Infinity'::numeric, '-Infinity'::numeric",
    )
    .await
    .expect("NUMERIC special values must decode");
    let [StatementOutcome::ResultSet(rows)] = result.statements.as_slice() else {
        panic!("expected one typed result set");
    };
    assert_eq!(
        rows.cells,
        [
            DbValue::Decimal("NaN".into()),
            DbValue::Decimal("Infinity".into()),
            DbValue::Decimal("-Infinity".into()),
        ]
    );
    tab_query(&config, tab, "COMMIT")
        .await
        .expect("decoded special values must not abort the transaction");
    close_pg_connection_sessions(connection);
}

#[tokio::test]
async fn pg_tab_error_and_cancel_keep_transaction_aborted_until_explicit_rollback() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let connection = ConnectionId(Uuid::new_v4());
    let tab_a = new_key(connection);
    let tab_b = new_key(connection);
    tab_query(&config, tab_a, "BEGIN").await.expect("BEGIN");
    assert!(matches!(
        tab_query(&config, tab_a, "SELECT 1 / 0").await,
        Err(DbError::Query(_))
    ));
    assert!(matches!(
        tab_query(&config, tab_a, "SELECT 1").await,
        Err(DbError::Query(_))
    ));
    assert!(matches!(
        tab_query(&config, tab_a, "COMMIT").await,
        Err(DbError::Query(_))
    ));
    assert_eq!(scalar(&config, tab_b, "SELECT 2").await, 2);
    tab_query(&config, tab_a, "ROLLBACK")
        .await
        .expect("rollback error state");
    assert_eq!(scalar(&config, tab_a, "SELECT 3").await, 3);

    tab_query(&config, tab_a, "BEGIN")
        .await
        .expect("second BEGIN");
    let marker = format!("gridix-tab-cancel:{}", Uuid::new_v4());
    let cancellation = CancellationToken::new();
    let pending = enqueue_pg_tab_query(
        &config,
        &format!("SELECT pg_sleep(30) /* {marker} */"),
        tab_a.0,
        tab_a.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue cancellable query");
    wait_for_running_marker(&config, &marker).await;
    let other_tab_value =
        tokio::time::timeout(OBSERVER_TIMEOUT, scalar(&config, tab_b, "SELECT 4"))
            .await
            .expect("another Tab must not wait for the running query");
    assert_eq!(other_tab_value, 4);
    cancellation.cancel();
    let result = tokio::time::timeout(OBSERVER_TIMEOUT, await_pg_tab_query(pending))
        .await
        .expect("cancelled query must finish before deadline");
    assert!(matches!(result, Err(DbError::Cancelled)));
    assert!(matches!(
        tab_query(&config, tab_a, "SELECT 1").await,
        Err(DbError::Query(_))
    ));
    assert!(matches!(
        tab_query(&config, tab_a, "COMMIT").await,
        Err(DbError::Query(_))
    ));
    tab_query(&config, tab_a, "ROLLBACK")
        .await
        .expect("rollback cancellation state");
    assert_eq!(scalar(&config, tab_a, "SELECT 5").await, 5);
    close_pg_connection_sessions(connection);
}

#[tokio::test]
async fn pg_tab_close_and_disconnect_rollback_uncommitted_changes() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab_a = new_key(connection);
    let tab_b = new_key(connection);
    let count = format!("SELECT count(*) FROM {table}");
    tab_query(&config, tab_a, "BEGIN").await.expect("BEGIN");
    tab_query(&config, tab_a, &format!("INSERT INTO {table} VALUES (1)"))
        .await
        .expect("uncommitted insert");
    let marker = format!("gridix-tab-close:{}", Uuid::new_v4());
    let cancellation = CancellationToken::new();
    let pending = enqueue_pg_tab_query(
        &config,
        &format!("SELECT pg_sleep(30) /* {marker} */"),
        tab_a.0,
        tab_a.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue query before Tab close");
    wait_for_running_marker(&config, &marker).await;
    close_pg_tab_session(tab_a.0, tab_a.1);
    let closed_result = tokio::time::timeout(OBSERVER_TIMEOUT, await_pg_tab_query(pending))
        .await
        .expect("closed Tab request must finish promptly");
    assert!(matches!(closed_result, Err(DbError::Connection(_))));
    assert_eq!(scalar(&config, tab_b, &count).await, 0);

    tab_query(&config, tab_b, "BEGIN")
        .await
        .expect("BEGIN on other tab");
    tab_query(&config, tab_b, &format!("INSERT INTO {table} VALUES (2)"))
        .await
        .expect("second uncommitted insert");
    close_pg_connection_sessions(connection);
    assert_eq!(
        scalar_result(
            execute_typed(&config, &count)
                .await
                .expect("pooled observer")
        ),
        0
    );
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_tab_rejected_begin_never_runs_queued_insert_as_autocommit() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    let cancellation = CancellationToken::new();
    let begin = enqueue_pg_tab_query(
        &config,
        "BEGIN ISOLATION LEVEL SERIALIZABLE",
        tab.0,
        tab.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue unsupported BEGIN");
    let insert = enqueue_pg_tab_query(
        &config,
        &format!("INSERT INTO {table} VALUES (1)"),
        tab.0,
        tab.1,
        &cancellation,
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue dependent INSERT");
    assert!(matches!(
        await_pg_tab_query(begin).await,
        Err(DbError::Query(_))
    ));
    assert!(matches!(
        await_pg_tab_query(insert).await,
        Err(DbError::Connection(_))
    ));
    let count = format!("SELECT count(*) FROM {table}");
    assert_eq!(
        scalar_result(
            execute_typed(&config, &count)
                .await
                .expect("pooled observer")
        ),
        0
    );
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_tab_leading_empty_rollback_cannot_commit_following_insert() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    tab_query(&config, tab, "BEGIN").await.expect("BEGIN");
    assert!(matches!(
        tab_query(&config, tab, " /* note */ ;ROLLBACK").await,
        Err(DbError::Query(_))
    ));
    tab_query(&config, tab, &format!("INSERT INTO {table} VALUES (1)"))
        .await
        .expect("insert remains inside original transaction");
    tab_query(&config, tab, "ROLLBACK").await.expect("ROLLBACK");
    assert_eq!(
        scalar_result(
            execute_typed(&config, &format!("SELECT count(*) FROM {table}"))
                .await
                .expect("observer")
        ),
        0
    );
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_tab_nested_comment_rollback_updates_actor_transaction_state() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    tab_query(&config, tab, "BEGIN").await.expect("BEGIN");
    tab_query(&config, tab, "/* outer /* inner */ */ ROLLBACK")
        .await
        .expect("nested-comment ROLLBACK");
    tab_query(&config, tab, &format!("INSERT INTO {table} VALUES (1)"))
        .await
        .expect("insert after rollback");
    assert!(matches!(
        tab_query(&config, tab, "ROLLBACK").await,
        Err(DbError::Query(_))
    ));
    assert_eq!(
        scalar_result(
            execute_typed(&config, &format!("SELECT count(*) FROM {table}"))
                .await
                .expect("observer")
        ),
        1
    );
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_tab_cancel_blocked_commit_reports_unknown_and_discards_queued_sql() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let suffix = Uuid::new_v4().simple();
    let parent = format!("gridix_commit_parent_{suffix}");
    let child = format!("gridix_commit_child_{suffix}");
    execute_typed(
        &config,
        &format!("CREATE TABLE {parent} (id INTEGER PRIMARY KEY)"),
    )
    .await
    .expect("create parent");
    execute_typed(&config, &format!(
        "CREATE TABLE {child} (id INTEGER REFERENCES {parent}(id) DEFERRABLE INITIALLY DEFERRED)"
    )).await.expect("create child");
    execute_typed(&config, &format!("INSERT INTO {parent} VALUES (1)"))
        .await
        .expect("seed parent");
    execute_typed(&config, &format!("INSERT INTO {parent} VALUES (2)"))
        .await
        .expect("seed dependent row for queued SQL");
    let connection = ConnectionId(Uuid::new_v4());
    let tab_a = new_key(connection);
    let tab_b = new_key(connection);
    tab_query(&config, tab_b, "BEGIN")
        .await
        .expect("lock holder BEGIN");
    tab_query(
        &config,
        tab_b,
        &format!("SELECT id FROM {parent} WHERE id = 1 FOR UPDATE"),
    )
    .await
    .expect("hold row lock");
    tab_query(&config, tab_a, "BEGIN")
        .await
        .expect("writer BEGIN");
    let backend_pid = scalar(&config, tab_a, "SELECT pg_backend_pid()").await;
    tab_query(&config, tab_a, &format!("INSERT INTO {child} VALUES (1)"))
        .await
        .expect("deferred foreign key insert");
    let cancellation = CancellationToken::new();
    let runtime = tokio::runtime::Handle::current();
    let commit = enqueue_pg_tab_query(&config, "COMMIT", tab_a.0, tab_a.1, &cancellation, &runtime)
        .expect("enqueue COMMIT");
    let queued = enqueue_pg_tab_query(
        &config,
        &format!("INSERT INTO {child} VALUES (2)"),
        tab_a.0,
        tab_a.1,
        &CancellationToken::new(),
        &runtime,
    )
    .expect("enqueue dependent insert");
    wait_for_commit_lock(&config, backend_pid).await;
    cancellation.cancel();
    let result = tokio::time::timeout(OBSERVER_TIMEOUT, await_pg_tab_query(commit))
        .await
        .expect("cancelled COMMIT must respond without waiting for the lock");
    assert!(matches!(
        result,
        Err(DbError::TransactionOutcomeUnknown {
            command: "COMMIT",
            ..
        })
    ));
    assert!(matches!(
        await_pg_tab_query(queued).await,
        Err(DbError::Connection(_))
    ));
    tab_query(&config, tab_b, "ROLLBACK")
        .await
        .expect("release row lock");
    // COMMIT may have reached the server before the socket closed: its outcome is unknown.
    // The dependent, queued insert must never escape onto a new autocommit backend.
    assert_eq!(
        scalar_result(
            execute_typed(
                &config,
                &format!("SELECT count(*) FROM {child} WHERE id = 2")
            )
            .await
            .expect("observer after cancellation")
        ),
        0
    );
    assert_eq!(scalar(&config, tab_a, "SELECT 7").await, 7);
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &child).await;
    drop_test_table(&config, &parent).await;
}

#[tokio::test]
async fn pg_tab_close_during_blocked_commit_requires_outcome_check() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let suffix = Uuid::new_v4().simple();
    let parent = format!("gridix_close_parent_{suffix}");
    let child = format!("gridix_close_child_{suffix}");
    execute_typed(
        &config,
        &format!("CREATE TABLE {parent} (id INTEGER PRIMARY KEY)"),
    )
    .await
    .expect("create parent");
    execute_typed(&config, &format!(
        "CREATE TABLE {child} (id INTEGER REFERENCES {parent}(id) DEFERRABLE INITIALLY DEFERRED)"
    ))
    .await
    .expect("create child");
    execute_typed(&config, &format!("INSERT INTO {parent} VALUES (1)"))
        .await
        .expect("seed parent");
    execute_typed(&config, &format!("INSERT INTO {parent} VALUES (2)"))
        .await
        .expect("seed dependent row for queued SQL");
    let connection = ConnectionId(Uuid::new_v4());
    let writer = new_key(connection);
    let holder = new_key(connection);
    tab_query(&config, holder, "BEGIN")
        .await
        .expect("holder BEGIN");
    tab_query(
        &config,
        holder,
        &format!("SELECT id FROM {parent} WHERE id = 1 FOR UPDATE"),
    )
    .await
    .expect("hold row lock");
    tab_query(&config, writer, "BEGIN")
        .await
        .expect("writer BEGIN");
    let backend_pid = scalar(&config, writer, "SELECT pg_backend_pid()").await;
    tab_query(&config, writer, &format!("INSERT INTO {child} VALUES (1)"))
        .await
        .expect("deferred insert");
    let pending = enqueue_pg_tab_query(
        &config,
        "COMMIT",
        writer.0,
        writer.1,
        &CancellationToken::new(),
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue COMMIT");
    let queued = enqueue_pg_tab_query(
        &config,
        &format!("INSERT INTO {child} VALUES (2)"),
        writer.0,
        writer.1,
        &CancellationToken::new(),
        &tokio::runtime::Handle::current(),
    )
    .expect("enqueue dependent insert");
    wait_for_commit_lock(&config, backend_pid).await;
    close_pg_tab_session(writer.0, writer.1);
    assert!(matches!(
        tokio::time::timeout(OBSERVER_TIMEOUT, await_pg_tab_query(pending))
            .await
            .expect("closed Tab responds"),
        Err(DbError::Connection(_))
    ));
    assert!(matches!(
        await_pg_tab_query(queued).await,
        Err(DbError::Connection(_))
    ));
    tab_query(&config, holder, "ROLLBACK")
        .await
        .expect("release lock");
    // Closing a Tab during COMMIT cannot prove rollback: verify only that queued SQL
    // was discarded, regardless of whether the first insert committed.
    assert_eq!(
        scalar_result(
            execute_typed(
                &config,
                &format!("SELECT count(*) FROM {child} WHERE id = 2")
            )
            .await
            .expect("observer")
        ),
        0
    );
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &child).await;
    drop_test_table(&config, &parent).await;
}

#[tokio::test]
async fn pg_tab_cr_comment_transaction_controls_preserve_rollback_isolation() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    assert!(matches!(
        execute_typed(&config, "-- note\rBEGIN").await,
        Err(DbError::Query(_))
    ));
    tab_query(&config, tab, "-- note\rBEGIN")
        .await
        .expect("CR terminates comment before BEGIN");
    tab_query(&config, tab, &format!("INSERT INTO {table} VALUES (1)"))
        .await
        .expect("first uncommitted insert");
    tab_query(&config, tab, "-- note\rROLLBACK")
        .await
        .expect("CR terminates comment before ROLLBACK");
    tab_query(&config, tab, "BEGIN").await.expect("new BEGIN");
    tab_query(&config, tab, &format!("INSERT INTO {table} VALUES (2)"))
        .await
        .expect("second uncommitted insert");
    tab_query(&config, tab, "-- note\r\nROLLBACK")
        .await
        .expect("CRLF terminates comment before second ROLLBACK");
    assert_eq!(
        scalar_result(
            execute_typed(&config, &format!("SELECT count(*) FROM {table}"))
                .await
                .expect("observer")
        ),
        0
    );
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_tab_terminated_backend_discards_queued_sql_then_recovers() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    let table = create_test_table(&config).await;
    let connection = ConnectionId(Uuid::new_v4());
    let tab = new_key(connection);
    tab_query(&config, tab, "BEGIN").await.expect("BEGIN");
    let backend_pid = scalar(&config, tab, "SELECT pg_backend_pid()").await;
    tab_query(&config, tab, &format!("INSERT INTO {table} VALUES (1)"))
        .await
        .expect("uncommitted insert");
    execute_typed(
        &config,
        &format!("SELECT pg_terminate_backend({backend_pid})"),
    )
    .await
    .expect("terminate dedicated backend");
    let runtime = tokio::runtime::Handle::current();
    let first = enqueue_pg_tab_query(
        &config,
        &format!("INSERT INTO {table} VALUES (2)"),
        tab.0,
        tab.1,
        &CancellationToken::new(),
        &runtime,
    )
    .expect("enqueue after termination");
    let queued = enqueue_pg_tab_query(
        &config,
        &format!("INSERT INTO {table} VALUES (3)"),
        tab.0,
        tab.1,
        &CancellationToken::new(),
        &runtime,
    )
    .expect("enqueue dependent statement");
    assert!(await_pg_tab_query(first).await.is_err());
    assert!(matches!(
        await_pg_tab_query(queued).await,
        Err(DbError::Connection(_))
    ));
    assert_eq!(scalar(&config, tab, "SELECT 7").await, 7);
    assert_eq!(
        scalar_result(
            execute_typed(&config, &format!("SELECT count(*) FROM {table}"))
                .await
                .expect("observer")
        ),
        0
    );
    close_pg_connection_sessions(connection);
    drop_test_table(&config, &table).await;
}

#[tokio::test]
async fn pg_standalone_transaction_control_requires_tab_session() {
    let Some(config) = pg_config_from_env() else {
        eprintln!("SKIP: GRIDIX_TEST_PG_URL not set");
        return;
    };
    assert!(matches!(
        execute_typed(&config, "BEGIN").await,
        Err(DbError::Query(_))
    ));
    assert!(matches!(
        execute_typed(&config, "ROLLBACK").await,
        Err(DbError::Query(_))
    ));
    assert!(matches!(
        execute_typed(&config, "/* outer /* inner */ */ BEGIN").await,
        Err(DbError::Query(_))
    ));
}
