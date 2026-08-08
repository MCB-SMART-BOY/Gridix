//! 统一异步任务注册表
//!
//! 取代 Session 中分散的 `pending_*` HashMap，提供：
//! - 按 `OperationKey` 去重（同一操作的新任务自动取消旧任务）
//! - 统一 stale-guard（通过 `is_current()` 丢弃过期回包）
//! - `CancellationToken` 生命周期管理

use crate::domain::ids::{ConnectionId, DocumentId, SurfaceId, TableViewId, TaskId, TransferId};
use std::collections::HashMap;
use std::time::Instant;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

/// 操作的唯一标识（同键互斥：新任务自动取消旧任务）
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum OperationKey {
    Connect(ConnectionId),
    SelectDatabase {
        connection: ConnectionId,
    },
    Query {
        document: DocumentId,
    },
    Metadata {
        connection: ConnectionId,
        scope: MetadataScope,
    },
    GridSave {
        table_view: TableViewId,
    },
    ErLoad {
        surface: SurfaceId,
    },
    Transfer {
        transfer: TransferId,
    },
    Catalog {
        connection: ConnectionId,
        database: String,
    },
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum MetadataScope {
    Tables,
    Triggers,
    Routines,
    ForeignKeys,
}

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum TaskKind {
    Connect,
    Query,
    Metadata,
    GridSave,
    Transfer,
    ErLoad,
    Catalog,
}

#[derive(Debug)]
pub enum TaskState {
    Running,
    Completed,
    Cancelled,
}

/// 单个异步任务条目
#[derive(Debug)]
pub struct TaskEntry {
    pub id: TaskId,
    pub key: OperationKey,
    pub kind: TaskKind,
    pub started_at: Instant,
    pub cancellation: CancellationToken,
    pub handle: JoinHandle<()>,
    pub state: TaskState,
}

/// 统一任务注册表
///
/// 使用模式：
/// 1. `register(key, kind)` → 取消同一 key 的旧任务，返回新 `TaskId`
/// 2. 异步任务完成后通过 `RuntimeEvent { task_id, key, outcome }` 回包
/// 3. `is_current(&key, task_id)` → 丢弃过期回包
#[derive(Debug, Default)]
pub struct TaskRegistry {
    tasks: HashMap<TaskId, TaskEntry>,
    latest: HashMap<OperationKey, TaskId>,
    next_id: u64,
}

impl TaskRegistry {
    /// 注册新任务。同一 key 的旧任务（如果存在）会被取消并从注册表中移除。
    pub fn register(&mut self, key: OperationKey, _kind: TaskKind) -> (TaskId, CancellationToken) {
        // 取消同一 key 的旧任务
        if let Some(old_id) = self.latest.get(&key).copied()
            && let Some(mut old_entry) = self.tasks.remove(&old_id)
        {
            Self::request_cancellation(&mut old_entry);
        }

        let id = self.next_task_id();
        let token = CancellationToken::new();

        self.latest.insert(key.clone(), id);

        (id, token)
    }

    /// 任务 spawn 完成后调用，将 JoinHandle 关联到注册表。
    pub fn attach(
        &mut self,
        id: TaskId,
        key: OperationKey,
        kind: TaskKind,
        handle: JoinHandle<()>,
        token: CancellationToken,
    ) {
        let entry = TaskEntry {
            id,
            key,
            kind,
            started_at: Instant::now(),
            cancellation: token,
            handle,
            state: TaskState::Running,
        };
        // 保留 register() 设置的 latest 映射，只更新 tasks 中的条目
        self.tasks.insert(id, entry);
    }

    /// 标记任务完成
    pub fn complete(&mut self, id: TaskId) {
        if let Some(entry) = self.tasks.get_mut(&id) {
            entry.state = TaskState::Completed;
        }
    }

    /// 取消任务
    pub fn cancel(&mut self, id: TaskId) {
        if let Some(entry) = self.tasks.get_mut(&id) {
            Self::request_cancellation(entry);
        }
    }

    /// 判断给定 task_id 是否仍为指定 key 的最新任务。
    /// 用于丢弃过期回包：`is_current` 返回 false → 忽略该回包。
    pub fn is_current(&self, key: &OperationKey, id: TaskId) -> bool {
        self.latest.get(key).copied() == Some(id)
    }

    /// 从 latest 映射中移除指定 key，使得 late-arriving completion 被 `is_current` 丢弃。
    /// 配合 `cancel()` 使用：先 cancel 再 remove_key，确保用户取消后旧回包不覆盖结果。
    pub fn remove_key(&mut self, key: &OperationKey) {
        self.latest.remove(key);
    }

    /// 获取所有活跃的操作键与对应任务 ID 的迭代器。
    /// 用于连接断开时批量取消相关任务。
    pub fn active_keys(&self) -> impl Iterator<Item = (&OperationKey, TaskId)> {
        self.latest.iter().map(|(k, v)| (k, *v))
    }

    /// 获取指定 key 的最新 TaskId（如果存在）。
    pub fn task_id_for_key(&self, key: &OperationKey) -> Option<TaskId> {
        self.latest.get(key).copied()
    }

    /// 取消并移除指定 key 的当前任务。
    /// 等价于 `cancel(task_id)` + `remove_key(key)`，确保 late-arriving completion 被丢弃。
    pub fn cancel_by_key(&mut self, key: &OperationKey) {
        if let Some(id) = self.latest.remove(key)
            && let Some(entry) = self.tasks.get_mut(&id)
        {
            Self::request_cancellation(entry);
        }
    }

    /// 移除已完成的任务（清理内存）
    pub fn cleanup(&mut self) {
        self.tasks
            .retain(|_, entry| matches!(entry.state, TaskState::Running));
        // 清理 latest 中指向已移除任务的条目
        self.latest
            .retain(|_, task_id| self.tasks.contains_key(task_id));
    }

    fn request_cancellation(entry: &mut TaskEntry) {
        entry.cancellation.cancel();
        if entry.kind != TaskKind::Query {
            entry.handle.abort();
            entry.state = TaskState::Cancelled;
        }
    }

    fn next_task_id(&mut self) -> TaskId {
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1;
        }
        TaskId(std::num::NonZeroU64::new(self.next_id).expect("task ID must be non-zero"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_task_supersedes_old_same_operation() {
        let mut registry = TaskRegistry::default();

        let key = OperationKey::GridSave {
            table_view: crate::domain::ids::TableViewId::default(),
        };

        let (task1, _token1) = registry.register(key.clone(), TaskKind::GridSave);
        let (task2, _token2) = registry.register(key.clone(), TaskKind::GridSave);

        assert!(registry.is_current(&key, task2), "task2 should be current");
        assert!(
            !registry.is_current(&key, task1),
            "task1 should be superseded"
        );
    }

    #[test]
    fn stale_completion_is_detected() {
        let mut registry = TaskRegistry::default();

        let key = OperationKey::GridSave {
            table_view: crate::domain::ids::TableViewId::default(),
        };

        let (task1, _token1) = registry.register(key.clone(), TaskKind::GridSave);
        registry.complete(task1);

        let (task2, _token2) = registry.register(key.clone(), TaskKind::GridSave);

        assert!(
            !registry.is_current(&key, task1),
            "completed task1 should not be current after task2 registered"
        );
        assert!(registry.is_current(&key, task2), "task2 should be current");
    }

    #[test]
    fn different_keys_are_independent() {
        let mut registry = TaskRegistry::default();

        let key_a = OperationKey::GridSave {
            table_view: crate::domain::ids::TableViewId::default(),
        };
        let key_b = OperationKey::GridSave {
            table_view: crate::domain::ids::TableViewId(
                crate::domain::ids::TableViewId::default().0,
            ),
        };

        let (task_a1, _t1) = registry.register(key_a.clone(), TaskKind::GridSave);
        let (task_b1, _t2) = registry.register(key_b.clone(), TaskKind::GridSave);
        let (task_a2, _t3) = registry.register(key_a.clone(), TaskKind::GridSave);

        assert!(
            registry.is_current(&key_a, task_a2),
            "task_a2 should be current for key_a"
        );
        assert!(
            !registry.is_current(&key_a, task_a1),
            "task_a1 should be superseded"
        );
        assert!(
            registry.is_current(&key_b, task_b1),
            "task_b1 should still be current for key_b"
        );
    }

    #[tokio::test]
    async fn cancel_query_retains_entry_until_completion_and_cleanup() {
        let mut registry = TaskRegistry::default();
        let key = OperationKey::Query {
            document: crate::domain::ids::DocumentId::from(uuid::Uuid::new_v4()),
        };
        let (task_id, token) = registry.register(key.clone(), TaskKind::Query);
        let (completed_tx, completed_rx) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn({
            let token = token.clone();
            async move {
                token.cancelled().await;
                let _ = completed_tx.send(());
            }
        });
        registry.attach(task_id, key.clone(), TaskKind::Query, handle, token);

        registry.cancel_by_key(&key);

        assert!(
            registry.tasks.contains_key(&task_id),
            "cooperative query cancellation must retain the task until its worker completes"
        );
        assert!(
            !registry.is_current(&key, task_id),
            "a cancelled query must reject its late completion event"
        );
        completed_rx
            .await
            .expect("query worker must observe cooperative cancellation");

        registry.complete(task_id);
        registry.cleanup();

        assert!(!registry.tasks.contains_key(&task_id));
        assert!(!registry.latest.contains_key(&key));
    }

    // ─── Sprint 1.5: Query identity 合约测试 ───

    #[test]
    fn same_document_new_query_supersedes_old() {
        let mut registry = TaskRegistry::default();
        let doc = crate::domain::ids::DocumentId::from(uuid::Uuid::new_v4());
        let key = OperationKey::Query { document: doc };

        let (task1, _t1) = registry.register(key.clone(), TaskKind::Query);
        let (task2, _t2) = registry.register(key.clone(), TaskKind::Query);

        assert!(registry.is_current(&key, task2), "task2 should be current");
        assert!(
            !registry.is_current(&key, task1),
            "task1 should be superseded by task2"
        );
    }

    #[test]
    fn different_document_queries_are_independent() {
        let mut registry = TaskRegistry::default();
        let doc_a = crate::domain::ids::DocumentId::from(uuid::Uuid::new_v4());
        let doc_b = crate::domain::ids::DocumentId::from(uuid::Uuid::new_v4());
        assert_ne!(
            doc_a, doc_b,
            "different documents should have different IDs"
        );

        let key_a = OperationKey::Query { document: doc_a };
        let key_b = OperationKey::Query { document: doc_b };

        let (task_a1, _t1) = registry.register(key_a.clone(), TaskKind::Query);
        let (task_b1, _t2) = registry.register(key_b.clone(), TaskKind::Query);
        let (task_a2, _t3) = registry.register(key_a.clone(), TaskKind::Query);

        assert!(
            registry.is_current(&key_a, task_a2),
            "task_a2 should be current for doc_a"
        );
        assert!(
            !registry.is_current(&key_a, task_a1),
            "task_a1 should be superseded"
        );
        assert!(
            registry.is_current(&key_b, task_b1),
            "task_b1 should still be current for doc_b"
        );
    }

    #[test]
    fn stale_query_completion_is_detected() {
        let mut registry = TaskRegistry::default();
        let doc = crate::domain::ids::DocumentId::from(uuid::Uuid::new_v4());
        let key = OperationKey::Query { document: doc };

        let (task1, _t1) = registry.register(key.clone(), TaskKind::Query);
        let (task2, _t2) = registry.register(key.clone(), TaskKind::Query);

        // task1 completes after task2 was registered — should NOT be considered current
        registry.complete(task1);
        assert!(
            !registry.is_current(&key, task1),
            "stale task1 should not be current after task2 registered"
        );
        assert!(
            registry.is_current(&key, task2),
            "task2 should remain current"
        );
    }
}
