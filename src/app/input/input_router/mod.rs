//! 输入路由模块
//!
//! 将真正的全局快捷键集中到一个入口，逐步替换当前分散在不同模块中的全局抢键逻辑。

use eframe::egui;

use crate::app::dialogs::host::DialogId;
use crate::core::Action;
use crate::ui::{self, ToolbarActions};

use super::DbManagerApp;
use super::action_system::AppAction;

mod actions;
mod context;
mod resolve;
mod scopes;

pub(in crate::app) use actions::*;
pub(in crate::app) use context::*;
pub(in crate::app) use scopes::*;

use resolve::*;

impl DbManagerApp {
    pub(in crate::app) fn set_focus_area(&mut self, area: ui::FocusArea) {
        if let Some(workspace_area) = tracked_er_return_focus_area(area) {
            self.state.last_non_er_workspace_focus = workspace_area;
        }

        self.state.focus_area = area;
        self.state.workbench.set_focus_area(area);
        match area {
            ui::FocusArea::DataGrid => {
                self.state.grid_state.focused = true;
                self.state.focus_sql_editor = false;
            }
            ui::FocusArea::SqlEditor => {
                self.state.grid_state.focused = false;
                self.state.focus_sql_editor = true;
            }
            ui::FocusArea::Toolbar
            | ui::FocusArea::QueryTabs
            | ui::FocusArea::Sidebar
            | ui::FocusArea::ErDiagram
            | ui::FocusArea::Dialog => {
                self.state.grid_state.focused = false;
                self.state.focus_sql_editor = false;
            }
        }

        if area == ui::FocusArea::ErDiagram {
            self.state
                .er_diagram_state
                .ensure_selection(self.state.selected_table.as_deref());
        }
    }

    fn resolve_er_return_focus_area(&self) -> ui::FocusArea {
        resolve_er_return_focus_area(
            self.state.last_non_er_workspace_focus,
            self.state.show_sidebar,
            self.state.show_sql_editor,
        )
    }

    /// 处理集中式输入路由。
    ///
    /// 这里先只接管真正跨区域的快捷键，避免继续在多个模块里重复拦截。
    pub(in crate::app) fn handle_input_router(
        &mut self,
        ctx: &egui::Context,
        toolbar_actions: &mut ToolbarActions,
    ) {
        let input_context = self.capture_input_context(ctx);
        let resolved_action = ctx.input(|input| self.resolve_input_action(input_context, input));
        self.apply_resolved_input_action(ctx, toolbar_actions, resolved_action);
    }

    fn scoped_keybinding_triggered_in_input(
        &self,
        input_context: InputContextSnapshot,
        action: Action,
        input: &egui::InputState,
    ) -> bool {
        scoped_keybinding_triggered_in_input(&self.keybindings, input_context, action, input)
    }

    fn global_keybinding_triggered_in_input(
        &self,
        action: Action,
        input: &egui::InputState,
    ) -> bool {
        global_keybinding_triggered_in_input(&self.keybindings, action, input)
    }

    fn resolve_input_action(
        &self,
        input_context: InputContextSnapshot,
        input: &egui::InputState,
    ) -> ResolvedInputAction {
        resolve_input_action_with(
            input_context,
            input,
            |action| self.scoped_keybinding_triggered_in_input(input_context, action, input),
            |action| self.global_keybinding_triggered_in_input(action, input),
            |shortcut| local_shortcut_triggered_in_input(&self.keybindings, shortcut, input),
            || focus_area_switch_triggered_in_input(&self.keybindings, input),
        )
    }

    pub(in crate::app) fn capture_input_context(
        &self,
        ctx: &egui::Context,
    ) -> InputContextSnapshot {
        InputContextSnapshot {
            has_modal_dialog: self.has_modal_dialog_open(),
            active_dialog: self.active_dialog_id().map(DialogScope::from),
            text_focus: ctx.memory(|memory| memory.focused().is_some()),
            egui_captures_keyboard: ctx.egui_wants_keyboard_input(),
            show_autocomplete: self.state.show_autocomplete,
            show_sql_editor: self.state.show_sql_editor,
            focus_sql_editor: self.state.focus_sql_editor,
            focus_area: self.state.focus_area,
            editor_mode: self.state.editor_mode,
            sidebar_section: self.state.sidebar_section,
            filter_input_has_focus: self.state.sidebar_panel_state.filter_input_has_focus,
            grid_mode: self.state.grid_state.mode,
            grid_editing_cell: self.state.grid_state.editing_cell.is_some(),
            show_connection_dialog: self.state.show_connection_dialog,
            show_export_dialog: self.state.show_export_dialog,
            show_import_dialog: self.state.show_import_dialog,
            show_delete_confirm: self.state.show_delete_confirm,
            show_help: self.state.show_help,
            show_about: self.state.show_about,
            show_welcome_setup_dialog: self.state.show_welcome_setup_dialog,
            show_history_panel: self.state.show_history_panel,
            show_ddl_dialog: self.state.ddl_dialog_state.show,
            show_create_db_dialog: self.state.create_db_dialog_state.show,
            show_create_user_dialog: self.state.create_user_dialog_state.show,
            show_keybindings_dialog: self.state.keybindings_dialog_state.show,
            show_command_palette: self.command_palette_state.open,
            show_er_diagram: self.state.show_er_diagram,
            er_diagram_viewport_mode: self.state.er_diagram_state.is_viewport_mode(),
            keybindings_recording: self.state.keybindings_dialog_state.is_recording(),
        }
    }

    fn apply_resolved_input_action(
        &mut self,
        ctx: &egui::Context,
        toolbar_actions: &mut ToolbarActions,
        action: ResolvedInputAction,
    ) {
        match action {
            ResolvedInputAction::NoOp
            | ResolvedInputAction::PreservedTrueGlobalFallback(TrueGlobalFallbackAction::Zoom) => {}
            ResolvedInputAction::HandledApp(action) => self.dispatch_app_action(ctx, action),
            ResolvedInputAction::HandledLocal(action) => {
                self.apply_router_local_action(ctx, toolbar_actions, action);
            }
        }
    }

    fn apply_router_local_action(
        &mut self,
        ctx: &egui::Context,
        _toolbar_actions: &mut ToolbarActions,
        action: RouterLocalAction,
    ) {
        match action {
            RouterLocalAction::CommitFocusTransition(transition) => match transition {
                PendingFocusTransition::NextFocusArea => self.cycle_focus(false),
                PendingFocusTransition::PrevFocusArea => self.cycle_focus(true),
            },
            RouterLocalAction::FocusSidebarSection(section) => {
                self.activate_sidebar_section_shortcut(section);
                self.session.notifications.info(format!(
                    "切换到: {}",
                    sidebar_section_shortcut_name(section)
                ));
            }
            RouterLocalAction::ErDiagram(action) => match action {
                ErDiagramLocalAction::PrevTable => {
                    self.state.er_diagram_state.select_prev_table();
                }
                ErDiagramLocalAction::NextTable => {
                    self.state.er_diagram_state.select_next_table();
                }
                ErDiagramLocalAction::PrevRelatedTable => {
                    self.state.er_diagram_state.select_prev_related_table();
                }
                ErDiagramLocalAction::NextRelatedTable => {
                    self.state.er_diagram_state.select_next_related_table();
                }
                ErDiagramLocalAction::GeometryLeft => {
                    self.state
                        .er_diagram_state
                        .select_geometric_neighbor(ui::GeometricDirection::Left);
                }
                ErDiagramLocalAction::GeometryDown => {
                    self.state
                        .er_diagram_state
                        .select_geometric_neighbor(ui::GeometricDirection::Down);
                }
                ErDiagramLocalAction::GeometryUp => {
                    self.state
                        .er_diagram_state
                        .select_geometric_neighbor(ui::GeometricDirection::Up);
                }
                ErDiagramLocalAction::GeometryRight => {
                    self.state
                        .er_diagram_state
                        .select_geometric_neighbor(ui::GeometricDirection::Right);
                }
                ErDiagramLocalAction::OpenSelectedTable => {
                    self.state
                        .er_diagram_state
                        .ensure_selection(self.state.selected_table.as_deref());
                    if let Some(table_name) = self
                        .state
                        .er_diagram_state
                        .selected_table_name()
                        .map(str::to_owned)
                    {
                        self.state.selected_table = Some(table_name);
                        self.dispatch_app_action(ctx, AppAction::QuerySelectedTable);
                        self.set_focus_area(ui::FocusArea::DataGrid);
                    }
                }
                ErDiagramLocalAction::ReturnToWorkspace => {
                    self.set_focus_area(self.resolve_er_return_focus_area());
                }
                ErDiagramLocalAction::CloseDiagram => {
                    self.set_er_diagram_visible(false);
                }
                ErDiagramLocalAction::ToggleViewportMode => {
                    self.state.er_diagram_state.toggle_interaction_mode();
                }
                ErDiagramLocalAction::ExitViewportMode => {
                    self.state.er_diagram_state.exit_viewport_mode();
                }
            },
            RouterLocalAction::CloseWorkspaceOverlay => {
                if self.is_dialog_visible(DialogId::Help) {
                    self.close_dialog(DialogId::Help);
                } else if self.is_dialog_visible(DialogId::History) {
                    self.close_dialog(DialogId::History);
                } else if self.state.show_er_diagram {
                    self.set_er_diagram_visible_with_notice(
                        false,
                        ErDiagramVisibilityNotice::Silent,
                    );
                }
            }
            RouterLocalAction::CloseDialog(scope) => match scope {
                DialogScope::Help => self.close_dialog(DialogId::Help),
                DialogScope::About => self.close_dialog(DialogId::About),
                DialogScope::History => self.close_dialog(DialogId::History),
                DialogScope::DeleteConfirm => self.close_dialog(DialogId::DeleteConfirm),
                DialogScope::Keybindings => self.close_dialog(DialogId::Keybindings),
                DialogScope::ToolbarActionsMenu => self.close_dialog(DialogId::ToolbarActionsMenu),
                DialogScope::ToolbarCreateMenu => self.close_dialog(DialogId::ToolbarCreateMenu),
                DialogScope::ToolbarThemeMenu => self.close_dialog(DialogId::ToolbarThemeMenu),
                DialogScope::Connection => self.close_dialog(DialogId::Connection),
                DialogScope::Export => self.close_dialog(DialogId::Export),
                DialogScope::Import => self.close_dialog(DialogId::Import),
                DialogScope::Ddl => self.close_dialog(DialogId::Ddl),
                DialogScope::SchemaDiff => self.close_dialog(DialogId::SchemaDiff),
                DialogScope::CreateDatabase => self.close_dialog(DialogId::CreateDatabase),
                DialogScope::CreateUser => self.close_dialog(DialogId::CreateUser),
                _ => {}
            },
            RouterLocalAction::KeybindingsRecordingInput => {
                let _ = ctx.input(|input| {
                    self.state
                        .keybindings_dialog_state
                        .consume_recording_input(input)
                });
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
            ui::SidebarSection::Connections => self.state.sidebar_panel_state.show_connections,
            ui::SidebarSection::Databases | ui::SidebarSection::Tables => {
                self.state.sidebar_panel_state.show_connections
            }
            ui::SidebarSection::Filters => self.state.sidebar_panel_state.show_filters,
            ui::SidebarSection::Triggers => self.state.sidebar_panel_state.show_triggers,
            ui::SidebarSection::Routines => self.state.sidebar_panel_state.show_routines,
        };

        if is_toggle_panel
            && self.state.show_sidebar
            && self.state.sidebar_section == section
            && panel_visible
        {
            match section {
                ui::SidebarSection::Connections => {
                    self.state.sidebar_panel_state.show_connections = false;
                }
                ui::SidebarSection::Filters => {
                    self.state.sidebar_panel_state.show_filters = false;
                }
                ui::SidebarSection::Triggers => {
                    self.state.sidebar_panel_state.show_triggers = false;
                }
                ui::SidebarSection::Routines => {
                    self.state.sidebar_panel_state.show_routines = false;
                }
                _ => {}
            }

            if self.state.focus_area == ui::FocusArea::Sidebar {
                self.set_focus_area(ui::FocusArea::DataGrid);
            }
            return;
        }

        self.state.show_sidebar = true;
        self.set_focus_area(ui::FocusArea::Sidebar);
        self.state.sidebar_section = section;

        match section {
            ui::SidebarSection::Connections
            | ui::SidebarSection::Databases
            | ui::SidebarSection::Tables => {
                self.state.sidebar_panel_state.show_connections = true;
            }
            ui::SidebarSection::Filters => {
                self.state.sidebar_panel_state.show_filters = true;
            }
            ui::SidebarSection::Triggers => {
                self.state.sidebar_panel_state.show_triggers = true;
            }
            ui::SidebarSection::Routines => {
                self.state.sidebar_panel_state.show_routines = true;
            }
        }
    }

    pub(in crate::app) fn set_sidebar_visible(&mut self, visible: bool) {
        self.state.show_sidebar = visible;
        self.state.workbench.primary_sidebar.visible = visible;
        self.app_config.workbench.sidebar.visible = visible;
        self.save_config_debounced();
        let next_focus = focus_after_sidebar_visibility_change(self.state.focus_area, visible);
        if next_focus != self.state.focus_area {
            self.set_focus_area(next_focus);
        }
    }

    pub(in crate::app) fn toggle_sidebar_visibility(&mut self) {
        self.set_sidebar_visible(!self.state.show_sidebar);
    }

    pub(in crate::app) fn set_sql_editor_visible(&mut self, visible: bool) {
        self.state.show_sql_editor = visible;
        if visible {
            self.set_focus_area(ui::FocusArea::SqlEditor);
        } else if self.state.focus_area == ui::FocusArea::SqlEditor {
            self.set_focus_area(ui::FocusArea::DataGrid);
        } else {
            self.state.focus_sql_editor = false;
        }
    }

    pub(crate) fn toggle_sql_editor_visibility(&mut self) {
        self.set_sql_editor_visible(!self.state.show_sql_editor);
    }

    pub(in crate::app) fn open_export_dialog(&mut self) {
        if self.state.grid_state.result_set.is_some() {
            self.open_dialog(DialogId::Export);
            self.state.export_status = None;
        }
    }

    pub(in crate::app) fn open_import_dialog(&mut self) {
        self.handle_import();
    }

    pub(in crate::app) fn open_create_table_dialog(&mut self) {
        let Some(db_type) = self.session.manager.get_active().map(|c| c.config.db_type) else {
            self.session.notifications.warning("请先连接数据库再创建表");
            return;
        };
        self.open_dialog(DialogId::Ddl);
        self.state.ddl_dialog_state.open_create_table(db_type);
    }

    /// 打开 Schema 对比对话框，并把当前选中的表作为源表。
    pub(in crate::app) fn open_schema_diff_dialog(&mut self) {
        if self.session.manager.get_active().is_none() {
            self.session
                .notifications
                .warning("请先连接数据库再对比 schema");
            return;
        }
        let source_table = self.state.selected_table.clone();
        self.open_dialog(DialogId::SchemaDiff);
        self.state.schema_diff_dialog_state.open_for(source_table);
    }

    pub(in crate::app) fn open_create_database_dialog(&mut self) {
        let Some(db_type) = self.session.manager.get_active().map(|c| c.config.db_type) else {
            self.session
                .notifications
                .warning("请先连接数据库再创建数据库");
            return;
        };
        self.open_dialog(DialogId::CreateDatabase);
        self.state.create_db_dialog_state.open(db_type);
    }

    pub(in crate::app) fn open_create_user_dialog(&mut self) {
        self.handle_create_user_action();
    }

    pub(in crate::app) fn set_history_panel_visible(&mut self, visible: bool) {
        if visible {
            self.open_dialog(DialogId::History);
        } else {
            self.close_dialog(DialogId::History);
        }
    }

    pub(in crate::app) fn toggle_history_panel(&mut self) {
        self.set_history_panel_visible(!self.state.show_history_panel);
    }

    pub(in crate::app) fn set_er_diagram_visible_with_notice(
        &mut self,
        visible: bool,
        notice: ErDiagramVisibilityNotice,
    ) {
        if self.state.show_er_diagram == visible {
            return;
        }

        let restored_focus = focus_after_er_visibility_change(
            visible,
            self.state.focus_area,
            self.state.last_non_er_workspace_focus,
            self.state.show_sidebar,
            self.state.show_sql_editor,
        );

        self.state.show_er_diagram = visible;
        if self.state.show_er_diagram {
            self.reveal_workbench_surface(crate::state::WorkbenchSurfaceKind::ErDiagram);
            self.load_er_diagram_data();
            self.state.er_diagram_state.request_fit_to_view();
        }

        if let Some(message) = resolve_er_diagram_visibility_notice(visible, notice) {
            self.session.notifications.info(message);
        }

        if let Some(restored_focus) = restored_focus {
            self.set_focus_area(restored_focus);
        }
    }

    pub(in crate::app) fn set_er_diagram_visible(&mut self, visible: bool) {
        self.set_er_diagram_visible_with_notice(visible, ErDiagramVisibilityNotice::Default);
    }

    pub(crate) fn toggle_er_diagram_visibility(&mut self) {
        self.set_er_diagram_visible(!self.state.show_er_diagram);
    }

    pub(in crate::app) fn open_new_query_tab(&mut self) {
        self.persist_active_tab_state_for_navigation();
        self.session.tab_manager.new_tab();
        self.sync_from_active_tab();
    }

    pub(in crate::app) fn select_next_query_tab(&mut self) {
        self.persist_active_tab_state_for_navigation();
        self.session.tab_manager.next_tab();
        self.sync_from_active_tab();
    }

    pub(in crate::app) fn select_previous_query_tab(&mut self) {
        self.persist_active_tab_state_for_navigation();
        self.session.tab_manager.prev_tab();
        self.sync_from_active_tab();
    }

    pub(in crate::app) fn close_active_query_tab(&mut self) {
        let closing_tab_id = self
            .session
            .tab_manager
            .get_active()
            .map(|tab| tab.id.clone());
        if self.session.tab_manager.tabs.len() > 1
            && let Some(request_id) = self
                .session
                .tab_manager
                .get_active()
                .and_then(|tab| tab.pending_request_id)
        {
            self.cancel_query_request_silently(request_id);
        }
        self.session.tab_manager.close_active_tab();
        if let Some(tab_id) = closing_tab_id {
            self.warn_if_tab_has_unsaved_grid_edits(&tab_id);
            self.remove_grid_workspaces_for_tab(&tab_id);
        }
        self.sync_from_active_tab();
    }
}

#[cfg(test)]
mod tests;
