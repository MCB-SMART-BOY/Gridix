//! ER 关系图模块
//!
//! 提供数据库表关系可视化功能：
//! - 显示表结构（列名、类型、主键、外键）
//! - 显示表之间的关系（外键连接）
//! - 支持拖动、缩放、自动布局

mod graph;
mod layout;
mod render;
mod state;

pub use graph::{
    ERComponent, ERComponentDirection, EREdge, EREdgeStrength, ERGraph, ERGraphSummary,
    ERLayoutStrategy, ERNode, ERNodeRole, analyze_er_graph, build_er_graph,
    select_er_layout_strategy, selected_neighborhood,
};
pub use layout::{
    apply_er_layout_strategy, force_directed_layout, grid_layout, relationship_seeded_layout,
    stabilize_incremental_layout_positions,
};
pub use render::{ERDiagramResponse, calculate_table_size, calculate_table_size_for_mode};
pub use state::{
    ERCardDisplayMode, ERColumn, ERDiagramState, EREdgeDisplayMode, ERTable, GeometricDirection,
    RelationType, Relationship, RelationshipOrigin,
};
