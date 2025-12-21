//! 侧边栏组件 - 连接管理和表列表
//!
//! 侧边栏分为三个独立面板：
//! - 连接/数据库/表列表
//! - 触发器列表
//! - 存储过程/函数列表
//!
//! 每个面板可以：
//! - 独立显示/隐藏（通过顶部工具栏按钮）
//! - 独立折叠/展开（通过面板标题栏的折叠按钮）
//! - 通过拖动分割条调整大小

mod state;
mod actions;
mod connection_list;
mod database_list;
mod table_list;
mod trigger_panel;
mod routine_panel;
mod filter_panel;

pub use state::{SidebarPanelState, SidebarSelectionState};
pub use actions::{SidebarActions, SidebarFocusTransfer};
pub use filter_panel::FilterPanel;

use connection_list::ConnectionList;
use database_list::DatabaseList;
use table_list::TableList;
use trigger_panel::TriggerPanel;
use routine_panel::RoutinePanel;

use crate::database::ConnectionManager;
use crate::ui::SidebarSection;
use egui::{self, Color32, CornerRadius, Vec2};

/// 分割条高度
const DIVIDER_HEIGHT: f32 = 6.0;

pub struct Sidebar;

use crate::ui::ColumnFilter;

impl Sidebar {
    /// 在给定的 UI 区域内显示侧边栏内容
    #[allow(clippy::too_many_arguments)]
    pub fn show_in_ui(
        ui: &mut egui::Ui,
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        show_connection_dialog: &mut bool,
        is_focused: bool,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        width: f32,
        filters: &mut Vec<ColumnFilter>,
        columns: &[String],
    ) -> (SidebarActions, bool) {
        let mut filter_changed = false;
        let mut actions = SidebarActions::default();
        let ctx = ui.ctx().clone();

        // ====== 面板可见性控制工具栏 ======
        Self::show_visibility_toolbar(ui, panel_state);

        // 处理键盘导航
        let (item_count, selected_index) = Self::get_section_info(focused_section, connection_manager, panel_state, filters);
        if item_count > 0 && *selected_index >= item_count {
            *selected_index = item_count.saturating_sub(1);
        }
        if is_focused && item_count > 0 {
            Self::handle_keyboard_navigation(
                &ctx,
                focused_section,
                panel_state,
                item_count,
                connection_manager,
                selected_table,
                filters,
                &mut actions,
            );
        }

        // 固定侧边栏宽度
        ui.set_max_width(width);
        ui.set_min_width(width);
        let available_height = ui.available_height();

        // 计算各面板的实际高度
        let heights = Self::calculate_panel_heights(panel_state, available_height);

        // ====== 连接面板 ======
        if panel_state.show_connections {
            ConnectionList::show(
                ui,
                connection_manager,
                selected_table,
                show_connection_dialog,
                is_focused,
                focused_section,
                panel_state,
                &mut actions,
                heights.connections,
            );

            // 分割条：连接 <-> 筛选/触发器/存储过程
            if panel_state.show_filters || panel_state.show_triggers || panel_state.show_routines {
                Self::show_divider(ui, panel_state, 0, width);
            }
        }

        // ====== 筛选面板（第二个位置）======
        if panel_state.show_filters {
            if FilterPanel::show(
                ui,
                is_focused,
                focused_section,
                filters,
                columns,
                heights.filters,
            ) {
                filter_changed = true;
            }

            // 分割条：筛选 <-> 触发器/存储过程
            if panel_state.show_triggers || panel_state.show_routines {
                Self::show_divider(ui, panel_state, 1, width);
            }
        }

        // ====== 触发器面板 ======
        if panel_state.show_triggers {
            TriggerPanel::show(
                ui,
                is_focused,
                focused_section,
                panel_state,
                heights.triggers,
            );

            // 分割条：触发器 <-> 存储过程
            if panel_state.show_routines {
                Self::show_divider(ui, panel_state, 2, width);
            }
        }

        // ====== 存储过程面板 ======
        if panel_state.show_routines {
            RoutinePanel::show(
                ui,
                is_focused,
                focused_section,
                panel_state,
                heights.routines,
            );
        }

        // 如果没有任何面板显示
        if !panel_state.show_connections && !panel_state.show_triggers && !panel_state.show_routines && !panel_state.show_filters {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(egui::RichText::new("点击上方按钮显示面板").color(Color32::GRAY));
            });
        }

        (actions, filter_changed)
    }

    /// 获取当前 section 的项目数量和选中索引
    fn get_section_info<'a>(
        focused_section: SidebarSection,
        connection_manager: &ConnectionManager,
        panel_state: &'a mut SidebarPanelState,
        filters: &[crate::ui::ColumnFilter],
    ) -> (usize, &'a mut usize) {
        match focused_section {
            SidebarSection::Connections => (
                connection_manager.connections.len(),
                &mut panel_state.selection.connections,
            ),
            SidebarSection::Databases => (
                connection_manager.get_active().map(|c| c.databases.len()).unwrap_or(0),
                &mut panel_state.selection.databases,
            ),
            SidebarSection::Tables => (
                connection_manager.get_active().map(|c| c.tables.len()).unwrap_or(0),
                &mut panel_state.selection.tables,
            ),
            SidebarSection::Triggers => (
                panel_state.triggers.len(),
                &mut panel_state.selection.triggers,
            ),
            SidebarSection::Routines => (
                panel_state.routines.len(),
                &mut panel_state.selection.routines,
            ),
            SidebarSection::Filters => (
                filters.len(),
                &mut panel_state.selection.filters,
            ),
        }
    }

    /// 计算各面板高度
    /// 面板顺序：连接(0) -> 筛选(1) -> 触发器(2) -> 存储过程(3)
    fn calculate_panel_heights(panel_state: &SidebarPanelState, available_height: f32) -> PanelHeights {
        // 统计可见面板
        let visible_panels: Vec<(usize, f32)> = [
            (0, panel_state.connections_ratio, panel_state.show_connections),
            (1, panel_state.filters_ratio, panel_state.show_filters),
            (2, panel_state.triggers_ratio, panel_state.show_triggers),
            (3, panel_state.routines_ratio, panel_state.show_routines),
        ]
        .iter()
        .filter(|(_, _, visible)| *visible)
        .map(|(idx, ratio, _)| (*idx, *ratio))
        .collect();
        
        let visible_count = visible_panels.len();
        
        if visible_count == 0 {
            return PanelHeights { connections: 0.0, filters: 0.0, triggers: 0.0, routines: 0.0 };
        }

        // 计算分割条占用的空间
        let divider_count = visible_count.saturating_sub(1);
        let dividers_height = divider_count as f32 * DIVIDER_HEIGHT;
        
        // 可分配的高度
        let expandable_height = (available_height - dividers_height).max(0.0);
        
        // 计算总比例
        let total_ratio: f32 = visible_panels.iter().map(|(_, r)| r).sum();
        let total_ratio = if total_ratio > 0.0 { total_ratio } else { 1.0 };
        
        // 按比例分配高度
        let mut heights = PanelHeights { connections: 0.0, filters: 0.0, triggers: 0.0, routines: 0.0 };
        
        for (idx, ratio) in &visible_panels {
            let height = (expandable_height * ratio / total_ratio).max(60.0);
            match idx {
                0 => heights.connections = height,
                1 => heights.filters = height,
                2 => heights.triggers = height,
                3 => heights.routines = height,
                _ => {}
            }
        }
        
        heights
    }

    /// 显示可拖动分割条
    fn show_divider(ui: &mut egui::Ui, panel_state: &mut SidebarPanelState, divider_index: usize, width: f32) {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(width, DIVIDER_HEIGHT),
            egui::Sense::drag(),
        );

        // 绘制分割条
        let is_dragging = panel_state.dragging_divider == Some(divider_index);
        let color = if response.dragged() || response.hovered() || is_dragging {
            Color32::from_rgb(100, 150, 255)
        } else {
            Color32::from_rgba_unmultiplied(128, 128, 128, 60)
        };

        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(4.0, 1.0)),
            CornerRadius::same(2),
            color,
        );

        // 中间的拖动指示器（三个小点水平排列）
        let center = rect.center();
        for offset in [-12.0, 0.0, 12.0] {
            ui.painter().circle_filled(
                egui::pos2(center.x + offset, center.y),
                2.0,
                Color32::from_gray(160),
            );
        }

        // 处理拖动
        if response.dragged() {
            panel_state.dragging_divider = Some(divider_index);
            let delta = response.drag_delta().y;
            
            // 根据分割条位置调整相应面板的比例
            Self::adjust_panel_ratios(panel_state, divider_index, delta);
        } else if response.drag_stopped() {
            panel_state.dragging_divider = None;
        }

        // 鼠标光标
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
    }

    /// 调整面板比例
    /// 分割条顺序：0=连接↔筛选, 1=筛选↔触发器, 2=触发器↔存储过程
    fn adjust_panel_ratios(panel_state: &mut SidebarPanelState, divider_index: usize, delta: f32) {
        let delta_ratio = delta / 500.0; // 转换为比例变化
        
        match divider_index {
            0 => {
                // 连接 <-> 筛选
                if panel_state.show_connections {
                    panel_state.connections_ratio = (panel_state.connections_ratio + delta_ratio).clamp(0.1, 0.8);
                }
                if panel_state.show_filters {
                    panel_state.filters_ratio = (panel_state.filters_ratio - delta_ratio).clamp(0.1, 0.8);
                }
            }
            1 => {
                // 筛选 <-> 触发器
                if panel_state.show_filters {
                    panel_state.filters_ratio = (panel_state.filters_ratio + delta_ratio).clamp(0.1, 0.8);
                }
                if panel_state.show_triggers {
                    panel_state.triggers_ratio = (panel_state.triggers_ratio - delta_ratio).clamp(0.1, 0.8);
                }
            }
            2 => {
                // 触发器 <-> 存储过程
                if panel_state.show_triggers {
                    panel_state.triggers_ratio = (panel_state.triggers_ratio + delta_ratio).clamp(0.1, 0.8);
                }
                if panel_state.show_routines {
                    panel_state.routines_ratio = (panel_state.routines_ratio - delta_ratio).clamp(0.1, 0.8);
                }
            }
            _ => {}
        }
    }

    /// 处理键盘导航
    #[allow(clippy::too_many_arguments)]
    fn handle_keyboard_navigation(
        ctx: &egui::Context,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        item_count: usize,
        connection_manager: &ConnectionManager,
        selected_table: &mut Option<String>,
        filters: &mut Vec<crate::ui::ColumnFilter>,
        actions: &mut SidebarActions,
    ) {
        let selected_index = match focused_section {
            SidebarSection::Connections => &mut panel_state.selection.connections,
            SidebarSection::Databases => &mut panel_state.selection.databases,
            SidebarSection::Tables => &mut panel_state.selection.tables,
            SidebarSection::Triggers => &mut panel_state.selection.triggers,
            SidebarSection::Routines => &mut panel_state.selection.routines,
            SidebarSection::Filters => &mut panel_state.selection.filters,
        };

        ctx.input(|i| {
            // j 或下箭头：向下导航
            if i.key_pressed(egui::Key::J) || i.key_pressed(egui::Key::ArrowDown) {
                *selected_index = (*selected_index + 1).min(item_count.saturating_sub(1));
                panel_state.command_buffer.clear();
            }
            // k 或上箭头：向上导航
            if i.key_pressed(egui::Key::K) || i.key_pressed(egui::Key::ArrowUp) {
                *selected_index = selected_index.saturating_sub(1);
                panel_state.command_buffer.clear();
            }
            
            // === g 前缀命令 ===
            // g：进入 g 命令模式或跳到第一个（gg）
            if i.key_pressed(egui::Key::G) && !i.modifiers.shift {
                if panel_state.command_buffer == "g" {
                    // gg：跳到第一个
                    *selected_index = 0;
                    panel_state.command_buffer.clear();
                } else {
                    // 进入 g 命令模式
                    panel_state.command_buffer = "g".to_string();
                }
            }
            // gs：查看表结构（需要在 Tables section）
            if i.key_pressed(egui::Key::S) && panel_state.command_buffer == "g" {
                if let SidebarSection::Tables = focused_section {
                    if let Some(conn) = connection_manager.get_active()
                        && let Some(table) = conn.tables.get(*selected_index) {
                            actions.show_table_schema = Some(table.clone());
                        }
                }
                panel_state.command_buffer.clear();
            }
            // G (Shift+g)：跳到最后一个
            if i.key_pressed(egui::Key::G) && i.modifiers.shift {
                *selected_index = item_count.saturating_sub(1);
                panel_state.command_buffer.clear();
            }
            // Enter：选择/激活当前项
            if i.key_pressed(egui::Key::Enter) {
                match focused_section {
                    SidebarSection::Connections => {
                        let names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                        if let Some(name) = names.get(*selected_index) {
                            actions.connect = Some(name.clone());
                        }
                    }
                    SidebarSection::Databases => {
                        if let Some(conn) = connection_manager.get_active()
                            && let Some(db) = conn.databases.get(*selected_index) {
                                actions.select_database = Some(db.clone());
                            }
                    }
                    SidebarSection::Tables => {
                        if let Some(conn) = connection_manager.get_active()
                            && let Some(table) = conn.tables.get(*selected_index) {
                                actions.query_table = Some(table.clone());
                                *selected_table = Some(table.clone());
                            }
                    }
                    SidebarSection::Triggers => {
                        if let Some(trigger) = panel_state.triggers.get(*selected_index) {
                            actions.show_trigger_definition = Some(trigger.definition.clone());
                        }
                    }
                    SidebarSection::Routines => {
                        if let Some(routine) = panel_state.routines.get(*selected_index) {
                            actions.show_routine_definition = Some(routine.definition.clone());
                        }
                    }
                    SidebarSection::Filters => {
                        // Enter 切换筛选条件的启用状态
                        if let Some(filter) = filters.get_mut(*selected_index) {
                            filter.enabled = !filter.enabled;
                            actions.filter_changed = true;
                        }
                    }
                }
            }
            
            // Space：在 Filters section 切换启用状态
            if i.key_pressed(egui::Key::Space) && focused_section == SidebarSection::Filters {
                if let Some(filter) = filters.get_mut(*selected_index) {
                    filter.enabled = !filter.enabled;
                    actions.filter_changed = true;
                }
            }

            // h 或左箭头：向上层级导航
            // 顺序：存储过程 <- 触发器 <- 筛选 <- 表 <- 数据库 <- 连接
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::ArrowLeft) {
                let new_section = match focused_section {
                    SidebarSection::Routines => Some(SidebarSection::Triggers),
                    SidebarSection::Triggers => Some(SidebarSection::Filters),
                    SidebarSection::Filters => Some(SidebarSection::Tables),
                    SidebarSection::Tables => {
                        if connection_manager.get_active().map(|c| !c.databases.is_empty()).unwrap_or(false) {
                            Some(SidebarSection::Databases)
                        } else {
                            Some(SidebarSection::Connections)
                        }
                    }
                    SidebarSection::Databases => Some(SidebarSection::Connections),
                    SidebarSection::Connections => None,
                };
                if let Some(section) = new_section {
                    actions.section_change = Some(section);
                }
            }

            // l 或右箭头：向下层级导航
            // 顺序：连接 -> 数据库 -> 表 -> 筛选 -> 触发器 -> 存储过程
            if i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight) {
                let conn = connection_manager.get_active();
                let has_databases = conn.map(|c| !c.databases.is_empty()).unwrap_or(false);
                let has_tables = conn.map(|c| !c.tables.is_empty()).unwrap_or(false);
                let has_filters = !filters.is_empty();
                let has_triggers = !panel_state.triggers.is_empty();
                let has_routines = !panel_state.routines.is_empty();

                let new_section = match focused_section {
                    SidebarSection::Connections => {
                        if has_databases {
                            Some(SidebarSection::Databases)
                        } else if has_tables {
                            Some(SidebarSection::Tables)
                        } else {
                            None
                        }
                    }
                    SidebarSection::Databases => {
                        if has_tables { Some(SidebarSection::Tables) } else { None }
                    }
                    SidebarSection::Tables => {
                        if has_filters {
                            Some(SidebarSection::Filters)
                        } else if has_triggers {
                            Some(SidebarSection::Triggers)
                        } else if has_routines {
                            Some(SidebarSection::Routines)
                        } else {
                            None
                        }
                    }
                    SidebarSection::Filters => {
                        if has_triggers { 
                            Some(SidebarSection::Triggers) 
                        } else if has_routines {
                            Some(SidebarSection::Routines)
                        } else {
                            None
                        }
                    }
                    SidebarSection::Triggers => {
                        if has_routines { Some(SidebarSection::Routines) } else { None }
                    }
                    SidebarSection::Routines => None,
                };

                if let Some(section) = new_section {
                    actions.section_change = Some(section);
                } else {
                    actions.focus_transfer = Some(SidebarFocusTransfer::ToDataGrid);
                }
            }

            // d：删除选中项（连接/表/筛选条件）
            if i.key_pressed(egui::Key::D) && !i.modifiers.ctrl && !i.modifiers.shift {
                match focused_section {
                    SidebarSection::Connections => {
                        let names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                        if let Some(name) = names.get(*selected_index) {
                            actions.delete = Some(name.clone());
                        }
                    }
                    SidebarSection::Tables => {
                        // 表删除需要确认对话框，设置删除请求
                        if let Some(conn) = connection_manager.get_active()
                            && let Some(table) = conn.tables.get(*selected_index) {
                                actions.delete = Some(format!("table:{}", table));
                            }
                    }
                    SidebarSection::Filters => {
                        // 删除选中的筛选条件
                        if *selected_index < filters.len() {
                            filters.remove(*selected_index);
                            // 调整选中索引
                            if *selected_index >= filters.len() && !filters.is_empty() {
                                *selected_index = filters.len() - 1;
                            }
                            actions.filter_changed = true;
                        }
                    }
                    _ => {} // 其他 section 暂不支持删除
                }
            }
            
            // x：在 Filters section 也支持删除（Helix 风格）
            if i.key_pressed(egui::Key::X) && focused_section == SidebarSection::Filters {
                if *selected_index < filters.len() {
                    filters.remove(*selected_index);
                    if *selected_index >= filters.len() && !filters.is_empty() {
                        *selected_index = filters.len() - 1;
                    }
                    actions.filter_changed = true;
                }
            }

            // e：编辑选中的连接配置
            if i.key_pressed(egui::Key::E) && !i.modifiers.ctrl {
                if let SidebarSection::Connections = focused_section {
                    let names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                    if let Some(name) = names.get(*selected_index) {
                        actions.edit_connection = Some(name.clone());
                    }
                }
            }

            // r：重命名选中项
            if i.key_pressed(egui::Key::R) && !i.modifiers.ctrl {
                let item_name = match focused_section {
                    SidebarSection::Connections => {
                        let names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                        names.get(*selected_index).cloned()
                    }
                    SidebarSection::Tables => {
                        connection_manager.get_active()
                            .and_then(|c| c.tables.get(*selected_index).cloned())
                    }
                    _ => None,
                };
                if let Some(name) = item_name {
                    actions.rename_item = Some((focused_section, name));
                }
            }

            // R (Shift+r)：刷新当前列表
            if i.key_pressed(egui::Key::R) && i.modifiers.shift {
                actions.refresh = true;
            }

            // === Filters section 专用快捷键 (Helix 风格) ===
            // 
            // 筛选条件操作快捷键：
            // j/k     - 选择筛选条件（上/下）
            // a/o     - 增加筛选条件
            // d/x     - 删除筛选条件
            // c       - 清空所有筛选条件
            // w/b     - 切换筛选对象（列）到下一个/上一个
            // n/N     - 切换筛选规则（操作符）到下一个/上一个
            // i       - 编辑筛选值
            // t       - 切换 AND/OR 逻辑
            // s       - 切换大小写敏感
            // Space   - 启用/禁用筛选条件
            //
            if focused_section == SidebarSection::Filters {
                // a/o：增加筛选条件（Helix: a = append, o = open below）
                if i.key_pressed(egui::Key::A) || i.key_pressed(egui::Key::O) {
                    actions.add_filter = true;
                }
                
                // c：清空所有筛选条件（Helix: c = change）
                if i.key_pressed(egui::Key::C) && !i.modifiers.ctrl {
                    actions.clear_filters = true;
                }
                
                // w：切换筛选对象（列）到下一个（Helix: w = word forward）
                if i.key_pressed(egui::Key::W) && !i.modifiers.ctrl {
                    if *selected_index < filters.len() {
                        actions.cycle_filter_column = Some((*selected_index, true));
                    }
                }
                
                // b：切换筛选对象（列）到上一个（Helix: b = word backward）
                if i.key_pressed(egui::Key::B) && !i.modifiers.ctrl {
                    if *selected_index < filters.len() {
                        actions.cycle_filter_column = Some((*selected_index, false));
                    }
                }
                
                // n：切换筛选规则（操作符）到下一个（Helix: n = next search）
                if i.key_pressed(egui::Key::N) && !i.modifiers.ctrl && !i.modifiers.shift {
                    if let Some(filter) = filters.get_mut(*selected_index) {
                        filter.operator = next_operator(&filter.operator);
                        actions.filter_changed = true;
                    }
                }
                
                // N (Shift+n)：切换筛选规则（操作符）到上一个
                if i.key_pressed(egui::Key::N) && i.modifiers.shift {
                    if let Some(filter) = filters.get_mut(*selected_index) {
                        filter.operator = prev_operator(&filter.operator);
                        actions.filter_changed = true;
                    }
                }
                
                // t：切换当前筛选条件的 AND/OR 逻辑
                if i.key_pressed(egui::Key::T) {
                    if *selected_index < filters.len() {
                        actions.toggle_filter_logic = Some(*selected_index);
                    }
                }
                
                // i：编辑筛选值（Helix: i = insert mode）
                if i.key_pressed(egui::Key::I) {
                    if *selected_index < filters.len() {
                        actions.focus_filter_input = Some(*selected_index);
                    }
                }
                
                // s：切换大小写敏感（Helix: s = select）
                if i.key_pressed(egui::Key::S) && panel_state.command_buffer.is_empty() {
                    if let Some(filter) = filters.get_mut(*selected_index) {
                        if filter.operator.supports_case_sensitivity() {
                            filter.case_sensitive = !filter.case_sensitive;
                            actions.filter_changed = true;
                        }
                    }
                }
            }
        });

        // 同步到旧的 trigger_selected_index 字段（保持向后兼容）
        if focused_section == SidebarSection::Triggers {
            panel_state.trigger_selected_index = panel_state.selection.triggers;
        }
    }

    /// 显示面板可见性控制工具栏
    fn show_visibility_toolbar(ui: &mut egui::Ui, panel_state: &mut SidebarPanelState) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            // 无边框图标按钮
            let icon_toggle = |ui: &mut egui::Ui, icon: &str, active: bool, tooltip: &str| -> bool {
                let color = if active { Color32::from_rgb(100, 200, 150) } else { Color32::from_gray(100) };
                ui.add(
                    egui::Button::new(egui::RichText::new(icon).size(14.0).color(color))
                        .frame(false)
                        .min_size(egui::Vec2::new(22.0, 22.0)),
                )
                .on_hover_text(tooltip)
                .clicked()
            };

            // 1. 连接面板
            if icon_toggle(ui, "🔗", panel_state.show_connections, "连接面板 (Ctrl+1)") {
                panel_state.show_connections = !panel_state.show_connections;
            }

            // 2-3. 数据库和表在连接面板内，无需单独按钮

            // 4. 筛选面板
            if icon_toggle(ui, "🔍", panel_state.show_filters, "筛选面板 (Ctrl+4)") {
                panel_state.show_filters = !panel_state.show_filters;
            }

            // 5. 触发器面板
            if icon_toggle(ui, "⚡", panel_state.show_triggers, "触发器面板 (Ctrl+5)") {
                panel_state.show_triggers = !panel_state.show_triggers;
            }

            // 6. 存储过程面板
            if icon_toggle(ui, "📦", panel_state.show_routines, "存储过程面板 (Ctrl+6)") {
                panel_state.show_routines = !panel_state.show_routines;
            }
        });

        ui.separator();
    }
}

/// 面板高度计算结果
struct PanelHeights {
    connections: f32,
    triggers: f32,
    routines: f32,
    filters: f32,
}

/// 获取下一个筛选操作符
fn next_operator(current: &crate::ui::FilterOperator) -> crate::ui::FilterOperator {
    use crate::ui::FilterOperator::*;
    match current {
        Contains => NotContains,
        NotContains => Equals,
        Equals => NotEquals,
        NotEquals => StartsWith,
        StartsWith => EndsWith,
        EndsWith => GreaterThan,
        GreaterThan => GreaterOrEqual,
        GreaterOrEqual => LessThan,
        LessThan => LessOrEqual,
        LessOrEqual => Between,
        Between => NotBetween,
        NotBetween => In,
        In => NotIn,
        NotIn => IsNull,
        IsNull => IsNotNull,
        IsNotNull => IsEmpty,
        IsEmpty => IsNotEmpty,
        IsNotEmpty => Regex,
        Regex => Contains,
    }
}

/// 获取上一个筛选操作符
fn prev_operator(current: &crate::ui::FilterOperator) -> crate::ui::FilterOperator {
    use crate::ui::FilterOperator::*;
    match current {
        Contains => Regex,
        NotContains => Contains,
        Equals => NotContains,
        NotEquals => Equals,
        StartsWith => NotEquals,
        EndsWith => StartsWith,
        GreaterThan => EndsWith,
        GreaterOrEqual => GreaterThan,
        LessThan => GreaterOrEqual,
        LessOrEqual => LessThan,
        Between => LessOrEqual,
        NotBetween => Between,
        In => NotBetween,
        NotIn => In,
        IsNull => NotIn,
        IsNotNull => IsNull,
        IsEmpty => IsNotNull,
        IsNotEmpty => IsEmpty,
        Regex => IsNotEmpty,
    }
}
