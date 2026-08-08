//! 数据库模块 - 连接管理、查询执行
//!
//! 支持 SQLite、PostgreSQL、MySQL 三种数据库，使用连接池优化性能。

// ============================================================================
// 子模块
// ============================================================================

mod config;
mod connection;
mod error;
mod pool;
pub(crate) mod query;
pub(crate) mod secret;
pub mod ssh_tunnel;

// ============================================================================
// 公开导出
// ============================================================================

// 类型
pub use crate::types::{DatabaseType, MySqlSslMode, PostgresSslMode};

// 错误
pub use error::DbError;

// 配置
pub use config::ConnectionConfig;
pub(crate) use config::{
    decrypt_password, delete_password_secret, load_password_secret, store_password_secret,
};

// 连接管理
#[allow(unused_imports)] // Connection 公开 API
pub use connection::{Connection, ConnectionManager};

// 连接池
#[allow(unused_imports)] // PoolManager 公开 API
pub use pool::{POOL_MANAGER, PoolManager};

// 查询
pub use query::{
    ConnectResult, ImportExecutionReport, RoutineInfo, RoutineType, TriggerInfo, apply_mutations,
    connect_database, drop_database, execute_import_batch, execute_typed,
    execute_typed_cancellable, get_routines, get_tables_for_database, get_triggers,
    infer_type_family, infer_value, load_schema_catalog,
};
pub(crate) use query::{SqlUiHints, analyze_sql_for_ui};

// SSH 隧道
#[allow(unused_imports)] // SshTunnelConfig 公开 API
pub use ssh_tunnel::{SshAuthMethod, SshTunnelConfig};
