//! 帧效果类型定义（Layer 2 → Layer 3 通信）
//!
//! Session 处理后向 State 层发送的结构化效果。
//! 当前 `handle_messages()` 仍在 `DbManagerApp` 上；
//! 未来将由 `Session::poll_messages()` 产生这些效果。

use crate::data::QueryResult;

/// 通知级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyLevel {
    Info,
    Success,
    Warning,
    Error,
}

/// 查询结果变化效果
#[derive(Debug, Clone)]
pub struct QueryResultEffect {
    pub tab_id: String,
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub was_cancelled: bool,
}

/// 连接状态变化效果
#[derive(Debug, Clone)]
pub struct ConnectionEffect {
    pub name: String,
    pub tables_or_databases: Vec<String>,
    pub error: Option<String>,
}

/// 元数据加载完成效果
#[derive(Debug, Clone)]
pub enum MetadataEffect {
    TriggersFetched {
        connection_name: String,
        database: Option<String>,
        result: Result<Vec<crate::data::TriggerInfo>, String>,
    },
    RoutinesFetched {
        connection_name: String,
        database: Option<String>,
        result: Result<Vec<crate::data::RoutineInfo>, String>,
    },
    ForeignKeysFetched {
        result: Result<Vec<crate::data::ForeignKeyInfo>, String>,
    },
    ERTableColumnsFetched {
        table_name: String,
        result: Result<Vec<crate::data::ColumnInfo>, String>,
    },
    PrimaryKeyFetched {
        table_name: String,
        pk_column: Option<String>,
    },
}

/// 一帧中 Session 产生的所有效果
#[derive(Debug, Default)]
pub struct FrameEffects {
    pub queries: Vec<QueryResultEffect>,
    pub connections: Vec<ConnectionEffect>,
    pub metadata: Vec<MetadataEffect>,
    pub notifications: Vec<(NotifyLevel, String)>,
    pub repaint: bool,
}

impl FrameEffects {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_notification(&mut self, level: NotifyLevel, message: String) {
        self.notifications.push((level, message));
    }

    pub fn request_repaint(&mut self) {
        self.repaint = true;
    }

    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
            && self.connections.is_empty()
            && self.metadata.is_empty()
            && self.notifications.is_empty()
            && !self.repaint
    }
}
