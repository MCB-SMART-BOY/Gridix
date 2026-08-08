//! MySQL 服务端查询取消集成测试。

use std::time::{Duration, Instant};

use gridix::data::{
    ConnectionConfig, DatabaseType, DbError, execute_typed, execute_typed_cancellable,
};
use gridix::domain::execution::StatementOutcome;
use mysql_async::prelude::Queryable;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const OBSERVER_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);

fn parse_mysql_url(url: &str) -> Result<ConnectionConfig, &'static str> {
    let rest = url
        .strip_prefix("mysql://")
        .ok_or("MySQL URL must start with mysql://")?;
    let (user_info, rest) = rest
        .split_once('@')
        .ok_or("MySQL URL must include credentials")?;
    let (user, password) = user_info.split_once(':').unwrap_or((user_info, ""));
    let (host_port, database) = rest
        .split_once('/')
        .ok_or("MySQL URL must include database")?;
    let (host, port_str) = host_port.split_once(':').unwrap_or((host_port, "3306"));
    let port = port_str
        .parse()
        .map_err(|_| "MySQL URL port must be a u16")?;

    Ok(ConnectionConfig {
        db_type: DatabaseType::MySQL,
        host: host.to_string(),
        port,
        username: user.to_string(),
        password: password.to_string(),
        database: database.to_string(),
        ..Default::default()
    })
}

async fn marker_is_visible(
    observer: &mut mysql_async::Conn,
    marker: &str,
) -> Result<bool, mysql_async::Error> {
    let statements: Vec<Option<String>> = observer
        .query("SELECT INFO FROM information_schema.processlist WHERE INFO IS NOT NULL")
        .await?;
    Ok(statements
        .into_iter()
        .flatten()
        .any(|statement| statement.contains(marker)))
}

async fn wait_for_marker(observer: &mut mysql_async::Conn, marker: &str, should_be_visible: bool) {
    let deadline = Instant::now() + OBSERVER_TIMEOUT;
    loop {
        let visible = marker_is_visible(observer, marker)
            .await
            .expect("observer information_schema.processlist query must succeed");
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
    let url = match std::env::var("GRIDIX_TEST_MYSQL_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => {
            eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
            return;
        }
        Err(error) => panic!("GRIDIX_TEST_MYSQL_URL could not be read: {}", error),
    };
    let config = parse_mysql_url(&url).expect("GRIDIX_TEST_MYSQL_URL must be valid");
    let opts = mysql_async::Opts::from_url(&url).expect("observer MySQL URL must be valid");
    let mut observer = mysql_async::Conn::new(opts)
        .await
        .expect("observer MySQL connection must succeed");

    let marker = format!("gridix-cancel:{}", Uuid::new_v4());
    let sql = format!("SELECT SLEEP(30) /* {} */", marker);
    let cancellation = CancellationToken::new();
    let worker_config = config.clone();
    let worker_token = cancellation.clone();
    println!("marker={} milestone=start", marker);
    let worker = tokio::spawn(async move {
        execute_typed_cancellable(&worker_config, &sql, &worker_token).await
    });

    wait_for_marker(&mut observer, &marker, true).await;
    println!("marker={} milestone=observed", marker);
    cancellation.cancel();

    let result = tokio::time::timeout(OBSERVER_TIMEOUT, worker)
        .await
        .expect("cancelled MySQL query must complete before deadline")
        .expect("MySQL worker must not panic");
    assert!(matches!(result, Err(DbError::Cancelled)));
    println!("marker={} milestone=cancelled", marker);

    wait_for_marker(&mut observer, &marker, false).await;
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
    let url = match std::env::var("GRIDIX_TEST_MYSQL_URL") {
        Ok(url) => url,
        Err(std::env::VarError::NotPresent) => {
            eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
            return;
        }
        Err(error) => panic!("GRIDIX_TEST_MYSQL_URL could not be read: {}", error),
    };
    let config = parse_mysql_url(&url).expect("GRIDIX_TEST_MYSQL_URL must be valid");
    let opts = mysql_async::Opts::from_url(&url).expect("observer MySQL URL must be valid");
    let mut observer = mysql_async::Conn::new(opts)
        .await
        .expect("observer MySQL connection must succeed");

    let marker = format!("gridix-pre-cancel:{}", Uuid::new_v4());
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let result = execute_typed_cancellable(
        &config,
        &format!("SELECT SLEEP(30) /* {} */", marker),
        &cancellation,
    )
    .await;

    assert!(matches!(result, Err(DbError::Cancelled)));
    assert!(
        !marker_is_visible(&mut observer, &marker)
            .await
            .expect("observer information_schema.processlist query must succeed"),
        "pre-cancelled query must never be dispatched"
    );
}
