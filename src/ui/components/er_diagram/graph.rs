//! ER 图语义图层与布局策略选择。
//!
//! 这一层不直接负责绘制，只负责把表/关系转换成更稳定的语义摘要，
//! 供默认布局、工具条状态和后续渲染决策使用。

use super::state::{ERTable, Relationship, RelationshipOrigin};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ERLayoutStrategy {
    Grid,
    Relation,
    Component,
}

impl ERLayoutStrategy {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Grid => "网格布局",
            Self::Relation => "关系布局",
            Self::Component => "组件布局",
        }
    }
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
    pub strategy: ERLayoutStrategy,
}

pub fn analyze_er_graph(tables: &[ERTable], relationships: &[Relationship]) -> ERGraphSummary {
    let explicit_relationship_count = relationships
        .iter()
        .filter(|relationship| relationship.origin == RelationshipOrigin::Explicit)
        .count();
    let inferred_relationship_count = relationships
        .len()
        .saturating_sub(explicit_relationship_count);

    let table_indices: HashMap<&str, usize> = tables
        .iter()
        .enumerate()
        .map(|(index, table)| (table.name.as_str(), index))
        .collect();

    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); tables.len()];
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

        adjacency[from_index].push(to_index);
        adjacency[to_index].push(from_index);
    }

    let mut visited = vec![false; tables.len()];
    let mut component_sizes = Vec::new();

    for start in 0..tables.len() {
        if visited[start] {
            continue;
        }

        let mut stack = vec![start];
        let mut size = 0;
        while let Some(index) = stack.pop() {
            if visited[index] {
                continue;
            }
            visited[index] = true;
            size += 1;
            for &neighbor in &adjacency[index] {
                if !visited[neighbor] {
                    stack.push(neighbor);
                }
            }
        }

        component_sizes.push(size);
    }

    let isolated_table_count = adjacency
        .iter()
        .filter(|neighbors| neighbors.is_empty())
        .count();
    let component_count = component_sizes.len().max(usize::from(!tables.is_empty()));
    let largest_component_size = component_sizes.into_iter().max().unwrap_or(0);
    let strategy = select_layout_strategy(
        tables.len(),
        relationships.len(),
        component_count,
        isolated_table_count,
    );

    ERGraphSummary {
        table_count: tables.len(),
        relationship_count: relationships.len(),
        explicit_relationship_count,
        inferred_relationship_count,
        component_count,
        isolated_table_count,
        largest_component_size,
        strategy,
    }
}

fn select_layout_strategy(
    table_count: usize,
    relationship_count: usize,
    component_count: usize,
    isolated_table_count: usize,
) -> ERLayoutStrategy {
    if table_count == 0 || relationship_count == 0 {
        ERLayoutStrategy::Grid
    } else if component_count > 1 || isolated_table_count > 0 {
        ERLayoutStrategy::Component
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
    use super::{ERLayoutStrategy, analyze_er_graph, selected_neighborhood};
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
