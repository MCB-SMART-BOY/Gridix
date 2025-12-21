//! UI 组件
//!
//! 包含所有可重用的 UI 组件

pub mod er_diagram;
mod grid;
mod notifications;
mod progress_indicator;
mod query_tabs;
mod sql_editor;
mod toolbar;
mod welcome;

// 工具栏
pub use toolbar::{Toolbar, ToolbarActions, ToolbarFocusTransfer};

// SQL 编辑器
pub use sql_editor::{EditorMode, SqlEditor, SqlEditorActions};

// 数据表格（Helix 风格）
pub use grid::{
    check_filter_match, escape_identifier, escape_value,
    filter_rows_cached, quote_identifier, ColumnFilter, DataGrid,
    DataGridState, FilterCache, FilterLogic, FilterOperator, FocusTransfer,
};

// 欢迎页面
pub use welcome::Welcome;

// 多 Tab 查询窗口
pub use query_tabs::{QueryTab, QueryTabBar, QueryTabManager, TabBarActions, TabBarFocusTransfer};

// ER 关系图
#[allow(unused_imports)] // 公开 API
pub use er_diagram::{
    ERColumn, ERDiagramResponse, ERDiagramState, ERTable, Relationship, RelationType,
    calculate_table_size, force_directed_layout, grid_layout,
};

// 通知组件
pub use notifications::NotificationToast;

// 进度指示器
pub use progress_indicator::ProgressIndicator;
