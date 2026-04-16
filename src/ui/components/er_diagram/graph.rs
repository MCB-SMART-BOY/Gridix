//! ER 图语义图层与布局策略选择。
//!
//! 这一层不直接负责绘制，只负责把表/关系转换成更稳定的语义图，
//! 供默认布局、工具条状态和后续渲染决策使用。

use super::state::{ERTable, Relationship, RelationshipOrigin};
use egui::Vec2;
use std::collections::{HashMap, HashSet, VecDeque};

const DEFAULT_NODE_WIDTH: f32 = 180.0;
const DEFAULT_NODE_HEIGHT: f32 = 120.0;
const DENSE_COMPONENT_THRESHOLD: f32 = 0.6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ERLayoutStrategy {
    Grid,
    Relation,
    Component,
    DenseGraph,
    StableIncremental,
}

impl ERLayoutStrategy {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Grid => "网格布局",
            Self::Relation => "关系布局",
            Self::Component => "组件布局",
            Self::DenseGraph => "高密度布局",
            Self::StableIncremental => "稳定增量布局",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ERNodeRole {
    Root,
    Leaf,
    Bridge,
    Hub,
    Isolated,
    Regular,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EREdgeStrength {
    Strong,
    Medium,
    Weak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ERComponentDirection {
    TopDown,
    LeftRight,
    Mixed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ERNode {
    pub table_name: String,
    pub table_index: usize,
    pub size_hint: Vec2,
    pub column_count: usize,
    pub pk_count: usize,
    pub fk_count: usize,
    pub in_degree: usize,
    pub out_degree: usize,
    pub component_id: usize,
    pub layer_hint: Option<usize>,
    pub role: ERNodeRole,
    pub is_isolated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EREdge {
    pub from_index: usize,
    pub to_index: usize,
    pub source: RelationshipOrigin,
    pub strength: EREdgeStrength,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ERComponent {
    pub component_id: usize,
    pub node_indices: Vec<usize>,
    pub edge_indices: Vec<usize>,
    pub isolated_count: usize,
    pub estimated_density: f32,
    pub dominant_direction: Option<ERComponentDirection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ERGraphSummary {
    pub table_count: usize,
    pub relationship_count: usize,
    pub explicit_relationship_count: usize,
    pub inferred_relationship_count: usize,
    pub component_count: usize,
    pub isolated_table_count: usize,
    pub largest_component_size: usize,
    pub bridge_table_count: usize,
    pub dense_component_count: usize,
    pub strategy: ERLayoutStrategy,
    pub dominant_strategy_hint: ERLayoutStrategy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ERGraph {
    pub nodes: Vec<ERNode>,
    pub edges: Vec<EREdge>,
    pub components: Vec<ERComponent>,
    pub summary: ERGraphSummary,
}

pub fn build_er_graph(tables: &[ERTable], relationships: &[Relationship]) -> ERGraph {
    let table_indices: HashMap<&str, usize> = tables
        .iter()
        .enumerate()
        .map(|(index, table)| (table.name.as_str(), index))
        .collect();

    let mut nodes: Vec<ERNode> = tables
        .iter()
        .enumerate()
        .map(|(index, table)| {
            let size_hint = Vec2::new(
                if table.size.x > 0.0 {
                    table.size.x
                } else {
                    DEFAULT_NODE_WIDTH
                },
                if table.size.y > 0.0 {
                    table.size.y
                } else {
                    DEFAULT_NODE_HEIGHT
                },
            );
            let pk_count = table
                .columns
                .iter()
                .filter(|column| column.is_primary_key)
                .count();
            let fk_count = table
                .columns
                .iter()
                .filter(|column| column.is_foreign_key)
                .count();
            ERNode {
                table_name: table.name.clone(),
                table_index: index,
                size_hint,
                column_count: table.columns.len(),
                pk_count,
                fk_count,
                in_degree: 0,
                out_degree: 0,
                component_id: index,
                layer_hint: None,
                role: ERNodeRole::Regular,
                is_isolated: false,
            }
        })
        .collect();

    let mut edges = Vec::new();
    let mut undirected_adjacency: Vec<Vec<usize>> = vec![Vec::new(); tables.len()];
    let mut parent_to_children: Vec<Vec<usize>> = vec![Vec::new(); tables.len()];

    let explicit_relationship_count = relationships
        .iter()
        .filter(|relationship| relationship.origin == RelationshipOrigin::Explicit)
        .count();
    let inferred_relationship_count = relationships
        .len()
        .saturating_sub(explicit_relationship_count);

    for relationship in relationships {
        let Some(&from_index) = table_indices.get(relationship.from_table.as_str()) else {
            continue;
        };
        let Some(&to_index) = table_indices.get(relationship.to_table.as_str()) else {
            continue;
        };
        if from_index == to_index {
            continue;
        }

        let strength = match relationship.origin {
            RelationshipOrigin::Explicit => EREdgeStrength::Strong,
            RelationshipOrigin::Inferred => EREdgeStrength::Medium,
        };

        nodes[from_index].out_degree += 1;
        nodes[to_index].in_degree += 1;

        undirected_adjacency[from_index].push(to_index);
        undirected_adjacency[to_index].push(from_index);
        parent_to_children[to_index].push(from_index);

        edges.push(EREdge {
            from_index,
            to_index,
            source: relationship.origin,
            strength,
        });
    }

    let mut component_assignments = vec![usize::MAX; tables.len()];
    let mut component_node_indices: Vec<Vec<usize>> = Vec::new();

    for start in 0..tables.len() {
        if component_assignments[start] != usize::MAX {
            continue;
        }

        let component_id = component_node_indices.len();
        let mut stack = vec![start];
        let mut nodes_in_component = Vec::new();

        while let Some(index) = stack.pop() {
            if component_assignments[index] != usize::MAX {
                continue;
            }
            component_assignments[index] = component_id;
            nodes_in_component.push(index);
            for &neighbor in &undirected_adjacency[index] {
                if component_assignments[neighbor] == usize::MAX {
                    stack.push(neighbor);
                }
            }
        }

        component_node_indices.push(nodes_in_component);
    }

    for (index, node) in nodes.iter_mut().enumerate() {
        let component_id = component_assignments[index];
        node.component_id = component_id;
        node.is_isolated = undirected_adjacency[index].is_empty();
        node.role = classify_node_role(node.in_degree, node.out_degree, node.is_isolated);
    }

    assign_layer_hints(&mut nodes, &component_node_indices, &parent_to_children);

    let mut component_edge_indices: Vec<Vec<usize>> =
        vec![Vec::new(); component_node_indices.len()];
    for (edge_index, edge) in edges.iter().enumerate() {
        let component_id = nodes[edge.from_index].component_id;
        component_edge_indices[component_id].push(edge_index);
    }

    let components: Vec<ERComponent> = component_node_indices
        .into_iter()
        .enumerate()
        .map(|(component_id, node_indices)| {
            let edge_indices = component_edge_indices[component_id].clone();
            let isolated_count = node_indices
                .iter()
                .filter(|&&index| nodes[index].is_isolated)
                .count();
            let node_count = node_indices.len();
            let possible_edges = node_count.saturating_mul(node_count.saturating_sub(1)) / 2;
            let estimated_density = if possible_edges == 0 {
                0.0
            } else {
                edge_indices.len() as f32 / possible_edges as f32
            };
            let dominant_direction = if edge_indices.is_empty() {
                None
            } else if node_count <= 2 {
                Some(ERComponentDirection::LeftRight)
            } else if estimated_density >= DENSE_COMPONENT_THRESHOLD {
                Some(ERComponentDirection::Mixed)
            } else {
                Some(ERComponentDirection::TopDown)
            };

            ERComponent {
                component_id,
                node_indices,
                edge_indices,
                isolated_count,
                estimated_density,
                dominant_direction,
            }
        })
        .collect();

    let isolated_table_count = nodes.iter().filter(|node| node.is_isolated).count();
    let component_count = components.len().max(usize::from(!tables.is_empty()));
    let largest_component_size = components
        .iter()
        .map(|component| component.node_indices.len())
        .max()
        .unwrap_or(0);
    let bridge_table_count = nodes
        .iter()
        .filter(|node| matches!(node.role, ERNodeRole::Bridge | ERNodeRole::Hub))
        .count();
    let dense_component_count = components
        .iter()
        .filter(|component| {
            component.node_indices.len() > 2
                && component.estimated_density >= DENSE_COMPONENT_THRESHOLD
        })
        .count();
    let mut graph = ERGraph {
        nodes,
        edges,
        components,
        summary: ERGraphSummary {
            table_count: tables.len(),
            relationship_count: relationships.len(),
            explicit_relationship_count,
            inferred_relationship_count,
            component_count,
            isolated_table_count,
            largest_component_size,
            bridge_table_count,
            dense_component_count,
            strategy: ERLayoutStrategy::Grid,
            dominant_strategy_hint: ERLayoutStrategy::Grid,
        },
    };
    let strategy = select_er_layout_strategy(&graph);
    graph.summary.strategy = strategy;
    graph.summary.dominant_strategy_hint = strategy;

    graph
}

pub fn analyze_er_graph(tables: &[ERTable], relationships: &[Relationship]) -> ERGraphSummary {
    build_er_graph(tables, relationships).summary
}

pub fn select_er_layout_strategy(graph: &ERGraph) -> ERLayoutStrategy {
    select_layout_strategy_from_summary(&graph.summary)
}

fn classify_node_role(in_degree: usize, out_degree: usize, is_isolated: bool) -> ERNodeRole {
    if is_isolated {
        ERNodeRole::Isolated
    } else if out_degree == 0 && in_degree > 0 {
        ERNodeRole::Root
    } else if in_degree == 0 && out_degree > 0 {
        ERNodeRole::Leaf
    } else if in_degree > 0 && out_degree > 0 {
        if in_degree + out_degree >= 4 {
            ERNodeRole::Hub
        } else {
            ERNodeRole::Bridge
        }
    } else {
        ERNodeRole::Regular
    }
}

fn assign_layer_hints(
    nodes: &mut [ERNode],
    component_node_indices: &[Vec<usize>],
    parent_to_children: &[Vec<usize>],
) {
    for node_indices in component_node_indices {
        if node_indices.is_empty() {
            continue;
        }

        let mut roots: Vec<usize> = node_indices
            .iter()
            .copied()
            .filter(|&index| matches!(nodes[index].role, ERNodeRole::Root))
            .collect();
        if roots.is_empty()
            && let Some(index) = node_indices
                .iter()
                .copied()
                .max_by_key(|&index| nodes[index].in_degree)
        {
            roots.push(index);
        }

        let component_set: HashSet<usize> = node_indices.iter().copied().collect();
        let mut queue: VecDeque<(usize, usize)> =
            roots.iter().copied().map(|index| (index, 0usize)).collect();
        let mut seen = HashSet::new();

        while let Some((index, layer)) = queue.pop_front() {
            if !seen.insert(index) {
                continue;
            }
            nodes[index].layer_hint = Some(layer);

            for &child in &parent_to_children[index] {
                if component_set.contains(&child) {
                    queue.push_back((child, layer + 1));
                }
            }
        }
    }
}

fn select_layout_strategy_from_summary(summary: &ERGraphSummary) -> ERLayoutStrategy {
    if summary.table_count == 0 || summary.relationship_count == 0 {
        ERLayoutStrategy::Grid
    } else if summary.component_count > 1 || summary.isolated_table_count > 0 {
        ERLayoutStrategy::Component
    } else if summary.dense_component_count > 0 && summary.relationship_count >= summary.table_count
    {
        ERLayoutStrategy::DenseGraph
    } else {
        ERLayoutStrategy::Relation
    }
}

pub fn selected_neighborhood(table_name: &str, relationships: &[Relationship]) -> HashSet<String> {
    let mut names = HashSet::from([table_name.to_owned()]);
    for relationship in relationships {
        if relationship.from_table == table_name {
            names.insert(relationship.to_table.clone());
        }
        if relationship.to_table == table_name {
            names.insert(relationship.from_table.clone());
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::{
        EREdgeStrength, ERLayoutStrategy, ERNodeRole, analyze_er_graph, build_er_graph,
        select_er_layout_strategy, selected_neighborhood,
    };
    use crate::ui::{ERTable, RelationType, Relationship, RelationshipOrigin};

    fn table(name: &str) -> ERTable {
        ERTable::new(name.to_string())
    }

    fn relationship(from: &str, to: &str, origin: RelationshipOrigin) -> Relationship {
        Relationship {
            from_table: from.to_string(),
            from_column: "from_id".to_string(),
            to_table: to.to_string(),
            to_column: "id".to_string(),
            relation_type: RelationType::OneToMany,
            origin,
        }
    }

    #[test]
    fn build_er_graph_marks_isolated_tables_and_component_ids() {
        let graph = build_er_graph(
            &[table("customers"), table("orders"), table("event_logs")],
            &[relationship(
                "orders",
                "customers",
                RelationshipOrigin::Explicit,
            )],
        );

        let customers = graph
            .nodes
            .iter()
            .find(|node| node.table_name == "customers")
            .unwrap();
        let orders = graph
            .nodes
            .iter()
            .find(|node| node.table_name == "orders")
            .unwrap();
        let logs = graph
            .nodes
            .iter()
            .find(|node| node.table_name == "event_logs")
            .unwrap();

        assert_eq!(customers.component_id, orders.component_id);
        assert_ne!(customers.component_id, logs.component_id);
        assert!(logs.is_isolated);
        assert_eq!(logs.role, ERNodeRole::Isolated);
    }

    #[test]
    fn build_er_graph_assigns_explicit_and_inferred_edge_strengths() {
        let graph = build_er_graph(
            &[table("customers"), table("orders"), table("payments")],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "orders", RelationshipOrigin::Inferred),
            ],
        );

        assert_eq!(graph.edges[0].strength, EREdgeStrength::Strong);
        assert_eq!(graph.edges[1].strength, EREdgeStrength::Medium);
    }

    #[test]
    fn build_er_graph_marks_bridge_candidates_from_degree_pattern() {
        let graph = build_er_graph(
            &[table("customers"), table("orders"), table("payments")],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "orders", RelationshipOrigin::Explicit),
            ],
        );

        let orders = graph
            .nodes
            .iter()
            .find(|node| node.table_name == "orders")
            .unwrap();

        assert_eq!(orders.in_degree, 1);
        assert_eq!(orders.out_degree, 1);
        assert_eq!(orders.role, ERNodeRole::Bridge);
    }

    #[test]
    fn build_er_graph_emits_relation_hint_for_single_connected_component() {
        let graph = build_er_graph(
            &[table("customers"), table("orders"), table("payments")],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "orders", RelationshipOrigin::Inferred),
            ],
        );

        assert_eq!(graph.summary.strategy, ERLayoutStrategy::Relation);
        assert_eq!(
            graph.summary.dominant_strategy_hint,
            ERLayoutStrategy::Relation
        );
        assert_eq!(graph.summary.component_count, 1);
    }

    #[test]
    fn build_er_graph_emits_component_hint_for_disconnected_graph() {
        let graph = build_er_graph(
            &[
                table("customers"),
                table("orders"),
                table("products"),
                table("suppliers"),
                table("event_logs"),
            ],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("products", "suppliers", RelationshipOrigin::Explicit),
            ],
        );

        assert_eq!(graph.summary.strategy, ERLayoutStrategy::Component);
        assert_eq!(graph.summary.component_count, 3);
        assert_eq!(graph.summary.isolated_table_count, 1);
    }

    #[test]
    fn analyze_er_graph_remains_backward_compatible_with_existing_summary_consumers() {
        let relationships = vec![
            relationship("orders", "customers", RelationshipOrigin::Explicit),
            relationship("payments", "orders", RelationshipOrigin::Inferred),
        ];
        let tables = vec![table("customers"), table("orders"), table("payments")];
        let graph = build_er_graph(&tables, &relationships);
        let summary = analyze_er_graph(&tables, &relationships);

        assert_eq!(summary, graph.summary);
        assert_eq!(summary.explicit_relationship_count, 1);
        assert_eq!(summary.inferred_relationship_count, 1);
        assert_eq!(summary.largest_component_size, 3);
    }

    #[test]
    fn select_er_layout_strategy_uses_grid_for_relationship_free_graph() {
        let graph = build_er_graph(&[table("a"), table("b")], &[]);

        assert_eq!(select_er_layout_strategy(&graph), ERLayoutStrategy::Grid);
    }

    #[test]
    fn select_er_layout_strategy_uses_component_for_disconnected_graph() {
        let graph = build_er_graph(
            &[
                table("customers"),
                table("orders"),
                table("products"),
                table("suppliers"),
            ],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("products", "suppliers", RelationshipOrigin::Explicit),
            ],
        );

        assert_eq!(
            select_er_layout_strategy(&graph),
            ERLayoutStrategy::Component
        );
    }

    #[test]
    fn select_er_layout_strategy_uses_relation_for_single_connected_graph() {
        let graph = build_er_graph(
            &[table("customers"), table("orders"), table("payments")],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "orders", RelationshipOrigin::Explicit),
            ],
        );

        assert_eq!(
            select_er_layout_strategy(&graph),
            ERLayoutStrategy::Relation
        );
    }

    #[test]
    fn select_er_layout_strategy_uses_dense_graph_for_high_density_connected_graph() {
        let graph = build_er_graph(
            &[
                table("customers"),
                table("orders"),
                table("payments"),
                table("products"),
            ],
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "customers", RelationshipOrigin::Explicit),
                relationship("products", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "orders", RelationshipOrigin::Explicit),
                relationship("products", "orders", RelationshipOrigin::Explicit),
            ],
        );

        assert_eq!(graph.summary.dense_component_count, 1);
        assert_eq!(
            select_er_layout_strategy(&graph),
            ERLayoutStrategy::DenseGraph
        );
    }

    #[test]
    fn analyze_er_graph_uses_grid_without_relationships() {
        let summary = analyze_er_graph(&[table("a"), table("b")], &[]);

        assert_eq!(summary.strategy, ERLayoutStrategy::Grid);
        assert_eq!(summary.component_count, 2);
        assert_eq!(summary.isolated_table_count, 2);
    }

    #[test]
    fn analyze_er_graph_prefers_relation_for_single_connected_component() {
        let relationships = vec![
            relationship("orders", "customers", RelationshipOrigin::Explicit),
            relationship("payments", "orders", RelationshipOrigin::Inferred),
        ];
        let summary = analyze_er_graph(
            &[table("customers"), table("orders"), table("payments")],
            &relationships,
        );

        assert_eq!(summary.strategy, ERLayoutStrategy::Relation);
        assert_eq!(summary.explicit_relationship_count, 1);
        assert_eq!(summary.inferred_relationship_count, 1);
        assert_eq!(summary.component_count, 1);
        assert_eq!(summary.largest_component_size, 3);
    }

    #[test]
    fn analyze_er_graph_prefers_dense_graph_for_high_density_connected_component() {
        let relationships = vec![
            relationship("orders", "customers", RelationshipOrigin::Explicit),
            relationship("payments", "customers", RelationshipOrigin::Explicit),
            relationship("products", "customers", RelationshipOrigin::Explicit),
            relationship("payments", "orders", RelationshipOrigin::Explicit),
            relationship("products", "orders", RelationshipOrigin::Explicit),
        ];
        let summary = analyze_er_graph(
            &[
                table("customers"),
                table("orders"),
                table("payments"),
                table("products"),
            ],
            &relationships,
        );

        assert_eq!(summary.strategy, ERLayoutStrategy::DenseGraph);
        assert_eq!(summary.dense_component_count, 1);
        assert_eq!(summary.component_count, 1);
    }

    #[test]
    fn analyze_er_graph_prefers_component_when_graph_is_disconnected() {
        let relationships = vec![
            relationship("orders", "customers", RelationshipOrigin::Explicit),
            relationship("products", "suppliers", RelationshipOrigin::Explicit),
        ];
        let summary = analyze_er_graph(
            &[
                table("customers"),
                table("orders"),
                table("products"),
                table("suppliers"),
                table("event_logs"),
            ],
            &relationships,
        );

        assert_eq!(summary.strategy, ERLayoutStrategy::Component);
        assert_eq!(summary.component_count, 3);
        assert_eq!(summary.isolated_table_count, 1);
    }

    #[test]
    fn selected_neighborhood_collects_connected_table_names() {
        let neighborhood = selected_neighborhood(
            "orders",
            &[
                relationship("orders", "customers", RelationshipOrigin::Explicit),
                relationship("payments", "orders", RelationshipOrigin::Inferred),
                relationship("products", "suppliers", RelationshipOrigin::Explicit),
            ],
        );

        assert!(neighborhood.contains("orders"));
        assert!(neighborhood.contains("customers"));
        assert!(neighborhood.contains("payments"));
        assert!(!neighborhood.contains("suppliers"));
    }
}
