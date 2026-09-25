use super::super::*;
use super::*;

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
