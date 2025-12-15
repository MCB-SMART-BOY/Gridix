//! UI 组件
//!
//! 包含所有可重用的 UI 组件

mod grid;
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
pub use grid::{count_search_matches, quote_identifier, ColumnFilter, DataGrid, DataGridState, FocusTransfer};

// 欢迎页面
pub use welcome::Welcome;

// 多 Tab 查询窗口
pub use query_tabs::{QueryTabBar, QueryTabManager};
