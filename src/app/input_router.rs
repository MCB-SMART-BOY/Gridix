//! 输入路由模块
//!
//! 将真正的全局快捷键集中到一个入口，逐步替换当前分散在不同模块中的全局抢键逻辑。

use eframe::egui;

use crate::core::Action;
use crate::ui::{self, EditorMode, ToolbarActions};

use super::DbManagerApp;

/// 当前输入聚焦的逻辑作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InputScope {
    Dialog,
    TextInput,
    Toolbar,
    QueryTabs,
    Sidebar,
    DataGrid,
    SqlEditorNormal,
    SqlEditorInsert,
}

/// 从应用状态提取出的输入上下文快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct InputContextSnapshot {
    pub has_modal_dialog: bool,
    pub text_focus: bool,
    pub egui_captures_keyboard: bool,
    pub show_autocomplete: bool,
    pub show_sql_editor: bool,
    pub focus_sql_editor: bool,
    pub focus_area: ui::FocusArea,
    pub editor_mode: EditorMode,
}

impl InputContextSnapshot {
    pub(super) fn focused_scope(self) -> InputScope {
        if self.has_modal_dialog {
            return InputScope::Dialog;
        }

        let editor_has_priority = self.show_sql_editor
            && (self.focus_area == ui::FocusArea::SqlEditor || self.focus_sql_editor);

        if editor_has_priority {
            return match self.editor_mode {
                EditorMode::Insert => InputScope::SqlEditorInsert,
                EditorMode::Normal => InputScope::SqlEditorNormal,
            };
        }

        if self.text_focus || self.egui_captures_keyboard {
            return InputScope::TextInput;
        }

        match self.focus_area {
            ui::FocusArea::Toolbar => InputScope::Toolbar,
            ui::FocusArea::QueryTabs => InputScope::QueryTabs,
            ui::FocusArea::Sidebar => InputScope::Sidebar,
            ui::FocusArea::DataGrid => InputScope::DataGrid,
            ui::FocusArea::SqlEditor => InputScope::SqlEditorNormal,
            ui::FocusArea::Dialog => InputScope::Dialog,
        }
    }

    fn can_cycle_focus(self, modifiers: &egui::Modifiers) -> bool {
        !self.has_modal_dialog
            && !modifiers.ctrl
            && !modifiers.alt
            && !self.show_autocomplete
            && self.focused_scope() != InputScope::Dialog
            && self.focused_scope() != InputScope::TextInput
            && self.focused_scope() != InputScope::SqlEditorInsert
    }

    fn can_dispatch_global_shortcut(self) -> bool {
        !self.has_modal_dialog
    }

    fn can_focus_sidebar_section(self) -> bool {
        self.can_dispatch_global_shortcut()
    }

    pub(super) fn is_text_entry_scope(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::TextInput | InputScope::SqlEditorInsert
        )
    }

    pub(super) fn allows_filter_shortcuts(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::DataGrid | InputScope::Sidebar
        )
    }

    pub(super) fn allows_data_grid_shortcuts(self) -> bool {
        self.focused_scope() == InputScope::DataGrid
    }

    pub(super) fn allows_refresh(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::Toolbar
                | InputScope::QueryTabs
                | InputScope::Sidebar
                | InputScope::DataGrid
        )
    }

    pub(super) fn allows_panel_visibility_toggle(self) -> bool {
        !self.is_text_entry_scope() && self.focused_scope() != InputScope::Dialog
    }

    pub(super) fn allows_import_export_shortcuts(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::Toolbar
                | InputScope::QueryTabs
                | InputScope::Sidebar
                | InputScope::DataGrid
        )
    }

    pub(super) fn allows_search_shortcuts(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::QueryTabs | InputScope::Sidebar | InputScope::DataGrid
        )
    }

    pub(super) fn allows_clear_command_line(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::SqlEditorNormal | InputScope::SqlEditorInsert
        )
    }

    pub(super) fn allows_workspace_creation_shortcuts(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::Toolbar
                | InputScope::QueryTabs
                | InputScope::Sidebar
                | InputScope::DataGrid
                | InputScope::SqlEditorNormal
        )
    }

    pub(super) fn allows_workspace_overlay_shortcuts(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::Toolbar
                | InputScope::QueryTabs
                | InputScope::Sidebar
                | InputScope::DataGrid
                | InputScope::SqlEditorNormal
        )
    }

    pub(super) fn allows_tab_management_shortcuts(self) -> bool {
        matches!(
            self.focused_scope(),
            InputScope::Toolbar
                | InputScope::QueryTabs
                | InputScope::Sidebar
                | InputScope::DataGrid
                | InputScope::SqlEditorNormal
        )
    }
}

impl DbManagerApp {
    /// 处理集中式输入路由。
    ///
    /// 这里先只接管真正跨区域的快捷键，避免继续在多个模块里重复拦截。
    pub(super) fn handle_input_router(
        &mut self,
        ctx: &egui::Context,
        toolbar_actions: &mut ToolbarActions,
    ) {
        let input_context = self.capture_input_context(ctx);
        let keybindings = self.keybindings.clone();

        ctx.input(|input| {
            let action_triggered = |action: Action| {
                keybindings.get(action).is_some_and(|binding| {
                    binding.modifiers.matches(&input.modifiers)
                        && input.key_pressed(binding.key.to_egui_key())
                })
            };

            // 帮助切换保持为真正的全局入口。
            if action_triggered(Action::ShowHelp) {
                self.show_help = !self.show_help;
            }

            // 主题选择器放在统一路由层，避免散落在渲染逻辑里。
            if input.modifiers.ctrl && input.modifiers.shift && input.key_pressed(egui::Key::T) {
                toolbar_actions.open_theme_selector = true;
            }

            if !input_context.can_dispatch_global_shortcut() {
                return;
            }

            if action_triggered(Action::NewConnection) {
                self.show_connection_dialog = true;
            }

            if input.modifiers.ctrl && !input.modifiers.shift && input.key_pressed(egui::Key::D) {
                toolbar_actions.toggle_dark_mode = true;
            }

            if input_context.can_cycle_focus(&input.modifiers) && input.key_pressed(egui::Key::Tab)
            {
                self.cycle_focus(input.modifiers.shift);
            }

            if input_context.can_focus_sidebar_section() {
                self.handle_sidebar_focus_shortcuts(input);
            }

            if input.modifiers.alt && !input.modifiers.ctrl && input.key_pressed(egui::Key::K) {
                self.keybindings_dialog_state.open(&self.keybindings);
            }
        });
    }

    pub(super) fn capture_input_context(&self, ctx: &egui::Context) -> InputContextSnapshot {
        InputContextSnapshot {
            has_modal_dialog: self.has_modal_dialog_open(),
            text_focus: ctx.memory(|memory| memory.focused().is_some()),
            egui_captures_keyboard: ctx.egui_wants_keyboard_input(),
            show_autocomplete: self.show_autocomplete,
            show_sql_editor: self.show_sql_editor,
            focus_sql_editor: self.focus_sql_editor,
            focus_area: self.focus_area,
            editor_mode: self.editor_mode,
        }
    }

    /// 处理侧边栏焦点快捷键
    fn handle_sidebar_focus_shortcuts(&mut self, input: &egui::InputState) {
        // 顺序：1连接 2数据库 3表 4筛选 5触发器 6存储过程
        let shortcuts = [
            (egui::Key::Num1, ui::SidebarSection::Connections, "连接列表"),
            (egui::Key::Num2, ui::SidebarSection::Databases, "数据库列表"),
            (egui::Key::Num3, ui::SidebarSection::Tables, "表列表"),
            (egui::Key::Num4, ui::SidebarSection::Filters, "筛选面板"),
            (egui::Key::Num5, ui::SidebarSection::Triggers, "触发器列表"),
            (
                egui::Key::Num6,
                ui::SidebarSection::Routines,
                "存储过程列表",
            ),
        ];

        for (key, section, name) in shortcuts {
            if input.modifiers.ctrl && input.key_pressed(key) {
                self.activate_sidebar_section_shortcut(section);
                self.notifications.info(format!("切换到: {}", name));
                break;
            }
        }
    }

    fn activate_sidebar_section_shortcut(&mut self, section: ui::SidebarSection) {
        let is_toggle_panel = matches!(
            section,
            ui::SidebarSection::Connections
                | ui::SidebarSection::Filters
                | ui::SidebarSection::Triggers
                | ui::SidebarSection::Routines
        );

        let panel_visible = match section {
            ui::SidebarSection::Connections => self.sidebar_panel_state.show_connections,
            ui::SidebarSection::Databases | ui::SidebarSection::Tables => {
                self.sidebar_panel_state.show_connections
            }
            ui::SidebarSection::Filters => self.sidebar_panel_state.show_filters,
            ui::SidebarSection::Triggers => self.sidebar_panel_state.show_triggers,
            ui::SidebarSection::Routines => self.sidebar_panel_state.show_routines,
        };

        if is_toggle_panel && self.show_sidebar && self.sidebar_section == section && panel_visible
        {
            match section {
                ui::SidebarSection::Connections => {
                    self.sidebar_panel_state.show_connections = false;
                }
                ui::SidebarSection::Filters => {
                    self.sidebar_panel_state.show_filters = false;
                }
                ui::SidebarSection::Triggers => {
                    self.sidebar_panel_state.show_triggers = false;
                }
                ui::SidebarSection::Routines => {
                    self.sidebar_panel_state.show_routines = false;
                }
                _ => {}
            }

            if self.focus_area == ui::FocusArea::Sidebar {
                self.focus_area = ui::FocusArea::DataGrid;
                self.grid_state.focused = true;
                self.focus_sql_editor = false;
            }
            return;
        }

        self.show_sidebar = true;
        self.focus_area = ui::FocusArea::Sidebar;
        self.sidebar_section = section;
        self.grid_state.focused = false;
        self.focus_sql_editor = false;

        match section {
            ui::SidebarSection::Connections
            | ui::SidebarSection::Databases
            | ui::SidebarSection::Tables => {
                self.sidebar_panel_state.show_connections = true;
            }
            ui::SidebarSection::Filters => {
                self.sidebar_panel_state.show_filters = true;
            }
            ui::SidebarSection::Triggers => {
                self.sidebar_panel_state.show_triggers = true;
            }
            ui::SidebarSection::Routines => {
                self.sidebar_panel_state.show_routines = true;
            }
        }
    }

    pub(super) fn set_sidebar_visible(&mut self, visible: bool) {
        self.show_sidebar = visible;
        if visible {
            self.focus_area = ui::FocusArea::Sidebar;
            self.grid_state.focused = false;
            self.focus_sql_editor = false;
        } else if self.focus_area == ui::FocusArea::Sidebar {
            self.focus_area = ui::FocusArea::DataGrid;
            self.grid_state.focused = true;
            self.focus_sql_editor = false;
        }
    }

    pub(super) fn toggle_sidebar_visibility(&mut self) {
        self.set_sidebar_visible(!self.show_sidebar);
    }

    pub(super) fn set_sql_editor_visible(&mut self, visible: bool) {
        self.show_sql_editor = visible;
        if visible {
            self.focus_area = ui::FocusArea::SqlEditor;
            self.focus_sql_editor = true;
            self.grid_state.focused = false;
        } else if self.focus_area == ui::FocusArea::SqlEditor {
            self.focus_area = ui::FocusArea::DataGrid;
            self.grid_state.focused = true;
            self.focus_sql_editor = false;
        } else {
            self.focus_sql_editor = false;
        }
    }

    pub(super) fn toggle_sql_editor_visibility(&mut self) {
        self.set_sql_editor_visible(!self.show_sql_editor);
    }

    pub(super) fn open_export_dialog(&mut self) {
        if self.result.is_some() {
            self.show_export_dialog = true;
            self.export_status = None;
        }
    }

    pub(super) fn open_import_dialog(&mut self) {
        self.handle_import();
    }

    pub(super) fn open_create_table_dialog(&mut self) {
        let db_type = self
            .manager
            .get_active()
            .map(|c| c.config.db_type)
            .unwrap_or_default();
        self.ddl_dialog_state.open_create_table(db_type);
    }

    pub(super) fn open_create_database_dialog(&mut self) {
        let db_type = self
            .manager
            .get_active()
            .map(|c| c.config.db_type)
            .unwrap_or_default();
        self.create_db_dialog_state.open(db_type);
    }

    pub(super) fn open_create_user_dialog(&mut self) {
        self.handle_create_user_action();
    }

    pub(super) fn set_history_panel_visible(&mut self, visible: bool) {
        self.show_history_panel = visible;
    }

    pub(super) fn toggle_history_panel(&mut self) {
        self.set_history_panel_visible(!self.show_history_panel);
    }

    pub(super) fn set_er_diagram_visible(&mut self, visible: bool) {
        if self.show_er_diagram == visible {
            return;
        }

        self.show_er_diagram = visible;
        if self.show_er_diagram {
            self.load_er_diagram_data();
            self.notifications.info("ER 关系图已打开");
        } else {
            self.notifications.info("ER 关系图已关闭");
        }
    }

    pub(super) fn toggle_er_diagram_visibility(&mut self) {
        self.set_er_diagram_visible(!self.show_er_diagram);
    }

    pub(super) fn open_new_query_tab(&mut self) {
        self.sync_sql_to_active_tab();
        self.tab_manager.new_tab();
        self.sync_from_active_tab();
    }

    pub(super) fn select_next_query_tab(&mut self) {
        self.sync_sql_to_active_tab();
        self.tab_manager.next_tab();
        self.sync_from_active_tab();
    }

    pub(super) fn select_previous_query_tab(&mut self) {
        self.sync_sql_to_active_tab();
        self.tab_manager.prev_tab();
        self.sync_from_active_tab();
    }

    pub(super) fn close_active_query_tab(&mut self) {
        if self.tab_manager.tabs.len() > 1
            && let Some(request_id) = self
                .tab_manager
                .get_active()
                .and_then(|tab| tab.pending_request_id)
        {
            self.cancel_query_request(request_id);
        }
        self.tab_manager.close_active_tab();
        self.sync_from_active_tab();
    }
}

#[cfg(test)]
mod tests {
    use super::{InputContextSnapshot, InputScope};
    use crate::ui::{EditorMode, FocusArea};
    use egui::Modifiers;

    fn snapshot() -> InputContextSnapshot {
        InputContextSnapshot {
            has_modal_dialog: false,
            text_focus: false,
            egui_captures_keyboard: false,
            show_autocomplete: false,
            show_sql_editor: true,
            focus_sql_editor: false,
            focus_area: FocusArea::DataGrid,
            editor_mode: EditorMode::Insert,
        }
    }

    #[test]
    fn modal_dialog_overrides_other_scopes() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.focus_area = FocusArea::Sidebar;

        assert_eq!(context.focused_scope(), InputScope::Dialog);
        assert!(!context.can_cycle_focus(&Modifiers::NONE));
    }

    #[test]
    fn sql_editor_insert_blocks_focus_cycle() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.focus_sql_editor = true;
        context.editor_mode = EditorMode::Insert;

        assert_eq!(context.focused_scope(), InputScope::SqlEditorInsert);
        assert!(!context.can_cycle_focus(&Modifiers::NONE));
    }

    #[test]
    fn data_grid_scope_allows_unmodified_focus_cycle() {
        let context = snapshot();

        assert_eq!(context.focused_scope(), InputScope::DataGrid);
        assert!(context.can_cycle_focus(&Modifiers::NONE));
    }

    #[test]
    fn text_input_scope_blocks_focus_cycle() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Sidebar;

        assert_eq!(context.focused_scope(), InputScope::TextInput);
        assert!(!context.can_cycle_focus(&Modifiers::NONE));
    }

    #[test]
    fn text_input_scope_blocks_filter_shortcuts() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Sidebar;

        assert!(context.is_text_entry_scope());
        assert!(!context.allows_filter_shortcuts());
        assert!(!context.allows_refresh());
    }

    #[test]
    fn data_grid_scope_allows_grid_shortcuts() {
        let context = snapshot();

        assert!(context.allows_data_grid_shortcuts());
        assert!(context.allows_filter_shortcuts());
        assert!(context.allows_refresh());
    }

    #[test]
    fn sql_editor_normal_blocks_refresh_but_keeps_editor_actions() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.editor_mode = EditorMode::Normal;

        assert_eq!(context.focused_scope(), InputScope::SqlEditorNormal);
        assert!(!context.allows_refresh());
        assert!(context.allows_clear_command_line());
        assert!(!context.allows_import_export_shortcuts());
    }

    #[test]
    fn sql_editor_insert_blocks_workspace_level_shortcuts() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.focus_sql_editor = true;
        context.editor_mode = EditorMode::Insert;

        assert!(!context.allows_workspace_creation_shortcuts());
        assert!(!context.allows_workspace_overlay_shortcuts());
        assert!(!context.allows_tab_management_shortcuts());
    }

    #[test]
    fn sql_editor_normal_keeps_non_text_workspace_shortcuts() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.editor_mode = EditorMode::Normal;

        assert!(context.allows_workspace_creation_shortcuts());
        assert!(context.allows_workspace_overlay_shortcuts());
        assert!(context.allows_tab_management_shortcuts());
    }

    #[test]
    fn text_input_scope_blocks_workspace_level_shortcuts() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Sidebar;

        assert!(!context.allows_workspace_creation_shortcuts());
        assert!(!context.allows_workspace_overlay_shortcuts());
        assert!(!context.allows_tab_management_shortcuts());
    }
}
