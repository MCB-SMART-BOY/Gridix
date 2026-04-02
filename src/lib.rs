//! # Rust 数据库管理器
//!
//! 一个跨平台的数据库管理 GUI 工具，支持 SQLite、PostgreSQL 和 MySQL。
//!
//! ## 功能特性
//!
//! - 多数据库支持：SQLite、PostgreSQL、MySQL
//! - 多行 SQL 编辑器，支持语法高亮
//! - SQL 自动补全和格式化
//! - 查询结果导出 (CSV/SQL/JSON)
//! - 批量数据导入 (CSV/TSV/JSON/SQL)
//! - 19 种主题预设
//! - 查询历史记录
//! - 多 Tab 查询窗口
//! - SSH 隧道支持
//!
//! ## 模块结构
//!
//! - `core`: 核心功能（配置、主题、导出、语法高亮等）
//! - `database`: 数据库连接和查询
//! - `ui`: 用户界面组件
//! - `prelude`: 常用类型预导入

// ============================================================================
// 内部模块
// ============================================================================

pub mod app;
pub mod bootstrap;
pub mod core;
pub mod database;
pub mod prelude;
pub mod ui;

// ============================================================================
// 公开 API 重导出
// ============================================================================

/// 核心功能模块的便捷访问
pub use core::{
    // 配置
    AppConfig,
    // 自动补全
    AutoComplete,
    CompletionKind,
    // 导出功能
    ExportFormat,
    // 历史记录
    QueryHistory,
    // 主题
    ThemeManager,
    ThemePreset,
    // SQL 处理
    format_sql,
    highlight_sql,
};

/// 数据库模块的便捷访问
pub use database::{
    Connection,
    ConnectionConfig,
    // 连接管理
    ConnectionManager,
    // 类型定义
    DatabaseType,
    QueryResult,
    // SSH 支持
    SshAuthMethod,
    SshTunnelConfig,
    // 异步操作
    connect_database,
    execute_query,
    get_tables_for_database,
};

/// UI 组件的便捷访问
pub use ui::{
    // 数据表格
    DataGrid,
    DataGridState,
    // 焦点管理
    FocusArea,
    SidebarSection,
};
