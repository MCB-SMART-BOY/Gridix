use super::super::*;
use super::*;

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
fn dialog_help_scope_routes_dismiss_shortcut_to_close_dialog() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_help = true;
    context.active_dialog = Some(DialogScope::Help);
    context.focus_area = FocusArea::Dialog;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Escape), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(DialogScope::Help))
    );
}

#[test]
fn dialog_about_scope_routes_confirm_shortcut_to_close_dialog() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_about = true;
    context.active_dialog = Some(DialogScope::About);
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
    context.active_dialog = Some(DialogScope::History);
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
    context.active_dialog = Some(DialogScope::DeleteConfirm);
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
    context.active_dialog = Some(DialogScope::DeleteConfirm);
    context.focus_area = FocusArea::Dialog;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Escape), &KeyBindings::default()),
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
    context.active_dialog = Some(DialogScope::Keybindings);
    context.focus_area = FocusArea::Dialog;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Escape), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::CloseDialog(DialogScope::Keybindings))
    );
}

#[test]
fn dialog_dismiss_fallback_is_blocked_in_text_entry_mode() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_help = true;
    context.active_dialog = Some(DialogScope::Help);
    context.focus_area = FocusArea::Dialog;
    context.text_focus = true;
    context.egui_captures_keyboard = true;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Q), &KeyBindings::default()),
        ResolvedInputAction::NoOp
    );
}
