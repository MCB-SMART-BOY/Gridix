//! 强类型领域标识符
//!
//! 所有业务实体使用 NewType 包装的 UUID 或单调序列号，
//! 禁止使用显示名称（name/title/index）作为 HashMap key。

use std::num::NonZeroU64;
use uuid::Uuid;

/// 数据库连接标识符
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ConnectionId(pub Uuid);

/// 文档标识符（SQL 文档、表文档、ER 文档）
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct DocumentId(pub Uuid);

/// Workbench 中的 Surface 标识符
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct SurfaceId(pub Uuid);

/// 查询执行标识符
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct ExecutionId(pub Uuid);

/// 表视图标识符（用于 grid workspace）
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct TableViewId(pub Uuid);

/// 数据传输任务标识符
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct TransferId(pub Uuid);

/// 异步任务标识符（单调递增）
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct TaskId(pub NonZeroU64);

/// Schema 版本号（用于元数据过期保护）
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub struct SchemaRevision(pub u64);

// ── Default impls ──

impl Default for ConnectionId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DocumentId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SurfaceId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for ExecutionId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for TableViewId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

impl TableViewId {
    /// 从完整的 table scope 生成确定性 TableViewId（UUID v5 SHA-1）。
    /// 相同 (connection_id, database, schema, table_name) 始终产生相同 ID，
    /// 避免跨 database/schema 的同名表冲突。
    pub fn from_components(
        connection_id: ConnectionId,
        database: &str,
        schema: &str,
        table_name: &str,
    ) -> Self {
        Self(Uuid::new_v5(
            &Uuid::NAMESPACE_OID,
            format!("{}:{}:{}:{}", connection_id.0, database, schema, table_name).as_bytes(),
        ))
    }
}

impl Default for TransferId {
    fn default() -> Self {
        Self(Uuid::new_v4())
    }
}

// ── From<Uuid> impls ──

impl From<Uuid> for ConnectionId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<Uuid> for DocumentId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<Uuid> for SurfaceId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<Uuid> for ExecutionId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<Uuid> for TableViewId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}

impl From<Uuid> for TransferId {
    fn from(u: Uuid) -> Self {
        Self(u)
    }
}
