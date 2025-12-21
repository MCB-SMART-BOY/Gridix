//! 数据库模块 - 连接管理、查询执行
//!
//! 支持 SQLite、PostgreSQL、MySQL 三种数据库，使用连接池优化性能。

// ============================================================================
// 子模块
// ============================================================================

mod config;
mod connection;
mod driver;
mod error;
mod pool;
mod query;
pub mod ssh_tunnel;
mod types;

// ============================================================================
// 公开导出
// ============================================================================

// 类型
pub use types::{DatabaseType, MySqlSslMode, PostgresSslMode, QueryResult};

// 错误
pub use error::DbError;

// 配置
pub use config::ConnectionConfig;

// 连接管理
#[allow(unused_imports)] // Connection 公开 API
pub use connection::{Connection, ConnectionManager};

// 连接池
#[allow(unused_imports)] // PoolManager 公开 API
pub use pool::{PoolManager, POOL_MANAGER};

// 查询
#[allow(unused_imports)] // get_primary_key_column 预留供将来使用
pub use query::{
    connect_database, execute_query, get_foreign_keys, get_primary_key_column, get_routines,
    get_table_columns, get_tables_for_database, get_triggers, ColumnInfo, ConnectResult,
    ForeignKeyInfo, RoutineInfo, RoutineType, TriggerInfo,
};

// SSH 隧道
#[allow(unused_imports)] // SshTunnelConfig 公开 API
pub use ssh_tunnel::{SshAuthMethod, SshTunnelConfig};

// 驱动抽象
#[allow(unused_imports)] // 驱动抽象 API，供未来扩展使用
pub use driver::{
    ColumnMeta, ConnectResultType, DatabaseDriver, DriverCapabilities, DriverInfo, DriverRegistry,
    TableMeta,
};
