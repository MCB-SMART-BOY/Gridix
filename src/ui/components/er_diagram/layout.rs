//! ER 图布局算法

use super::graph::ERLayoutStrategy;
use super::state::{ERTable, Relationship};
use egui::{Pos2, Rect, Vec2};
use std::collections::{HashMap, HashSet};

const DEFAULT_TABLE_WIDTH: f32 = 180.0;
const DEFAULT_TABLE_HEIGHT: f32 = 200.0;
const LAYOUT_CLEARANCE: f32 = 40.0;
const COMPONENT_SPACING_X: f32 = 160.0;
const COMPONENT_SPACING_Y: f32 = 120.0;

/// 网格布局
///
/// 将表格按网格排列，根据实际表格尺寸计算位置
pub fn grid_layout(tables: &mut [ERTable], columns: usize, spacing: Vec2) {
    if tables.is_empty() {
        return;
    }

    let columns = columns.max(1);

    // 计算每列的最大宽度和每行的最大高度
    let rows = tables.len().div_ceil(columns);
    let mut col_widths: Vec<f32> = vec![180.0; columns]; // 默认宽度
    let mut row_heights: Vec<f32> = vec![120.0; rows]; // 默认高度

    for (i, table) in tables.iter().enumerate() {
        let row = i / columns;
        let col = i % columns;

        // 使用表格的实际尺寸（如果已计算）
        let width = if table.size.x > 0.0 {
            table.size.x
        } else {
            180.0
        };
        let height = if table.size.y > 0.0 {
            table.size.y
        } else {
            120.0
        };

        col_widths[col] = col_widths[col].max(width);
        row_heights[row] = row_heights[row].max(height);
    }

    // 计算每列的 X 起始位置
    let mut col_x: Vec<f32> = vec![spacing.x; columns];
    for col in 1..columns {
        col_x[col] = col_x[col - 1] + col_widths[col - 1] + spacing.x;
    }

    // 计算每行的 Y 起始位置
    let mut row_y: Vec<f32> = vec![spacing.y; rows];
    for row in 1..rows {
        row_y[row] = row_y[row - 1] + row_heights[row - 1] + spacing.y;
    }

    // 设置表格位置
    for (i, table) in tables.iter_mut().enumerate() {
        let row = i / columns;
        let col = i % columns;

        table.position.x = col_x[col];
        table.position.y = row_y[row];
    }
}

/// 力导向布局算法
///
/// 使用简化的力导向算法来布局表格：
/// - 表格之间有斥力（避免重叠）
/// - 有关系的表格之间有引力（使相关表靠近）
pub fn force_directed_layout(
    tables: &mut [ERTable],
    relationships: &[Relationship],
    iterations: usize,
) {
    if tables.is_empty() {
        return;
    }

    // 初始化位置（如果还没有）
    let center_x = 400.0;
    let center_y = 300.0;
    let radius = 200.0;

    let table_count = tables.len();
    for (i, table) in tables.iter_mut().enumerate() {
        if table.position.x == 0.0 && table.position.y == 0.0 {
            // 初始位置按圆形分布
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (table_count as f32);
            table.position.x = center_x + radius * angle.cos();
            table.position.y = center_y + radius * angle.sin();
        }
    }

    // 力导向迭代
    let repulsion_strength = 50000.0;
    let attraction_strength = 0.01;
    let damping = 0.85;
    let min_distance = 50.0;
    let max_force = 100.0;

    for _ in 0..iterations {
        let mut forces: Vec<Vec2> = vec![Vec2::ZERO; tables.len()];

        // 计算斥力（所有表之间）
        for i in 0..tables.len() {
            for j in (i + 1)..tables.len() {
                let center_i = tables[i].center();
                let center_j = tables[j].center();
                let dx = center_j.x - center_i.x;
                let dy = center_j.y - center_i.y;
                let distance = (dx * dx + dy * dy).sqrt().max(1.0);
                let desired_clearance = desired_center_clearance(&tables[i], &tables[j]);
                let effective_distance = (distance - desired_clearance).max(min_distance);

                // 斥力与距离平方成反比
                let force = repulsion_strength / (effective_distance * effective_distance);
                let force = force.min(max_force);

                let dir_x = dx / distance;
                let dir_y = dy / distance;
                let fx = force * dir_x;
                let fy = force * dir_y;

                forces[i].x -= fx;
                forces[i].y -= fy;
                forces[j].x += fx;
                forces[j].y += fy;

                let overlap_x = overlap_distance_x(&tables[i], &tables[j], dx);
                let overlap_y = overlap_distance_y(&tables[i], &tables[j], dy);
                if overlap_x > 0.0 && overlap_y > 0.0 {
                    let overlap_force = (overlap_x.max(overlap_y) * 2.0).min(max_force);
                    forces[i].x -= overlap_force * dir_x;
                    forces[i].y -= overlap_force * dir_y;
                    forces[j].x += overlap_force * dir_x;
                    forces[j].y += overlap_force * dir_y;
                }
            }
        }

        // 计算引力（有关系的表之间）
        for rel in relationships {
            let from_idx = tables.iter().position(|t| t.name == rel.from_table);
            let to_idx = tables.iter().position(|t| t.name == rel.to_table);

            if let (Some(from), Some(to)) = (from_idx, to_idx) {
                let from_center = tables[from].center();
                let to_center = tables[to].center();
                let dx = to_center.x - from_center.x;
                let dy = to_center.y - from_center.y;
                let distance = (dx * dx + dy * dy).sqrt().max(1.0);

                // 引力与距离成正比（弹簧模型）
                let force = attraction_strength * distance;
                let force = force.min(max_force);

                let fx = force * dx / distance;
                let fy = force * dy / distance;

                forces[from].x += fx;
                forces[from].y += fy;
                forces[to].x -= fx;
                forces[to].y -= fy;
            }
        }

        // 应用力并添加阻尼
        for (i, table) in tables.iter_mut().enumerate() {
            table.position.x += forces[i].x * damping;
            table.position.y += forces[i].y * damping;

            // 确保不会跑到负坐标
            table.position.x = table.position.x.max(10.0);
            table.position.y = table.position.y.max(10.0);
        }
    }
}

/// 关系优先布局
///
/// 先按引用关系生成稳定层级种子，再用力导向做局部收敛，
/// 避免纯网格骨架让关系图在默认完成态下仍显得过于机械。
pub fn relationship_seeded_layout(
    tables: &mut [ERTable],
    relationships: &[Relationship],
    iterations: usize,
) {
    if tables.is_empty() || relationships.is_empty() {
        return;
    }

    seed_relationship_components(tables, relationships);
    force_directed_layout(tables, relationships, iterations);
}

pub fn apply_er_layout_strategy(
    tables: &mut [ERTable],
    relationships: &[Relationship],
    strategy: ERLayoutStrategy,
) {
    grid_layout(tables, 4, Vec2::new(60.0, 50.0));

    match strategy {
        ERLayoutStrategy::Grid => {}
        ERLayoutStrategy::Relation => relationship_seeded_layout(tables, relationships, 50),
        ERLayoutStrategy::Component => relationship_seeded_layout(tables, relationships, 36),
    }
}

pub fn stabilize_incremental_layout_positions(
    tables: &mut [ERTable],
    relationships: &[Relationship],
    locked_names: &HashSet<String>,
) {
    if tables.is_empty() || locked_names.is_empty() {
        return;
    }

    let mut settled_indices: Vec<usize> = tables
        .iter()
        .enumerate()
        .filter_map(|(index, table)| locked_names.contains(table.name.as_str()).then_some(index))
        .collect();

    let mut movable_indices: Vec<usize> = tables
        .iter()
        .enumerate()
        .filter_map(|(index, table)| (!locked_names.contains(table.name.as_str())).then_some(index))
        .collect();
    movable_indices.sort_by(|left, right| compare_table_anchor(&tables[*left], &tables[*right]));

    for table_idx in movable_indices {
        let anchored_position =
            relationship_neighbor_seed_position(tables, table_idx, &settled_indices, relationships)
                .unwrap_or(tables[table_idx].position);
        let adjusted = resolve_incremental_table_position(
            tables,
            table_idx,
            &settled_indices,
            anchored_position,
        );
        tables[table_idx].position = adjusted;
        settled_indices.push(table_idx);
    }
}

/// 层次布局（适合有明确层次关系的表）
///
/// 根据外键关系确定层次，被引用的表在上层
#[allow(dead_code)]
pub fn hierarchical_layout(tables: &mut [ERTable], relationships: &[Relationship], spacing: Vec2) {
    if tables.is_empty() {
        return;
    }

    let relationship_indices = relationship_index_pairs(tables, relationships);

    // 计算每个表的层级（被引用次数越多，层级越高）
    let mut levels: Vec<usize> = vec![0; tables.len()];

    for &(_, to_idx) in &relationship_indices {
        // 被引用的表层级+1
        levels[to_idx] = levels[to_idx].max(1);
    }

    // 传播层级
    for _ in 0..tables.len() {
        for &(from_idx, to_idx) in &relationship_indices {
            if levels[from_idx] <= levels[to_idx] {
                levels[from_idx] = levels[to_idx] + 1;
            }
        }
    }

    // 按层级分组
    let max_level = *levels.iter().max().unwrap_or(&0);
    let mut level_groups: Vec<Vec<usize>> = vec![Vec::new(); max_level + 1];

    for (table_idx, &level) in levels.iter().enumerate() {
        level_groups[level].push(table_idx);
    }

    // 先按名称做稳定初始化，避免继续依赖原始输入顺序。
    for group in &mut level_groups {
        group.sort_by(|left, right| tables[*left].name.cmp(&tables[*right].name));
    }

    // 三次轻量 sweep：先由上层引用关系确定下层，再由下层反推上层，
    // 最后再做一次向下 sweep，把更新后的上层顺序重新传给下层。
    reorder_levels_by_neighbor_barycenter(
        &mut level_groups,
        &levels,
        &relationship_indices,
        tables,
        NeighborDirection::Above,
    );
    reorder_levels_by_neighbor_barycenter(
        &mut level_groups,
        &levels,
        &relationship_indices,
        tables,
        NeighborDirection::Below,
    );
    reorder_levels_by_neighbor_barycenter(
        &mut level_groups,
        &levels,
        &relationship_indices,
        tables,
        NeighborDirection::Above,
    );

    let mut level_y = vec![spacing.y; level_groups.len()];
    for level in 1..level_groups.len() {
        level_y[level] =
            level_y[level - 1] + max_level_height(&level_groups[level - 1], tables) + spacing.y;
    }

    for (level, group) in level_groups.iter().enumerate() {
        let mut current_x = spacing.x;
        for &table_idx in group {
            let table = &mut tables[table_idx];
            let table_size = effective_table_size(table);

            table.position.x = current_x;
            table.position.y = level_y[level];

            current_x += table_size.x + spacing.x;
        }
    }
}

#[derive(Clone, Copy)]
enum NeighborDirection {
    Above,
    Below,
}

fn relationship_index_pairs(
    tables: &[ERTable],
    relationships: &[Relationship],
) -> Vec<(usize, usize)> {
    let table_indices: HashMap<&str, usize> = tables
        .iter()
        .enumerate()
        .map(|(index, table)| (table.name.as_str(), index))
        .collect();

    relationships
        .iter()
        .filter_map(|relationship| {
            let from_idx = table_indices.get(relationship.from_table.as_str())?;
            let to_idx = table_indices.get(relationship.to_table.as_str())?;
            Some((*from_idx, *to_idx))
        })
        .collect()
}

fn seed_relationship_components(tables: &mut [ERTable], relationships: &[Relationship]) {
    let relationship_indices = relationship_index_pairs(tables, relationships);
    if relationship_indices.is_empty() {
        return;
    }

    let components = relationship_components(tables, &relationship_indices);
    if components.is_empty() {
        return;
    }

    let spacing = Vec2::new(80.0, 80.0);
    let mut component_layouts = Vec::with_capacity(components.len());

    for component in components {
        let component_names: HashSet<&str> = component
            .iter()
            .map(|&table_idx| tables[table_idx].name.as_str())
            .collect();
        let mut component_tables: Vec<ERTable> = component
            .iter()
            .map(|&table_idx| tables[table_idx].clone())
            .collect();
        let component_relationships: Vec<Relationship> = relationships
            .iter()
            .filter(|relationship| {
                component_names.contains(relationship.from_table.as_str())
                    && component_names.contains(relationship.to_table.as_str())
            })
            .cloned()
            .collect();

        hierarchical_layout(&mut component_tables, &component_relationships, spacing);
        let bounds = component_bounds(&component_tables);
        component_layouts.push((component, component_tables, bounds));
    }

    component_layouts.sort_by(|left, right| {
        compare_component_priority(&left.0, left.2, &right.0, right.2, tables)
    });

    let target_row_width = component_layout_target_width(
        &component_layouts
            .iter()
            .map(|(_, _, bounds)| bounds)
            .collect::<Vec<_>>(),
    );
    let mut current_x = spacing.x;
    let mut current_y = spacing.y;
    let mut row_height = 0.0;

    for (component, component_tables, bounds) in component_layouts {
        let component_width = bounds.width();
        let component_height = bounds.height();

        if current_x > spacing.x && current_x + component_width > target_row_width {
            current_x = spacing.x;
            current_y += row_height + COMPONENT_SPACING_Y;
            row_height = 0.0;
        }

        let x_offset = current_x - bounds.min_x;
        let y_offset = current_y - bounds.min_y;
        for (component_idx, &table_idx) in component.iter().enumerate() {
            tables[table_idx].position = component_tables[component_idx].position;
            tables[table_idx].position.x += x_offset;
            tables[table_idx].position.y += y_offset;
        }

        current_x += component_width + COMPONENT_SPACING_X;
        row_height = row_height.max(component_height);
    }
}

fn relationship_components(
    tables: &[ERTable],
    relationship_indices: &[(usize, usize)],
) -> Vec<Vec<usize>> {
    let mut adjacency = vec![Vec::new(); tables.len()];
    for &(from_idx, to_idx) in relationship_indices {
        adjacency[from_idx].push(to_idx);
        adjacency[to_idx].push(from_idx);
    }

    let mut visited = vec![false; tables.len()];
    let mut components = Vec::new();

    for start_idx in 0..tables.len() {
        if visited[start_idx] {
            continue;
        }

        let mut stack = vec![start_idx];
        let mut component = Vec::new();
        visited[start_idx] = true;

        while let Some(table_idx) = stack.pop() {
            component.push(table_idx);
            for &neighbor_idx in &adjacency[table_idx] {
                if !visited[neighbor_idx] {
                    visited[neighbor_idx] = true;
                    stack.push(neighbor_idx);
                }
            }
        }

        component.sort_by(|left, right| tables[*left].name.cmp(&tables[*right].name));
        components.push(component);
    }

    components.sort_by(|left, right| {
        let left_name = left
            .first()
            .map(|&table_idx| tables[table_idx].name.as_str())
            .unwrap_or("");
        let right_name = right
            .first()
            .map(|&table_idx| tables[table_idx].name.as_str())
            .unwrap_or("");
        left_name.cmp(right_name)
    });

    components
}

#[derive(Clone, Copy)]
struct ComponentBounds {
    min_x: f32,
    max_x: f32,
    min_y: f32,
    max_y: f32,
}

impl ComponentBounds {
    fn width(self) -> f32 {
        self.max_x - self.min_x
    }

    fn height(self) -> f32 {
        self.max_y - self.min_y
    }
}

fn component_bounds(tables: &[ERTable]) -> ComponentBounds {
    let min_x = tables
        .iter()
        .map(|table| table.rect().left())
        .fold(f32::INFINITY, f32::min);
    let max_x = tables
        .iter()
        .map(|table| table.rect().right())
        .fold(f32::NEG_INFINITY, f32::max);
    let min_y = tables
        .iter()
        .map(|table| table.rect().top())
        .fold(f32::INFINITY, f32::min);
    let max_y = tables
        .iter()
        .map(|table| table.rect().bottom())
        .fold(f32::NEG_INFINITY, f32::max);
    ComponentBounds {
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

fn component_layout_target_width(bounds: &[&ComponentBounds]) -> f32 {
    let max_width = bounds
        .iter()
        .map(|bounds| bounds.width())
        .fold(0.0, f32::max);
    let total_area = bounds
        .iter()
        .map(|bounds| bounds.width() * bounds.height())
        .sum::<f32>();
    max_width.max(total_area.sqrt() * 1.6)
}

fn compare_component_priority(
    left_component: &[usize],
    left_bounds: ComponentBounds,
    right_component: &[usize],
    right_bounds: ComponentBounds,
    tables: &[ERTable],
) -> std::cmp::Ordering {
    let left_area = left_bounds.width() * left_bounds.height();
    let right_area = right_bounds.width() * right_bounds.height();

    right_area
        .partial_cmp(&left_area)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            right_bounds
                .width()
                .partial_cmp(&left_bounds.width())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| right_component.len().cmp(&left_component.len()))
        .then_with(|| {
            let left_name = left_component
                .first()
                .map(|&table_idx| tables[table_idx].name.as_str())
                .unwrap_or("");
            let right_name = right_component
                .first()
                .map(|&table_idx| tables[table_idx].name.as_str())
                .unwrap_or("");
            left_name.cmp(right_name)
        })
}

fn reorder_levels_by_neighbor_barycenter(
    level_groups: &mut [Vec<usize>],
    levels: &[usize],
    relationship_indices: &[(usize, usize)],
    tables: &[ERTable],
    direction: NeighborDirection,
) {
    if level_groups.len() <= 1 {
        return;
    }

    let level_indexes: Vec<usize> = match direction {
        NeighborDirection::Above => (1..level_groups.len()).collect(),
        NeighborDirection::Below => (0..level_groups.len() - 1).rev().collect(),
    };

    for level in level_indexes {
        let slot_map = slot_map_for_level_groups(level_groups, tables.len());

        level_groups[level].sort_by(|left, right| {
            let left_score = neighbor_barycenter(
                *left,
                level,
                levels,
                relationship_indices,
                &slot_map,
                direction,
            );
            let right_score = neighbor_barycenter(
                *right,
                level,
                levels,
                relationship_indices,
                &slot_map,
                direction,
            );

            compare_optional_barycenter(left_score, right_score)
                .then_with(|| tables[*left].name.cmp(&tables[*right].name))
        });
    }
}

fn slot_map_for_level_groups(
    level_groups: &[Vec<usize>],
    table_count: usize,
) -> Vec<Option<usize>> {
    let mut slot_map = vec![None; table_count];

    for group in level_groups {
        for (slot, &table_idx) in group.iter().enumerate() {
            slot_map[table_idx] = Some(slot);
        }
    }

    slot_map
}

fn effective_table_size(table: &ERTable) -> Vec2 {
    Vec2::new(
        if table.size.x > 0.0 {
            table.size.x
        } else {
            DEFAULT_TABLE_WIDTH
        },
        if table.size.y > 0.0 {
            table.size.y
        } else {
            DEFAULT_TABLE_HEIGHT
        },
    )
}

fn max_level_height(level_group: &[usize], tables: &[ERTable]) -> f32 {
    level_group
        .iter()
        .map(|&table_idx| effective_table_size(&tables[table_idx]).y)
        .fold(DEFAULT_TABLE_HEIGHT, f32::max)
}

fn desired_center_clearance(left: &ERTable, right: &ERTable) -> f32 {
    let left_size = effective_table_size(left);
    let right_size = effective_table_size(right);
    ((left_size.x + right_size.x) * 0.5).max((left_size.y + right_size.y) * 0.5) + LAYOUT_CLEARANCE
}

fn overlap_distance_x(left: &ERTable, right: &ERTable, delta_x: f32) -> f32 {
    let left_size = effective_table_size(left);
    let right_size = effective_table_size(right);
    ((left_size.x + right_size.x) * 0.5 + LAYOUT_CLEARANCE) - delta_x.abs()
}

fn overlap_distance_y(left: &ERTable, right: &ERTable, delta_y: f32) -> f32 {
    let left_size = effective_table_size(left);
    let right_size = effective_table_size(right);
    ((left_size.y + right_size.y) * 0.5 + LAYOUT_CLEARANCE) - delta_y.abs()
}

fn neighbor_barycenter(
    table_idx: usize,
    current_level: usize,
    levels: &[usize],
    relationship_indices: &[(usize, usize)],
    slot_map: &[Option<usize>],
    direction: NeighborDirection,
) -> Option<f32> {
    let mut total = 0.0;
    let mut count = 0.0;

    for &(from_idx, to_idx) in relationship_indices {
        let neighbor_idx = if from_idx == table_idx {
            Some(to_idx)
        } else if to_idx == table_idx {
            Some(from_idx)
        } else {
            None
        };

        let Some(neighbor_idx) = neighbor_idx else {
            continue;
        };

        let neighbor_level = levels[neighbor_idx];
        let matches_direction = match direction {
            NeighborDirection::Above => neighbor_level < current_level,
            NeighborDirection::Below => neighbor_level > current_level,
        };
        if !matches_direction {
            continue;
        }

        let Some(slot) = slot_map[neighbor_idx] else {
            continue;
        };

        total += slot as f32;
        count += 1.0;
    }

    (count > 0.0).then_some(total / count)
}

fn compare_optional_barycenter(left: Option<f32>, right: Option<f32>) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left
            .partial_cmp(&right)
            .unwrap_or(std::cmp::Ordering::Equal),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    }
}

fn compare_table_anchor(left: &ERTable, right: &ERTable) -> std::cmp::Ordering {
    left.position
        .y
        .partial_cmp(&right.position.y)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| {
            left.position
                .x
                .partial_cmp(&right.position.x)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .then_with(|| left.name.cmp(&right.name))
}

fn relationship_neighbor_seed_position(
    tables: &[ERTable],
    table_idx: usize,
    settled_indices: &[usize],
    relationships: &[Relationship],
) -> Option<Pos2> {
    let settled_lookup: HashMap<&str, usize> = settled_indices
        .iter()
        .map(|&index| (tables[index].name.as_str(), index))
        .collect();
    let current_name = tables[table_idx].name.as_str();
    let mut parent_indices = Vec::new();
    let mut child_indices = Vec::new();
    let mut related_indices = Vec::new();
    let mut seen = HashSet::new();

    for relationship in relationships {
        if relationship.from_table == current_name {
            let Some(&related_idx) = settled_lookup.get(relationship.to_table.as_str()) else {
                continue;
            };
            if seen.insert(related_idx) {
                parent_indices.push(related_idx);
                related_indices.push(related_idx);
            }
        } else if relationship.to_table == current_name {
            let Some(&related_idx) = settled_lookup.get(relationship.from_table.as_str()) else {
                continue;
            };
            if seen.insert(related_idx) {
                child_indices.push(related_idx);
                related_indices.push(related_idx);
            }
        }
    }

    if related_indices.is_empty() {
        return None;
    }

    let current_size = effective_table_size(&tables[table_idx]);
    if let Some(position) = relationship_directional_seed_position(
        tables,
        current_size,
        &parent_indices,
        &child_indices,
    ) {
        return Some(position);
    }

    let current_center = tables[table_idx].center();
    let mut min_left = f32::INFINITY;
    let mut max_right = f32::NEG_INFINITY;
    let mut min_top = f32::INFINITY;
    let mut max_bottom = f32::NEG_INFINITY;
    let mut center_sum = Vec2::ZERO;

    for related_idx in related_indices {
        let related = &tables[related_idx];
        let rect = related.rect();
        min_left = min_left.min(rect.left());
        max_right = max_right.max(rect.right());
        min_top = min_top.min(rect.top());
        max_bottom = max_bottom.max(rect.bottom());
        center_sum += related.center().to_vec2();
    }

    let cluster_center = Pos2::new(
        center_sum.x / seen.len() as f32,
        center_sum.y / seen.len() as f32,
    );
    let dx = current_center.x - cluster_center.x;
    let dy = current_center.y - cluster_center.y;
    let clearance = LAYOUT_CLEARANCE;
    let anchored = if dx.abs() >= dy.abs() {
        if dx >= 0.0 {
            Pos2::new(
                max_right + clearance,
                cluster_center.y - current_size.y * 0.5,
            )
        } else {
            Pos2::new(
                min_left - current_size.x - clearance,
                cluster_center.y - current_size.y * 0.5,
            )
        }
    } else if dy >= 0.0 {
        Pos2::new(
            cluster_center.x - current_size.x * 0.5,
            max_bottom + clearance,
        )
    } else {
        Pos2::new(
            cluster_center.x - current_size.x * 0.5,
            min_top - current_size.y - clearance,
        )
    };

    Some(Pos2::new(anchored.x.max(10.0), anchored.y.max(10.0)))
}

fn relationship_directional_seed_position(
    tables: &[ERTable],
    current_size: Vec2,
    parent_indices: &[usize],
    child_indices: &[usize],
) -> Option<Pos2> {
    if parent_indices.is_empty() && child_indices.is_empty() {
        return None;
    }

    let clearance = LAYOUT_CLEARANCE;
    let center_x_for = |indices: &[usize]| {
        indices
            .iter()
            .map(|&index| tables[index].center().x)
            .sum::<f32>()
            / indices.len() as f32
    };

    let anchored = if !parent_indices.is_empty() && child_indices.is_empty() {
        let anchor_x = center_x_for(parent_indices) - current_size.x * 0.5;
        let anchor_y = parent_indices
            .iter()
            .map(|&index| tables[index].rect().bottom())
            .fold(f32::NEG_INFINITY, f32::max)
            + clearance;
        Pos2::new(anchor_x, anchor_y)
    } else if parent_indices.is_empty() && !child_indices.is_empty() {
        let anchor_x = center_x_for(child_indices) - current_size.x * 0.5;
        let anchor_y = child_indices
            .iter()
            .map(|&index| tables[index].rect().top())
            .fold(f32::INFINITY, f32::min)
            - current_size.y
            - clearance;
        Pos2::new(anchor_x, anchor_y)
    } else {
        let all_indices: Vec<usize> = parent_indices
            .iter()
            .chain(child_indices.iter())
            .copied()
            .collect();
        let anchor_x = center_x_for(&all_indices) - current_size.x * 0.5;
        let parent_bottom = parent_indices
            .iter()
            .map(|&index| tables[index].rect().bottom())
            .fold(f32::NEG_INFINITY, f32::max);
        let child_top = child_indices
            .iter()
            .map(|&index| tables[index].rect().top())
            .fold(f32::INFINITY, f32::min);
        let anchor_y = ((parent_bottom + child_top) * 0.5) - current_size.y * 0.5;
        Pos2::new(anchor_x, anchor_y)
    };

    Some(Pos2::new(anchored.x.max(10.0), anchored.y.max(10.0)))
}

fn resolve_incremental_table_position(
    tables: &[ERTable],
    table_idx: usize,
    settled_indices: &[usize],
    start_position: Pos2,
) -> Pos2 {
    let size = effective_table_size(&tables[table_idx]);
    let mut position = start_position;

    for _ in 0..(settled_indices.len().max(1) * 4) {
        let current_rect = Rect::from_min_size(position, size);
        let blocker = settled_indices
            .iter()
            .filter_map(|&other_idx| {
                let other_rect = tables[other_idx]
                    .rect()
                    .expand2(Vec2::splat(LAYOUT_CLEARANCE * 0.5));
                current_rect
                    .intersects(other_rect)
                    .then_some((overlap_area(current_rect, other_rect), other_rect))
            })
            .max_by(|(left_area, _), (right_area, _)| {
                left_area
                    .partial_cmp(right_area)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, rect)| rect);

        let Some(blocker) = blocker else {
            break;
        };

        position = resolve_incremental_candidate_position(
            tables,
            settled_indices,
            position,
            start_position,
            size,
            blocker,
        );
    }

    position
}

fn resolve_incremental_candidate_position(
    tables: &[ERTable],
    settled_indices: &[usize],
    current_position: Pos2,
    anchor_position: Pos2,
    size: Vec2,
    blocker: Rect,
) -> Pos2 {
    [
        Pos2::new(blocker.right() + LAYOUT_CLEARANCE, current_position.y),
        Pos2::new(
            (blocker.left() - size.x - LAYOUT_CLEARANCE).max(10.0),
            current_position.y,
        ),
        Pos2::new(current_position.x, blocker.bottom() + LAYOUT_CLEARANCE),
        Pos2::new(
            current_position.x,
            (blocker.top() - size.y - LAYOUT_CLEARANCE).max(10.0),
        ),
    ]
    .into_iter()
    .min_by(|left, right| {
        total_overlap_area_for_candidate(tables, settled_indices, size, *left)
            .partial_cmp(&total_overlap_area_for_candidate(
                tables,
                settled_indices,
                size,
                *right,
            ))
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                candidate_vertical_band_distance(*left, anchor_position)
                    .partial_cmp(&candidate_vertical_band_distance(*right, anchor_position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                candidate_anchor_distance(*left, anchor_position)
                    .partial_cmp(&candidate_anchor_distance(*right, anchor_position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                candidate_step_distance(*left, current_position)
                    .partial_cmp(&candidate_step_distance(*right, current_position))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                left.y
                    .partial_cmp(&right.y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                left.x
                    .partial_cmp(&right.x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    })
    .unwrap_or(current_position)
}

fn candidate_anchor_distance(candidate: Pos2, anchor_position: Pos2) -> f32 {
    let dx = candidate.x - anchor_position.x;
    let dy = candidate.y - anchor_position.y;
    dx * dx + dy * dy
}

fn candidate_vertical_band_distance(candidate: Pos2, anchor_position: Pos2) -> f32 {
    (candidate.y - anchor_position.y).abs()
}

fn candidate_step_distance(candidate: Pos2, current_position: Pos2) -> f32 {
    let dx = candidate.x - current_position.x;
    let dy = candidate.y - current_position.y;
    dx * dx + dy * dy
}

fn total_overlap_area_for_candidate(
    tables: &[ERTable],
    settled_indices: &[usize],
    size: Vec2,
    candidate: Pos2,
) -> f32 {
    let candidate_rect = Rect::from_min_size(candidate, size);
    settled_indices
        .iter()
        .map(|&other_idx| {
            overlap_area(
                candidate_rect,
                tables[other_idx]
                    .rect()
                    .expand2(Vec2::splat(LAYOUT_CLEARANCE * 0.5)),
            )
        })
        .sum()
}

fn overlap_area(left: Rect, right: Rect) -> f32 {
    let overlap_x = (left.right().min(right.right()) - left.left().max(right.left())).max(0.0);
    let overlap_y = (left.bottom().min(right.bottom()) - left.top().max(right.top())).max(0.0);
    overlap_x * overlap_y
}

#[cfg(test)]
mod tests {
    use super::{
        COMPONENT_SPACING_X, COMPONENT_SPACING_Y, LAYOUT_CLEARANCE, apply_er_layout_strategy,
        hierarchical_layout, relationship_seeded_layout, stabilize_incremental_layout_positions,
    };
    use crate::ui::{ERLayoutStrategy, ERTable, RelationType, Relationship, RelationshipOrigin};
    use egui::{Vec2, pos2};
    use std::collections::HashSet;

    fn relationship(from_table: &str, to_table: &str) -> Relationship {
        Relationship {
            from_table: from_table.to_string(),
            from_column: "fk_id".to_string(),
            to_table: to_table.to_string(),
            to_column: "id".to_string(),
            relation_type: RelationType::OneToMany,
            origin: RelationshipOrigin::Explicit,
        }
    }

    #[test]
    fn apply_er_layout_strategy_keeps_grid_without_relationships() {
        let mut tables = vec![
            ERTable::new("orders".into()),
            ERTable::new("customers".into()),
        ];

        apply_er_layout_strategy(&mut tables, &[], ERLayoutStrategy::Grid);

        assert_eq!(tables[0].position, pos2(60.0, 50.0));
        assert_eq!(tables[1].position, pos2(300.0, 50.0));
    }

    #[test]
    fn hierarchical_layout_places_referenced_table_above_referencing_table() {
        let mut tables = vec![
            ERTable::new("orders".into()),
            ERTable::new("customers".into()),
        ];

        hierarchical_layout(
            &mut tables,
            &[relationship("orders", "customers")],
            Vec2::new(80.0, 80.0),
        );

        let orders = tables.iter().find(|t| t.name == "orders").unwrap();
        let customers = tables.iter().find(|t| t.name == "customers").unwrap();
        assert!(customers.position.y < orders.position.y);
    }

    #[test]
    fn relationship_seeded_layout_noops_without_relationships() {
        let mut tables = vec![
            ERTable::new("orders".into()),
            ERTable::new("customers".into()),
        ];
        let original = [pos2(0.0, 0.0), pos2(0.0, 0.0)];

        relationship_seeded_layout(&mut tables, &[], 50);

        assert_eq!(tables[0].position, original[0]);
        assert_eq!(tables[1].position, original[1]);
    }

    #[test]
    fn hierarchical_layout_orders_same_level_tables_by_referenced_parent_barycenter() {
        let mut tables = vec![
            ERTable::new("inventory".into()),
            ERTable::new("orders".into()),
            ERTable::new("customers".into()),
            ERTable::new("products".into()),
        ];

        hierarchical_layout(
            &mut tables,
            &[
                relationship("inventory", "products"),
                relationship("orders", "customers"),
            ],
            Vec2::new(80.0, 80.0),
        );

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let inventory = tables
            .iter()
            .find(|table| table.name == "inventory")
            .unwrap();
        assert!(orders.position.x < inventory.position.x);
    }

    #[test]
    fn hierarchical_layout_uses_name_order_for_same_parent_barycenter() {
        let mut tables = vec![
            ERTable::new("orders".into()),
            ERTable::new("invoices".into()),
            ERTable::new("customers".into()),
        ];

        hierarchical_layout(
            &mut tables,
            &[
                relationship("orders", "customers"),
                relationship("invoices", "customers"),
            ],
            Vec2::new(80.0, 80.0),
        );

        let invoices = tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();
        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        assert!(invoices.position.x < orders.position.x);
    }

    #[test]
    fn hierarchical_layout_uses_actual_table_sizes_to_avoid_level_overlap() {
        let mut customers = ERTable::new("customers".into());
        customers.size = Vec2::new(220.0, 320.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = Vec2::new(240.0, 280.0);
        let mut tables = vec![orders, customers];

        hierarchical_layout(
            &mut tables,
            &[relationship("orders", "customers")],
            Vec2::new(80.0, 80.0),
        );

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let customers = tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        assert!(customers.rect().bottom() + 80.0 <= orders.rect().top());
    }

    #[test]
    fn relationship_seeded_layout_separates_large_related_tables() {
        let mut customers = ERTable::new("customers".into());
        customers.size = Vec2::new(260.0, 280.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = Vec2::new(260.0, 280.0);
        let mut tables = vec![orders, customers];

        relationship_seeded_layout(&mut tables, &[relationship("orders", "customers")], 50);

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let customers = tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        assert!(!orders.rect().intersects(customers.rect()));
    }

    #[test]
    fn relationship_seeded_layout_separates_disconnected_components_horizontally() {
        let mut customers = ERTable::new("customers".into());
        customers.size = Vec2::new(260.0, 280.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = Vec2::new(260.0, 280.0);
        let mut inventory = ERTable::new("inventory".into());
        inventory.size = Vec2::new(260.0, 280.0);
        let mut products = ERTable::new("products".into());
        products.size = Vec2::new(260.0, 280.0);
        let mut tables = vec![orders, customers, inventory, products];

        relationship_seeded_layout(
            &mut tables,
            &[
                relationship("orders", "customers"),
                relationship("inventory", "products"),
            ],
            50,
        );

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let customers = tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let inventory = tables
            .iter()
            .find(|table| table.name == "inventory")
            .unwrap();
        let products = tables
            .iter()
            .find(|table| table.name == "products")
            .unwrap();

        let left_cluster_right = orders.rect().right().max(customers.rect().right());
        let right_cluster_left = inventory.rect().left().min(products.rect().left());
        assert!(left_cluster_right + COMPONENT_SPACING_X * 0.25 <= right_cluster_left);
    }

    #[test]
    fn relationship_seeded_layout_separates_isolated_table_from_related_cluster() {
        let mut customers = ERTable::new("customers".into());
        customers.size = Vec2::new(260.0, 280.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = Vec2::new(260.0, 280.0);
        let mut logs = ERTable::new("z_logs".into());
        logs.size = Vec2::new(260.0, 280.0);
        let mut tables = vec![orders, customers, logs];

        relationship_seeded_layout(&mut tables, &[relationship("orders", "customers")], 50);

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let customers = tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let logs = tables.iter().find(|table| table.name == "z_logs").unwrap();

        let related_cluster_right = orders.rect().right().max(customers.rect().right());
        assert!(related_cluster_right + COMPONENT_SPACING_X * 0.25 <= logs.rect().left());
    }

    #[test]
    fn relationship_seeded_layout_wraps_many_components_into_multiple_rows() {
        let make_table = |name: &str| {
            let mut table = ERTable::new(name.into());
            table.size = Vec2::new(280.0, 320.0);
            table
        };

        let mut tables = vec![
            make_table("alpha_events"),
            make_table("alpha_logs"),
            make_table("beta_events"),
            make_table("beta_logs"),
            make_table("gamma_events"),
            make_table("gamma_logs"),
            make_table("delta_events"),
            make_table("delta_logs"),
        ];

        relationship_seeded_layout(
            &mut tables,
            &[
                relationship("alpha_logs", "alpha_events"),
                relationship("beta_logs", "beta_events"),
                relationship("gamma_logs", "gamma_events"),
                relationship("delta_logs", "delta_events"),
            ],
            50,
        );

        let top_positions: Vec<f32> = tables.iter().map(|table| table.rect().top()).collect();
        let min_y = top_positions.iter().copied().fold(f32::INFINITY, f32::min);
        let max_y = top_positions
            .iter()
            .copied()
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(max_y - min_y >= COMPONENT_SPACING_Y * 0.5);
    }

    #[test]
    fn relationship_seeded_layout_anchors_largest_component_before_small_isolated_table() {
        let mut alpha_logs = ERTable::new("alpha_logs".into());
        alpha_logs.size = Vec2::new(240.0, 280.0);
        let mut z_customers = ERTable::new("z_customers".into());
        z_customers.size = Vec2::new(260.0, 280.0);
        let mut z_invoices = ERTable::new("z_invoices".into());
        z_invoices.size = Vec2::new(260.0, 280.0);
        let mut z_orders = ERTable::new("z_orders".into());
        z_orders.size = Vec2::new(260.0, 280.0);
        let mut tables = vec![alpha_logs, z_orders, z_customers, z_invoices];

        relationship_seeded_layout(
            &mut tables,
            &[
                relationship("z_orders", "z_customers"),
                relationship("z_invoices", "z_customers"),
            ],
            50,
        );

        let alpha_logs = tables
            .iter()
            .find(|table| table.name == "alpha_logs")
            .unwrap();
        let z_customers = tables
            .iter()
            .find(|table| table.name == "z_customers")
            .unwrap();
        let z_invoices = tables
            .iter()
            .find(|table| table.name == "z_invoices")
            .unwrap();
        let z_orders = tables
            .iter()
            .find(|table| table.name == "z_orders")
            .unwrap();

        let cluster_left = z_customers
            .rect()
            .left()
            .min(z_invoices.rect().left())
            .min(z_orders.rect().left());
        let cluster_top = z_customers
            .rect()
            .top()
            .min(z_invoices.rect().top())
            .min(z_orders.rect().top());

        assert!(
            cluster_top < alpha_logs.rect().top()
                || (cluster_top == alpha_logs.rect().top()
                    && cluster_left < alpha_logs.rect().left())
        );
    }

    #[test]
    fn stabilize_incremental_layout_positions_keeps_locked_tables_fixed_and_moves_new_table() {
        let mut customers = ERTable::new("customers".into());
        customers.position = pos2(540.0, 50.0);
        customers.size = Vec2::new(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.position = pos2(300.0, 50.0);
        orders.size = Vec2::new(180.0, 200.0);
        let mut invoices = ERTable::new("invoices".into());
        invoices.position = pos2(540.0, 50.0);
        invoices.size = Vec2::new(180.0, 200.0);
        let mut tables = vec![customers, orders, invoices];

        stabilize_incremental_layout_positions(
            &mut tables,
            &[relationship("invoices", "orders")],
            &HashSet::from(["customers".to_string(), "orders".to_string()]),
        );

        let customers = tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let invoices = tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();

        assert_eq!(customers.position, pos2(540.0, 50.0));
        assert_eq!(orders.position, pos2(300.0, 50.0));
        assert!(!invoices.rect().intersects(customers.rect()));
        assert!(!invoices.rect().intersects(orders.rect()));
    }

    #[test]
    fn stabilize_incremental_layout_positions_reanchors_related_new_table_near_settled_neighbor() {
        let mut customers = ERTable::new("customers".into());
        customers.position = pos2(900.0, 50.0);
        customers.size = Vec2::new(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.position = pos2(660.0, 50.0);
        orders.size = Vec2::new(180.0, 200.0);
        let mut invoices = ERTable::new("invoices".into());
        invoices.position = pos2(60.0, 520.0);
        invoices.size = Vec2::new(180.0, 200.0);
        let mut tables = vec![customers, orders, invoices];
        let before_distance = tables[2].center().distance(tables[1].center());

        stabilize_incremental_layout_positions(
            &mut tables,
            &[relationship("invoices", "orders")],
            &HashSet::from(["customers".to_string(), "orders".to_string()]),
        );

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let invoices = tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();
        let after_distance = invoices.center().distance(orders.center());

        assert!(after_distance < before_distance);
        assert!(!invoices.rect().intersects(orders.rect()));
        assert!(invoices.position.x > 300.0);
    }

    #[test]
    fn stabilize_incremental_layout_positions_places_referencing_new_table_below_restored_parent() {
        let mut orders = ERTable::new("orders".into());
        orders.position = pos2(660.0, 50.0);
        orders.size = Vec2::new(180.0, 200.0);
        let mut invoices = ERTable::new("invoices".into());
        invoices.position = pos2(1200.0, 50.0);
        invoices.size = Vec2::new(180.0, 200.0);
        let mut tables = vec![orders, invoices];

        stabilize_incremental_layout_positions(
            &mut tables,
            &[relationship("invoices", "orders")],
            &HashSet::from(["orders".to_string()]),
        );

        let orders = tables.iter().find(|table| table.name == "orders").unwrap();
        let invoices = tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();

        assert!(invoices.rect().top() >= orders.rect().bottom() + LAYOUT_CLEARANCE - 1.0);
        assert!(!invoices.rect().intersects(orders.rect()));
    }

    #[test]
    fn stabilize_incremental_layout_positions_keeps_bridge_table_between_restored_parent_and_child()
    {
        let mut customers = ERTable::new("customers".into());
        customers.position = pos2(660.0, 50.0);
        customers.size = Vec2::new(180.0, 200.0);
        let mut order_items = ERTable::new("order_items".into());
        order_items.position = pos2(940.0, 250.0);
        order_items.size = Vec2::new(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.position = pos2(1200.0, 50.0);
        orders.size = Vec2::new(180.0, 200.0);
        let mut tables = vec![customers, order_items, orders];

        stabilize_incremental_layout_positions(
            &mut tables,
            &[
                relationship("orders", "customers"),
                relationship("order_items", "orders"),
            ],
            &HashSet::from(["customers".to_string(), "order_items".to_string()]),
        );

        let customers = tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let order_items = tables
            .iter()
            .find(|table| table.name == "order_items")
            .unwrap();
        let orders = tables.iter().find(|table| table.name == "orders").unwrap();

        assert!(orders.center().y > customers.center().y);
        assert!(orders.center().y < order_items.center().y);
        assert!(!orders.rect().intersects(customers.rect()));
        assert!(!orders.rect().intersects(order_items.rect()));
    }
}
