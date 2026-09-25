//! 纯解析函数：在不触碰应用状态的前提下决定输入归属。

use eframe::egui;

use crate::core::{Action, KeyBinding, KeyBindings};
use crate::ui::{self, LocalShortcut};

use crate::app::input::action_system::AppAction;

use super::actions::*;
use super::context::*;
use super::scopes::*;

pub(in crate::app) fn resolve_input_action_with(
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

    if let Some(action) =
        resolve_er_diagram_shortcut_action_with(input_context, &mut local_shortcut_triggered)
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

pub(in crate::app) fn resolve_er_diagram_visibility_notice(
    visible: bool,
    notice: ErDiagramVisibilityNotice,
) -> Option<&'static str> {
    match notice {
        ErDiagramVisibilityNotice::Default => Some(if visible {
            "ER 关系图已打开"
        } else {
            "ER 关系图已关闭"
        }),
        ErDiagramVisibilityNotice::Silent => None,
        ErDiagramVisibilityNotice::Custom(message) => Some(message),
    }
}

pub(in crate::app) fn resolve_dialog_shortcut_fallback_with(
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
        // 表单类对话框：Esc 关闭，补全键盘契约（修复审计 DLG-A2-2）。
        DialogScope::Connection
        | DialogScope::Export
        | DialogScope::Import
        | DialogScope::Ddl
        | DialogScope::CreateDatabase
        | DialogScope::CreateUser => local_shortcut_triggered(LocalShortcut::Dismiss).then_some(
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(scope)),
        ),
        _ => None,
    }
}

pub(in crate::app) fn resolve_er_diagram_shortcut_action_with(
    input_context: InputContextSnapshot,
    local_shortcut_triggered: &mut impl FnMut(LocalShortcut) -> bool,
) -> Option<ResolvedInputAction> {
    if !matches!(
        input_context.input_mode(),
        InputMode::Command | InputMode::Select
    ) {
        return None;
    }

    let FocusScope::ErDiagram(scope) = input_context.focus_scope() else {
        return None;
    };

    let action = match scope {
        ErDiagramFocusScope::Navigation => {
            if local_shortcut_triggered(LocalShortcut::ErDiagramViewportMode) {
                ErDiagramLocalAction::ToggleViewportMode
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramBack) {
                ErDiagramLocalAction::ReturnToWorkspace
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramClose) {
                ErDiagramLocalAction::CloseDiagram
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramPrevTable) {
                ErDiagramLocalAction::PrevTable
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramNextTable) {
                ErDiagramLocalAction::NextTable
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramPrevRelated) {
                ErDiagramLocalAction::PrevRelatedTable
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramNextRelated) {
                ErDiagramLocalAction::NextRelatedTable
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramGeometryLeft) {
                ErDiagramLocalAction::GeometryLeft
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramGeometryDown) {
                ErDiagramLocalAction::GeometryDown
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramGeometryUp) {
                ErDiagramLocalAction::GeometryUp
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramGeometryRight) {
                ErDiagramLocalAction::GeometryRight
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramOpenSelected) {
                ErDiagramLocalAction::OpenSelectedTable
            } else {
                return None;
            }
        }
        ErDiagramFocusScope::Viewport => {
            if local_shortcut_triggered(LocalShortcut::ErDiagramViewportMode) {
                ErDiagramLocalAction::ToggleViewportMode
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramViewportExit) {
                ErDiagramLocalAction::ExitViewportMode
            } else if local_shortcut_triggered(LocalShortcut::ErDiagramClose) {
                ErDiagramLocalAction::CloseDiagram
            } else {
                return None;
            }
        }
    };

    Some(ResolvedInputAction::HandledLocal(
        RouterLocalAction::ErDiagram(action),
    ))
}

pub(in crate::app) fn scoped_keybinding_triggered_in_input(
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

pub(in crate::app) fn global_keybinding_triggered_in_input(
    keybindings: &KeyBindings,
    action: Action,
    input: &egui::InputState,
) -> bool {
    keybindings
        .get(action)
        .is_some_and(|binding| keybinding_matches_input(binding, input))
}

pub(in crate::app) fn local_shortcut_triggered_in_input(
    keybindings: &KeyBindings,
    shortcut: LocalShortcut,
    input: &egui::InputState,
) -> bool {
    shortcut
        .bindings_for(keybindings)
        .iter()
        .any(|binding| keybinding_matches_input(binding, input))
}

pub(in crate::app) fn focus_area_switch_triggered_in_input(
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

pub(in crate::app) fn keybinding_matches_input(
    binding: &KeyBinding,
    input: &egui::InputState,
) -> bool {
    binding.modifiers.matches(&input.modifiers) && input.key_pressed(binding.key.to_egui_key())
}

pub(in crate::app) fn resolve_focus_area_switch_with(
    input_context: InputContextSnapshot,
    focus_area_switch_triggered: &mut impl FnMut() -> Option<PendingFocusTransition>,
) -> Option<PendingFocusTransition> {
    input_context
        .allows_focus_area_switch()
        .then(focus_area_switch_triggered)
        .flatten()
}

pub(in crate::app) fn resolve_workspace_fallback_action_with(
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

pub(in crate::app) fn resolve_keymap_routed_app_action_with(
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

pub(in crate::app) fn resolve_minimal_global_action_with(
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

pub(in crate::app) fn resolve_workspace_shortcut_action_with(
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

pub(in crate::app) fn is_true_global_fallback_input(
    input: &egui::InputState,
    action_triggered: &mut impl FnMut(Action) -> bool,
) -> bool {
    action_triggered(Action::ZoomIn)
        || action_triggered(Action::ZoomOut)
        || action_triggered(Action::ZoomReset)
        || (input.modifiers.ctrl && input.smooth_scroll_delta.y != 0.0)
}

pub(in crate::app) fn sidebar_section_shortcut_name(section: ui::SidebarSection) -> &'static str {
    match section {
        ui::SidebarSection::Connections => "连接列表",
        ui::SidebarSection::Databases => "数据库列表",
        ui::SidebarSection::Tables => "表列表",
        ui::SidebarSection::Filters => "筛选面板",
        ui::SidebarSection::Triggers => "触发器列表",
        ui::SidebarSection::Routines => "存储过程列表",
    }
}

pub(in crate::app) fn focus_after_sidebar_visibility_change(
    current_focus: ui::FocusArea,
    visible: bool,
) -> ui::FocusArea {
    if !visible && current_focus == ui::FocusArea::Sidebar {
        ui::FocusArea::DataGrid
    } else {
        current_focus
    }
}

pub(in crate::app) fn tracked_er_return_focus_area(area: ui::FocusArea) -> Option<ui::FocusArea> {
    match area {
        ui::FocusArea::Sidebar | ui::FocusArea::DataGrid | ui::FocusArea::SqlEditor => Some(area),
        ui::FocusArea::Toolbar
        | ui::FocusArea::QueryTabs
        | ui::FocusArea::ErDiagram
        | ui::FocusArea::Dialog => None,
    }
}

pub(in crate::app) fn focus_after_er_visibility_change(
    visible: bool,
    current_focus: ui::FocusArea,
    last_non_er_workspace_focus: ui::FocusArea,
    show_sidebar: bool,
    show_sql_editor: bool,
) -> Option<ui::FocusArea> {
    if visible {
        return Some(ui::FocusArea::ErDiagram);
    }

    if current_focus != ui::FocusArea::ErDiagram {
        return None;
    }

    Some(resolve_er_return_focus_area(
        last_non_er_workspace_focus,
        show_sidebar,
        show_sql_editor,
    ))
}

pub(in crate::app) fn resolve_er_return_focus_area(
    last_non_er_workspace_focus: ui::FocusArea,
    show_sidebar: bool,
    show_sql_editor: bool,
) -> ui::FocusArea {
    match last_non_er_workspace_focus {
        ui::FocusArea::Sidebar if show_sidebar => ui::FocusArea::Sidebar,
        ui::FocusArea::SqlEditor if show_sql_editor => ui::FocusArea::SqlEditor,
        ui::FocusArea::DataGrid => ui::FocusArea::DataGrid,
        ui::FocusArea::Sidebar
        | ui::FocusArea::SqlEditor
        | ui::FocusArea::Toolbar
        | ui::FocusArea::QueryTabs
        | ui::FocusArea::ErDiagram
        | ui::FocusArea::Dialog => ui::FocusArea::DataGrid,
    }
}
