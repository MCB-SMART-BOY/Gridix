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
//!
//! 键盘操作（统一使用侧边栏局部 action 层）：
//! - `j/k` - 上下导航
//! - `gg/G` - 跳转到首/末项
//! - `h` - 返回左侧层级 / 上一个分区
//! - `l` - 进入更深层级或离开侧边栏进入结果表格
//! - `Enter` - 激活/选择
//! - `Space` - 切换状态
//! - `d` - 删除
//! - `e` - 编辑
//! - `r` - 重命名
//! - `R` - 刷新

mod actions;
mod connection_list;
mod database_list;
mod filter_panel;
mod routine_panel;
mod state;
mod table_list;
mod trigger_panel;

pub use actions::{SidebarActions, SidebarFocusTransfer};
pub use filter_panel::FilterPanel;
pub use state::{SidebarPanelState, SidebarSelectionState};

use connection_list::ConnectionList;
use database_list::DatabaseList;
use routine_panel::RoutinePanel;
use table_list::TableList;
use trigger_panel::TriggerPanel;

use crate::core::KeyBindings;
use crate::database::ConnectionManager;
use crate::ui::SidebarSection;
use crate::ui::{LocalShortcut, consume_local_shortcut, shortcut_tooltip};
use egui::{self, Color32, CornerRadius, Key, Vec2};

/// 分割条高度
const DIVIDER_HEIGHT: f32 = 6.0;

pub struct Sidebar;

use crate::ui::ColumnFilter;

#[derive(Debug, Clone, Copy)]
struct SidebarFlowState {
    show_connections: bool,
    show_filters: bool,
    show_triggers: bool,
    show_routines: bool,
    has_databases: bool,
    has_tables: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarKeyAction {
    ItemPrev,
    ItemNext,
    ItemStart,
    ItemEnd,
    MoveLeft,
    MoveRight,
    Toggle,
    Delete,
    Activate,
    Edit,
    Rename,
    Refresh,
    InspectSchema,
    AddFilter,
    DeleteFilterAlternative,
    ClearFilters,
    FilterColumnNext,
    FilterColumnPrev,
    FilterOperatorNext,
    FilterOperatorPrev,
    FilterLogicToggle,
    FilterFocusInput,
    FilterCaseToggle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidebarMoveRightTarget {
    Section(SidebarSection),
    DataGrid,
}

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
        keybindings: &KeyBindings,
        filters: &mut Vec<ColumnFilter>,
        columns: &[String],
        pending_focus_filter_input: &mut Option<usize>,
    ) -> (SidebarActions, bool) {
        let mut filter_changed = false;
        let mut actions = SidebarActions::default();
        let ctx = ui.ctx().clone();

        // ====== 面板可见性控制工具栏 ======
        Self::show_visibility_toolbar(ui, panel_state);

        // 处理键盘导航
        let (item_count, selected_index) =
            Self::get_section_info(focused_section, connection_manager, panel_state, filters);
        if item_count == 0 {
            *selected_index = 0;
        } else if *selected_index >= item_count {
            *selected_index = item_count.saturating_sub(1);
        }
        if is_focused {
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
                keybindings,
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
            let filter_panel_result = FilterPanel::show(
                ui,
                keybindings,
                is_focused,
                focused_section,
                panel_state,
                filters,
                columns,
                heights.filters,
                pending_focus_filter_input,
            );
            if filter_panel_result.changed {
                filter_changed = true;
            }
            if filter_panel_result.clicked {
                actions.section_change = Some(SidebarSection::Filters);
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
        if !panel_state.show_connections
            && !panel_state.show_triggers
            && !panel_state.show_routines
            && !panel_state.show_filters
        {
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
                connection_manager
                    .get_active()
                    .map(|c| c.databases.len())
                    .unwrap_or(0),
                &mut panel_state.selection.databases,
            ),
            SidebarSection::Tables => (
                connection_manager
                    .get_active()
                    .map(|c| c.tables.len())
                    .unwrap_or(0),
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
            SidebarSection::Filters => (filters.len(), &mut panel_state.selection.filters),
        }
    }

    /// 计算各面板高度
    /// 面板顺序：连接(0) -> 筛选(1) -> 触发器(2) -> 存储过程(3)
    fn calculate_panel_heights(
        panel_state: &SidebarPanelState,
        available_height: f32,
    ) -> PanelHeights {
        // 统计可见面板
        let visible_panels: Vec<(usize, f32)> = [
            (
                0,
                panel_state.connections_ratio,
                panel_state.show_connections,
            ),
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
            return PanelHeights {
                connections: 0.0,
                filters: 0.0,
                triggers: 0.0,
                routines: 0.0,
            };
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
        let mut heights = PanelHeights {
            connections: 0.0,
            filters: 0.0,
            triggers: 0.0,
            routines: 0.0,
        };

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
    fn show_divider(
        ui: &mut egui::Ui,
        panel_state: &mut SidebarPanelState,
        divider_index: usize,
        width: f32,
    ) {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, DIVIDER_HEIGHT), egui::Sense::drag());

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
                    panel_state.connections_ratio =
                        (panel_state.connections_ratio + delta_ratio).clamp(0.1, 0.8);
                }
                if panel_state.show_filters {
                    panel_state.filters_ratio =
                        (panel_state.filters_ratio - delta_ratio).clamp(0.1, 0.8);
                }
            }
            1 => {
                // 筛选 <-> 触发器
                if panel_state.show_filters {
                    panel_state.filters_ratio =
                        (panel_state.filters_ratio + delta_ratio).clamp(0.1, 0.8);
                }
                if panel_state.show_triggers {
                    panel_state.triggers_ratio =
                        (panel_state.triggers_ratio - delta_ratio).clamp(0.1, 0.8);
                }
            }
            2 => {
                // 触发器 <-> 存储过程
                if panel_state.show_triggers {
                    panel_state.triggers_ratio =
                        (panel_state.triggers_ratio + delta_ratio).clamp(0.1, 0.8);
                }
                if panel_state.show_routines {
                    panel_state.routines_ratio =
                        (panel_state.routines_ratio - delta_ratio).clamp(0.1, 0.8);
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
        let flow_state = Self::sidebar_flow_state(panel_state, connection_manager);

        // 文本输入焦点优先于侧边栏导航，避免筛选值输入时被 j/k/h/l 等快捷键抢占。
        if focused_section == SidebarSection::Filters && panel_state.filter_input_has_focus {
            panel_state.command_buffer.clear();
            return;
        }

        let key_action = Self::detect_key_action(ctx, panel_state);
        let selected_index = match focused_section {
            SidebarSection::Connections => &mut panel_state.selection.connections,
            SidebarSection::Databases => &mut panel_state.selection.databases,
            SidebarSection::Tables => &mut panel_state.selection.tables,
            SidebarSection::Triggers => &mut panel_state.selection.triggers,
            SidebarSection::Routines => &mut panel_state.selection.routines,
            SidebarSection::Filters => &mut panel_state.selection.filters,
        };

        match key_action {
            Some(SidebarKeyAction::ItemNext) => {
                if item_count == 0 || *selected_index >= item_count.saturating_sub(1) {
                    if let Some(next_section) = next_section_in_flow(focused_section, flow_state) {
                        actions.section_change = Some(next_section);
                    }
                } else {
                    *selected_index = (*selected_index + 1).min(item_count.saturating_sub(1));
                }
            }
            Some(SidebarKeyAction::ItemPrev) => {
                if item_count == 0 || *selected_index == 0 {
                    if let Some(previous_section) =
                        previous_section_in_flow(focused_section, flow_state)
                    {
                        actions.section_change = Some(previous_section);
                    }
                } else {
                    *selected_index = selected_index.saturating_sub(1);
                }
            }
            Some(SidebarKeyAction::ItemStart) => {
                *selected_index = 0;
            }
            Some(SidebarKeyAction::ItemEnd) => {
                *selected_index = item_count.saturating_sub(1);
            }
            Some(SidebarKeyAction::Toggle) => {
                // Space：在 Filters section 切换启用状态
                if focused_section == SidebarSection::Filters
                    && let Some(filter) = filters.get_mut(*selected_index)
                {
                    filter.enabled = !filter.enabled;
                    actions.filter_changed = true;
                }
            }
            Some(SidebarKeyAction::Delete) => {
                Self::handle_delete_action(
                    focused_section,
                    *selected_index,
                    connection_manager,
                    filters,
                    actions,
                );
            }
            Some(SidebarKeyAction::MoveLeft) => {
                // h：向上层级导航
                let new_section = match focused_section {
                    SidebarSection::Routines => Some(SidebarSection::Triggers),
                    SidebarSection::Triggers => Some(SidebarSection::Filters),
                    SidebarSection::Filters => Some(SidebarSection::Tables),
                    SidebarSection::Tables => {
                        if connection_manager
                            .get_active()
                            .map(|c| !c.databases.is_empty())
                            .unwrap_or(false)
                        {
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
            Some(SidebarKeyAction::MoveRight) => {
                match move_right_target(focused_section, flow_state) {
                    SidebarMoveRightTarget::Section(section) => {
                        actions.section_change = Some(section);
                    }
                    SidebarMoveRightTarget::DataGrid => {
                        actions.focus_transfer = Some(SidebarFocusTransfer::ToDataGrid);
                    }
                }
            }
            Some(SidebarKeyAction::InspectSchema) => {
                if let SidebarSection::Tables = focused_section
                    && let Some(conn) = connection_manager.get_active()
                    && let Some(table) = conn.tables.get(*selected_index)
                {
                    actions.show_table_schema = Some(table.clone());
                }
            }
            Some(SidebarKeyAction::Activate) => {
                match focused_section {
                    SidebarSection::Connections => {
                        let mut names: Vec<_> =
                            connection_manager.connections.keys().cloned().collect();
                        names.sort_unstable();
                        if let Some(name) = names.get(*selected_index) {
                            actions.connect = Some(name.clone());
                        }
                    }
                    SidebarSection::Databases => {
                        if let Some(conn) = connection_manager.get_active()
                            && let Some(db) = conn.databases.get(*selected_index)
                        {
                            actions.select_database = Some(db.clone());
                        }
                    }
                    SidebarSection::Tables => {
                        if let Some(conn) = connection_manager.get_active()
                            && let Some(table) = conn.tables.get(*selected_index)
                        {
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
            Some(SidebarKeyAction::Edit) if focused_section == SidebarSection::Connections => {
                let mut names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                names.sort_unstable();
                if let Some(name) = names.get(*selected_index) {
                    actions.edit_connection = Some(name.clone());
                }
            }
            Some(SidebarKeyAction::Rename) => {
                let item_name = match focused_section {
                    SidebarSection::Connections => {
                        let mut names: Vec<_> =
                            connection_manager.connections.keys().cloned().collect();
                        names.sort_unstable();
                        names.get(*selected_index).cloned()
                    }
                    SidebarSection::Tables => connection_manager
                        .get_active()
                        .and_then(|c| c.tables.get(*selected_index).cloned()),
                    _ => None,
                };
                if let Some(name) = item_name {
                    actions.rename_item = Some((focused_section, name));
                }
            }
            Some(SidebarKeyAction::Refresh) => {
                actions.refresh = true;
            }
            Some(SidebarKeyAction::AddFilter) if focused_section == SidebarSection::Filters => {
                actions.add_filter = true;
            }
            Some(SidebarKeyAction::DeleteFilterAlternative)
                if focused_section == SidebarSection::Filters
                    && *selected_index < filters.len() =>
            {
                filters.remove(*selected_index);
                if *selected_index >= filters.len() && !filters.is_empty() {
                    *selected_index = filters.len() - 1;
                }
                actions.filter_changed = true;
            }
            Some(SidebarKeyAction::ClearFilters) if focused_section == SidebarSection::Filters => {
                actions.clear_filters = true;
            }
            Some(SidebarKeyAction::FilterColumnNext)
                if focused_section == SidebarSection::Filters
                    && *selected_index < filters.len() =>
            {
                actions.cycle_filter_column = Some((*selected_index, true));
            }
            Some(SidebarKeyAction::FilterColumnPrev)
                if focused_section == SidebarSection::Filters
                    && *selected_index < filters.len() =>
            {
                actions.cycle_filter_column = Some((*selected_index, false));
            }
            Some(SidebarKeyAction::FilterOperatorNext) => {
                if focused_section == SidebarSection::Filters
                    && let Some(filter) = filters.get_mut(*selected_index)
                {
                    filter.operator = next_operator(&filter.operator);
                    actions.filter_changed = true;
                }
            }
            Some(SidebarKeyAction::FilterOperatorPrev) => {
                if focused_section == SidebarSection::Filters
                    && let Some(filter) = filters.get_mut(*selected_index)
                {
                    filter.operator = prev_operator(&filter.operator);
                    actions.filter_changed = true;
                }
            }
            Some(SidebarKeyAction::FilterLogicToggle)
                if focused_section == SidebarSection::Filters
                    && *selected_index < filters.len() =>
            {
                actions.toggle_filter_logic = Some(*selected_index);
            }
            Some(SidebarKeyAction::FilterFocusInput)
                if focused_section == SidebarSection::Filters
                    && *selected_index < filters.len() =>
            {
                actions.focus_filter_input = Some(*selected_index);
            }
            Some(SidebarKeyAction::FilterCaseToggle) => {
                if focused_section == SidebarSection::Filters
                    && panel_state.command_buffer.is_empty()
                    && let Some(filter) = filters.get_mut(*selected_index)
                    && filter.operator.supports_case_sensitivity()
                {
                    filter.case_sensitive = !filter.case_sensitive;
                    actions.filter_changed = true;
                }
            }
            _ => {}
        }

        // 同步到旧的 trigger_selected_index 字段（保持向后兼容）
        if focused_section == SidebarSection::Triggers {
            panel_state.trigger_selected_index = panel_state.selection.triggers;
        }
    }

    fn detect_key_action(
        ctx: &egui::Context,
        panel_state: &mut SidebarPanelState,
    ) -> Option<SidebarKeyAction> {
        ctx.input_mut(|i| {
            if i.key_pressed(Key::G) && i.modifiers.is_none() {
                if panel_state.command_buffer == "g" {
                    panel_state.command_buffer.clear();
                    return Some(SidebarKeyAction::ItemStart);
                }

                panel_state.command_buffer.clear();
                panel_state.command_buffer.push('g');
                return None;
            }

            if i.key_pressed(Key::S) && i.modifiers.is_none() && panel_state.command_buffer == "g" {
                panel_state.command_buffer.clear();
                return Some(SidebarKeyAction::InspectSchema);
            }

            let action = if consume_local_shortcut(i, LocalShortcut::SidebarItemNext) {
                Some(SidebarKeyAction::ItemNext)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarItemPrev) {
                Some(SidebarKeyAction::ItemPrev)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarItemStart) {
                Some(SidebarKeyAction::ItemStart)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarItemEnd) {
                Some(SidebarKeyAction::ItemEnd)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarMoveLeft) {
                Some(SidebarKeyAction::MoveLeft)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarMoveRight) {
                Some(SidebarKeyAction::MoveRight)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarToggle) {
                Some(SidebarKeyAction::Toggle)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarDelete) {
                Some(SidebarKeyAction::Delete)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarActivate) {
                Some(SidebarKeyAction::Activate)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarEdit) {
                Some(SidebarKeyAction::Edit)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarRename) {
                Some(SidebarKeyAction::Rename)
            } else if consume_local_shortcut(i, LocalShortcut::SidebarRefresh) {
                Some(SidebarKeyAction::Refresh)
            } else if consume_local_shortcut(i, LocalShortcut::FilterAdd) {
                Some(SidebarKeyAction::AddFilter)
            } else if consume_local_shortcut(i, LocalShortcut::FilterDelete) {
                Some(SidebarKeyAction::DeleteFilterAlternative)
            } else if consume_local_shortcut(i, LocalShortcut::FilterClearAll) {
                Some(SidebarKeyAction::ClearFilters)
            } else if consume_local_shortcut(i, LocalShortcut::FilterColumnNext) {
                Some(SidebarKeyAction::FilterColumnNext)
            } else if consume_local_shortcut(i, LocalShortcut::FilterColumnPrev) {
                Some(SidebarKeyAction::FilterColumnPrev)
            } else if consume_local_shortcut(i, LocalShortcut::FilterOperatorNext) {
                Some(SidebarKeyAction::FilterOperatorNext)
            } else if consume_local_shortcut(i, LocalShortcut::FilterOperatorPrev) {
                Some(SidebarKeyAction::FilterOperatorPrev)
            } else if consume_local_shortcut(i, LocalShortcut::FilterLogicToggle) {
                Some(SidebarKeyAction::FilterLogicToggle)
            } else if consume_local_shortcut(i, LocalShortcut::FilterFocusInput) {
                Some(SidebarKeyAction::FilterFocusInput)
            } else if consume_local_shortcut(i, LocalShortcut::FilterCaseToggle) {
                Some(SidebarKeyAction::FilterCaseToggle)
            } else {
                None
            };

            if action.is_some() {
                panel_state.command_buffer.clear();
            }

            action
        })
    }

    /// 处理删除操作
    fn handle_delete_action(
        focused_section: SidebarSection,
        selected_index: usize,
        connection_manager: &ConnectionManager,
        filters: &mut Vec<crate::ui::ColumnFilter>,
        actions: &mut SidebarActions,
    ) {
        match focused_section {
            SidebarSection::Connections => {
                let mut names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                names.sort_unstable();
                if let Some(name) = names.get(selected_index) {
                    actions.delete = Some(name.clone());
                }
            }
            SidebarSection::Tables => {
                if let Some(conn) = connection_manager.get_active()
                    && let Some(table) = conn.tables.get(selected_index)
                {
                    actions.delete = Some(format!("table:{}", table));
                }
            }
            SidebarSection::Filters => {
                if selected_index < filters.len() {
                    filters.remove(selected_index);
                    actions.filter_changed = true;
                }
            }
            _ => {}
        }
    }

    /// 显示面板可见性控制工具栏
    fn show_visibility_toolbar(ui: &mut egui::Ui, panel_state: &mut SidebarPanelState) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;

            // 无边框图标按钮
            let icon_toggle =
                |ui: &mut egui::Ui, icon: &str, active: bool, tooltip: &str| -> bool {
                    let color = if active {
                        Color32::from_rgb(100, 200, 150)
                    } else {
                        Color32::from_gray(100)
                    };
                    ui.add(
                        egui::Button::new(egui::RichText::new(icon).size(14.0).color(color))
                            .frame(false)
                            .min_size(egui::Vec2::new(22.0, 22.0)),
                    )
                    .on_hover_text(tooltip)
                    .clicked()
                };

            // 1. 连接面板
            if icon_toggle(
                ui,
                "🔗",
                panel_state.show_connections,
                &shortcut_tooltip("切换连接面板", &["Ctrl+1"]),
            ) {
                panel_state.show_connections = !panel_state.show_connections;
            }

            // 2-3. 数据库和表在连接面板内，无需单独按钮

            // 4. 筛选面板
            if icon_toggle(
                ui,
                "🔍",
                panel_state.show_filters,
                &shortcut_tooltip("切换筛选面板", &["Ctrl+4"]),
            ) {
                panel_state.show_filters = !panel_state.show_filters;
            }

            // 5. 触发器面板
            if icon_toggle(
                ui,
                "⚡",
                panel_state.show_triggers,
                &shortcut_tooltip("切换触发器面板", &["Ctrl+5"]),
            ) {
                panel_state.show_triggers = !panel_state.show_triggers;
            }

            // 6. 存储过程面板
            if icon_toggle(
                ui,
                "📦",
                panel_state.show_routines,
                &shortcut_tooltip("切换存储过程面板", &["Ctrl+6"]),
            ) {
                panel_state.show_routines = !panel_state.show_routines;
            }
        });

        ui.separator();
    }

    fn sidebar_flow_state(
        panel_state: &SidebarPanelState,
        connection_manager: &ConnectionManager,
    ) -> SidebarFlowState {
        let active_connection = connection_manager.get_active();

        SidebarFlowState {
            show_connections: panel_state.show_connections,
            show_filters: panel_state.show_filters,
            show_triggers: panel_state.show_triggers,
            show_routines: panel_state.show_routines,
            has_databases: active_connection
                .map(|connection| !connection.databases.is_empty())
                .unwrap_or(false),
            has_tables: active_connection
                .map(|connection| !connection.tables.is_empty())
                .unwrap_or(false),
        }
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

fn next_section_in_flow(current: SidebarSection, flow: SidebarFlowState) -> Option<SidebarSection> {
    match current {
        SidebarSection::Connections => {
            if flow.show_connections && flow.has_databases {
                Some(SidebarSection::Databases)
            } else if flow.show_connections && flow.has_tables {
                Some(SidebarSection::Tables)
            } else if flow.show_filters {
                Some(SidebarSection::Filters)
            } else if flow.show_triggers {
                Some(SidebarSection::Triggers)
            } else if flow.show_routines {
                Some(SidebarSection::Routines)
            } else {
                None
            }
        }
        SidebarSection::Databases => {
            if flow.show_connections && flow.has_tables {
                Some(SidebarSection::Tables)
            } else if flow.show_filters {
                Some(SidebarSection::Filters)
            } else if flow.show_triggers {
                Some(SidebarSection::Triggers)
            } else if flow.show_routines {
                Some(SidebarSection::Routines)
            } else {
                None
            }
        }
        SidebarSection::Tables => {
            if flow.show_filters {
                Some(SidebarSection::Filters)
            } else if flow.show_triggers {
                Some(SidebarSection::Triggers)
            } else if flow.show_routines {
                Some(SidebarSection::Routines)
            } else {
                None
            }
        }
        SidebarSection::Filters => {
            if flow.show_triggers {
                Some(SidebarSection::Triggers)
            } else if flow.show_routines {
                Some(SidebarSection::Routines)
            } else {
                None
            }
        }
        SidebarSection::Triggers => {
            if flow.show_routines {
                Some(SidebarSection::Routines)
            } else {
                None
            }
        }
        SidebarSection::Routines => None,
    }
}

fn move_right_target(current: SidebarSection, flow: SidebarFlowState) -> SidebarMoveRightTarget {
    match current {
        SidebarSection::Connections => {
            if flow.show_connections && flow.has_databases {
                SidebarMoveRightTarget::Section(SidebarSection::Databases)
            } else if flow.show_connections && flow.has_tables {
                SidebarMoveRightTarget::Section(SidebarSection::Tables)
            } else {
                SidebarMoveRightTarget::DataGrid
            }
        }
        SidebarSection::Databases => {
            if flow.show_connections && flow.has_tables {
                SidebarMoveRightTarget::Section(SidebarSection::Tables)
            } else {
                SidebarMoveRightTarget::DataGrid
            }
        }
        SidebarSection::Tables
        | SidebarSection::Filters
        | SidebarSection::Triggers
        | SidebarSection::Routines => SidebarMoveRightTarget::DataGrid,
    }
}

fn previous_section_in_flow(
    current: SidebarSection,
    flow: SidebarFlowState,
) -> Option<SidebarSection> {
    match current {
        SidebarSection::Connections => None,
        SidebarSection::Databases => {
            if flow.show_connections {
                Some(SidebarSection::Connections)
            } else {
                None
            }
        }
        SidebarSection::Tables => {
            if flow.show_connections && flow.has_databases {
                Some(SidebarSection::Databases)
            } else if flow.show_connections {
                Some(SidebarSection::Connections)
            } else {
                None
            }
        }
        SidebarSection::Filters => {
            if flow.show_connections && flow.has_tables {
                Some(SidebarSection::Tables)
            } else if flow.show_connections && flow.has_databases {
                Some(SidebarSection::Databases)
            } else if flow.show_connections {
                Some(SidebarSection::Connections)
            } else {
                None
            }
        }
        SidebarSection::Triggers => {
            if flow.show_filters {
                Some(SidebarSection::Filters)
            } else if flow.show_connections && flow.has_tables {
                Some(SidebarSection::Tables)
            } else if flow.show_connections && flow.has_databases {
                Some(SidebarSection::Databases)
            } else if flow.show_connections {
                Some(SidebarSection::Connections)
            } else {
                None
            }
        }
        SidebarSection::Routines => {
            if flow.show_triggers {
                Some(SidebarSection::Triggers)
            } else if flow.show_filters {
                Some(SidebarSection::Filters)
            } else if flow.show_connections && flow.has_tables {
                Some(SidebarSection::Tables)
            } else if flow.show_connections && flow.has_databases {
                Some(SidebarSection::Databases)
            } else if flow.show_connections {
                Some(SidebarSection::Connections)
            } else {
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SidebarFlowState, SidebarMoveRightTarget, move_right_target, next_section_in_flow,
        previous_section_in_flow,
    };
    use crate::ui::SidebarSection;

    #[test]
    fn tables_move_down_into_filters_when_filter_panel_is_open() {
        let flow = SidebarFlowState {
            show_connections: true,
            show_filters: true,
            show_triggers: false,
            show_routines: false,
            has_databases: true,
            has_tables: true,
        };

        assert_eq!(
            next_section_in_flow(SidebarSection::Tables, flow),
            Some(SidebarSection::Filters)
        );
    }

    #[test]
    fn tables_move_right_enters_data_grid_instead_of_next_sidebar_panel() {
        let flow = SidebarFlowState {
            show_connections: true,
            show_filters: true,
            show_triggers: true,
            show_routines: false,
            has_databases: true,
            has_tables: true,
        };

        assert_eq!(
            move_right_target(SidebarSection::Tables, flow),
            SidebarMoveRightTarget::DataGrid
        );
    }

    #[test]
    fn filters_move_right_enters_data_grid() {
        let flow = SidebarFlowState {
            show_connections: true,
            show_filters: true,
            show_triggers: true,
            show_routines: true,
            has_databases: true,
            has_tables: true,
        };

        assert_eq!(
            move_right_target(SidebarSection::Filters, flow),
            SidebarMoveRightTarget::DataGrid
        );
    }

    #[test]
    fn connections_move_right_prefers_database_hierarchy_before_grid() {
        let flow = SidebarFlowState {
            show_connections: true,
            show_filters: true,
            show_triggers: false,
            show_routines: false,
            has_databases: true,
            has_tables: true,
        };

        assert_eq!(
            move_right_target(SidebarSection::Connections, flow),
            SidebarMoveRightTarget::Section(SidebarSection::Databases)
        );
    }

    #[test]
    fn filters_move_up_back_to_tables_in_default_learning_flow() {
        let flow = SidebarFlowState {
            show_connections: true,
            show_filters: true,
            show_triggers: false,
            show_routines: false,
            has_databases: true,
            has_tables: true,
        };

        assert_eq!(
            previous_section_in_flow(SidebarSection::Filters, flow),
            Some(SidebarSection::Tables)
        );
    }

    #[test]
    fn filters_fall_through_to_triggers_when_enabled() {
        let flow = SidebarFlowState {
            show_connections: true,
            show_filters: true,
            show_triggers: true,
            show_routines: false,
            has_databases: false,
            has_tables: true,
        };

        assert_eq!(
            next_section_in_flow(SidebarSection::Filters, flow),
            Some(SidebarSection::Triggers)
        );
    }
}
