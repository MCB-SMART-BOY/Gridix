//! 集成测试：TaskRegistry 取消生命周期
//!
//! 验证 CancellationToken 触发、stale-guard 丢弃过期事件、
//! cleanup 清理已完成任务、以及 supersede 取消旧任务。

use std::time::Duration;

use gridix::domain::ids::DocumentId;
use gridix::session::task_registry::{OperationKey, TaskKind, TaskRegistry};
use tokio_util::sync::CancellationToken;

fn make_query_key() -> OperationKey {
    OperationKey::Query {
        document: DocumentId::from(uuid::Uuid::new_v4()),
    }
}

// ── Test 1: cancel_by_key fires token and discards stale events ──

#[tokio::test]
async fn cancel_by_key_fires_token() {
    let mut registry = TaskRegistry::default();
    let key = make_query_key();

    let (task_id, cancel_token) = registry.register(key.clone(), TaskKind::Query);

    // Clone before attach so we can observe cancellation externally.
    // cancel_by_key calls entry.cancellation.cancel() + entry.handle.abort().
    // The abort races with any spawned task, so we observe via is_cancelled()
    // on a cloned token instead.
    let observer = cancel_token.clone();

    let handle = tokio::spawn(async {});
    registry.attach(task_id, key.clone(), TaskKind::Query, handle, cancel_token);

    registry.cancel_by_key(&key);

    // The CancellationToken was cancelled by cancel_by_key
    assert!(
        observer.is_cancelled(),
        "cancel_by_key should cancel the CancellationToken"
    );

    // Stale guard: after cancel_by_key the key is removed from latest
    assert!(
        !registry.is_current(&key, task_id),
        "is_current should return false after cancel_by_key"
    );
}

// ── Test 1b: cancellation token is observable in tokio::select! ──

#[tokio::test]
async fn cancellation_token_signals_in_select() {
    let token = CancellationToken::new();
    let child = token.clone();

    let result = tokio::spawn(async move {
        tokio::select! {
            _ = child.cancelled() => "cancelled",
            () = tokio::time::sleep(Duration::from_secs(10)) => "timeout",
        }
    });

    tokio::time::sleep(Duration::from_millis(10)).await;
    token.cancel();

    let outcome = result.await.expect("task should complete normally");
    assert_eq!(
        outcome, "cancelled",
        "token.cancel() should unblock select!"
    );
}

// ── Test 2: stale event discarded by supersede ──

#[tokio::test]
async fn stale_event_discarded() {
    let mut registry = TaskRegistry::default();
    let key = make_query_key();

    let (task1, _t1) = registry.register(key.clone(), TaskKind::Query);
    let (task2, _t2) = registry.register(key.clone(), TaskKind::Query);

    // task2 supersedes task1 — task1 should not be current
    assert!(
        !registry.is_current(&key, task1),
        "task1 should be stale after task2 registered"
    );
    assert!(registry.is_current(&key, task2), "task2 should be current");
}

// ── Test 3: complete + cleanup removes entry ──

#[tokio::test]
async fn complete_and_cleanup() {
    let mut registry = TaskRegistry::default();
    let key = make_query_key();

    let (task_id, cancel_token) = registry.register(key.clone(), TaskKind::Query);

    let handle = tokio::spawn(async {});
    registry.attach(task_id, key.clone(), TaskKind::Query, handle, cancel_token);
    assert!(
        registry.is_current(&key, task_id),
        "task should be current after attach"
    );

    // Mark completed
    registry.complete(task_id);

    // cleanup removes non-Running tasks from tasks map,
    // and also prunes latest entries pointing to removed tasks
    registry.cleanup();

    // After cleanup, the key should no longer have a current task
    assert!(
        !registry.is_current(&key, task_id),
        "is_current should be false after complete + cleanup prunes the entry"
    );
    assert!(
        registry.task_id_for_key(&key).is_none(),
        "task_id_for_key should return None after cleanup"
    );
}

// ── Test 4: supersede cancels old task's token ──

#[tokio::test]
async fn supersede_cancels_old() {
    let mut registry = TaskRegistry::default();
    let key = make_query_key();

    // Register task1 and attach it so it lives in the tasks map
    let (task1, token1) = registry.register(key.clone(), TaskKind::Query);

    // Clone before attach for external observation
    let obs1 = token1.clone();

    let handle1 = tokio::spawn(async {});
    registry.attach(task1, key.clone(), TaskKind::Query, handle1, token1);

    // Register task2 with the same key — register() cancels old task's token
    let (task2, _token2) = registry.register(key.clone(), TaskKind::Query);

    // task1's token should have been cancelled by register()
    assert!(
        obs1.is_cancelled(),
        "register with same key should cancel old task's token"
    );

    assert!(
        !registry.is_current(&key, task1),
        "task1 should be superseded"
    );
    assert!(registry.is_current(&key, task2), "task2 should be current");
}
