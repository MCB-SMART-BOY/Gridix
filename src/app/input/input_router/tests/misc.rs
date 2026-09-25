use super::super::*;
use super::*;

#[test]
fn modal_dialog_overrides_other_scopes() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_connection_dialog = true;
    context.active_dialog = Some(DialogScope::Connection);
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
fn global_text_entry_blocks_focus_cycle() {
    let mut context = snapshot();
    context.text_focus = true;
    context.focus_area = FocusArea::Toolbar;

    assert_eq!(context.focus_scope(), FocusScope::Global);
    assert_eq!(context.input_mode(), InputMode::TextEntry);
    assert!(!context.allows_focus_area_switch());
}

#[test]
fn legacy_dialog_booleans_without_active_owner_fall_back_to_generic_scope() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_help = true;

    assert_eq!(
        context.focus_scope(),
        FocusScope::Dialog(DialogScope::Generic)
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
fn form_dialog_scopes_route_escape_to_close_dialog() {
    // 审计 DLG-A2-2：表单类对话框在无文本焦点时 Esc 应关闭。
    for scope in [
        DialogScope::Connection,
        DialogScope::Export,
        DialogScope::Import,
        DialogScope::Ddl,
        DialogScope::CreateDatabase,
        DialogScope::CreateUser,
    ] {
        let mut context = snapshot();
        context.has_modal_dialog = true;
        context.active_dialog = Some(scope);
        context.focus_area = FocusArea::Dialog;

        assert_eq!(
            resolve_event_with_keybindings(
                context,
                key_event(Key::Escape),
                &KeyBindings::default()
            ),
            ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(scope)),
            "scope {scope:?} must route Esc to CloseDialog"
        );
    }
}

#[test]
fn ctrl_shift_j_and_i_route_to_panel_focus_app_actions() {
    // 审计 DLG-B1-1：Ctrl+Shift+J / Ctrl+Shift+I 在工作区命令模式下解析为面板聚焦动作。
    let context = snapshot(); // 默认 focus_area=DataGrid，即工作区命令模式
    let mut mods = Modifiers::NONE;
    mods.ctrl = true;
    mods.shift = true;
    let keys = KeyBindings::default();

    assert_eq!(
        resolve_event_with_keybindings(context, key_event_with_modifiers(Key::J, mods), &keys),
        ResolvedInputAction::HandledApp(AppAction::FocusBottomPanel)
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event_with_modifiers(Key::I, mods), &keys),
        ResolvedInputAction::HandledApp(AppAction::FocusRightInspector)
    );
}

#[test]
fn hidden_er_diagram_focus_area_does_not_retain_er_input_scope() {
    let mut context = snapshot();
    context.focus_area = FocusArea::ErDiagram;

    assert_eq!(context.focus_scope(), FocusScope::Global);
}

#[test]
fn showing_sidebar_keeps_existing_focus() {
    assert_eq!(
        super::super::focus_after_sidebar_visibility_change(FocusArea::DataGrid, true),
        FocusArea::DataGrid
    );
    assert_eq!(
        super::super::focus_after_sidebar_visibility_change(FocusArea::SqlEditor, true),
        FocusArea::SqlEditor
    );
}

#[test]
fn hiding_sidebar_returns_sidebar_focus_to_data_grid() {
    assert_eq!(
        super::super::focus_after_sidebar_visibility_change(FocusArea::Sidebar, false),
        FocusArea::DataGrid
    );
}
