//! ER 关系图模块
//!
//! 提供数据库表关系可视化功能：
//! - 显示表结构（列名、类型、主键、外键）
//! - 显示表之间的关系（外键连接）
//! - 支持拖动、缩放、自动布局

mod layout;
mod render;
mod state;

pub use layout::{force_directed_layout, grid_layout};
pub use render::ERDiagramResponse;
pub use state::{ERColumn, ERDiagramState, ERTable, Relationship, RelationType};
