//! 路由器局部动作类型与快捷键常量表。

use crate::core::Action;
use crate::ui::{self};

use crate::app::input::action_system::AppAction;

use super::scopes::*;

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
pub(in crate::app) enum RouterLocalAction {
    CommitFocusTransition(PendingFocusTransition),
    FocusSidebarSection(ui::SidebarSection),
    ErDiagram(ErDiagramLocalAction),
    CloseWorkspaceOverlay,
    CloseDialog(DialogScope),
    KeybindingsRecordingInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ErDiagramLocalAction {
    PrevTable,
    NextTable,
    PrevRelatedTable,
    NextRelatedTable,
    GeometryLeft,
    GeometryDown,
    GeometryUp,
    GeometryRight,
    OpenSelectedTable,
    ReturnToWorkspace,
    CloseDiagram,
    ToggleViewportMode,
    ExitViewportMode,
}

/// 仍保留在旧兼容层中的真正全局动作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum TrueGlobalFallbackAction {
    Zoom,
}

/// 输入路由解析后的统一结果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum ResolvedInputAction {
    NoOp,
    HandledLocal(RouterLocalAction),
    HandledApp(AppAction),
    PreservedTrueGlobalFallback(TrueGlobalFallbackAction),
}

/// 最小全局动作。
///
/// 这些动作只在没有 dialog/text-entry/local/workspace fallback 消费输入时才允许触发。
pub(in crate::app) const MINIMAL_GLOBAL_ACTION_SHORTCUTS: &[Action] = &[
    Action::ShowHelp,
    Action::CommandPalette,
    Action::NewConnection,
];

pub(in crate::app) const WORKSPACE_FALLBACK_ACTION_SHORTCUTS: &[Action] = &[
    Action::OpenToolbarActionsMenu,
    Action::OpenToolbarCreateMenu,
    Action::OpenThemeSelector,
    Action::OpenKeybindingsDialog,
    Action::FocusErDiagram,
    Action::FocusBottomPanel,
    Action::FocusRightInspector,
    Action::ToggleDarkMode,
    Action::FocusSidebarConnections,
    Action::FocusSidebarDatabases,
    Action::FocusSidebarTables,
    Action::FocusSidebarFilters,
    Action::FocusSidebarTriggers,
    Action::FocusSidebarRoutines,
];

pub(in crate::app) const KEYMAP_ROUTED_APP_ACTIONS: &[Action] = &[
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
