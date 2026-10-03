//! PostgreSQL 连接池验收测试（需要真实 PostgreSQL 服务端）。
//!
//! 通过 `GRIDIX_ACCEPTANCE=1` 配合 `GRIDIX_TEST_PG_URL` 启用：未设置开关时
//! 打印 SKIP 并返回，开关已设置却缺少 URL 时硬失败。需要 PG 夹具的用例只在
//! 这个文件里，因此只提供 PostgreSQL 的 integration workflow 可以单独运行它。
//!
//! 覆盖范围（池层行为，非查询语义）：
//! - 超出客户端缓存容量的并发调用者共享同一个客户端并全部完成（同一 pool key
//!   只有一个客户端，`Mutex` 串行化调用，因此这里证明的是复用与结果隔离，
//!   不是连接上限——PG 侧客户端缓存容量与淘汰由 `postgres_typed_e2e.rs` 覆盖）
//! - `clear_all` 之后客户端被回收重建（`Arc` 身份变化），查询继续可用
//! - `remove_pool` 之后客户端被回收重建（`Arc` 身份变化），查询继续可用
//!
//! PostgreSQL 在 TLS 或 SSH 隧道下的池行为仍未覆盖，记录于
//! `.claude/references/testing-guide.md`。

mod common;

use std::sync::Arc;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use common::{
    acceptance_serial_guard, assert_concurrent_queries_served, assert_select_round_trip,
    pg_acceptance_config_or_skip,
};
use gridix::core::constants;
use gridix::data::{DbError, POOL_MANAGER, PoolManager, execute_typed_cancellable};
use tokio_util::sync::CancellationToken;

/// 并发调用者数量：客户端缓存容量的倍数，远超单个客户端的串行度。
const CONCURRENT_CALLERS: usize = constants::database::pool::MAX_POSTGRES_CLIENTS * 2;
const COLD_CALLERS: usize = 6;

#[tokio::test]
async fn postgres_pool_simultaneous_cold_gets_share_one_live_backend() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = pg_acceptance_config_or_skip() else {
        return;
    };
    let manager = Arc::new(PoolManager::new());
    let start = Arc::new(tokio::sync::Barrier::new(COLD_CALLERS));
    let mut requests = Vec::new();
    for _ in 0..COLD_CALLERS {
        let manager = manager.clone();
        let config = config.clone();
        let start = start.clone();
        requests.push(tokio::spawn(async move {
            start.wait().await;
            manager
                .get_pg_client(&config)
                .await
                .expect("cold connection")
        }));
    }
    let first = requests.remove(0).await.expect("first request");
    for request in requests {
        let client = request.await.expect("concurrent request");
        assert!(
            Arc::ptr_eq(&first, &client),
            "same-key misses must share one backend"
        );
    }
    let guard = first.lock().await;
    let row = guard
        .query_one("SELECT 7::integer", &[])
        .await
        .expect("live backend");
    assert_eq!(row.get::<_, i32>(0), 7);
}

#[tokio::test]
async fn postgres_pool_lru_pressure_preserves_borrowed_backend() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = pg_acceptance_config_or_skip() else {
        return;
    };
    let manager = PoolManager::new();
    let borrowed = manager.get_pg_client(&config).await.expect("first backend");
    let guard = borrowed.lock().await;
    for index in 0..constants::database::pool::MAX_POSTGRES_CLIENTS {
        let mut pressure = config.clone();
        pressure.ssl_ca_cert = format!("gridix-pg-borrowed-{index}");
        manager
            .get_pg_client(&pressure)
            .await
            .expect("pressure backend");
    }
    let row = guard
        .query_one("SELECT 23::integer", &[])
        .await
        .expect("eviction must not terminate an in-use backend");
    assert_eq!(row.get::<_, i32>(0), 23);
    drop(guard);
    let recreated = manager
        .get_pg_client(&config)
        .await
        .expect("recreated backend");
    assert!(
        !Arc::ptr_eq(&borrowed, &recreated),
        "LRU entry should leave the cache"
    );
}

#[tokio::test]
async fn postgres_client_busy_client_does_not_block_unrelated_client_creation() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = pg_acceptance_config_or_skip() else {
        return;
    };
    POOL_MANAGER.clear_all().await;
    let client = POOL_MANAGER
        .get_pg_client(&config)
        .await
        .expect("client fixture must connect");
    let guard = client.lock().await;
    let mut waiting = Box::pin(POOL_MANAGER.get_pg_client(&config));
    assert!(matches!(
        waiting
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    let mut unrelated = config.clone();
    unrelated.database = "postgres".into();
    let other_client = tokio::time::timeout(
        Duration::from_secs(3),
        POOL_MANAGER.get_pg_client(&unrelated),
    )
    .await
    .expect("busy PostgreSQL client must not block unrelated lookup")
    .expect("unrelated PostgreSQL client must connect");
    drop(waiting);
    drop(guard);
    drop(other_client);
}

#[tokio::test]
async fn postgres_client_queued_query_cancel_returns_without_waiting_for_busy_client() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = pg_acceptance_config_or_skip() else {
        return;
    };
    POOL_MANAGER.clear_all().await;
    let client = POOL_MANAGER
        .get_pg_client(&config)
        .await
        .expect("client fixture must connect");
    let guard = client.lock().await;
    let cancellation = CancellationToken::new();
    let mut pending = Box::pin(execute_typed_cancellable(
        &config,
        "SELECT 1",
        &cancellation,
    ));
    assert!(matches!(
        pending
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Pending
    ));
    cancellation.cancel();
    let result = tokio::time::timeout(Duration::from_secs(3), pending)
        .await
        .expect("cancelled query must not wait for another query's client lock");
    assert!(matches!(result, Err(DbError::Cancelled)));
    drop(guard);
}

#[tokio::test]
async fn postgres_client_serves_concurrent_queries_and_survives_clear_all() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = pg_acceptance_config_or_skip() else {
        return;
    };
    println!("milestone=start backend=postgres concurrency={CONCURRENT_CALLERS}");

    POOL_MANAGER.clear_all().await;
    assert_concurrent_queries_served(config.clone(), CONCURRENT_CALLERS).await;

    let before = POOL_MANAGER
        .get_pg_client(&config)
        .await
        .expect("PostgreSQL client fixture must connect");
    POOL_MANAGER.clear_all().await;
    let after = POOL_MANAGER
        .get_pg_client(&config)
        .await
        .expect("PostgreSQL client must be recreated after clear_all");
    assert!(
        !Arc::ptr_eq(&before, &after),
        "clear_all must drop the cached client instead of keeping it"
    );
    assert_select_round_trip(&config, 1).await;

    println!("milestone=complete backend=postgres concurrency={CONCURRENT_CALLERS}");
}

#[tokio::test]
async fn postgres_client_recreation_after_remove_pool_keeps_queries_working() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = pg_acceptance_config_or_skip() else {
        return;
    };
    println!("milestone=start backend=postgres case=remove-pool");

    POOL_MANAGER.clear_all().await;
    assert_select_round_trip(&config, 1).await;

    let first = POOL_MANAGER
        .get_pg_client(&config)
        .await
        .expect("PostgreSQL client fixture must connect");
    POOL_MANAGER.remove_pool(&config).await;
    let recreated = POOL_MANAGER
        .get_pg_client(&config)
        .await
        .expect("PostgreSQL client must be recreated after remove_pool");
    assert!(
        !Arc::ptr_eq(&first, &recreated),
        "remove_pool must recycle the cached client instead of keeping it"
    );
    assert_select_round_trip(&config, 2).await;

    println!("milestone=remove-pool-complete backend=postgres");
}
