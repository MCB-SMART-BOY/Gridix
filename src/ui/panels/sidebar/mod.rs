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

pub use actions::{
    SidebarActions, SidebarDeleteTarget, SidebarFilterInsertMode, SidebarFocusTransfer,
};
pub use filter_panel::FilterPanel;
pub use state::{
    SidebarFilterWorkspaceMode, SidebarPanelState, SidebarSelectionState, SidebarWorkflowState,
};

use connection_list::ConnectionList;
use database_list::DatabaseList;
use routine_panel::RoutinePanel;
use state::{
    SidebarWorkflowAction, SidebarWorkflowContext, SidebarWorkflowEffect, SidebarWorkflowReduction,
    reduce_sidebar_workflow,
};
use table_list::TableList;
use trigger_panel::TriggerPanel;

use crate::core::KeyBindings;
use crate::database::ConnectionManager;
use crate::ui::SidebarSection;
use crate::ui::{
    LocalShortcut, consume_local_shortcut_with_text_priority, shortcut_tooltip,
    text_entry_has_priority,
};
use egui::{self, Color32, CornerRadius, Key, Vec2};

/// 分割条高度
const DIVIDER_HEIGHT: f32 = 6.0;

pub struct Sidebar;

use crate::ui::ColumnFilter;

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
    AddFilterBelow,
    AppendFilter,
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
                let workflow_context =
                    Self::sidebar_workflow_context(panel_state, connection_manager);
                let reduction = reduce_sidebar_workflow(
                    &mut panel_state.workflow,
                    workflow_context,
                    SidebarWorkflowAction::FocusSection(SidebarSection::Filters),
                );
                Self::apply_workflow_reduction(&mut actions, reduction);
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
                ui.label(
                    egui::RichText::new("按 Ctrl+1 或 Ctrl+4 打开连接/筛选工作区")
                        .color(Color32::GRAY),
                );
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
        let workflow_context = Self::sidebar_workflow_context(panel_state, connection_manager);

        // 筛选输入是独立的 text-entry 工作区，只接受退出输入动作。
        if focused_section == SidebarSection::Filters && panel_state.filter_input_mode() {
            if Self::detect_filter_input_exit(ctx) {
                let reduction = reduce_sidebar_workflow(
                    &mut panel_state.workflow,
                    workflow_context,
                    SidebarWorkflowAction::ExitFilterInput,
                );
                panel_state.filter_input_has_focus = false;
                Self::apply_workflow_reduction(actions, reduction);
            }
            panel_state.command_buffer.clear();
            return;
        }

        if focused_section == SidebarSection::Filters && Self::detect_filter_list_back(ctx) {
            let reduction = reduce_sidebar_workflow(
                &mut panel_state.workflow,
                workflow_context,
                SidebarWorkflowAction::MoveLeft {
                    current: focused_section,
                },
            );
            panel_state.filter_input_has_focus = false;
            Self::apply_workflow_reduction(actions, reduction);
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
                    let reduction = reduce_sidebar_workflow(
                        &mut panel_state.workflow,
                        workflow_context,
                        SidebarWorkflowAction::EdgeNext {
                            current: focused_section,
                        },
                    );
                    Self::apply_workflow_reduction(actions, reduction);
                } else {
                    *selected_index = (*selected_index + 1).min(item_count.saturating_sub(1));
                }
            }
            Some(SidebarKeyAction::ItemPrev) => {
                if item_count == 0 || *selected_index == 0 {
                    let reduction = reduce_sidebar_workflow(
                        &mut panel_state.workflow,
                        workflow_context,
                        SidebarWorkflowAction::EdgePrevious {
                            current: focused_section,
                        },
                    );
                    Self::apply_workflow_reduction(actions, reduction);
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
                if focused_section == SidebarSection::Filters
                    && let Some(filter) = filters.get_mut(*selected_index)
                {
                    filter.enabled = !filter.enabled;
                    actions.filter_changed = true;
                }
            }
            Some(SidebarKeyAction::Delete) => {
                if focused_section == SidebarSection::Filters {
                    Self::remove_selected_filter(selected_index, filters, actions);
                } else {
                    Self::handle_delete_action(
                        focused_section,
                        *selected_index,
                        connection_manager,
                        filters,
                        actions,
                    );
                }
            }
            Some(SidebarKeyAction::MoveLeft) => {
                let reduction = reduce_sidebar_workflow(
                    &mut panel_state.workflow,
                    workflow_context,
                    SidebarWorkflowAction::MoveLeft {
                        current: focused_section,
                    },
                );
                panel_state.filter_input_has_focus = false;
                Self::apply_workflow_reduction(actions, reduction);
            }
            Some(SidebarKeyAction::MoveRight) => {
                let filter_needs_value = filters
                    .get(*selected_index)
                    .is_some_and(|filter| filter.operator.needs_value());
                let reduction = reduce_sidebar_workflow(
                    &mut panel_state.workflow,
                    workflow_context,
                    SidebarWorkflowAction::MoveRight {
                        current: focused_section,
                        selected_filter_index: *selected_index,
                        filter_needs_value,
                    },
                );
                Self::apply_workflow_reduction(actions, reduction);
            }
            Some(SidebarKeyAction::InspectSchema) => {
                if let SidebarSection::Tables = focused_section
                    && let Some(conn) = connection_manager.get_active()
                    && let Some(table) = conn.tables.get(*selected_index)
                {
                    actions.show_table_schema = Some(table.clone());
                }
            }
            Some(SidebarKeyAction::Activate) => match focused_section {
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
                    if let Some(filter) = filters.get_mut(*selected_index) {
                        filter.enabled = !filter.enabled;
                        actions.filter_changed = true;
                    }
                }
            },
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
            Some(SidebarKeyAction::AddFilterBelow)
                if focused_section == SidebarSection::Filters =>
            {
                actions.insert_filter = Some(SidebarFilterInsertMode::BelowSelection);
            }
            Some(SidebarKeyAction::AppendFilter) if focused_section == SidebarSection::Filters => {
                actions.insert_filter = Some(SidebarFilterInsertMode::AppendEnd);
            }
            Some(SidebarKeyAction::DeleteFilterAlternative)
                if focused_section == SidebarSection::Filters
                    && *selected_index < filters.len() =>
            {
                Self::remove_selected_filter(selected_index, filters, actions);
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
                    && *selected_index < filters.len()
                    && filters[*selected_index].operator.needs_value() =>
            {
                let reduction = reduce_sidebar_workflow(
                    &mut panel_state.workflow,
                    workflow_context,
                    SidebarWorkflowAction::EnterFilterInput {
                        index: *selected_index,
                        filter_needs_value: true,
                    },
                );
                Self::apply_workflow_reduction(actions, reduction);
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

        if focused_section == SidebarSection::Triggers {
            panel_state.trigger_selected_index = panel_state.selection.triggers;
        }
    }

    fn detect_key_action(
        ctx: &egui::Context,
        panel_state: &mut SidebarPanelState,
    ) -> Option<SidebarKeyAction> {
        let text_entry_active = text_entry_has_priority(ctx);
        ctx.input_mut(|i| {
            if text_entry_active {
                panel_state.command_buffer.clear();
                return None;
            }

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

            let action = if i.key_pressed(Key::A) && i.modifiers.shift_only() {
                Some(SidebarKeyAction::AppendFilter)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarItemNext,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::ItemNext)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarItemPrev,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::ItemPrev)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarItemStart,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::ItemStart)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarItemEnd,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::ItemEnd)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarMoveLeft,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::MoveLeft)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarMoveRight,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::MoveRight)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarToggle,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::Toggle)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarDelete,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::Delete)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarActivate,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::Activate)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarEdit,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::Edit)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarRename,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::Rename)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::SidebarRefresh,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::Refresh)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterAdd,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::AddFilterBelow)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterDelete,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::DeleteFilterAlternative)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterClearAll,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::ClearFilters)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterColumnNext,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::FilterColumnNext)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterColumnPrev,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::FilterColumnPrev)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterOperatorNext,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::FilterOperatorNext)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterOperatorPrev,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::FilterOperatorPrev)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterLogicToggle,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::FilterLogicToggle)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterFocusInput,
                text_entry_active,
            ) {
                Some(SidebarKeyAction::FilterFocusInput)
            } else if consume_local_shortcut_with_text_priority(
                i,
                LocalShortcut::FilterCaseToggle,
                text_entry_active,
            ) {
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

    fn detect_filter_input_exit(ctx: &egui::Context) -> bool {
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape))
    }

    fn detect_filter_list_back(ctx: &egui::Context) -> bool {
        ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, Key::Escape))
    }

    fn remove_selected_filter(
        selected_index: &mut usize,
        filters: &mut Vec<crate::ui::ColumnFilter>,
        actions: &mut SidebarActions,
    ) {
        if *selected_index >= filters.len() {
            return;
        }

        filters.remove(*selected_index);
        if filters.is_empty() {
            *selected_index = 0;
        } else if *selected_index >= filters.len() {
            *selected_index = filters.len() - 1;
        }
        actions.filter_changed = true;
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
                    ConnectionList::request_connection_delete(name, actions);
                }
            }
            SidebarSection::Tables => {
                if let Some(conn) = connection_manager.get_active()
                    && let Some(table) = conn.tables.get(selected_index)
                {
                    ConnectionList::request_table_delete(&conn.config.name, table, actions);
                }
            }
            SidebarSection::Databases => {
                if let Some(conn) = connection_manager.get_active()
                    && let Some(database) = conn.databases.get(selected_index)
                {
                    ConnectionList::request_database_delete(&conn.config.name, database, actions);
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
        let toggle_chip = |ui: &mut egui::Ui,
                           label: &str,
                           active: bool,
                           tooltip: &str,
                           accent: Color32|
         -> bool {
            let text_color = if active {
                accent
            } else {
                Color32::from_gray(130)
            };
            let fill = if active {
                Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 36)
            } else {
                Color32::from_rgba_unmultiplied(60, 60, 68, 20)
            };

            ui.add(
                egui::Button::new(egui::RichText::new(label).size(11.0).color(text_color))
                    .fill(fill)
                    .stroke(egui::Stroke::NONE)
                    .corner_radius(CornerRadius::same(255))
                    .min_size(egui::Vec2::new(0.0, 22.0)),
            )
            .on_hover_text(tooltip)
            .clicked()
        };

        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                ui.label(
                    egui::RichText::new("工作区")
                        .small()
                        .color(Color32::from_gray(120)),
                );

                if toggle_chip(
                    ui,
                    "连接",
                    panel_state.show_connections,
                    &shortcut_tooltip("切换连接工作区", &["Ctrl+1"]),
                    Color32::from_rgb(100, 200, 150),
                ) {
                    panel_state.show_connections = !panel_state.show_connections;
                }

                if toggle_chip(
                    ui,
                    "筛选",
                    panel_state.show_filters,
                    &shortcut_tooltip("切换筛选工作区", &["Ctrl+4"]),
                    Color32::from_rgb(120, 185, 255),
                ) {
                    panel_state.show_filters = !panel_state.show_filters;
                }
            });

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(6.0, 0.0);
                ui.label(
                    egui::RichText::new("高级")
                        .small()
                        .color(Color32::from_gray(110)),
                );

                if toggle_chip(
                    ui,
                    "触发器",
                    panel_state.show_triggers,
                    &shortcut_tooltip("切换触发器面板", &["Ctrl+5"]),
                    Color32::from_rgb(230, 180, 90),
                ) {
                    panel_state.show_triggers = !panel_state.show_triggers;
                }

                if toggle_chip(
                    ui,
                    "过程",
                    panel_state.show_routines,
                    &shortcut_tooltip("切换存储过程面板", &["Ctrl+6"]),
                    Color32::from_rgb(170, 150, 220),
                ) {
                    panel_state.show_routines = !panel_state.show_routines;
                }
            });
        });

        ui.separator();
    }

    fn sidebar_workflow_context(
        panel_state: &SidebarPanelState,
        connection_manager: &ConnectionManager,
    ) -> SidebarWorkflowContext {
        let active_connection = connection_manager.get_active();

        SidebarWorkflowContext::new(
            panel_state.show_connections,
            panel_state.show_filters,
            panel_state.show_triggers,
            panel_state.show_routines,
            active_connection
                .map(|connection| !connection.databases.is_empty())
                .unwrap_or(false),
            active_connection
                .map(|connection| !connection.tables.is_empty())
                .unwrap_or(false),
        )
    }

    fn apply_workflow_reduction(actions: &mut SidebarActions, reduction: SidebarWorkflowReduction) {
        match reduction.effect {
            Some(SidebarWorkflowEffect::SectionChanged(section)) => {
                actions.section_change = Some(section);
            }
            Some(SidebarWorkflowEffect::FocusFilterInput(index)) => {
                actions.focus_filter_input = Some(index);
            }
            Some(SidebarWorkflowEffect::FocusTransferToDataGrid) => {
                actions.focus_transfer = Some(SidebarFocusTransfer::ToDataGrid);
            }
            None => {}
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{ConnectionConfig, ConnectionManager};
    use crate::ui::{ColumnFilter, SidebarDeleteTarget, SidebarFilterWorkspaceMode};
    use egui::{Context, Event, Key, Modifiers, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn key_event_with_modifiers(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn run_sidebar_key(
        event: Event,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        item_count: usize,
        connection_manager: &ConnectionManager,
        filters: &mut Vec<ColumnFilter>,
    ) -> SidebarActions {
        let modifiers = match &event {
            Event::Key { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        };
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            events: vec![event],
            modifiers,
            ..Default::default()
        });

        let mut actions = SidebarActions::default();
        let mut selected_table = None;
        Sidebar::handle_keyboard_navigation(
            &ctx,
            focused_section,
            panel_state,
            item_count,
            connection_manager,
            &mut selected_table,
            filters,
            &mut actions,
        );
        let _ = ctx.end_pass();
        actions
    }

    fn active_manager_with_tables() -> ConnectionManager {
        let mut manager = ConnectionManager::default();
        let config = ConnectionConfig {
            name: "primary".to_string(),
            ..ConnectionConfig::default()
        };
        manager.add(config);
        manager.active = Some("primary".to_string());

        let connection = manager
            .connections
            .get_mut("primary")
            .expect("active connection");
        connection.connected = true;
        connection.databases = vec!["main".to_string()];
        connection.selected_database = Some("main".to_string());
        connection.tables = vec!["users".to_string(), "orders".to_string()];
        manager
    }

    #[test]
    fn typing_plain_characters_in_filters_input_does_not_trigger_external_commands() {
        let manager = ConnectionManager::default();
        let mut panel_state = SidebarPanelState::default();
        panel_state.workflow.filter_workspace = SidebarFilterWorkspaceMode::Input;
        panel_state.filter_input_has_focus = true;
        panel_state.selection.filters = 0;
        let mut filters = vec![ColumnFilter::new("name".to_string())];

        let actions = run_sidebar_key(
            key_event(Key::A),
            SidebarSection::Filters,
            &mut panel_state,
            filters.len(),
            &manager,
            &mut filters,
        );

        assert!(!actions.has_action());
        assert_eq!(filters.len(), 1);
        assert_eq!(
            panel_state.workflow.filter_workspace,
            SidebarFilterWorkspaceMode::Input
        );
        assert!(panel_state.filter_input_has_focus);
    }

    #[test]
    fn escape_in_filters_input_returns_to_filters_list() {
        let manager = ConnectionManager::default();
        let mut panel_state = SidebarPanelState::default();
        panel_state.workflow.filter_workspace = SidebarFilterWorkspaceMode::Input;
        panel_state.filter_input_has_focus = true;
        panel_state.selection.filters = 0;
        let mut filters = vec![ColumnFilter::new("name".to_string())];

        let actions = run_sidebar_key(
            key_event(Key::Escape),
            SidebarSection::Filters,
            &mut panel_state,
            filters.len(),
            &manager,
            &mut filters,
        );

        assert!(!actions.has_action());
        assert_eq!(
            panel_state.workflow.filter_workspace,
            SidebarFilterWorkspaceMode::List
        );
        assert!(!panel_state.filter_input_has_focus);
    }

    #[test]
    fn escape_in_filters_list_moves_back_to_previous_layer() {
        let manager = active_manager_with_tables();
        let mut panel_state = SidebarPanelState::default();
        panel_state.selection.filters = 0;
        let mut filters = vec![ColumnFilter::new("name".to_string())];

        let actions = run_sidebar_key(
            key_event(Key::Escape),
            SidebarSection::Filters,
            &mut panel_state,
            filters.len(),
            &manager,
            &mut filters,
        );

        assert_eq!(actions.section_change, Some(SidebarSection::Tables));
    }

    #[test]
    fn filters_keyboard_commands_cover_add_toggle_edit_and_remove() {
        let manager = ConnectionManager::default();

        let mut add_state = SidebarPanelState::default();
        add_state.selection.filters = 0;
        let mut add_filters = vec![ColumnFilter::new("name".to_string())];
        let add_actions = run_sidebar_key(
            key_event(Key::A),
            SidebarSection::Filters,
            &mut add_state,
            add_filters.len(),
            &manager,
            &mut add_filters,
        );
        assert_eq!(
            add_actions.insert_filter,
            Some(SidebarFilterInsertMode::BelowSelection)
        );

        let mut append_state = SidebarPanelState::default();
        append_state.selection.filters = 0;
        let mut append_filters = vec![ColumnFilter::new("name".to_string())];
        let append_actions = run_sidebar_key(
            key_event_with_modifiers(
                Key::A,
                Modifiers {
                    shift: true,
                    ..Modifiers::NONE
                },
            ),
            SidebarSection::Filters,
            &mut append_state,
            append_filters.len(),
            &manager,
            &mut append_filters,
        );
        assert_eq!(
            append_actions.insert_filter,
            Some(SidebarFilterInsertMode::AppendEnd)
        );

        let mut toggle_state = SidebarPanelState::default();
        toggle_state.selection.filters = 0;
        let mut toggle_filters = vec![ColumnFilter::new("name".to_string())];
        let toggle_actions = run_sidebar_key(
            key_event(Key::Space),
            SidebarSection::Filters,
            &mut toggle_state,
            toggle_filters.len(),
            &manager,
            &mut toggle_filters,
        );
        assert!(toggle_actions.filter_changed);
        assert!(!toggle_filters[0].enabled);

        let mut focus_state = SidebarPanelState::default();
        focus_state.selection.filters = 0;
        let mut focus_filters = vec![ColumnFilter::new("name".to_string())];
        let focus_actions = run_sidebar_key(
            key_event(Key::L),
            SidebarSection::Filters,
            &mut focus_state,
            focus_filters.len(),
            &manager,
            &mut focus_filters,
        );
        assert_eq!(focus_actions.focus_filter_input, Some(0));
        assert_eq!(
            focus_state.workflow.filter_workspace,
            SidebarFilterWorkspaceMode::Input
        );

        let mut delete_state = SidebarPanelState::default();
        delete_state.selection.filters = 0;
        let mut delete_filters = vec![ColumnFilter::new("name".to_string())];
        let delete_actions = run_sidebar_key(
            key_event(Key::X),
            SidebarSection::Filters,
            &mut delete_state,
            delete_filters.len(),
            &manager,
            &mut delete_filters,
        );
        assert!(delete_actions.filter_changed);
        assert!(delete_filters.is_empty());
    }

    #[test]
    fn filters_keyboard_commands_cover_column_and_operator_cycle() {
        let manager = ConnectionManager::default();

        let mut column_state = SidebarPanelState::default();
        column_state.selection.filters = 0;
        let mut column_filters = vec![ColumnFilter::new("name".to_string())];
        let next_column_actions = run_sidebar_key(
            key_event(Key::CloseBracket),
            SidebarSection::Filters,
            &mut column_state,
            column_filters.len(),
            &manager,
            &mut column_filters,
        );
        assert_eq!(next_column_actions.cycle_filter_column, Some((0, true)));

        let prev_column_actions = run_sidebar_key(
            key_event(Key::OpenBracket),
            SidebarSection::Filters,
            &mut column_state,
            column_filters.len(),
            &manager,
            &mut column_filters,
        );
        assert_eq!(prev_column_actions.cycle_filter_column, Some((0, false)));

        let mut operator_state = SidebarPanelState::default();
        operator_state.selection.filters = 0;
        let mut operator_filters = vec![ColumnFilter::new("name".to_string())];
        let operator_actions = run_sidebar_key(
            key_event(Key::Equals),
            SidebarSection::Filters,
            &mut operator_state,
            operator_filters.len(),
            &manager,
            &mut operator_filters,
        );
        assert!(operator_actions.filter_changed);
        assert_ne!(
            operator_filters[0].operator,
            crate::ui::FilterOperator::Contains
        );

        let previous_actions = run_sidebar_key(
            key_event(Key::Minus),
            SidebarSection::Filters,
            &mut operator_state,
            operator_filters.len(),
            &manager,
            &mut operator_filters,
        );
        assert!(previous_actions.filter_changed);
        assert_eq!(
            operator_filters[0].operator,
            crate::ui::FilterOperator::Contains
        );
    }

    #[test]
    fn delete_in_databases_section_requests_database_target() {
        let manager = active_manager_with_tables();
        let mut panel_state = SidebarPanelState::default();
        panel_state.selection.databases = 0;
        let mut filters = Vec::new();

        let actions = run_sidebar_key(
            key_event(Key::D),
            SidebarSection::Databases,
            &mut panel_state,
            1,
            &manager,
            &mut filters,
        );

        assert_eq!(
            actions.delete,
            Some(SidebarDeleteTarget::Database {
                connection_name: "primary".to_string(),
                database_name: "main".to_string(),
            })
        );
    }

    #[test]
    fn delete_in_connections_section_requests_connection_target() {
        let manager = active_manager_with_tables();
        let mut panel_state = SidebarPanelState::default();
        panel_state.selection.connections = 0;
        let mut filters = Vec::new();

        let actions = run_sidebar_key(
            key_event(Key::D),
            SidebarSection::Connections,
            &mut panel_state,
            1,
            &manager,
            &mut filters,
        );

        assert_eq!(
            actions.delete,
            Some(SidebarDeleteTarget::connection("primary".to_string()))
        );
    }

    #[test]
    fn delete_in_tables_section_requests_table_target_with_connection_context() {
        let manager = active_manager_with_tables();
        let mut panel_state = SidebarPanelState::default();
        panel_state.selection.tables = 0;
        let mut filters = Vec::new();

        let actions = run_sidebar_key(
            key_event(Key::D),
            SidebarSection::Tables,
            &mut panel_state,
            1,
            &manager,
            &mut filters,
        );

        assert_eq!(
            actions.delete,
            Some(SidebarDeleteTarget::Table {
                connection_name: "primary".to_string(),
                table_name: "users".to_string(),
            })
        );
    }
}
