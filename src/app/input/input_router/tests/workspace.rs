use super::super::*;
use super::*;

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
fn focus_scope_keymap_paths_are_stable() {
    assert_eq!(FocusScope::Global.path().as_str(), "global");
    assert_eq!(FocusScope::Toolbar.path().as_str(), "toolbar");
    assert_eq!(FocusScope::QueryTabs.path().as_str(), "query_tabs");
    assert_eq!(
        FocusScope::ErDiagram(ErDiagramFocusScope::Navigation)
            .path()
            .as_str(),
        "er_diagram"
    );
    assert_eq!(
        FocusScope::ErDiagram(ErDiagramFocusScope::Viewport)
            .path()
            .as_str(),
        "er_diagram.viewport"
    );

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

    context.show_er_diagram = true;
    context.focus_area = FocusArea::ErDiagram;
    context.focus_sql_editor = false;
    assert_eq!(context.keymap_scope_path(), "er_diagram");

    context.er_diagram_viewport_mode = true;
    assert_eq!(context.keymap_scope_path(), "er_diagram.viewport");

    context.has_modal_dialog = true;
    context.active_dialog = Some(DialogScope::Help);
    context.show_help = true;
    assert_eq!(context.keymap_scope_path(), "dialog.help");
}

#[test]
fn workspace_fallback_routes_action_backed_commands_in_command_mode() {
    let mut context = snapshot();
    context.show_er_diagram = true;

    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::OpenThemeSelector),
        Some(ResolvedInputAction::HandledApp(
            AppAction::OpenThemeSelectorDialog
        ))
    );
    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::OpenToolbarActionsMenu),
        Some(ResolvedInputAction::HandledApp(
            AppAction::OpenToolbarActionsMenu
        ))
    );
    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::OpenToolbarCreateMenu),
        Some(ResolvedInputAction::HandledApp(
            AppAction::OpenToolbarCreateMenu
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
    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::FocusErDiagram),
        Some(ResolvedInputAction::HandledApp(AppAction::FocusErDiagram))
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
    context.show_er_diagram = true;

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
    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::FocusErDiagram),
        None
    );
}

#[test]
fn workspace_fallback_respects_dialog_priority() {
    let mut context = snapshot();
    context.has_modal_dialog = true;
    context.show_command_palette = true;
    context.show_er_diagram = true;

    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::ToggleDarkMode),
        None
    );
    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::FocusSidebarFilters),
        None
    );
    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::FocusErDiagram),
        None
    );
}

#[test]
fn workspace_fallback_blocks_focus_er_diagram_when_hidden() {
    let context = snapshot();

    assert_eq!(
        context.resolve_workspace_fallback_shortcut_action(Action::FocusErDiagram),
        None
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
fn workspace_fallback_routes_focus_er_diagram_from_default_binding_only_when_visible() {
    let mut visible = snapshot();
    visible.show_er_diagram = true;

    assert_eq!(
        resolve_event_with_keybindings(
            visible,
            key_event_with_modifiers(
                Key::R,
                Modifiers {
                    alt: true,
                    ..Modifiers::NONE
                }
            ),
            &KeyBindings::default()
        ),
        ResolvedInputAction::HandledApp(AppAction::FocusErDiagram)
    );

    assert_eq!(
        resolve_event_with_keybindings(
            snapshot(),
            key_event_with_modifiers(
                Key::R,
                Modifiers {
                    alt: true,
                    ..Modifiers::NONE
                }
            ),
            &KeyBindings::default()
        ),
        ResolvedInputAction::NoOp
    );
}

#[test]
fn workspace_fallback_prefers_focus_transition_before_workspace_shortcut() {
    let context = snapshot();
    let ctx = egui::Context::default();
    ctx.begin_pass(egui::RawInput::default());
    let mut action_triggered = |action| action == Action::OpenThemeSelector;
    let mut focus_transition = || Some(PendingFocusTransition::PrevFocusArea);

    let resolved = ctx.input(|input| {
        super::super::resolve_workspace_fallback_action_with(
            context,
            input,
            &mut action_triggered,
            &mut focus_transition,
        )
    });
    let _ = ctx.end_pass();

    assert_eq!(
        resolved,
        Some(ResolvedInputAction::HandledLocal(
            RouterLocalAction::CommitFocusTransition(PendingFocusTransition::PrevFocusArea)
        ))
    );
}

#[test]
fn sidebar_filter_text_entry_blocks_focus_cycle_shortcut() {
    let mut context = snapshot();
    context.focus_area = FocusArea::Sidebar;
    context.sidebar_section = SidebarSection::Filters;
    context.filter_input_has_focus = true;

    assert_eq!(
        context.focus_scope(),
        FocusScope::Sidebar(SidebarFocusScope::FiltersInput)
    );
    assert_eq!(context.input_mode(), InputMode::TextEntry);
    assert_eq!(
        resolve_event_with_keybindings(context, key_event(Key::Tab), &KeyBindings::default()),
        ResolvedInputAction::NoOp
    );
}

#[test]
fn focus_scope_transitions_preserve_surface_and_text_entry_semantics() {
    let mut context = snapshot();
    assert_eq!(
        context.focus_scope(),
        FocusScope::Grid(GridFocusScope::Normal)
    );
    assert_eq!(context.input_mode(), InputMode::Command);

    context.focus_area = FocusArea::Sidebar;
    context.sidebar_section = SidebarSection::Filters;
    assert_eq!(
        context.focus_scope(),
        FocusScope::Sidebar(SidebarFocusScope::FiltersList)
    );
    assert_eq!(context.input_mode(), InputMode::Command);

    context.filter_input_has_focus = true;
    assert_eq!(
        context.focus_scope(),
        FocusScope::Sidebar(SidebarFocusScope::FiltersInput)
    );
    assert_eq!(context.input_mode(), InputMode::TextEntry);

    context.focus_area = FocusArea::SqlEditor;
    context.focus_sql_editor = true;
    context.editor_mode = EditorMode::Insert;
    assert_eq!(
        context.focus_scope(),
        FocusScope::Editor(EditorFocusScope::Insert)
    );
    assert_eq!(context.input_mode(), InputMode::TextEntry);

    context.has_modal_dialog = true;
    context.active_dialog = Some(DialogScope::Help);
    assert_eq!(context.focus_scope(), FocusScope::Dialog(DialogScope::Help));
    assert_eq!(context.input_mode(), InputMode::Command);
}
