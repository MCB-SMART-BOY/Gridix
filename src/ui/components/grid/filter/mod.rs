//! 筛选模块
//!
//! 提供现代数据库工具风格的筛选功能，拆分为多个子模块以提高可维护性。

mod operators;
mod condition;
mod logic;
mod ui;
mod cache;
mod quick_filter;

// 重新导出公共接口（部分为预留 API）
pub use condition::ColumnFilter;
#[allow(unused_imports)]
pub use logic::FilterLogic;
#[allow(unused_imports)]
pub use operators::FilterOperator;
pub use ui::show_filter_bar;
pub use cache::{count_search_matches, filter_rows_cached, FilterCache};
pub use quick_filter::show_quick_filter_dialog;
