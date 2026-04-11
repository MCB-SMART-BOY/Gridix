//! 输入路由模块
//!
//! 将真正的全局快捷键集中到一个入口，逐步替换当前分散在不同模块中的全局抢键逻辑。

use eframe::egui;

use crate::app::dialogs::host::DialogId;
use crate::core::{Action, KeyBinding, KeyBindings};
use crate::ui::{self, EditorMode, GridMode, LocalShortcut, ToolbarActions};

use super::DbManagerApp;
use super::action_system::AppAction;
use super::owner::InputOwner;

/// 当前输入聚焦的主作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum FocusScope {
    Global,
    Toolbar,
    QueryTabs,
    Sidebar(SidebarFocusScope),
    Grid(GridFocusScope),
    Editor(EditorFocusScope),
    Dialog(DialogScope),
}

/// 稳定的 focus scope path。
///
/// 这是 router 与 keymap 层之间的边界类型，避免继续在输入主线里传裸字符串。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct FocusScopePath(&'static str);

impl FocusScopePath {
    pub(in crate::app) const fn as_str(self) -> &'static str {
        self.0
    }
}

impl FocusScope {
    pub(in crate::app) const fn path(self) -> FocusScopePath {
        FocusScopePath(match self {
            Self::Global => "global",
            Self::Toolbar => "toolbar",
            Self::QueryTabs => "query_tabs",
            Self::Sidebar(scope) => scope.keymap_scope_path(),
            Self::Grid(scope) => scope.keymap_scope_path(),
            Self::Editor(scope) => scope.keymap_scope_path(),
            Self::Dialog(scope) => scope.keymap_scope_path(),
        })
    }
}

/// 侧边栏子作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum SidebarFocusScope {
    Connections,
    Databases,
    Tables,
    FiltersList,
    FiltersInput,
    Triggers,
    Routines,
}

impl SidebarFocusScope {
    const fn keymap_scope_path(self) -> &'static str {
        match self {
            Self::Connections => "sidebar.connections",
            Self::Databases => "sidebar.databases",
            Self::Tables => "sidebar.tables",
            Self::FiltersList => "sidebar.filters.list",
            Self::FiltersInput => "sidebar.filters.input",
            Self::Triggers => "sidebar.triggers",
            Self::Routines => "sidebar.routines",
        }
    }
}

/// 表格子作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum GridFocusScope {
    Normal,
    Select,
    Insert,
}

impl GridFocusScope {
    const fn keymap_scope_path(self) -> &'static str {
        match self {
            Self::Normal => "grid.normal",
            Self::Select => "grid.select",
            Self::Insert => "grid.insert",
        }
    }
}

/// SQL 编辑器子作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum EditorFocusScope {
    Normal,
    Insert,
}

impl EditorFocusScope {
    const fn keymap_scope_path(self) -> &'static str {
        match self {
            Self::Normal => "editor.normal",
            Self::Insert => "editor.insert",
        }
    }
}

/// 对话框作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum DialogScope {
    Connection,
    Export,
    Import,
    DeleteConfirm,
    Help,
    About,
    WelcomeSetup,
    History,
    Ddl,
    CreateDatabase,
    CreateUser,
    Keybindings,
    CommandPalette,
    Generic,
}

impl DialogScope {
    const fn keymap_scope_path(self) -> &'static str {
        match self {
            Self::Connection => DialogId::Connection.scope_path(),
            Self::Export => DialogId::Export.scope_path(),
            Self::Import => DialogId::Import.scope_path(),
            Self::DeleteConfirm => DialogId::DeleteConfirm.scope_path(),
            Self::Help => DialogId::Help.scope_path(),
            Self::About => DialogId::About.scope_path(),
            Self::WelcomeSetup => DialogId::WelcomeSetup.scope_path(),
            Self::History => DialogId::History.scope_path(),
            Self::Ddl => DialogId::Ddl.scope_path(),
            Self::CreateDatabase => DialogId::CreateDatabase.scope_path(),
            Self::CreateUser => DialogId::CreateUser.scope_path(),
            Self::Keybindings => DialogId::Keybindings.scope_path(),
            Self::CommandPalette => DialogId::CommandPalette.scope_path(),
            Self::Generic => "dialog.generic",
        }
    }
}

impl From<DialogId> for DialogScope {
    fn from(id: DialogId) -> Self {
        match id {
            DialogId::Connection => Self::Connection,
            DialogId::Export => Self::Export,
            DialogId::Import => Self::Import,
            DialogId::DeleteConfirm => Self::DeleteConfirm,
            DialogId::Help => Self::Help,
            DialogId::About => Self::About,
            DialogId::WelcomeSetup => Self::WelcomeSetup,
            DialogId::History => Self::History,
            DialogId::Ddl => Self::Ddl,
            DialogId::CreateDatabase => Self::CreateDatabase,
            DialogId::CreateUser => Self::CreateUser,
            DialogId::Keybindings => Self::Keybindings,
            DialogId::CommandPalette => Self::CommandPalette,
        }
    }
}

/// 当前作用域的输入模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum InputMode {
    Command,
    TextEntry,
    Select,
    Recording,
    Disabled,
}

/// 文本输入保护态。
///
/// 只要该 guard 处于 active，router 就不能把字符键或 workspace fallback 抢走。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum TextEntryGuard {
    Inactive,
    Scoped(FocusScope),
    GlobalWidget,
}

impl TextEntryGuard {
    pub(in crate::app) const fn is_active(self) -> bool {
        !matches!(self, Self::Inactive)
    }
}

/// 待提交的焦点切换请求。
///
/// router 只解析输入意图，不直接把 `Tab` 这种具体 key 写死成最终行为。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum PendingFocusTransition {
    NextFocusArea,
    PrevFocusArea,
}

/// 路由器内部可以直接处理的局部动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouterLocalAction {
    OpenThemeSelector,
    CommitFocusTransition(PendingFocusTransition),
    FocusSidebarSection(ui::SidebarSection),
    CloseWorkspaceOverlay,
    CloseDialog(DialogScope),
    KeybindingsRecordingInput,
}

/// 仍保留在旧兼容层中的真正全局动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TrueGlobalFallbackAction {
    Zoom,
}

/// 输入路由解析后的统一结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResolvedInputAction {
    NoOp,
    HandledLocal(RouterLocalAction),
    HandledApp(AppAction),
    PreservedTrueGlobalFallback(TrueGlobalFallbackAction),
}

/// 最小全局动作。
///
/// 这些动作只在没有 dialog/text-entry/local/workspace fallback 消费输入时才允许触发。
const MINIMAL_GLOBAL_ACTION_SHORTCUTS: &[Action] = &[
    Action::ShowHelp,
    Action::CommandPalette,
    Action::NewConnection,
];

const WORKSPACE_FALLBACK_ACTION_SHORTCUTS: &[Action] = &[
    Action::OpenThemeSelector,
    Action::OpenKeybindingsDialog,
    Action::ToggleDarkMode,
    Action::FocusSidebarConnections,
    Action::FocusSidebarDatabases,
    Action::FocusSidebarTables,
    Action::FocusSidebarFilters,
    Action::FocusSidebarTriggers,
    Action::FocusSidebarRoutines,
];

const KEYMAP_ROUTED_APP_ACTIONS: &[Action] = &[
    Action::NewTable,
    Action::NewDatabase,
    Action::NewUser,
    Action::Export,
    Action::Import,
    Action::ShowHistory,
    Action::ToggleErDiagram,
    Action::Refresh,
    Action::ClearCommandLine,
    Action::ToggleEditor,
    Action::ToggleSidebar,
    Action::ClearSearch,
    Action::Save,
    Action::GotoLine,
    Action::NewTab,
    Action::NextTab,
    Action::PrevTab,
    Action::CloseTab,
];

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
    pub keybindings_recording: bool,
}

impl InputContextSnapshot {
    fn dialog_scope(self) -> Option<DialogScope> {
        if self.active_dialog.is_some() {
            return self.active_dialog;
        }

        if self.show_connection_dialog {
            Some(DialogScope::Connection)
        } else if self.show_export_dialog {
            Some(DialogScope::Export)
        } else if self.show_import_dialog {
            Some(DialogScope::Import)
        } else if self.show_delete_confirm {
            Some(DialogScope::DeleteConfirm)
        } else if self.show_help {
            Some(DialogScope::Help)
        } else if self.show_about {
            Some(DialogScope::About)
        } else if self.show_welcome_setup_dialog {
            Some(DialogScope::WelcomeSetup)
        } else if self.show_history_panel {
            Some(DialogScope::History)
        } else if self.show_ddl_dialog {
            Some(DialogScope::Ddl)
        } else if self.show_create_db_dialog {
            Some(DialogScope::CreateDatabase)
        } else if self.show_create_user_dialog {
            Some(DialogScope::CreateUser)
        } else if self.show_keybindings_dialog {
            Some(DialogScope::Keybindings)
        } else if self.show_command_palette {
            Some(DialogScope::CommandPalette)
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

        if self.text_focus || self.egui_captures_keyboard {
            return FocusScope::Global;
        }

        match self.focus_area {
            ui::FocusArea::Toolbar => FocusScope::Toolbar,
            ui::FocusArea::QueryTabs => FocusScope::QueryTabs,
            ui::FocusArea::Sidebar => FocusScope::Sidebar(self.sidebar_scope()),
            ui::FocusArea::DataGrid => FocusScope::Grid(self.grid_scope()),
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

    fn allows_focus_area_switch(self) -> bool {
        !self.show_autocomplete
            && !self.input_owner().is_modal()
            && !matches!(self.input_owner().mode(), InputMode::Disabled)
            && !self.text_entry_guard().is_active()
    }

    fn can_dispatch_global_shortcut(self) -> bool {
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
            | Action::OpenThemeSelector
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
            Action::ShowHistory | Action::ToggleErDiagram => {
                self.allows_workspace_overlay_shortcuts()
            }
            Action::Refresh => self.allows_refresh(),
            Action::ClearCommandLine => self.allows_clear_command_line(),
            Action::ToggleEditor => self.allows_editor_visibility_toggle(),
            Action::ToggleSidebar => self.allows_panel_visibility_toggle(),
            Action::ClearSearch => self.allows_search_shortcuts(),
            Action::AddFilter | Action::ClearFilters => false,
            Action::Save | Action::GotoLine => self.allows_data_grid_shortcuts(),
            Action::NewTab | Action::NextTab | Action::PrevTab | Action::CloseTab => {
                self.allows_tab_management_shortcuts()
            }
            Action::ZoomIn | Action::ZoomOut | Action::ZoomReset => false,
        };

        allowed.then_some(app_action)
    }

    fn resolve_escape_fallback(self, input: &egui::InputState) -> Option<ResolvedInputAction> {
        if input.key_pressed(egui::Key::Escape)
            && self.can_dispatch_global_shortcut()
            && !self.is_text_entry_scope()
            && (self.show_help || self.show_history_panel || self.show_er_diagram)
        {
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

    fn resolve_workspace_fallback_shortcut_action(
        self,
        action: Action,
    ) -> Option<ResolvedInputAction> {
        match action {
            Action::OpenThemeSelector => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledLocal(RouterLocalAction::OpenThemeSelector),
                )
            }
            Action::OpenKeybindingsDialog => {
                (self.can_dispatch_global_shortcut() && self.is_workspace_command_mode()).then_some(
                    ResolvedInputAction::HandledApp(AppAction::OpenKeybindingsDialog),
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

impl DbManagerApp {
    pub(in crate::app) fn set_focus_area(&mut self, area: ui::FocusArea) {
        self.focus_area = area;
        match area {
            ui::FocusArea::DataGrid => {
                self.grid_state.focused = true;
                self.focus_sql_editor = false;
            }
            ui::FocusArea::SqlEditor => {
                self.grid_state.focused = false;
                self.focus_sql_editor = true;
            }
            ui::FocusArea::Toolbar
            | ui::FocusArea::QueryTabs
            | ui::FocusArea::Sidebar
            | ui::FocusArea::Dialog => {
                self.grid_state.focused = false;
                self.focus_sql_editor = false;
            }
        }
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
            show_autocomplete: self.show_autocomplete,
            show_sql_editor: self.show_sql_editor,
            focus_sql_editor: self.focus_sql_editor,
            focus_area: self.focus_area,
            editor_mode: self.editor_mode,
            sidebar_section: self.sidebar_section,
            filter_input_has_focus: self.sidebar_panel_state.filter_input_has_focus,
            grid_mode: self.grid_state.mode,
            grid_editing_cell: self.grid_state.editing_cell.is_some(),
            show_connection_dialog: self.show_connection_dialog,
            show_export_dialog: self.show_export_dialog,
            show_import_dialog: self.show_import_dialog,
            show_delete_confirm: self.show_delete_confirm,
            show_help: self.show_help,
            show_about: self.show_about,
            show_welcome_setup_dialog: self.show_welcome_setup_dialog,
            show_history_panel: self.show_history_panel,
            show_ddl_dialog: self.ddl_dialog_state.show,
            show_create_db_dialog: self.create_db_dialog_state.show,
            show_create_user_dialog: self.create_user_dialog_state.show,
            show_keybindings_dialog: self.keybindings_dialog_state.show,
            show_command_palette: self.command_palette_state.open,
            show_er_diagram: self.show_er_diagram,
            keybindings_recording: self.keybindings_dialog_state.is_recording(),
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
        toolbar_actions: &mut ToolbarActions,
        action: RouterLocalAction,
    ) {
        match action {
            RouterLocalAction::OpenThemeSelector => {
                toolbar_actions.open_theme_selector = true;
            }
            RouterLocalAction::CommitFocusTransition(transition) => match transition {
                PendingFocusTransition::NextFocusArea => self.cycle_focus(false),
                PendingFocusTransition::PrevFocusArea => self.cycle_focus(true),
            },
            RouterLocalAction::FocusSidebarSection(section) => {
                self.activate_sidebar_section_shortcut(section);
                self.notifications.info(format!(
                    "切换到: {}",
                    sidebar_section_shortcut_name(section)
                ));
            }
            RouterLocalAction::CloseWorkspaceOverlay => {
                if self.show_help {
                    self.show_help = false;
                } else if self.show_history_panel {
                    self.show_history_panel = false;
                } else if self.show_er_diagram {
                    self.show_er_diagram = false;
                }
            }
            RouterLocalAction::CloseDialog(scope) => match scope {
                DialogScope::Help => self.show_help = false,
                DialogScope::About => self.show_about = false,
                DialogScope::History => self.show_history_panel = false,
                DialogScope::DeleteConfirm => self.show_delete_confirm = false,
                DialogScope::Keybindings => self.keybindings_dialog_state.close(),
                _ => {}
            },
            RouterLocalAction::KeybindingsRecordingInput => {
                let _ =
                    ctx.input(|input| self.keybindings_dialog_state.consume_recording_input(input));
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
                self.set_focus_area(ui::FocusArea::DataGrid);
            }
            return;
        }

        self.show_sidebar = true;
        self.set_focus_area(ui::FocusArea::Sidebar);
        self.sidebar_section = section;

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

    pub(in crate::app) fn set_sidebar_visible(&mut self, visible: bool) {
        self.show_sidebar = visible;
        let next_focus = focus_after_sidebar_visibility_change(self.focus_area, visible);
        if next_focus != self.focus_area {
            self.set_focus_area(next_focus);
        }
    }

    pub(in crate::app) fn toggle_sidebar_visibility(&mut self) {
        self.set_sidebar_visible(!self.show_sidebar);
    }

    pub(in crate::app) fn set_sql_editor_visible(&mut self, visible: bool) {
        self.show_sql_editor = visible;
        if visible {
            self.set_focus_area(ui::FocusArea::SqlEditor);
        } else if self.focus_area == ui::FocusArea::SqlEditor {
            self.set_focus_area(ui::FocusArea::DataGrid);
        } else {
            self.focus_sql_editor = false;
        }
    }

    pub(in crate::app) fn toggle_sql_editor_visibility(&mut self) {
        self.set_sql_editor_visible(!self.show_sql_editor);
    }

    pub(in crate::app) fn open_export_dialog(&mut self) {
        if self.result.is_some() {
            self.show_export_dialog = true;
            self.export_status = None;
        }
    }

    pub(in crate::app) fn open_import_dialog(&mut self) {
        self.handle_import();
    }

    pub(in crate::app) fn open_create_table_dialog(&mut self) {
        let db_type = self
            .manager
            .get_active()
            .map(|c| c.config.db_type)
            .unwrap_or_default();
        self.ddl_dialog_state.open_create_table(db_type);
    }

    pub(in crate::app) fn open_create_database_dialog(&mut self) {
        let db_type = self
            .manager
            .get_active()
            .map(|c| c.config.db_type)
            .unwrap_or_default();
        self.create_db_dialog_state.open(db_type);
    }

    pub(in crate::app) fn open_create_user_dialog(&mut self) {
        self.handle_create_user_action();
    }

    pub(in crate::app) fn set_history_panel_visible(&mut self, visible: bool) {
        self.show_history_panel = visible;
    }

    pub(in crate::app) fn toggle_history_panel(&mut self) {
        self.set_history_panel_visible(!self.show_history_panel);
    }

    pub(in crate::app) fn set_er_diagram_visible(&mut self, visible: bool) {
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

    pub(in crate::app) fn toggle_er_diagram_visibility(&mut self) {
        self.set_er_diagram_visible(!self.show_er_diagram);
    }

    pub(in crate::app) fn open_new_query_tab(&mut self) {
        self.sync_sql_to_active_tab();
        self.tab_manager.new_tab();
        self.sync_from_active_tab();
    }

    pub(in crate::app) fn select_next_query_tab(&mut self) {
        self.sync_sql_to_active_tab();
        self.tab_manager.next_tab();
        self.sync_from_active_tab();
    }

    pub(in crate::app) fn select_previous_query_tab(&mut self) {
        self.sync_sql_to_active_tab();
        self.tab_manager.prev_tab();
        self.sync_from_active_tab();
    }

    pub(in crate::app) fn close_active_query_tab(&mut self) {
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

fn resolve_input_action_with(
    input_context: InputContextSnapshot,
    input: &egui::InputState,
    mut scoped_action_triggered: impl FnMut(Action) -> bool,
    mut global_action_triggered: impl FnMut(Action) -> bool,
    mut local_shortcut_triggered: impl FnMut(LocalShortcut) -> bool,
    mut focus_area_switch_triggered: impl FnMut() -> Option<PendingFocusTransition>,
) -> ResolvedInputAction {
    if is_true_global_fallback_input(input, &mut global_action_triggered) {
        return ResolvedInputAction::PreservedTrueGlobalFallback(TrueGlobalFallbackAction::Zoom);
    }

    if input_context.input_mode() == InputMode::Recording
        && matches!(
            input_context.focus_scope(),
            FocusScope::Dialog(DialogScope::Keybindings)
        )
    {
        return ResolvedInputAction::HandledLocal(RouterLocalAction::KeybindingsRecordingInput);
    }

    if let Some(action) =
        resolve_dialog_shortcut_fallback_with(input_context, &mut local_shortcut_triggered)
    {
        return action;
    }

    if !input_context.can_dispatch_global_shortcut() {
        return ResolvedInputAction::NoOp;
    }

    if let Some(action) =
        resolve_keymap_routed_app_action_with(input_context, &mut scoped_action_triggered)
    {
        return ResolvedInputAction::HandledApp(action);
    }

    if let Some(action) = resolve_workspace_fallback_action_with(
        input_context,
        input,
        &mut scoped_action_triggered,
        &mut focus_area_switch_triggered,
    ) {
        return action;
    }

    if let Some(action) =
        resolve_minimal_global_action_with(input_context, &mut scoped_action_triggered)
    {
        return ResolvedInputAction::HandledApp(action);
    }

    ResolvedInputAction::NoOp
}

fn resolve_dialog_shortcut_fallback_with(
    input_context: InputContextSnapshot,
    local_shortcut_triggered: &mut impl FnMut(LocalShortcut) -> bool,
) -> Option<ResolvedInputAction> {
    if !matches!(
        input_context.input_mode(),
        InputMode::Command | InputMode::Select
    ) {
        return None;
    }

    let FocusScope::Dialog(scope) = input_context.focus_scope() else {
        return None;
    };

    match scope {
        DialogScope::Help | DialogScope::History | DialogScope::Keybindings => {
            local_shortcut_triggered(LocalShortcut::Dismiss).then_some(
                ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(scope)),
            )
        }
        DialogScope::About => (local_shortcut_triggered(LocalShortcut::Dismiss)
            || local_shortcut_triggered(LocalShortcut::Confirm))
        .then_some(ResolvedInputAction::HandledLocal(
            RouterLocalAction::CloseDialog(scope),
        )),
        DialogScope::DeleteConfirm => {
            if local_shortcut_triggered(LocalShortcut::DangerConfirm) {
                Some(ResolvedInputAction::HandledApp(
                    AppAction::ConfirmPendingDelete,
                ))
            } else if local_shortcut_triggered(LocalShortcut::DangerCancel) {
                Some(ResolvedInputAction::HandledLocal(
                    RouterLocalAction::CloseDialog(scope),
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn scoped_keybinding_triggered_in_input(
    keybindings: &KeyBindings,
    input_context: InputContextSnapshot,
    action: Action,
    input: &egui::InputState,
) -> bool {
    let scope_path = input_context.keymap_scope_path();

    if let Some(bindings) = keybindings.scoped_bindings_for_action(scope_path, action) {
        return bindings
            .iter()
            .any(|binding| keybinding_matches_input(binding, input));
    }

    global_keybinding_triggered_in_input(keybindings, action, input)
}

fn global_keybinding_triggered_in_input(
    keybindings: &KeyBindings,
    action: Action,
    input: &egui::InputState,
) -> bool {
    keybindings
        .get(action)
        .is_some_and(|binding| keybinding_matches_input(binding, input))
}

fn local_shortcut_triggered_in_input(
    keybindings: &KeyBindings,
    shortcut: LocalShortcut,
    input: &egui::InputState,
) -> bool {
    shortcut
        .bindings_for(keybindings)
        .iter()
        .any(|binding| keybinding_matches_input(binding, input))
}

fn focus_area_switch_triggered_in_input(
    keybindings: &KeyBindings,
    input: &egui::InputState,
) -> Option<PendingFocusTransition> {
    let next_triggered = keybindings
        .get(Action::NextFocusArea)
        .is_some_and(|binding| keybinding_matches_input(binding, input));
    if next_triggered {
        return Some(PendingFocusTransition::NextFocusArea);
    }

    keybindings
        .get(Action::PrevFocusArea)
        .is_some_and(|binding| keybinding_matches_input(binding, input))
        .then_some(PendingFocusTransition::PrevFocusArea)
}

fn keybinding_matches_input(binding: &KeyBinding, input: &egui::InputState) -> bool {
    binding.modifiers.matches(&input.modifiers) && input.key_pressed(binding.key.to_egui_key())
}

fn resolve_focus_area_switch_with(
    input_context: InputContextSnapshot,
    focus_area_switch_triggered: &mut impl FnMut() -> Option<PendingFocusTransition>,
) -> Option<PendingFocusTransition> {
    input_context
        .allows_focus_area_switch()
        .then(focus_area_switch_triggered)
        .flatten()
}

fn resolve_workspace_fallback_action_with(
    input_context: InputContextSnapshot,
    input: &egui::InputState,
    action_triggered: &mut impl FnMut(Action) -> bool,
    focus_area_switch_triggered: &mut impl FnMut() -> Option<PendingFocusTransition>,
) -> Option<ResolvedInputAction> {
    if let Some(transition) =
        resolve_focus_area_switch_with(input_context, focus_area_switch_triggered)
    {
        return Some(ResolvedInputAction::HandledLocal(
            RouterLocalAction::CommitFocusTransition(transition),
        ));
    }

    if let Some(action) = resolve_workspace_shortcut_action_with(input_context, action_triggered) {
        return Some(action);
    }

    input_context.resolve_escape_fallback(input)
}

fn resolve_keymap_routed_app_action_with(
    input_context: InputContextSnapshot,
    action_triggered: &mut impl FnMut(Action) -> bool,
) -> Option<AppAction> {
    KEYMAP_ROUTED_APP_ACTIONS
        .iter()
        .copied()
        .find_map(|shortcut_action| {
            action_triggered(shortcut_action)
                .then(|| input_context.resolve_shortcut_action(shortcut_action))
                .flatten()
        })
}

fn resolve_minimal_global_action_with(
    input_context: InputContextSnapshot,
    action_triggered: &mut impl FnMut(Action) -> bool,
) -> Option<AppAction> {
    MINIMAL_GLOBAL_ACTION_SHORTCUTS
        .iter()
        .copied()
        .find_map(|shortcut_action| {
            action_triggered(shortcut_action)
                .then(|| input_context.resolve_shortcut_action(shortcut_action))
                .flatten()
        })
}

fn resolve_workspace_shortcut_action_with(
    input_context: InputContextSnapshot,
    action_triggered: &mut impl FnMut(Action) -> bool,
) -> Option<ResolvedInputAction> {
    WORKSPACE_FALLBACK_ACTION_SHORTCUTS
        .iter()
        .copied()
        .find_map(|shortcut_action| {
            action_triggered(shortcut_action)
                .then(|| input_context.resolve_workspace_fallback_shortcut_action(shortcut_action))
                .flatten()
        })
}

fn is_true_global_fallback_input(
    input: &egui::InputState,
    action_triggered: &mut impl FnMut(Action) -> bool,
) -> bool {
    action_triggered(Action::ZoomIn)
        || action_triggered(Action::ZoomOut)
        || action_triggered(Action::ZoomReset)
        || (input.modifiers.ctrl && input.smooth_scroll_delta.y != 0.0)
}

fn sidebar_section_shortcut_name(section: ui::SidebarSection) -> &'static str {
    match section {
        ui::SidebarSection::Connections => "连接列表",
        ui::SidebarSection::Databases => "数据库列表",
        ui::SidebarSection::Tables => "表列表",
        ui::SidebarSection::Filters => "筛选面板",
        ui::SidebarSection::Triggers => "触发器列表",
        ui::SidebarSection::Routines => "存储过程列表",
    }
}

fn focus_after_sidebar_visibility_change(
    current_focus: ui::FocusArea,
    visible: bool,
) -> ui::FocusArea {
    if !visible && current_focus == ui::FocusArea::Sidebar {
        ui::FocusArea::DataGrid
    } else {
        current_focus
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AppAction, DialogScope, EditorFocusScope, FocusScope, GridFocusScope, InputContextSnapshot,
        InputMode, KEYMAP_ROUTED_APP_ACTIONS, PendingFocusTransition, ResolvedInputAction,
        RouterLocalAction, SidebarFocusScope, TrueGlobalFallbackAction,
    };
    use crate::core::{Action, KeyBinding, KeyBindings, KeyCode};
    use crate::ui::{EditorMode, FocusArea, GridMode, SidebarSection};
    use egui::{Event, Key, Modifiers};

    fn snapshot() -> InputContextSnapshot {
        InputContextSnapshot {
            has_modal_dialog: false,
            active_dialog: None,
            text_focus: false,
            egui_captures_keyboard: false,
            show_autocomplete: false,
            show_sql_editor: true,
            focus_sql_editor: false,
            focus_area: FocusArea::DataGrid,
            editor_mode: EditorMode::Insert,
            sidebar_section: SidebarSection::Connections,
            filter_input_has_focus: false,
            grid_mode: GridMode::Normal,
            grid_editing_cell: false,
            show_connection_dialog: false,
            show_export_dialog: false,
            show_import_dialog: false,
            show_delete_confirm: false,
            show_help: false,
            show_about: false,
            show_welcome_setup_dialog: false,
            show_history_panel: false,
            show_ddl_dialog: false,
            show_create_db_dialog: false,
            show_create_user_dialog: false,
            show_keybindings_dialog: false,
            show_command_palette: false,
            show_er_diagram: false,
            keybindings_recording: false,
        }
    }

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

    fn resolve_event(
        context: InputContextSnapshot,
        event: Event,
        triggered_action: Option<Action>,
    ) -> ResolvedInputAction {
        let modifiers = match &event {
            Event::Key { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        };
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput {
            events: vec![event],
            modifiers,
            ..Default::default()
        };

        ctx.begin_pass(raw_input);
        let resolved = ctx.input(|input| {
            super::resolve_input_action_with(
                context,
                input,
                |action| triggered_action == Some(action),
                |action| triggered_action == Some(action),
                |_shortcut| false,
                || None,
            )
        });
        let _ = ctx.end_pass();
        resolved
    }

    fn resolve_event_with_keybindings(
        context: InputContextSnapshot,
        event: Event,
        keybindings: &KeyBindings,
    ) -> ResolvedInputAction {
        let modifiers = match &event {
            Event::Key { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        };
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput {
            events: vec![event],
            modifiers,
            ..Default::default()
        };

        ctx.begin_pass(raw_input);
        let resolved = ctx.input(|input| {
            super::resolve_input_action_with(
                context,
                input,
                |action| {
                    super::scoped_keybinding_triggered_in_input(keybindings, context, action, input)
                },
                |action| super::global_keybinding_triggered_in_input(keybindings, action, input),
                |shortcut| super::local_shortcut_triggered_in_input(keybindings, shortcut, input),
                || super::focus_area_switch_triggered_in_input(keybindings, input),
            )
        });
        let _ = ctx.end_pass();
        resolved
    }

    #[test]
    fn modal_dialog_overrides_other_scopes() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_connection_dialog = true;
        context.focus_area = FocusArea::Sidebar;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Dialog(DialogScope::Connection)
        );
        assert_eq!(context.input_mode(), InputMode::Command);
        assert!(!context.allows_focus_area_switch());
    }

    #[test]
    fn active_dialog_owner_overrides_legacy_boolean_order() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.active_dialog = Some(DialogScope::CommandPalette);
        context.show_connection_dialog = true;
        context.focus_area = FocusArea::Sidebar;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Dialog(DialogScope::CommandPalette)
        );
        assert_eq!(context.keymap_scope_path(), "dialog.command_palette");
    }

    #[test]
    fn sidebar_filter_input_is_text_entry_child_scope() {
        let mut context = snapshot();
        context.focus_area = FocusArea::Sidebar;
        context.sidebar_section = SidebarSection::Filters;
        context.filter_input_has_focus = true;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Sidebar(SidebarFocusScope::FiltersInput)
        );
        assert_eq!(context.input_mode(), InputMode::TextEntry);
    }

    #[test]
    fn sql_editor_insert_blocks_focus_cycle() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.focus_sql_editor = true;
        context.editor_mode = EditorMode::Insert;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Editor(EditorFocusScope::Insert)
        );
        assert_eq!(context.input_mode(), InputMode::TextEntry);
        assert!(!context.allows_focus_area_switch());
    }

    #[test]
    fn data_grid_scope_allows_unmodified_focus_cycle() {
        let context = snapshot();

        assert_eq!(
            context.focus_scope(),
            FocusScope::Grid(GridFocusScope::Normal)
        );
        assert!(context.allows_focus_area_switch());
    }

    #[test]
    fn global_text_entry_blocks_focus_cycle() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(context.focus_scope(), FocusScope::Global);
        assert_eq!(context.input_mode(), InputMode::TextEntry);
        assert!(!context.allows_focus_area_switch());
    }

    #[test]
    fn keybindings_recording_uses_recording_mode() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_keybindings_dialog = true;
        context.keybindings_recording = true;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Dialog(DialogScope::Keybindings)
        );
        assert_eq!(context.input_mode(), InputMode::Recording);
    }

    #[test]
    fn keybindings_recording_routes_to_router_local_recording_handler() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_keybindings_dialog = true;
        context.keybindings_recording = true;

        assert_eq!(
            resolve_event(context, key_event(Key::Escape), None),
            ResolvedInputAction::HandledLocal(RouterLocalAction::KeybindingsRecordingInput)
        );
        assert_eq!(
            resolve_event(context, key_event(Key::Q), None),
            ResolvedInputAction::HandledLocal(RouterLocalAction::KeybindingsRecordingInput)
        );
    }

    #[test]
    fn focus_scope_keymap_paths_are_stable() {
        assert_eq!(FocusScope::Global.path().as_str(), "global");
        assert_eq!(FocusScope::Toolbar.path().as_str(), "toolbar");
        assert_eq!(FocusScope::QueryTabs.path().as_str(), "query_tabs");

        let mut context = snapshot();
        context.focus_area = FocusArea::Sidebar;
        context.sidebar_section = SidebarSection::Filters;
        context.filter_input_has_focus = true;
        assert_eq!(context.keymap_scope_path(), "sidebar.filters.input");

        context.filter_input_has_focus = false;
        assert_eq!(context.keymap_scope_path(), "sidebar.filters.list");

        context.focus_area = FocusArea::DataGrid;
        context.grid_mode = GridMode::Select;
        assert_eq!(context.keymap_scope_path(), "grid.select");

        context.focus_area = FocusArea::SqlEditor;
        context.focus_sql_editor = true;
        context.editor_mode = EditorMode::Insert;
        assert_eq!(context.keymap_scope_path(), "editor.insert");

        context.show_help = true;
        assert_eq!(context.keymap_scope_path(), "dialog.help");
    }

    #[test]
    fn data_grid_select_scope_allows_grid_shortcuts() {
        let mut context = snapshot();
        context.grid_mode = GridMode::Select;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Grid(GridFocusScope::Select)
        );
        assert_eq!(context.input_mode(), InputMode::Select);
        assert!(context.allows_data_grid_shortcuts());
        assert!(context.allows_refresh());
    }

    #[test]
    fn sql_editor_normal_blocks_refresh_but_keeps_editor_actions() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.editor_mode = EditorMode::Normal;

        assert_eq!(
            context.focus_scope(),
            FocusScope::Editor(EditorFocusScope::Normal)
        );
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
        assert!(context.allows_editor_visibility_toggle());
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
    fn text_entry_scope_blocks_workspace_level_shortcuts() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        assert!(!context.allows_workspace_creation_shortcuts());
        assert!(!context.allows_workspace_overlay_shortcuts());
        assert!(!context.allows_tab_management_shortcuts());
        assert!(!context.allows_editor_visibility_toggle());
    }

    #[test]
    fn text_entry_scope_blocks_command_mode_app_routes() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(context.input_mode(), InputMode::TextEntry);
        assert_eq!(context.resolve_shortcut_action(Action::NewTable), None);
        assert_eq!(
            context.resolve_shortcut_action(Action::CommandPalette),
            None
        );
        assert_eq!(context.resolve_shortcut_action(Action::AddFilter), None);
    }

    #[test]
    fn text_entry_blocks_all_keymap_routed_app_actions() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        for action in KEYMAP_ROUTED_APP_ACTIONS {
            assert_eq!(
                context.resolve_shortcut_action(*action),
                None,
                "{action:?} should not route while text entry owns input"
            );
        }
    }

    #[test]
    fn grid_insert_text_entry_blocks_workspace_and_app_shortcuts() {
        let mut context = snapshot();
        context.focus_area = FocusArea::DataGrid;
        context.grid_mode = GridMode::Insert;
        context.grid_editing_cell = true;

        assert_eq!(context.input_mode(), InputMode::TextEntry);
        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::F5), &KeyBindings::default()),
            ResolvedInputAction::NoOp
        );
        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Tab), &KeyBindings::default()),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn dialog_blocks_all_keymap_routed_app_actions() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_command_palette = true;

        for action in KEYMAP_ROUTED_APP_ACTIONS {
            assert_eq!(
                context.resolve_shortcut_action(*action),
                None,
                "{action:?} should not route while dialog owns input"
            );
        }
    }

    #[test]
    fn editor_insert_keeps_only_editor_compat_keymap_routes() {
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.focus_sql_editor = true;
        context.editor_mode = EditorMode::Insert;

        let routed_actions: Vec<Action> = KEYMAP_ROUTED_APP_ACTIONS
            .iter()
            .copied()
            .filter(|action| context.resolve_shortcut_action(*action).is_some())
            .collect();

        assert_eq!(
            routed_actions,
            vec![Action::ClearCommandLine, Action::ToggleEditor]
        );
    }

    #[test]
    fn grid_normal_routes_workspace_actions() {
        let context = snapshot();

        assert_eq!(
            context.resolve_shortcut_action(Action::Save),
            Some(AppAction::SaveGridChanges)
        );
        assert_eq!(
            context.resolve_shortcut_action(Action::Refresh),
            Some(AppAction::RefreshActiveConnection)
        );
    }

    #[test]
    fn dialog_scope_blocks_app_shortcut_routes() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_command_palette = true;

        assert_eq!(context.resolve_shortcut_action(Action::Refresh), None);
        assert_eq!(
            context.resolve_shortcut_action(Action::CommandPalette),
            None
        );
    }

    #[test]
    fn workspace_fallback_routes_action_backed_commands_in_command_mode() {
        let context = snapshot();

        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::OpenThemeSelector),
            Some(ResolvedInputAction::HandledLocal(
                RouterLocalAction::OpenThemeSelector
            ))
        );
        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::ToggleDarkMode),
            Some(ResolvedInputAction::HandledApp(AppAction::ToggleDarkMode))
        );
        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::OpenKeybindingsDialog),
            Some(ResolvedInputAction::HandledApp(
                AppAction::OpenKeybindingsDialog
            ))
        );
    }

    #[test]
    fn workspace_fallback_routes_sidebar_section_in_command_mode() {
        let context = snapshot();

        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::FocusSidebarFilters),
            Some(ResolvedInputAction::HandledLocal(
                RouterLocalAction::FocusSidebarSection(SidebarSection::Filters)
            ))
        );
    }

    #[test]
    fn workspace_fallback_blocks_workspace_routes_in_text_entry() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(context.input_mode(), InputMode::TextEntry);
        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::OpenThemeSelector),
            None
        );
        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::ToggleDarkMode),
            None
        );
        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::FocusSidebarFilters),
            None
        );
    }

    #[test]
    fn workspace_fallback_respects_dialog_priority() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_command_palette = true;

        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::ToggleDarkMode),
            None
        );
        assert_eq!(
            context.resolve_workspace_fallback_shortcut_action(Action::FocusSidebarFilters),
            None
        );
    }

    #[test]
    fn next_and_prev_focus_area_use_action_bindings_instead_of_hardcoded_tab() {
        let keybindings = KeyBindings::default();
        let context = snapshot();

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Tab), &keybindings),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CommitFocusTransition(
                PendingFocusTransition::NextFocusArea
            ))
        );

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event_with_modifiers(
                    Key::Tab,
                    Modifiers {
                        shift: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CommitFocusTransition(
                PendingFocusTransition::PrevFocusArea
            ))
        );
    }

    #[test]
    fn next_and_prev_focus_area_can_move_to_ctrl_tab_without_router_change() {
        let mut keybindings = KeyBindings::default();
        keybindings.set(
            Action::NextFocusArea,
            KeyBinding::new(KeyCode::Tab, crate::core::KeyModifiers::CTRL),
        );
        keybindings.set(
            Action::PrevFocusArea,
            KeyBinding::new(KeyCode::Tab, crate::core::KeyModifiers::CTRL_SHIFT),
        );
        // Move the default tab-switch actions away so this test only verifies
        // that the router follows the rebound action instead of hard-coding Tab.
        keybindings.set(Action::NextTab, KeyBinding::ctrl(KeyCode::R));
        keybindings.set(
            Action::PrevTab,
            KeyBinding::new(KeyCode::R, crate::core::KeyModifiers::CTRL_SHIFT),
        );
        let context = snapshot();

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event_with_modifiers(
                    Key::Tab,
                    Modifiers {
                        ctrl: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CommitFocusTransition(
                PendingFocusTransition::NextFocusArea
            ))
        );

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event_with_modifiers(
                    Key::Tab,
                    Modifiers {
                        ctrl: true,
                        shift: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CommitFocusTransition(
                PendingFocusTransition::PrevFocusArea
            ))
        );
    }

    #[test]
    fn next_focus_area_is_blocked_by_text_entry_and_completion_owner() {
        let keybindings = KeyBindings::default();

        let mut text_entry_context = snapshot();
        text_entry_context.text_focus = true;
        text_entry_context.focus_area = FocusArea::Toolbar;
        assert_eq!(
            resolve_event_with_keybindings(text_entry_context, key_event(Key::Tab), &keybindings),
            ResolvedInputAction::NoOp
        );

        let mut completion_context = snapshot();
        completion_context.focus_area = FocusArea::SqlEditor;
        completion_context.focus_sql_editor = true;
        completion_context.editor_mode = EditorMode::Insert;
        completion_context.show_autocomplete = true;
        assert_eq!(
            resolve_event_with_keybindings(completion_context, key_event(Key::Tab), &keybindings),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn editor_insert_confirm_completion_tab_outranks_next_focus_area_tab() {
        let keybindings = KeyBindings::default();
        let mut context = snapshot();
        context.focus_area = FocusArea::SqlEditor;
        context.focus_sql_editor = true;
        context.editor_mode = EditorMode::Insert;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Tab), &keybindings),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn workspace_fallback_wins_before_minimal_global_action() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings(
            "toolbar.toggle_sidebar",
            vec![KeyBinding::new(KeyCode::N, crate::core::KeyModifiers::CTRL)],
        );

        let mut context = snapshot();
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event_with_modifiers(
                    Key::N,
                    Modifiers {
                        ctrl: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::HandledApp(AppAction::ToggleSidebar)
        );
    }

    #[test]
    fn scoped_keymap_action_beats_next_focus_area_workspace_fallback() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings(
            "toolbar.toggle_sidebar",
            vec![KeyBinding::key_only(KeyCode::Tab)],
        );

        let mut context = snapshot();
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Tab), &keybindings),
            ResolvedInputAction::HandledApp(AppAction::ToggleSidebar)
        );
    }

    #[test]
    fn scoped_keymap_action_beats_direct_workspace_fallback_shortcut() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings(
            "toolbar.refresh",
            vec![KeyBinding::new(KeyCode::D, crate::core::KeyModifiers::CTRL)],
        );

        let mut context = snapshot();
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event_with_modifiers(
                    Key::D,
                    Modifiers {
                        ctrl: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::HandledApp(AppAction::RefreshActiveConnection)
        );
    }

    #[test]
    fn resolved_input_action_routes_keymap_app_action_from_router() {
        let context = snapshot();

        assert_eq!(
            resolve_event(context, key_event(Key::F5), Some(Action::Refresh)),
            ResolvedInputAction::HandledApp(AppAction::RefreshActiveConnection)
        );
    }

    #[test]
    fn scoped_keymap_routes_same_key_differently_by_focus_scope() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings("toolbar.refresh", vec![KeyBinding::key_only(KeyCode::R)]);
        keybindings.set_local_bindings(
            "sidebar.tables.show_history",
            vec![KeyBinding::key_only(KeyCode::R)],
        );

        let mut toolbar_context = snapshot();
        toolbar_context.focus_area = FocusArea::Toolbar;
        assert_eq!(
            resolve_event_with_keybindings(toolbar_context, key_event(Key::R), &keybindings),
            ResolvedInputAction::HandledApp(AppAction::RefreshActiveConnection)
        );

        let mut sidebar_context = snapshot();
        sidebar_context.focus_area = FocusArea::Sidebar;
        sidebar_context.sidebar_section = SidebarSection::Tables;
        assert_eq!(
            resolve_event_with_keybindings(sidebar_context, key_event(Key::R), &keybindings),
            ResolvedInputAction::HandledApp(AppAction::ToggleHistoryPanel)
        );
    }

    #[test]
    fn scoped_keymap_text_entry_scope_blocks_command_action() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings(
            "sidebar.filters.input.refresh",
            vec![KeyBinding::key_only(KeyCode::R)],
        );

        let mut context = snapshot();
        context.focus_area = FocusArea::Sidebar;
        context.sidebar_section = SidebarSection::Filters;
        context.filter_input_has_focus = true;

        assert_eq!(context.input_mode(), InputMode::TextEntry);
        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::R), &keybindings),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn filter_shortcut_actions_are_not_routed_by_input_router() {
        let keybindings = KeyBindings::default();

        let mut sidebar_context = snapshot();
        sidebar_context.focus_area = FocusArea::Sidebar;
        sidebar_context.sidebar_section = SidebarSection::Filters;

        assert_eq!(
            resolve_event_with_keybindings(
                sidebar_context,
                key_event_with_modifiers(
                    Key::F,
                    Modifiers {
                        ctrl: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::NoOp
        );

        assert_eq!(
            resolve_event_with_keybindings(
                sidebar_context,
                key_event_with_modifiers(
                    Key::F,
                    Modifiers {
                        ctrl: true,
                        shift: true,
                        command: true,
                        ..Modifiers::NONE
                    }
                ),
                &keybindings,
            ),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn scoped_keymap_falls_back_to_global_bindings_when_scope_has_no_override() {
        let keybindings = KeyBindings::default();
        let mut context = snapshot();
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::F5), &keybindings),
            ResolvedInputAction::HandledApp(AppAction::RefreshActiveConnection)
        );
    }

    #[test]
    fn scoped_keymap_override_takes_precedence_over_global_binding_for_same_action() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings("toolbar.refresh", vec![KeyBinding::key_only(KeyCode::R)]);

        let mut context = snapshot();
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::F5), &keybindings),
            ResolvedInputAction::NoOp
        );
        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::R), &keybindings),
            ResolvedInputAction::HandledApp(AppAction::RefreshActiveConnection)
        );
    }

    #[test]
    fn resolved_input_action_blocks_app_command_in_text_entry() {
        let mut context = snapshot();
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event(context, key_event(Key::F5), Some(Action::Refresh)),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn resolved_input_action_respects_dialog_priority() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_command_palette = true;

        assert_eq!(
            resolve_event(context, key_event(Key::F5), Some(Action::Refresh)),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn dialog_help_scope_routes_dismiss_shortcut_to_close_dialog() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_help = true;
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event(Key::Escape),
                &KeyBindings::default()
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(DialogScope::Help))
        );
    }

    #[test]
    fn dialog_about_scope_routes_confirm_shortcut_to_close_dialog() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_about = true;
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Enter), &KeyBindings::default()),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(DialogScope::About))
        );
    }

    #[test]
    fn dialog_history_scope_routes_dismiss_shortcut_to_close_dialog() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_history_panel = true;
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Q), &KeyBindings::default()),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(DialogScope::History))
        );
    }

    #[test]
    fn dialog_delete_confirm_scope_routes_danger_confirm_to_app_action() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_delete_confirm = true;
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Y), &KeyBindings::default()),
            ResolvedInputAction::HandledApp(AppAction::ConfirmPendingDelete)
        );
    }

    #[test]
    fn dialog_delete_confirm_scope_routes_danger_cancel_to_close_dialog() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_delete_confirm = true;
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event(Key::Escape),
                &KeyBindings::default()
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(
                DialogScope::DeleteConfirm
            ))
        );
    }

    #[test]
    fn dialog_keybindings_scope_routes_dismiss_shortcut_to_close_dialog() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_keybindings_dialog = true;
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event(Key::Escape),
                &KeyBindings::default()
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(
                DialogScope::Keybindings
            ))
        );
    }

    #[test]
    fn dialog_dismiss_fallback_is_blocked_in_text_entry_mode() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_help = true;
        context.focus_area = FocusArea::Dialog;
        context.text_focus = true;
        context.egui_captures_keyboard = true;

        assert_eq!(
            resolve_event_with_keybindings(context, key_event(Key::Q), &KeyBindings::default()),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn resolved_input_action_preserves_zoom_true_global_fallback() {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.show_command_palette = true;

        let mut modifiers = Modifiers::NONE;
        modifiers.ctrl = true;

        assert_eq!(
            resolve_event(
                context,
                key_event_with_modifiers(Key::Plus, modifiers),
                Some(Action::ZoomIn),
            ),
            ResolvedInputAction::PreservedTrueGlobalFallback(TrueGlobalFallbackAction::Zoom)
        );
    }

    #[test]
    fn resolved_input_action_routes_escape_overlay_fallback() {
        let mut context = snapshot();
        context.show_er_diagram = true;

        assert_eq!(
            resolve_event(context, key_event(Key::Escape), None),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseWorkspaceOverlay)
        );
    }

    #[test]
    fn resolved_input_action_does_not_route_escape_overlay_fallback_in_text_entry() {
        let mut context = snapshot();
        context.show_er_diagram = true;
        context.text_focus = true;
        context.focus_area = FocusArea::Toolbar;

        assert_eq!(
            resolve_event(context, key_event(Key::Escape), None),
            ResolvedInputAction::NoOp
        );
    }

    #[test]
    fn showing_sidebar_keeps_existing_focus() {
        assert_eq!(
            super::focus_after_sidebar_visibility_change(FocusArea::DataGrid, true),
            FocusArea::DataGrid
        );
        assert_eq!(
            super::focus_after_sidebar_visibility_change(FocusArea::SqlEditor, true),
            FocusArea::SqlEditor
        );
    }

    #[test]
    fn hiding_sidebar_returns_sidebar_focus_to_data_grid() {
        assert_eq!(
            super::focus_after_sidebar_visibility_change(FocusArea::Sidebar, false),
            FocusArea::DataGrid
        );
    }
}
