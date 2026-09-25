use super::super::*;
use super::*;

#[test]
fn er_diagram_scope_keeps_toggle_er_diagram_shortcut_available() {
    let mut navigation = snapshot();
    navigation.show_er_diagram = true;
    navigation.focus_area = FocusArea::ErDiagram;

    assert_eq!(
        navigation.resolve_shortcut_action(Action::ToggleErDiagram),
        Some(AppAction::ToggleErDiagram)
    );

    let mut viewport = navigation;
    viewport.er_diagram_viewport_mode = true;

    assert_eq!(
        viewport.resolve_shortcut_action(Action::ToggleErDiagram),
        Some(AppAction::ToggleErDiagram)
    );
}

#[test]
fn er_diagram_scope_does_not_expand_show_history_overlay_shortcut() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;

    assert_eq!(context.resolve_shortcut_action(Action::ShowHistory), None);

    context.er_diagram_viewport_mode = true;

    assert_eq!(context.resolve_shortcut_action(Action::ShowHistory), None);
}

#[test]
fn er_diagram_visibility_notice_defaults_to_generic_messages() {
    assert_eq!(
        super::super::resolve_er_diagram_visibility_notice(
            true,
            ErDiagramVisibilityNotice::Default,
        ),
        Some("ER 关系图已打开")
    );
    assert_eq!(
        super::super::resolve_er_diagram_visibility_notice(
            false,
            ErDiagramVisibilityNotice::Default,
        ),
        Some("ER 关系图已关闭")
    );
}

#[test]
fn er_diagram_visibility_notice_supports_silent_and_custom_modes() {
    assert_eq!(
        super::super::resolve_er_diagram_visibility_notice(true, ErDiagramVisibilityNotice::Silent,),
        None
    );
    assert_eq!(
        super::super::resolve_er_diagram_visibility_notice(
            true,
            ErDiagramVisibilityNotice::Custom("学习示例库的 ER 图已打开"),
        ),
        Some("学习示例库的 ER 图已打开")
    );
}

#[test]
fn er_diagram_scope_routes_escape_to_return_to_workspace() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Escape), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::ReturnToWorkspace
        ))
    );
}

#[test]
fn er_diagram_viewport_scope_routes_escape_to_navigation_mode() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;
    context.er_diagram_viewport_mode = true;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Escape), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::ExitViewportMode
        ))
    );
}

#[test]
fn er_diagram_viewport_scope_routes_q_to_close_diagram() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;
    context.er_diagram_viewport_mode = true;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Q), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::CloseDiagram
        ))
    );
}

#[test]
fn er_diagram_scope_routes_local_navigation_shortcuts() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::H), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::GeometryLeft
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::J), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::NextTable
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::K), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::PrevTable
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::J, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::NextRelatedTable
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::K, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::PrevRelatedTable
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::ArrowLeft, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::GeometryLeft
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::ArrowDown, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::GeometryDown
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::ArrowUp, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::GeometryUp
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::ArrowRight, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::GeometryRight
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::L), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::GeometryRight
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Enter), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::OpenSelectedTable
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Q), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::CloseDiagram
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::ArrowLeft), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::ReturnToWorkspace
        ))
    );
    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event(Key::ArrowRight),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::OpenSelectedTable
        ))
    );
}

#[test]
fn er_diagram_viewport_scope_keeps_hjkl_available_for_render_pan() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;
    context.er_diagram_viewport_mode = true;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::H), &KeyBindings::default()),
        ResolvedInputAction::NoOp
    );
}

#[test]
fn er_diagram_viewport_scope_does_not_route_geometry_navigation_shortcuts() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;
    context.er_diagram_viewport_mode = true;

    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::ArrowRight, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::NoOp
    );
}

#[test]
fn er_diagram_scope_routes_v_to_toggle_viewport_mode() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;

    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::V), &KeyBindings::default()),
        ResolvedInputAction::HandledLocal(RouterLocalAction::ErDiagram(
            ErDiagramLocalAction::ToggleViewportMode
        ))
    );
}

#[test]
fn er_diagram_scope_routes_ctrl_r_to_toggle_diagram_in_navigation_and_viewport_modes() {
    let mut navigation = snapshot();
    navigation.show_er_diagram = true;
    navigation.focus_area = FocusArea::ErDiagram;

    assert_eq!(
        resolve_event_with_keybindings(
            navigation,
            key_event_with_modifiers(
                Key::R,
                Modifiers {
                    ctrl: true,
                    ..Modifiers::NONE
                }
            ),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledApp(AppAction::ToggleErDiagram)
    );

    let mut viewport = navigation;
    viewport.er_diagram_viewport_mode = true;

    assert_eq!(
        resolve_event_with_keybindings(
            viewport,
            key_event_with_modifiers(
                Key::R,
                Modifiers {
                    ctrl: true,
                    ..Modifiers::NONE
                }
            ),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledApp(AppAction::ToggleErDiagram)
    );
}

#[test]
fn er_diagram_geometry_navigation_keeps_app_selected_table_unchanged_until_open_selected() {
    let ctx = egui::Context::default();
    let mut app = test_app();
    let mut toolbar_actions = ToolbarActions::default();

    app.state.show_er_diagram = true;
    app.state.focus_area = FocusArea::ErDiagram;
    app.state.selected_table = Some("main_workspace_table".to_string());
    app.state.er_diagram_state.tables = vec![
        er_table("customers", 0.0, 0.0),
        er_table("orders", 240.0, 0.0),
    ];
    assert!(app.state.er_diagram_state.select_table(0));

    app.apply_router_local_action(
        &ctx,
        &mut toolbar_actions,
        RouterLocalAction::ErDiagram(ErDiagramLocalAction::GeometryRight),
    );

    assert_eq!(
        app.state.er_diagram_state.selected_table_name(),
        Some("orders")
    );
    assert_eq!(
        app.state.selected_table.as_deref(),
        Some("main_workspace_table")
    );
}

#[test]
fn er_diagram_shift_l_is_reserved_for_layout_not_open_selected() {
    let mut context = snapshot();
    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;

    assert_eq!(
        resolve_event_with_keybindings(
            context,
            key_event_with_modifiers(Key::L, Modifiers::SHIFT),
            &KeyBindings::default()
        ),
        ResolvedInputAction::NoOp
    );
}
