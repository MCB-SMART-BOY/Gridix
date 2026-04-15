//! ER 图状态和数据结构

#![allow(dead_code)] // 公开 API

use std::collections::HashSet;

use egui::{Pos2, Rect, Vec2};

const SELECTION_REVEAL_MARGIN: f32 = 24.0;
const KEYBOARD_PAN_STEP: f32 = 64.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ERDiagramInteractionMode {
    #[default]
    Navigation,
    Viewport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeometricDirection {
    Left,
    Right,
    Up,
    Down,
}

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
    /// 当前选中表是否需要在下一帧滚回可见区域
    pending_selection_reveal: bool,
    /// 当前键盘交互模式
    interaction_mode: ERDiagramInteractionMode,
    /// 是否正在加载
    pub loading: bool,
    /// 是否需要重新布局
    pub needs_layout: bool,
    /// 仍在等待列回包的表
    pending_column_tables: HashSet<String>,
    /// 已解析出的外键列集合 `(table, column)`
    foreign_key_columns: HashSet<(String, String)>,
    /// 外键请求是否已完成（成功或失败）
    foreign_keys_resolved: bool,
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
        self.pending_selection_reveal = false;
        self.interaction_mode = ERDiagramInteractionMode::Navigation;
        self.needs_layout = true;
        self.pending_column_tables.clear();
        self.foreign_key_columns.clear();
        self.foreign_keys_resolved = false;
    }

    /// 开始一轮新的 ER 数据加载。
    pub fn begin_loading(&mut self, table_names: &[String]) {
        self.clear();
        self.loading = true;
        self.pending_column_tables = table_names.iter().cloned().collect();
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

    /// 标记外键请求已经完成，并缓存外键列集合。
    pub fn set_foreign_key_columns<I>(&mut self, pairs: I)
    where
        I: IntoIterator<Item = (String, String)>,
    {
        self.foreign_key_columns = pairs.into_iter().collect();
        self.foreign_keys_resolved = true;
        self.apply_foreign_key_flags();
        self.refresh_loading_state();
    }

    /// 标记外键请求结束但没有可用结果。
    pub fn mark_foreign_keys_resolved(&mut self) {
        self.foreign_keys_resolved = true;
        self.refresh_loading_state();
    }

    /// 标记某张表的列请求已经结束（成功或失败）。
    pub fn mark_table_request_resolved(&mut self, table_name: &str) {
        self.pending_column_tables.remove(table_name);
        self.refresh_loading_state();
    }

    /// 当前表列请求是否全部结束。
    pub fn all_table_requests_resolved(&self) -> bool {
        self.pending_column_tables.is_empty()
    }

    /// 查询某列是否应标记为外键。
    pub fn is_foreign_key_column(&self, table_name: &str, column_name: &str) -> bool {
        self.foreign_key_columns
            .contains(&(table_name.to_string(), column_name.to_string()))
    }

    /// 开始拖动表
    pub fn start_drag(&mut self, table_index: usize, mouse_pos: Pos2) {
        self.dragging_table = Some(table_index);
        self.drag_start = Some(mouse_pos);
        self.select_table(table_index);
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

    /// 返回当前选中表名。
    pub fn selected_table_name(&self) -> Option<&str> {
        self.selected_table
            .and_then(|index| self.tables.get(index))
            .map(|table| table.name.as_str())
    }

    fn related_table_indices_for(&self, index: usize) -> Vec<usize> {
        let Some(current) = self.tables.get(index) else {
            return Vec::new();
        };

        let mut related = HashSet::new();
        for relationship in &self.relationships {
            if relationship.from_table == current.name
                && let Some(target_index) = self.find_table_index(&relationship.to_table)
                && target_index != index
            {
                related.insert(target_index);
            }

            if relationship.to_table == current.name
                && let Some(source_index) = self.find_table_index(&relationship.from_table)
                && source_index != index
            {
                related.insert(source_index);
            }
        }

        let mut ordered: Vec<usize> = related.into_iter().collect();
        ordered.sort_unstable();
        ordered
    }

    fn geometric_neighbor_in_direction(
        &self,
        current_index: usize,
        direction: GeometricDirection,
    ) -> Option<usize> {
        let current = self.tables.get(current_index)?;
        let current_center = current.center();
        let mut broad_candidates = Vec::new();
        let mut directional_candidates = Vec::new();

        for (index, table) in self.tables.iter().enumerate() {
            if index == current_index {
                continue;
            }

            let center = table.center();
            let delta = center - current_center;
            let (primary, secondary) = match direction {
                GeometricDirection::Left => (-delta.x, delta.y.abs()),
                GeometricDirection::Right => (delta.x, delta.y.abs()),
                GeometricDirection::Up => (-delta.y, delta.x.abs()),
                GeometricDirection::Down => (delta.y, delta.x.abs()),
            };

            if primary <= 0.0 {
                continue;
            }

            let candidate = (index, primary, secondary);
            broad_candidates.push(candidate);
            if primary >= secondary {
                directional_candidates.push(candidate);
            }
        }

        let candidates = if directional_candidates.is_empty() {
            broad_candidates
        } else {
            directional_candidates
        };

        candidates
            .into_iter()
            .min_by(|a, b| {
                a.1.partial_cmp(&b.1)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal))
                    .then_with(|| a.0.cmp(&b.0))
            })
            .map(|(index, _, _)| index)
    }

    pub fn interaction_mode(&self) -> ERDiagramInteractionMode {
        self.interaction_mode
    }

    pub fn is_viewport_mode(&self) -> bool {
        self.interaction_mode == ERDiagramInteractionMode::Viewport
    }

    pub fn toggle_interaction_mode(&mut self) -> ERDiagramInteractionMode {
        self.interaction_mode = match self.interaction_mode {
            ERDiagramInteractionMode::Navigation => ERDiagramInteractionMode::Viewport,
            ERDiagramInteractionMode::Viewport => ERDiagramInteractionMode::Navigation,
        };
        self.interaction_mode
    }

    pub fn exit_viewport_mode(&mut self) -> bool {
        if self.is_viewport_mode() {
            self.interaction_mode = ERDiagramInteractionMode::Navigation;
            true
        } else {
            false
        }
    }

    /// 若当前没有有效选中项，则优先按表名恢复选择，否则退回首表。
    pub fn ensure_selection(&mut self, preferred_table: Option<&str>) -> bool {
        if self
            .selected_table
            .is_some_and(|index| self.tables.get(index).is_some())
        {
            self.pending_selection_reveal = true;
            self.sync_selected_flags();
            return true;
        }

        if let Some(preferred_table) = preferred_table
            && let Some(index) = self.find_table_index(preferred_table)
        {
            return self.select_table(index);
        }

        if self.tables.is_empty() {
            self.selected_table = None;
            return false;
        }

        self.select_table(0)
    }

    /// 选中指定索引的表。
    pub fn select_table(&mut self, index: usize) -> bool {
        if self.tables.get(index).is_none() {
            return false;
        }

        self.selected_table = Some(index);
        self.pending_selection_reveal = true;
        self.sync_selected_flags();
        true
    }

    /// 选中下一张表；若当前未选中则选中第一张。
    pub fn select_next_table(&mut self) -> bool {
        if self.tables.is_empty() {
            self.selected_table = None;
            return false;
        }

        let next_index = match self.selected_table {
            Some(index) => (index + 1).min(self.tables.len() - 1),
            None => 0,
        };

        self.select_table(next_index)
    }

    /// 选中上一张表；若当前未选中则选中最后一张。
    pub fn select_prev_table(&mut self) -> bool {
        if self.tables.is_empty() {
            self.selected_table = None;
            return false;
        }

        let prev_index = match self.selected_table {
            Some(index) => index.saturating_sub(1),
            None => self.tables.len() - 1,
        };

        self.select_table(prev_index)
    }

    /// 选中下一张关联表，优先按稳定表顺序寻找当前索引之后的关联项，否则回绕到首个关联项。
    pub fn select_next_related_table(&mut self) -> bool {
        if !self.ensure_selection(None) {
            return false;
        }

        let Some(current_index) = self.selected_table else {
            return false;
        };
        let related = self.related_table_indices_for(current_index);
        let Some(next_index) = related
            .iter()
            .copied()
            .find(|index| *index > current_index)
            .or_else(|| related.first().copied())
        else {
            return false;
        };

        self.select_table(next_index)
    }

    /// 选中上一张关联表，优先按稳定表顺序寻找当前索引之前的关联项，否则回绕到最后一个关联项。
    pub fn select_prev_related_table(&mut self) -> bool {
        if !self.ensure_selection(None) {
            return false;
        }

        let Some(current_index) = self.selected_table else {
            return false;
        };
        let related = self.related_table_indices_for(current_index);
        let Some(prev_index) = related
            .iter()
            .copied()
            .rev()
            .find(|index| *index < current_index)
            .or_else(|| related.last().copied())
        else {
            return false;
        };

        self.select_table(prev_index)
    }

    pub fn select_geometric_neighbor(&mut self, direction: GeometricDirection) -> bool {
        if !self.ensure_selection(None) {
            return false;
        }

        let Some(current_index) = self.selected_table else {
            return false;
        };
        let Some(next_index) = self.geometric_neighbor_in_direction(current_index, direction)
        else {
            return false;
        };

        self.select_table(next_index)
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

    /// 若当前选中表超出视口，则调整平移偏移使其重新回到可见区域。
    pub fn reveal_selected_table_in_view(&mut self, available_size: Vec2) -> bool {
        if !self.pending_selection_reveal {
            return false;
        }

        let Some(table) = self.selected_table.and_then(|index| self.tables.get(index)) else {
            self.pending_selection_reveal = false;
            return false;
        };

        let table_rect =
            Rect::from_min_size(self.table_screen_pos(table), self.table_screen_size(table));
        let visible_rect = Rect::from_min_max(
            Pos2::new(SELECTION_REVEAL_MARGIN, SELECTION_REVEAL_MARGIN),
            Pos2::new(
                (available_size.x - SELECTION_REVEAL_MARGIN).max(SELECTION_REVEAL_MARGIN),
                (available_size.y - SELECTION_REVEAL_MARGIN).max(SELECTION_REVEAL_MARGIN),
            ),
        );

        let mut delta_screen = Vec2::ZERO;

        if table_rect.width() > visible_rect.width() {
            delta_screen.x = visible_rect.center().x - table_rect.center().x;
        } else if table_rect.left() < visible_rect.left() {
            delta_screen.x = visible_rect.left() - table_rect.left();
        } else if table_rect.right() > visible_rect.right() {
            delta_screen.x = visible_rect.right() - table_rect.right();
        }

        if table_rect.height() > visible_rect.height() {
            delta_screen.y = visible_rect.center().y - table_rect.center().y;
        } else if table_rect.top() < visible_rect.top() {
            delta_screen.y = visible_rect.top() - table_rect.top();
        } else if table_rect.bottom() > visible_rect.bottom() {
            delta_screen.y = visible_rect.bottom() - table_rect.bottom();
        }

        self.pending_selection_reveal = false;

        if delta_screen == Vec2::ZERO {
            return false;
        }

        self.pan_offset += delta_screen / self.zoom;
        true
    }

    pub fn pan_keyboard_left(&mut self) {
        self.pan_offset.x += KEYBOARD_PAN_STEP / self.zoom;
    }

    pub fn pan_keyboard_right(&mut self) {
        self.pan_offset.x -= KEYBOARD_PAN_STEP / self.zoom;
    }

    pub fn pan_keyboard_up(&mut self) {
        self.pan_offset.y += KEYBOARD_PAN_STEP / self.zoom;
    }

    pub fn pan_keyboard_down(&mut self) {
        self.pan_offset.y -= KEYBOARD_PAN_STEP / self.zoom;
    }

    fn apply_foreign_key_flags(&mut self) {
        for table in &mut self.tables {
            for column in &mut table.columns {
                column.is_foreign_key = self
                    .foreign_key_columns
                    .contains(&(table.name.clone(), column.name.clone()));
            }
        }
    }

    fn refresh_loading_state(&mut self) {
        self.loading = !(self.foreign_keys_resolved && self.pending_column_tables.is_empty());
    }

    fn sync_selected_flags(&mut self) {
        for (index, table) in self.tables.iter_mut().enumerate() {
            table.selected = self.selected_table == Some(index);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ERColumn, ERDiagramInteractionMode, ERDiagramState, ERTable, GeometricDirection,
        RelationType, Relationship,
    };
    use egui::{Pos2, Vec2};

    fn make_table(name: &str, position: Pos2) -> ERTable {
        let mut table = ERTable::new(name.to_string());
        table.position = position;
        table.size = Vec2::new(120.0, 80.0);
        table
    }

    #[test]
    fn loading_waits_for_foreign_keys_and_all_table_requests() {
        let mut state = ERDiagramState::new();
        let tables = vec!["orders".to_string(), "customers".to_string()];

        state.begin_loading(&tables);
        assert!(state.loading);

        state.mark_table_request_resolved("orders");
        assert!(state.loading);

        state.mark_foreign_keys_resolved();
        assert!(state.loading);

        state.mark_table_request_resolved("customers");
        assert!(!state.loading);
    }

    #[test]
    fn foreign_key_columns_apply_to_existing_and_late_loaded_columns() {
        let mut state = ERDiagramState::new();
        state.begin_loading(&["orders".to_string()]);

        let mut table = ERTable::new("orders".to_string());
        table.columns.push(ERColumn {
            name: "customer_id".to_string(),
            data_type: "INTEGER".to_string(),
            is_primary_key: false,
            is_foreign_key: false,
            nullable: false,
            default_value: None,
        });
        state.tables.push(table);

        state.set_foreign_key_columns([("orders".to_string(), "customer_id".to_string())]);
        assert!(state.tables[0].columns[0].is_foreign_key);
        assert!(state.is_foreign_key_column("orders", "customer_id"));
        assert!(!state.is_foreign_key_column("orders", "id"));
    }

    #[test]
    fn ensure_selection_prefers_matching_table_then_first() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            ERTable::new("customers".to_string()),
            ERTable::new("orders".to_string()),
        ];

        assert!(state.ensure_selection(Some("orders")));
        assert_eq!(state.selected_table, Some(1));
        assert_eq!(state.selected_table_name(), Some("orders"));
        assert!(!state.tables[0].selected);
        assert!(state.tables[1].selected);

        state.selected_table = None;
        assert!(state.ensure_selection(Some("missing")));
        assert_eq!(state.selected_table, Some(0));
        assert_eq!(state.selected_table_name(), Some("customers"));
    }

    #[test]
    fn select_next_and_prev_table_use_stable_linear_order() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            ERTable::new("customers".to_string()),
            ERTable::new("orders".to_string()),
            ERTable::new("payments".to_string()),
        ];

        assert!(state.select_next_table());
        assert_eq!(state.selected_table_name(), Some("customers"));

        assert!(state.select_next_table());
        assert_eq!(state.selected_table_name(), Some("orders"));

        assert!(state.select_next_table());
        assert_eq!(state.selected_table_name(), Some("payments"));

        assert!(state.select_next_table());
        assert_eq!(state.selected_table_name(), Some("payments"));

        assert!(state.select_prev_table());
        assert_eq!(state.selected_table_name(), Some("orders"));

        state.selected_table = None;
        assert!(state.select_prev_table());
        assert_eq!(state.selected_table_name(), Some("payments"));
    }

    #[test]
    fn select_next_and_prev_related_table_follow_relationship_neighbors() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            ERTable::new("customers".to_string()),
            ERTable::new("orders".to_string()),
            ERTable::new("payments".to_string()),
            ERTable::new("audits".to_string()),
        ];
        state.relationships = vec![
            Relationship {
                from_table: "orders".to_string(),
                from_column: "customer_id".to_string(),
                to_table: "customers".to_string(),
                to_column: "id".to_string(),
                relation_type: RelationType::OneToMany,
            },
            Relationship {
                from_table: "payments".to_string(),
                from_column: "order_id".to_string(),
                to_table: "orders".to_string(),
                to_column: "id".to_string(),
                relation_type: RelationType::OneToMany,
            },
            Relationship {
                from_table: "audits".to_string(),
                from_column: "order_id".to_string(),
                to_table: "orders".to_string(),
                to_column: "id".to_string(),
                relation_type: RelationType::OneToMany,
            },
        ];

        assert!(state.select_table(1));
        assert!(state.select_next_related_table());
        assert_eq!(state.selected_table_name(), Some("payments"));

        assert!(state.select_prev_related_table());
        assert_eq!(state.selected_table_name(), Some("orders"));

        assert!(state.select_table(3));
        assert!(state.select_next_related_table());
        assert_eq!(state.selected_table_name(), Some("orders"));

        assert!(state.select_prev_related_table());
        assert_eq!(state.selected_table_name(), Some("customers"));
    }

    #[test]
    fn related_navigation_deduplicates_bidirectional_relationships_and_noops_without_neighbors() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            ERTable::new("customers".to_string()),
            ERTable::new("orders".to_string()),
            ERTable::new("logs".to_string()),
        ];
        state.relationships = vec![
            Relationship {
                from_table: "orders".to_string(),
                from_column: "customer_id".to_string(),
                to_table: "customers".to_string(),
                to_column: "id".to_string(),
                relation_type: RelationType::OneToMany,
            },
            Relationship {
                from_table: "customers".to_string(),
                from_column: "id".to_string(),
                to_table: "orders".to_string(),
                to_column: "customer_id".to_string(),
                relation_type: RelationType::OneToMany,
            },
        ];

        assert!(state.select_table(1));
        assert!(state.select_next_related_table());
        assert_eq!(state.selected_table_name(), Some("customers"));

        assert!(state.select_table(2));
        assert!(!state.select_next_related_table());
        assert_eq!(state.selected_table_name(), Some("logs"));
        assert!(!state.select_prev_related_table());
        assert_eq!(state.selected_table_name(), Some("logs"));
    }

    #[test]
    fn geometric_navigation_prefers_nearest_candidate_in_requested_direction() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            make_table("center", Pos2::new(100.0, 100.0)),
            make_table("left_near", Pos2::new(-70.0, 110.0)),
            make_table("left_far", Pos2::new(-280.0, 80.0)),
            make_table("right_near", Pos2::new(280.0, 105.0)),
            make_table("up_near", Pos2::new(105.0, -80.0)),
            make_table("down_near", Pos2::new(115.0, 300.0)),
        ];
        state.selected_table = Some(0);

        assert!(state.select_geometric_neighbor(GeometricDirection::Left));
        assert_eq!(state.selected_table_name(), Some("left_near"));

        state.selected_table = Some(0);
        assert!(state.select_geometric_neighbor(GeometricDirection::Right));
        assert_eq!(state.selected_table_name(), Some("right_near"));

        state.selected_table = Some(0);
        assert!(state.select_geometric_neighbor(GeometricDirection::Up));
        assert_eq!(state.selected_table_name(), Some("up_near"));

        state.selected_table = Some(0);
        assert!(state.select_geometric_neighbor(GeometricDirection::Down));
        assert_eq!(state.selected_table_name(), Some("down_near"));
    }

    #[test]
    fn geometric_navigation_noops_when_direction_has_no_candidate() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            make_table("origin", Pos2::new(0.0, 0.0)),
            make_table("right_only", Pos2::new(220.0, 0.0)),
        ];
        state.selected_table = Some(0);

        assert!(!state.select_geometric_neighbor(GeometricDirection::Left));
        assert_eq!(state.selected_table_name(), Some("origin"));

        assert!(state.select_geometric_neighbor(GeometricDirection::Right));
        assert_eq!(state.selected_table_name(), Some("right_only"));

        assert!(!state.select_geometric_neighbor(GeometricDirection::Down));
        assert_eq!(state.selected_table_name(), Some("right_only"));
    }

    #[test]
    fn geometric_navigation_falls_back_to_diagonal_when_no_axis_aligned_candidate_exists() {
        let mut state = ERDiagramState::new();
        state.tables = vec![
            make_table("center", Pos2::new(100.0, 100.0)),
            make_table("diagonal_right", Pos2::new(220.0, -40.0)),
        ];
        state.selected_table = Some(0);

        assert!(state.select_geometric_neighbor(GeometricDirection::Right));
        assert_eq!(state.selected_table_name(), Some("diagonal_right"));
    }

    #[test]
    fn ensure_selection_marks_existing_valid_selection_for_reveal() {
        let mut state = ERDiagramState::new();
        let mut customers = ERTable::new("customers".to_string());
        customers.size = Vec2::new(180.0, 120.0);
        let mut orders = ERTable::new("orders".to_string());
        orders.size = Vec2::new(180.0, 120.0);
        state.set_tables(vec![customers, orders]);
        state.selected_table = Some(1);
        state.pending_selection_reveal = false;

        assert!(state.ensure_selection(Some("customers")));
        assert!(state.pending_selection_reveal);
        assert_eq!(state.selected_table_name(), Some("orders"));
    }

    #[test]
    fn reveal_selected_table_moves_pan_offset_until_selection_is_visible() {
        let mut state = ERDiagramState::new();
        let mut customers = ERTable::new("customers".to_string());
        customers.position = Pos2::new(0.0, 0.0);
        customers.size = Vec2::new(180.0, 120.0);

        let mut orders = ERTable::new("orders".to_string());
        orders.position = Pos2::new(420.0, 280.0);
        orders.size = Vec2::new(180.0, 120.0);

        state.set_tables(vec![customers, orders]);
        assert!(state.select_table(1));

        assert!(state.reveal_selected_table_in_view(Vec2::new(320.0, 220.0)));
        assert!(!state.pending_selection_reveal);

        let selected = &state.tables[1];
        let rect = egui::Rect::from_min_size(
            state.table_screen_pos(selected),
            state.table_screen_size(selected),
        );

        assert!(rect.left() >= super::SELECTION_REVEAL_MARGIN);
        assert!(rect.right() <= 320.0 - super::SELECTION_REVEAL_MARGIN);
        assert!(rect.top() >= super::SELECTION_REVEAL_MARGIN);
        assert!(rect.bottom() <= 220.0 - super::SELECTION_REVEAL_MARGIN);
    }

    #[test]
    fn toggle_interaction_mode_switches_between_navigation_and_viewport() {
        let mut state = ERDiagramState::new();
        assert_eq!(
            state.interaction_mode(),
            ERDiagramInteractionMode::Navigation
        );

        assert_eq!(
            state.toggle_interaction_mode(),
            ERDiagramInteractionMode::Viewport
        );
        assert!(state.is_viewport_mode());

        assert_eq!(
            state.toggle_interaction_mode(),
            ERDiagramInteractionMode::Navigation
        );
        assert!(!state.is_viewport_mode());
    }

    #[test]
    fn keyboard_pan_moves_viewport_in_expected_directions() {
        let mut state = ERDiagramState::new();
        state.zoom = 2.0;

        state.pan_keyboard_left();
        assert_eq!(state.pan_offset.x, 32.0);

        state.pan_keyboard_right();
        assert_eq!(state.pan_offset.x, 0.0);

        state.pan_keyboard_up();
        assert_eq!(state.pan_offset.y, 32.0);

        state.pan_keyboard_down();
        assert_eq!(state.pan_offset.y, 0.0);
    }

    #[test]
    fn begin_loading_resets_interaction_mode_and_pending_selection_reveal() {
        let mut state = ERDiagramState::new();
        state.toggle_interaction_mode();
        state.selected_table = Some(0);
        state.pending_selection_reveal = true;

        state.begin_loading(&["orders".to_string()]);

        assert_eq!(
            state.interaction_mode(),
            ERDiagramInteractionMode::Navigation
        );
        assert!(!state.pending_selection_reveal);
        assert!(state.loading);
    }
}
