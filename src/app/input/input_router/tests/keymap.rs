use super::super::*;
use super::*;

#[test]
fn keybindings_recording_uses_recording_mode() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_keybindings_dialog = true;
    context.active_dialog = Some(DialogScope::Keybindings);
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
    context.active_dialog = Some(DialogScope::Keybindings);
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
