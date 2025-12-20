//! ER 图状态和数据结构

#![allow(dead_code)] // 公开 API

use egui::{Pos2, Vec2};

/// 关系类型
#[derive(Debug, Clone, PartialEq)]
pub enum RelationType {
    /// 一对一
    OneToOne,
    /// 一对多
    OneToMany,
    /// 多对多
    ManyToMany,
}

/// ER 图中的列信息
#[derive(Debug, Clone)]
pub struct ERColumn {
    /// 列名
    pub name: String,
    /// 数据类型
    pub data_type: String,
    /// 是否是主键
    pub is_primary_key: bool,
    /// 是否是外键
    pub is_foreign_key: bool,
    /// 是否允许 NULL
    pub nullable: bool,
    /// 默认值（如有）
    pub default_value: Option<String>,
}

/// ER 图中的表
#[derive(Debug, Clone)]
pub struct ERTable {
    /// 表名
    pub name: String,
    /// 列列表
    pub columns: Vec<ERColumn>,
    /// 在画布上的位置
    pub position: Pos2,
    /// 表格尺寸（渲染时计算）
    pub size: Vec2,
    /// 是否被选中
    pub selected: bool,
}

impl ERTable {
    /// 创建新表
    pub fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
            position: Pos2::ZERO,
            size: Vec2::ZERO,
            selected: false,
        }
    }

    /// 获取表的中心点
    pub fn center(&self) -> Pos2 {
        self.position + self.size / 2.0
    }

    /// 获取表的边界矩形
    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(self.position, self.size)
    }
}

/// 表之间的关系（外键）
#[derive(Debug, Clone)]
pub struct Relationship {
    /// 源表名
    pub from_table: String,
    /// 源列名
    pub from_column: String,
    /// 目标表名
    pub to_table: String,
    /// 目标列名
    pub to_column: String,
    /// 关系类型
    pub relation_type: RelationType,
}

/// ER 图状态
#[derive(Default)]
pub struct ERDiagramState {
    /// 所有表
    pub tables: Vec<ERTable>,
    /// 所有关系
    pub relationships: Vec<Relationship>,
    /// 画布平移偏移
    pub pan_offset: Vec2,
    /// 缩放比例
    pub zoom: f32,
    /// 当前正在拖动的表索引
    pub dragging_table: Option<usize>,
    /// 拖动开始时的鼠标位置
    drag_start: Option<Pos2>,
    /// 当前选中的表索引
    pub selected_table: Option<usize>,
    /// 是否正在加载
    pub loading: bool,
    /// 是否需要重新布局
    pub needs_layout: bool,
    /// 是否显示 ER 图面板
    pub show: bool,
}

impl ERDiagramState {
    /// 创建新状态
    pub fn new() -> Self {
        Self {
            zoom: 1.0,
            needs_layout: true,
            ..Default::default()
        }
    }

    /// 清空数据
    pub fn clear(&mut self) {
        self.tables.clear();
        self.relationships.clear();
        self.selected_table = None;
        self.dragging_table = None;
        self.needs_layout = true;
    }

    /// 设置表数据
    pub fn set_tables(&mut self, tables: Vec<ERTable>) {
        self.tables = tables;
        self.needs_layout = true;
    }

    /// 设置关系数据
    pub fn set_relationships(&mut self, relationships: Vec<Relationship>) {
        self.relationships = relationships;
    }

    /// 开始拖动表
    pub fn start_drag(&mut self, table_index: usize, mouse_pos: Pos2) {
        self.dragging_table = Some(table_index);
        self.drag_start = Some(mouse_pos);
        self.selected_table = Some(table_index);
        if let Some(table) = self.tables.get_mut(table_index) {
            table.selected = true;
        }
        // 取消其他表的选中状态
        for (i, table) in self.tables.iter_mut().enumerate() {
            if i != table_index {
                table.selected = false;
            }
        }
    }

    /// 更新拖动位置
    pub fn update_drag(&mut self, mouse_pos: Pos2) {
        if let (Some(table_idx), Some(start)) = (self.dragging_table, self.drag_start) {
            if let Some(table) = self.tables.get_mut(table_idx) {
                let delta = mouse_pos - start;
                table.position += delta;
            }
            self.drag_start = Some(mouse_pos);
        }
    }

    /// 结束拖动
    pub fn end_drag(&mut self) {
        self.dragging_table = None;
        self.drag_start = None;
    }

    /// 缩放
    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.25, 4.0);
    }

    /// 重置视图
    pub fn reset_view(&mut self) {
        self.pan_offset = Vec2::ZERO;
        self.zoom = 1.0;
    }

    /// 适应视图（将所有表居中显示）
    pub fn fit_to_view(&mut self, available_size: Vec2) {
        if self.tables.is_empty() {
            return;
        }

        // 计算所有表的边界
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;

        for table in &self.tables {
            min_x = min_x.min(table.position.x);
            min_y = min_y.min(table.position.y);
            max_x = max_x.max(table.position.x + table.size.x);
            max_y = max_y.max(table.position.y + table.size.y);
        }

        let content_width = max_x - min_x;
        let content_height = max_y - min_y;

        if content_width > 0.0 && content_height > 0.0 {
            // 计算合适的缩放比例
            let scale_x = (available_size.x - 40.0) / content_width;
            let scale_y = (available_size.y - 40.0) / content_height;
            self.zoom = scale_x.min(scale_y).clamp(0.25, 2.0);

            // 计算偏移使内容居中
            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;
            self.pan_offset = Vec2::new(
                available_size.x / 2.0 / self.zoom - center_x,
                available_size.y / 2.0 / self.zoom - center_y,
            );
        }
    }

    /// 根据表名查找表索引
    pub fn find_table_index(&self, name: &str) -> Option<usize> {
        self.tables.iter().position(|t| t.name == name)
    }

    /// 获取表在屏幕上的位置（考虑缩放和平移）
    pub fn table_screen_pos(&self, table: &ERTable) -> Pos2 {
        Pos2::new(
            (table.position.x + self.pan_offset.x) * self.zoom,
            (table.position.y + self.pan_offset.y) * self.zoom,
        )
    }

    /// 获取表在屏幕上的尺寸
    pub fn table_screen_size(&self, table: &ERTable) -> Vec2 {
        table.size * self.zoom
    }
}
