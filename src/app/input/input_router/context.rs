//! 输入上下文快照：把 egui 与应用状态折叠成可测试的纯数据。

use eframe::egui;

use crate::core::Action;
use crate::ui::{self, EditorMode, GridMode};

use crate::app::input::action_system::AppAction;
use crate::app::input::owner::InputOwner;

use super::actions::*;
use super::scopes::*;

/// 从应用状态提取出的输入上下文快照。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct InputContextSnapshot {
    pub has_modal_dialog: bool,
    pub active_dialog: Option<DialogScope>,
    pub text_focus: bool,
    pub egui_captures_keyboard: bool,
    pub show_autocomplete: bool,
    pub show_sql_editor: bool,
    pub focus_sql_editor: bool,
    pub focus_area: ui::FocusArea,
    pub editor_mode: EditorMode,
    pub sidebar_section: ui::SidebarSection,
    pub filter_input_has_focus: bool,
    pub grid_mode: GridMode,
    pub grid_editing_cell: bool,
    pub show_connection_dialog: bool,
    pub show_export_dialog: bool,
    pub show_import_dialog: bool,
    pub show_delete_confirm: bool,
    pub show_help: bool,
    pub show_about: bool,
    pub show_welcome_setup_dialog: bool,
    pub show_history_panel: bool,
    pub show_ddl_dialog: bool,
    pub show_create_db_dialog: bool,
    pub show_create_user_dialog: bool,
    pub show_keybindings_dialog: bool,
    pub show_command_palette: bool,
    pub show_er_diagram: bool,
    pub er_diagram_viewport_mode: bool,
    pub keybindings_recording: bool,
}

impl InputContextSnapshot {
    fn dialog_scope(self) -> Option<DialogScope> {
        if self.active_dialog.is_some() {
            self.active_dialog
        } else if self.has_modal_dialog {
            Some(DialogScope::Generic)
        } else {
            None
        }
    }

    fn sidebar_scope(self) -> SidebarFocusScope {
        match self.sidebar_section {
            ui::SidebarSection::Connections => SidebarFocusScope::Connections,
            ui::SidebarSection::Databases => SidebarFocusScope::Databases,
            ui::SidebarSection::Tables => SidebarFocusScope::Tables,
            ui::SidebarSection::Filters => {
                if self.filter_input_has_focus {
                    SidebarFocusScope::FiltersInput
                } else {
                    SidebarFocusScope::FiltersList
                }
            }
            ui::SidebarSection::Triggers => SidebarFocusScope::Triggers,
            ui::SidebarSection::Routines => SidebarFocusScope::Routines,
        }
    }

    fn grid_scope(self) -> GridFocusScope {
        if self.grid_editing_cell || matches!(self.grid_mode, GridMode::Insert) {
            GridFocusScope::Insert
        } else if matches!(self.grid_mode, GridMode::Select) {
            GridFocusScope::Select
        } else {
            GridFocusScope::Normal
        }
    }

    pub(in crate::app) fn focus_scope(self) -> FocusScope {
        if let Some(dialog_scope) = self.dialog_scope() {
            return FocusScope::Dialog(dialog_scope);
        }

        if self.focus_area == ui::FocusArea::Sidebar {
            return FocusScope::Sidebar(self.sidebar_scope());
        }

        let editor_has_priority =
            self.show_sql_editor && self.focus_area == ui::FocusArea::SqlEditor;
        if editor_has_priority {
            return FocusScope::Editor(match self.editor_mode {
                EditorMode::Insert => EditorFocusScope::Insert,
                EditorMode::Normal => EditorFocusScope::Normal,
            });
        }

        if self.focus_area == ui::FocusArea::DataGrid {
            return FocusScope::Grid(self.grid_scope());
        }

        if self.show_er_diagram && self.focus_area == ui::FocusArea::ErDiagram {
            return FocusScope::ErDiagram(if self.er_diagram_viewport_mode {
                ErDiagramFocusScope::Viewport
            } else {
                ErDiagramFocusScope::Navigation
            });
        }

        if self.text_focus || self.egui_captures_keyboard {
            return FocusScope::Global;
        }

        match self.focus_area {
            ui::FocusArea::Toolbar => FocusScope::Toolbar,
            ui::FocusArea::QueryTabs => FocusScope::QueryTabs,
            ui::FocusArea::Sidebar => FocusScope::Sidebar(self.sidebar_scope()),
            ui::FocusArea::DataGrid => FocusScope::Grid(self.grid_scope()),
            ui::FocusArea::ErDiagram => FocusScope::Global,
            ui::FocusArea::SqlEditor => FocusScope::Editor(EditorFocusScope::Normal),
            ui::FocusArea::Dialog => FocusScope::Dialog(DialogScope::Generic),
        }
    }

    pub(in crate::app) fn keymap_scope_path(self) -> &'static str {
        self.focus_scope_path().as_str()
    }

    pub(in crate::app) fn focus_scope_path(self) -> FocusScopePath {
        self.input_owner().scope().path()
    }

    pub(in crate::app) fn text_entry_guard(self) -> TextEntryGuard {
        match self.focus_scope() {
            FocusScope::Sidebar(SidebarFocusScope::FiltersInput)
            | FocusScope::Grid(GridFocusScope::Insert)
            | FocusScope::Editor(EditorFocusScope::Insert) => {
                TextEntryGuard::Scoped(self.focus_scope())
            }
            FocusScope::Dialog(_) | FocusScope::Global
                if self.text_focus || self.egui_captures_keyboard =>
            {
                TextEntryGuard::GlobalWidget
            }
            _ => TextEntryGuard::Inactive,
        }
    }

    pub(in crate::app) fn input_mode(self) -> InputMode {
        if matches!(self.dialog_scope(), Some(DialogScope::Keybindings))
            && self.keybindings_recording
        {
            return InputMode::Recording;
        }

        if matches!(self.focus_scope(), FocusScope::Editor(_)) && !self.show_sql_editor {
            return InputMode::Disabled;
        }

        match self.focus_scope() {
            FocusScope::Grid(GridFocusScope::Select) => InputMode::Select,
            _ if self.text_entry_guard().is_active() => InputMode::TextEntry,
            _ => InputMode::Command,
        }
    }

    pub(in crate::app) fn input_owner(self) -> InputOwner {
        InputOwner::from_scope_and_mode(self.focus_scope(), self.input_mode())
    }

    pub(in crate::app) fn allows_focus_area_switch(self) -> bool {
        !self.show_autocomplete
            && !self.input_owner().is_modal()
            && !matches!(self.input_owner().mode(), InputMode::Disabled)
            && !self.text_entry_guard().is_active()
    }

    pub(in crate::app) fn can_dispatch_global_shortcut(self) -> bool {
        !self.input_owner().is_modal()
    }

    fn can_focus_sidebar_section(self) -> bool {
        self.can_dispatch_global_shortcut()
            && !matches!(
                self.input_mode(),
                InputMode::TextEntry | InputMode::Recording | InputMode::Disabled
            )
    }

    fn is_workspace_command_mode(self) -> bool {
        !matches!(
            self.input_mode(),
            InputMode::TextEntry | InputMode::Recording | InputMode::Disabled
        )
    }

    pub(in crate::app) fn resolve_shortcut_action(self, action: Action) -> Option<AppAction> {
        if !self.can_dispatch_global_shortcut() {
            return None;
        }

        let app_action = AppAction::from_shortcut_action(action)?;
        let allowed = match action {
            Action::NextFocusArea | Action::PrevFocusArea => false,
            Action::ShowHelp => true,
            Action::CommandPalette | Action::NewConnection => self.is_workspace_command_mode(),
            Action::OpenKeybindingsDialog
            | Action::OpenToolbarActionsMenu
            | Action::OpenToolbarCreateMenu
            | Action::OpenThemeSelector
            | Action::FocusErDiagram
            | Action::FocusBottomPanel
            | Action::FocusRightInspector
            | Action::ToggleDarkMode
            | Action::FocusSidebarConnections
            | Action::FocusSidebarDatabases
            | Action::FocusSidebarTables
            | Action::FocusSidebarFilters
            | Action::FocusSidebarTriggers
            | Action::FocusSidebarRoutines => false,
            Action::NewTable | Action::NewDatabase | Action::NewUser => {
                self.allows_workspace_creation_shortcuts()
            }
            Action::Export | Action::Import => self.allows_import_export_shortcuts(),
            Action::ShowHistory => self.allows_workspace_overlay_shortcuts(),
            Action::ToggleErDiagram => self.allows_toggle_er_diagram_shortcuts(),
            Action::Refresh => self.allows_refresh(),
            Action::ClearCommandLine => self.allows_clear_command_line(),
            Action::ToggleEditor => self.allows_editor_visibility_toggle(),
            Action::ToggleSidebar => self.allows_panel_visibility_toggle(),
            Action::ClearSearch => self.allows_search_shortcuts(),
            Action::Save | Action::GotoLine => self.allows_data_grid_shortcuts(),
            Action::NewTab | Action::NextTab | Action::PrevTab | Action::CloseTab => {
                self.allows_tab_management_shortcuts()
            }
            Action::ZoomIn | Action::ZoomOut | Action::ZoomReset => false,
        };

        allowed.then_some(app_action)
    }

    pub(in crate::app) fn resolve_escape_fallback(
        self,
        input: &egui::InputState,
    ) -> Option<ResolvedInputAction> {
        if input.key_pressed(egui::Key::Escape)
            && self.can_dispatch_global_shortcut()
            && !self.is_text_entry_scope()
            && (self.show_help || self.show_history_panel || self.show_er_diagram)
        {
            if matches!(self.focus_scope(), FocusScope::ErDiagram(_)) {
                return None;
            }
            Some(ResolvedInputAction::HandledLocal(
                RouterLocalAction::CloseWorkspaceOverlay,
            ))
        } else {
            None
        }
    }

    pub(in crate::app) fn is_text_entry_mode(self) -> bool {
        self.input_owner().is_text_entry()
    }

    pub(in crate::app) fn is_text_entry_scope(self) -> bool {
        self.is_text_entry_mode()
    }

    pub(in crate::app) fn allows_data_grid_shortcuts(self) -> bool {
        matches!(self.input_mode(), InputMode::Command | InputMode::Select)
            && matches!(
                self.focus_scope(),
                FocusScope::Grid(GridFocusScope::Normal | GridFocusScope::Select)
            )
    }

    pub(in crate::app) fn allows_refresh(self) -> bool {
        matches!(self.input_mode(), InputMode::Command | InputMode::Select)
            && matches!(
                self.focus_scope(),
                FocusScope::Toolbar
                    | FocusScope::QueryTabs
                    | FocusScope::Sidebar(_)
                    | FocusScope::Grid(GridFocusScope::Normal | GridFocusScope::Select)
            )
    }

    pub(in crate::app) fn allows_panel_visibility_toggle(self) -> bool {
        !matches!(self.focus_scope(), FocusScope::Dialog(_))
            && !matches!(
                self.input_mode(),
                InputMode::TextEntry | InputMode::Recording | InputMode::Disabled
            )
    }

    pub(in crate::app) fn allows_editor_visibility_toggle(self) -> bool {
        if matches!(self.focus_scope(), FocusScope::Dialog(_))
            || matches!(
                self.input_mode(),
                InputMode::Recording | InputMode::Disabled
            )
        {
            return false;
        }

        if matches!(self.focus_scope(), FocusScope::Editor(_)) {
            return true;
        }

        !matches!(self.input_mode(), InputMode::TextEntry)
    }

    pub(in crate::app) fn allows_import_export_shortcuts(self) -> bool {
        self.allows_workspace_surface_shortcuts()
    }

    pub(in crate::app) fn allows_search_shortcuts(self) -> bool {
        matches!(self.input_mode(), InputMode::Command | InputMode::Select)
            && matches!(
                self.focus_scope(),
                FocusScope::QueryTabs
                    | FocusScope::Sidebar(_)
                    | FocusScope::Grid(GridFocusScope::Normal | GridFocusScope::Select)
            )
    }

    pub(in crate::app) fn allows_clear_command_line(self) -> bool {
        matches!(self.focus_scope(), FocusScope::Editor(_))
            && !matches!(self.input_mode(), InputMode::Disabled)
    }

    pub(in crate::app) fn allows_workspace_creation_shortcuts(self) -> bool {
        self.allows_workspace_surface_shortcuts()
            || matches!(
                self.focus_scope(),
                FocusScope::Editor(EditorFocusScope::Normal)
            )
    }

    pub(in crate::app) fn allows_workspace_overlay_shortcuts(self) -> bool {
        self.allows_workspace_surface_shortcuts()
            || matches!(
                self.focus_scope(),
                FocusScope::Editor(EditorFocusScope::Normal)
            )
    }

    pub(in crate::app) fn allows_toggle_er_diagram_shortcuts(self) -> bool {
        self.allows_workspace_overlay_shortcuts()
            || (matches!(self.input_mode(), InputMode::Command | InputMode::Select)
                && matches!(self.focus_scope(), FocusScope::ErDiagram(_)))
    }

    pub(in crate::app) fn allows_tab_management_shortcuts(self) -> bool {
        self.allows_workspace_surface_shortcuts()
            || matches!(
                self.focus_scope(),
                FocusScope::Editor(EditorFocusScope::Normal)
            )
    }

    fn allows_workspace_surface_shortcuts(self) -> bool {
        matches!(self.input_mode(), InputMode::Command | InputMode::Select)
            && matches!(
                self.focus_scope(),
                FocusScope::Toolbar
                    | FocusScope::QueryTabs
                    | FocusScope::Sidebar(_)
                    | FocusScope::Grid(GridFocusScope::Normal | GridFocusScope::Select)
            )
    }

    pub(in crate::app) fn resolve_workspace_fallback_shortcut_action(
        self,
        action: Action,
    ) -> Option<ResolvedInputAction> {
        match action {
            Action::OpenToolbarActionsMenu => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledApp(AppAction::OpenToolbarActionsMenu),
                )
            }
            Action::OpenToolbarCreateMenu => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledApp(AppAction::OpenToolbarCreateMenu),
                )
            }
            Action::OpenThemeSelector => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledApp(AppAction::OpenThemeSelectorDialog),
                )
            }
            Action::OpenKeybindingsDialog => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledApp(AppAction::OpenKeybindingsDialog),
                )
            }
            Action::FocusErDiagram => (self.can_dispatch_global_shortcut()
                && self.is_workspace_command_mode()
                && self.show_er_diagram)
                .then_some(ResolvedInputAction::HandledApp(AppAction::FocusErDiagram)),
            Action::FocusBottomPanel => (self.can_dispatch_global_shortcut()
                && self.is_workspace_command_mode())
            .then_some(ResolvedInputAction::HandledApp(AppAction::FocusBottomPanel)),
            Action::FocusRightInspector => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledApp(AppAction::FocusRightInspector),
                )
            }
            Action::ToggleDarkMode => (self.can_dispatch_global_shortcut()
                && self.is_workspace_command_mode())
            .then_some(ResolvedInputAction::HandledApp(AppAction::ToggleDarkMode)),
            Action::FocusSidebarConnections => {
                self.resolve_sidebar_focus_action(ui::SidebarSection::Connections)
            }
            Action::FocusSidebarDatabases => {
                self.resolve_sidebar_focus_action(ui::SidebarSection::Databases)
            }
            Action::FocusSidebarTables => {
                self.resolve_sidebar_focus_action(ui::SidebarSection::Tables)
            }
            Action::FocusSidebarFilters => {
                self.resolve_sidebar_focus_action(ui::SidebarSection::Filters)
            }
            Action::FocusSidebarTriggers => {
                self.resolve_sidebar_focus_action(ui::SidebarSection::Triggers)
            }
            Action::FocusSidebarRoutines => {
                self.resolve_sidebar_focus_action(ui::SidebarSection::Routines)
            }
            _ => None,
        }
    }

    fn resolve_sidebar_focus_action(
        self,
        section: ui::SidebarSection,
    ) -> Option<ResolvedInputAction> {
        self.can_focus_sidebar_section()
            .then_some(ResolvedInputAction::HandledLocal(
                RouterLocalAction::FocusSidebarSection(section),
            ))
    }
}
