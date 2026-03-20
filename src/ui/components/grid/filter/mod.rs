//! 筛选模块
//!
//! 提供现代数据库工具风格的筛选功能，拆分为多个子模块以提高可维护性。

mod cache;
mod condition;
mod logic;
mod operators;
mod ui;

// 重新导出公共接口
pub use cache::{FilterCache, filter_rows_cached};
pub use condition::ColumnFilter;
pub use logic::FilterLogic;
pub use operators::{FilterOperator, check_filter_match};
pub use ui::show_filter_bar;
