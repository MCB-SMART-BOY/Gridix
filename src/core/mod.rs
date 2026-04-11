//! 核心模块 - 包含配置、主题、语法高亮、历史记录、导出等核心功能

mod autocomplete;
mod commands;
mod config;
pub mod constants;
mod export;
mod formatter;
mod history;
mod keybindings;
mod notification;
mod progress;
mod session;
mod syntax;
mod theme;
mod transfer;

pub use autocomplete::{AutoComplete, CompletionKind};
#[allow(unused_imports)] // 公开 API，供 UI 和 keymap 设置使用
pub use commands::{ScopedCommand, ScopedCommandBinding, scoped_command, scoped_commands};
pub use config::AppConfig;
#[allow(unused_imports)] // parse_csv_line 等供测试使用
pub use export::{
    CsvImportConfig, ExportFormat, ExportOptions, JsonImportConfig, SqlDialect, export_to_path,
    filter_result_for_export, import_csv_to_sql, import_json_to_sql, json_value_to_sql,
    parse_csv_line, preview_csv, preview_export, preview_json, sql_value_from_string,
};
pub use formatter::format_sql;
pub use history::QueryHistory;
#[allow(unused_imports)] // 公开 API，供未来使用
pub use keybindings::{
    Action, KeyBinding, KeyBindings, KeyCode, KeyModifiers, KeymapDiagnostic, KeymapDiagnosticCode,
    KeymapDiagnosticSeverity,
};
pub use notification::{Notification, NotificationLevel, NotificationManager};
#[allow(unused_imports)] // 公开 API，供外部使用
pub use progress::{ProgressManager, ProgressTask};
#[allow(unused_imports)] // 公开 API，供未来使用
pub use session::{SessionManager, SessionState, TabState, WindowState};
#[allow(unused_imports)] // 公开 API
pub use syntax::{HighlightColors, SqlHighlighter, clear_highlight_cache, highlight_sql};
pub use theme::{ThemeManager, ThemePreset};
#[allow(unused_imports)] // 公开 API，供应用层与 UI 的传输工作流使用
pub use transfer::{
    TransferDelimitedOptions, TransferDirection, TransferExecutionPayload, TransferExecutionPlan,
    TransferField, TransferFieldMapping, TransferFormat, TransferFormatOptions,
    TransferJsonOptions, TransferMapping, TransferPreview, TransferRowWindow, TransferSchema,
    TransferSession, TransferSqlOptions, plan_export_transfer, plan_import_transfer,
    plan_sql_transfer_content, preview_export_transfer, preview_import_transfer,
    preview_sql_transfer_content, write_transfer_plan,
};
