use super::super::*;
use super::*;

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
    assert_eq!(
        context.resolve_shortcut_action(Action::OpenKeybindingsDialog),
        None
    );
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
