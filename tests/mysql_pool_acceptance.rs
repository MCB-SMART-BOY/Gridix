//! MySQL 连接池验收测试（需要真实 MySQL 服务端）。
//!
//! 通过 `GRIDIX_ACCEPTANCE=1` 配合 `GRIDIX_TEST_MYSQL_URL` 启用：未设置开关时
//! 打印 SKIP 并返回，开关已设置却缺少 URL 时硬失败——验收运行不可能把静默跳过
//! 当成证据。需要 MySQL 夹具的用例只在这个文件里，因此只提供 MySQL 的
//! integration workflow 可以单独运行它。
//!
//! 覆盖范围（池层行为，非查询语义）：
//! - 单个受 `MYSQL_POOL_MAX_CONNECTIONS` 约束的池服务超出上限的并发查询
//!   （预热后确定性共享同一池，上限才会真正成为约束）
//! - `remove_pool` 之后池被回收并重建（`Arc` 身份变化），查询继续可用
//! - `clear_all` 之后池缓存被清空并重建，不留下僵尸状态
//!
//! 服务端取消后的连接复用由 `mysql_cancel_integration.rs` 覆盖，这里不重复。
//! MySQL 在 TLS 或 SSH 隧道下的池行为仍未覆盖，记录于
//! `.claude/references/testing-guide.md`。

mod common;

use std::sync::Arc;

use common::{
    acceptance_serial_guard, assert_concurrent_queries_served, assert_select_round_trip,
    mysql_acceptance_config_or_skip,
};
use gridix::core::constants;
use gridix::data::POOL_MANAGER;

/// 并发压力倍数：任务数为池连接上限的该倍数，超出部分由池排队复用连接。
const PRESSURE_MULTIPLIER: usize = 2;
/// 回收/清空用例中的小规模并发数。
const SMALL_CONCURRENCY: usize = 2;

#[tokio::test]
async fn mysql_pool_serves_more_concurrent_queries_than_pool_connection_limit() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = mysql_acceptance_config_or_skip() else {
        return;
    };
    let concurrency = constants::database::pool::MYSQL_POOL_MAX_CONNECTIONS * PRESSURE_MULTIPLIER;
    println!("milestone=start backend=mysql concurrency={concurrency}");

    POOL_MANAGER.clear_all().await;
    // 预热：先建好并缓存池，避免冷缓存下并发波次各自建池，使连接上限从未成为约束。
    let warmed = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool fixture must connect");
    let reused = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool fixture must be cached after warm-up");
    assert!(
        Arc::ptr_eq(&warmed.metrics(), &reused.metrics()),
        "warm-up must cache one pool so the connection limit applies to the burst"
    );

    assert_concurrent_queries_served(config, concurrency).await;

    println!("milestone=pressure-complete backend=mysql concurrency={concurrency}");
}

#[tokio::test]
async fn mysql_pool_recreation_after_remove_pool_keeps_queries_working() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = mysql_acceptance_config_or_skip() else {
        return;
    };
    println!("milestone=start backend=mysql case=remove-pool");

    POOL_MANAGER.clear_all().await;
    assert_select_round_trip(&config, 1).await;

    let first = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool fixture must connect");
    POOL_MANAGER.remove_pool(&config).await;
    let recreated = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool must be recreated after remove_pool");
    assert!(
        !Arc::ptr_eq(&first.metrics(), &recreated.metrics()),
        "remove_pool must recycle the cached pool instead of keeping it"
    );

    assert_concurrent_queries_served(config.clone(), SMALL_CONCURRENCY).await;
    POOL_MANAGER.remove_pool(&config).await;
    assert_select_round_trip(&config, 3).await;

    println!("milestone=remove-pool-complete backend=mysql");
}

#[tokio::test]
async fn mysql_pool_clear_all_leaves_no_zombie_state() {
    let _serial = acceptance_serial_guard().await;
    let Some(config) = mysql_acceptance_config_or_skip() else {
        return;
    };
    println!("milestone=start backend=mysql case=clear-all");

    POOL_MANAGER.clear_all().await;
    assert_select_round_trip(&config, 1).await;

    let before = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool fixture must connect");
    POOL_MANAGER.clear_all().await;
    let after = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool must be recreated after clear_all");
    assert!(
        !Arc::ptr_eq(&before.metrics(), &after.metrics()),
        "clear_all must drop the cached pool instead of keeping it"
    );
    assert_select_round_trip(&config, 2).await;

    println!("milestone=clear-all-complete backend=mysql");
}
