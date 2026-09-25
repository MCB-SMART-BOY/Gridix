//! 输入作用域、焦点状态与文本输入守卫类型。

use crate::app::dialogs::host::DialogId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ErDiagramVisibilityNotice {
    Default,
    Silent,
    Custom(&'static str),
}

/// 当前输入聚焦的主作用域。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum FocusScope {
    Global,
    Toolbar,
    QueryTabs,
    Sidebar(SidebarFocusScope),
    Grid(GridFocusScope),
    ErDiagram(ErDiagramFocusScope),
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
            Self::ErDiagram(scope) => scope.keymap_scope_path(),
            Self::Editor(scope) => scope.keymap_scope_path(),
            Self::Dialog(scope) => scope.keymap_scope_path(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ErDiagramFocusScope {
    Navigation,
    Viewport,
}

impl ErDiagramFocusScope {
    const fn keymap_scope_path(self) -> &'static str {
        match self {
            Self::Navigation => "er_diagram",
            Self::Viewport => "er_diagram.viewport",
        }
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
    SchemaDiff,
    CreateDatabase,
    CreateUser,
    Keybindings,
    ToolbarActionsMenu,
    ToolbarCreateMenu,
    ToolbarThemeMenu,
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
            Self::SchemaDiff => DialogId::SchemaDiff.scope_path(),
            Self::CreateDatabase => DialogId::CreateDatabase.scope_path(),
            Self::CreateUser => DialogId::CreateUser.scope_path(),
            Self::Keybindings => DialogId::Keybindings.scope_path(),
            Self::ToolbarActionsMenu => DialogId::ToolbarActionsMenu.scope_path(),
            Self::ToolbarCreateMenu => DialogId::ToolbarCreateMenu.scope_path(),
            Self::ToolbarThemeMenu => DialogId::ToolbarThemeMenu.scope_path(),
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
            DialogId::SchemaDiff => Self::SchemaDiff,
            DialogId::CreateDatabase => Self::CreateDatabase,
            DialogId::CreateUser => Self::CreateUser,
            DialogId::Keybindings => Self::Keybindings,
            DialogId::ToolbarActionsMenu => Self::ToolbarActionsMenu,
            DialogId::ToolbarCreateMenu => Self::ToolbarCreateMenu,
            DialogId::ToolbarThemeMenu => Self::ToolbarThemeMenu,
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
