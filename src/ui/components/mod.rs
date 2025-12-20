//! UI 组件
//!
//! 包含所有可重用的 UI 组件

pub mod er_diagram;
mod grid;
mod notifications;
mod progress_indicator;
mod query_tabs;
mod search_bar;
mod sql_editor;
mod toolbar;
mod welcome;

// 工具栏
pub use toolbar::{Toolbar, ToolbarActions};

// SQL 编辑器
pub use sql_editor::{SqlEditor, SqlEditorActions};

// 搜索栏
pub use search_bar::SearchBar;

// 数据表格（Helix 风格）
pub use grid::{
    check_filter_match, count_search_matches, escape_identifier, escape_value,
    filter_rows_cached, parse_quick_filter, quote_identifier, ColumnFilter, DataGrid,
    DataGridState, FilterCache, FilterLogic, FilterOperator, FocusTransfer,
};

// 欢迎页面
pub use welcome::Welcome;

// 多 Tab 查询窗口
pub use query_tabs::{QueryTab, QueryTabBar, QueryTabManager};

// ER 关系图
#[allow(unused_imports)] // 公开 API
pub use er_diagram::{
    ERColumn, ERDiagramResponse, ERDiagramState, ERTable, Relationship, RelationType,
    force_directed_layout, grid_layout,
};

// 通知组件
pub use notifications::NotificationToast;

// 进度指示器
pub use progress_indicator::ProgressIndicator;
