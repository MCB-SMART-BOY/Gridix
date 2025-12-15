//! 核心模块 - 包含配置、主题、语法高亮、历史记录、导出等核心功能

mod autocomplete;
mod config;
pub mod constants;
mod export;
mod formatter;
mod history;
mod syntax;
mod theme;

pub use autocomplete::{AutoComplete, CompletionKind};
pub use config::AppConfig;
pub use export::{
    // 导出功能（export_to_* 仅供测试使用）
    ExportFormat,
    // 导入功能
    import_csv_to_sql, import_json_to_sql,
    preview_csv, preview_json,
    CsvImportConfig, JsonImportConfig,
};
// 仅供测试使用的导出函数
#[allow(unused_imports)]
pub use export::{export_to_csv, export_to_json, export_to_sql};
// 导入格式（供测试使用）
#[allow(unused_imports)]
pub use export::ImportFormat;
pub use formatter::format_sql;
pub use history::QueryHistory;
pub use syntax::{highlight_sql, HighlightColors};
pub use theme::{ThemeManager, ThemePreset};
