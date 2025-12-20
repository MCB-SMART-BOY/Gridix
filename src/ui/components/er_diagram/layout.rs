//! ER 图布局算法

use super::state::{ERTable, Relationship};
use egui::Vec2;

/// 网格布局
/// 
/// 将表格按网格排列，根据实际表格尺寸计算位置
pub fn grid_layout(tables: &mut [ERTable], columns: usize, spacing: Vec2) {
    if tables.is_empty() {
        return;
    }
    
    let columns = columns.max(1);
    
    // 计算每列的最大宽度和每行的最大高度
    let rows = (tables.len() + columns - 1) / columns;
    let mut col_widths: Vec<f32> = vec![180.0; columns]; // 默认宽度
    let mut row_heights: Vec<f32> = vec![120.0; rows];   // 默认高度
    
    for (i, table) in tables.iter().enumerate() {
        let row = i / columns;
        let col = i % columns;
        
        // 使用表格的实际尺寸（如果已计算）
        let width = if table.size.x > 0.0 { table.size.x } else { 180.0 };
        let height = if table.size.y > 0.0 { table.size.y } else { 120.0 };
        
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
                let dx = tables[j].position.x - tables[i].position.x;
                let dy = tables[j].position.y - tables[i].position.y;
                let distance = (dx * dx + dy * dy).sqrt().max(min_distance);
                
                // 斥力与距离平方成反比
                let force = repulsion_strength / (distance * distance);
                let force = force.min(max_force);
                
                let fx = force * dx / distance;
                let fy = force * dy / distance;
                
                forces[i].x -= fx;
                forces[i].y -= fy;
                forces[j].x += fx;
                forces[j].y += fy;
            }
        }

        // 计算引力（有关系的表之间）
        for rel in relationships {
            let from_idx = tables.iter().position(|t| t.name == rel.from_table);
            let to_idx = tables.iter().position(|t| t.name == rel.to_table);
            
            if let (Some(from), Some(to)) = (from_idx, to_idx) {
                let dx = tables[to].position.x - tables[from].position.x;
                let dy = tables[to].position.y - tables[from].position.y;
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

/// 层次布局（适合有明确层次关系的表）
/// 
/// 根据外键关系确定层次，被引用的表在上层
#[allow(dead_code)]
pub fn hierarchical_layout(
    tables: &mut [ERTable],
    relationships: &[Relationship],
    spacing: Vec2,
) {
    if tables.is_empty() {
        return;
    }

    // 计算每个表的层级（被引用次数越多，层级越高）
    let mut levels: Vec<usize> = vec![0; tables.len()];
    
    for rel in relationships {
        if let Some(to_idx) = tables.iter().position(|t| t.name == rel.to_table) {
            // 被引用的表层级+1
            levels[to_idx] = levels[to_idx].max(1);
        }
    }

    // 传播层级
    for _ in 0..tables.len() {
        for rel in relationships {
            let from_idx = tables.iter().position(|t| t.name == rel.from_table);
            let to_idx = tables.iter().position(|t| t.name == rel.to_table);
            
            if let (Some(from), Some(to)) = (from_idx, to_idx) {
                if levels[from] <= levels[to] {
                    levels[from] = levels[to] + 1;
                }
            }
        }
    }

    // 按层级分组
    let max_level = *levels.iter().max().unwrap_or(&0);
    let mut level_counts: Vec<usize> = vec![0; max_level + 1];
    
    for (i, table) in tables.iter_mut().enumerate() {
        let level = levels[i];
        let count = level_counts[level];
        level_counts[level] += 1;
        
        let table_width = 180.0;
        let table_height = 200.0;
        
        table.position.x = count as f32 * (table_width + spacing.x) + spacing.x;
        table.position.y = level as f32 * (table_height + spacing.y) + spacing.y;
    }
}
