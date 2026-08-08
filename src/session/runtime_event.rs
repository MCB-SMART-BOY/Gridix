//! 统一异步运行时事件（Layer 2）
//!
//! 所有异步任务完成时发送 `RuntimeEvent`，携带 `TaskId` + `OperationKey`，
//! 接收端通过 `TaskRegistry::is_current()` 丢弃过期回包。

use crate::data::ImportExecutionReport;
use crate::domain::ids::{ConnectionId, DocumentId, SurfaceId, TableViewId, TaskId, TransferId};
use crate::session::task_registry::{MetadataScope, OperationKey};

/// 异步任务产生的运行时事件
pub struct RuntimeEvent {
    pub task_id: TaskId,
    pub key: OperationKey,
    pub outcome: RuntimeOutcome,
}

/// 运行时事件的具体结果
pub enum RuntimeOutcome {
    /// 连接完成
    Connected {
        connection: ConnectionId,
        conn_name: String,
        result: Result<Vec<String>, String>,
    },

    /// 数据库选择完成
    DatabaseSelected {
        connection: ConnectionId,
        conn_name: String,
        database: String,
        result: Result<Vec<String>, String>,
    },

    /// 查询执行完成
    ExecutionFinished {
        document: DocumentId,
        sql: String,
        connection_name: String,
        tab_id: String,
        result: Result<crate::domain::execution::ExecutionOutcome, String>,
        elapsed_ms: u64,
    },

    /// 元数据加载完成
    MetadataLoaded {
        scope: MetadataScope,
        connection: ConnectionId,
        result: Result<(), String>,
    },

    /// 网格保存完成
    GridSaved {
        table_view: TableViewId,
        table: String,
        result: Result<ImportExecutionReport, String>,
        elapsed_ms: u64,
    },

    /// Schema 目录加载完成
    CatalogLoaded {
        connection_id: ConnectionId,
        database: String,
        catalog: Result<crate::domain::metadata::SchemaCatalog, String>,
        revision: crate::domain::ids::SchemaRevision,
    },

    /// 导入完成
    ImportDone {
        result: Result<ImportExecutionReport, String>,
        elapsed_ms: u64,
    },

    /// ER 图加载完成
    ErLoaded { surface: SurfaceId },

    /// 数据传输完成
    TransferFinished { transfer: TransferId },

    /// 触发器获取完成
    TriggersFetched {
        connection: ConnectionId,
        database: Option<String>,
        result: Result<Vec<crate::data::TriggerInfo>, String>,
    },

    /// 存储过程获取完成
    RoutinesFetched {
        connection: ConnectionId,
        database: Option<String>,
        result: Result<Vec<crate::data::RoutineInfo>, String>,
    },
}
