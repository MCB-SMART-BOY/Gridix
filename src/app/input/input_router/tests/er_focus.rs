use super::super::*;
use super::*;

#[test]
fn tracked_er_return_focus_area_only_keeps_workspace_areas() {
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::Sidebar),
        Some(FocusArea::Sidebar)
    );
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::DataGrid),
        Some(FocusArea::DataGrid)
    );
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::SqlEditor),
        Some(FocusArea::SqlEditor)
    );
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::Toolbar),
        None
    );
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::QueryTabs),
        None
    );
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::ErDiagram),
        None
    );
    assert_eq!(
        super::super::tracked_er_return_focus_area(FocusArea::Dialog),
        None
    );
}

#[test]
fn resolve_er_return_focus_area_restores_sidebar_when_visible() {
    assert_eq!(
        super::super::resolve_er_return_focus_area(FocusArea::Sidebar, true, true),
        FocusArea::Sidebar
    );
}

#[test]
fn resolve_er_return_focus_area_restores_sql_editor_when_visible() {
    assert_eq!(
        super::super::resolve_er_return_focus_area(FocusArea::SqlEditor, true, true),
        FocusArea::SqlEditor
    );
}

#[test]
fn resolve_er_return_focus_area_falls_back_to_data_grid_when_target_is_unavailable() {
    assert_eq!(
        super::super::resolve_er_return_focus_area(FocusArea::Sidebar, false, true),
        FocusArea::DataGrid
    );
    assert_eq!(
        super::super::resolve_er_return_focus_area(FocusArea::SqlEditor, true, false),
        FocusArea::DataGrid
    );
    assert_eq!(
        super::super::resolve_er_return_focus_area(FocusArea::Toolbar, true, true),
        FocusArea::DataGrid
    );
}

#[test]
fn closing_er_restores_tracked_workspace_focus() {
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            false,
            FocusArea::ErDiagram,
            FocusArea::Sidebar,
            true,
            true,
        ),
        Some(FocusArea::Sidebar)
    );
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            false,
            FocusArea::ErDiagram,
            FocusArea::SqlEditor,
            true,
            true,
        ),
        Some(FocusArea::SqlEditor)
    );
}

#[test]
fn closing_er_only_restores_focus_from_er_scope() {
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            true,
            FocusArea::ErDiagram,
            FocusArea::Sidebar,
            true,
            true,
        ),
        Some(FocusArea::ErDiagram)
    );
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            false,
            FocusArea::DataGrid,
            FocusArea::Sidebar,
            true,
            true,
        ),
        None
    );
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            false,
            FocusArea::ErDiagram,
            FocusArea::Sidebar,
            false,
            true,
        ),
        Some(FocusArea::DataGrid)
    );
}

#[test]
fn opening_er_focuses_diagram_from_existing_workspace_focus() {
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            true,
            FocusArea::DataGrid,
            FocusArea::Sidebar,
            true,
            true,
        ),
        Some(FocusArea::ErDiagram)
    );
    assert_eq!(
        super::super::focus_after_er_visibility_change(
            true,
            FocusArea::Sidebar,
            FocusArea::Sidebar,
            true,
            true,
        ),
        Some(FocusArea::ErDiagram)
    );
}

#[test]
fn toggle_er_diagram_close_restores_tracked_sidebar_focus_from_viewport_mode() {
    let ctx = egui::Context::default();
    let mut app = test_app();
    app.state.show_sidebar = true;
    app.state.show_er_diagram = true;
    app.state.sidebar_section = SidebarSection::Connections;

    app.set_focus_area(FocusArea::Sidebar);
    app.dispatch_app_action(&ctx, AppAction::FocusErDiagram);
    app.state.er_diagram_state.toggle_interaction_mode();

    assert_eq!(app.state.focus_area, FocusArea::ErDiagram);
    assert!(app.state.er_diagram_state.is_viewport_mode());
    assert_eq!(app.state.last_non_er_workspace_focus, FocusArea::Sidebar);

    app.dispatch_app_action(&ctx, AppAction::ToggleErDiagram);

    assert!(!app.state.show_er_diagram);
    assert_eq!(app.state.focus_area, FocusArea::Sidebar);
    assert_eq!(
        app.capture_input_context(&ctx).focus_scope(),
        FocusScope::Sidebar(SidebarFocusScope::Connections)
    );
}

#[test]
fn toggle_er_diagram_close_falls_back_to_data_grid_when_tracked_focus_is_hidden() {
    let ctx = egui::Context::default();
    let mut app = test_app();
    app.state.show_sidebar = true;
    app.state.show_er_diagram = true;

    app.set_focus_area(FocusArea::Sidebar);
    app.state.show_sidebar = false;
    app.dispatch_app_action(&ctx, AppAction::FocusErDiagram);
    app.state.er_diagram_state.toggle_interaction_mode();

    assert_eq!(app.state.focus_area, FocusArea::ErDiagram);
    assert!(app.state.er_diagram_state.is_viewport_mode());
    assert_eq!(app.state.last_non_er_workspace_focus, FocusArea::Sidebar);

    app.dispatch_app_action(&ctx, AppAction::ToggleErDiagram);

    assert!(!app.state.show_er_diagram);
    assert_eq!(app.state.focus_area, FocusArea::DataGrid);
    assert_eq!(
        app.capture_input_context(&ctx).focus_scope(),
        FocusScope::Grid(GridFocusScope::Normal)
    );
}

#[test]
fn toggle_er_diagram_open_focuses_er_and_preserves_return_focus() {
    let ctx = egui::Context::default();
    let mut app = test_app();
    app.state.show_sidebar = true;
    app.state.show_er_diagram = false;

    app.set_focus_area(FocusArea::Sidebar);
    assert_eq!(app.state.last_non_er_workspace_focus, FocusArea::Sidebar);

    app.dispatch_app_action(&ctx, AppAction::ToggleErDiagram);

    assert!(app.state.show_er_diagram);
    assert_eq!(app.state.focus_area, FocusArea::ErDiagram);
    assert_eq!(app.state.last_non_er_workspace_focus, FocusArea::Sidebar);
    assert_eq!(
        app.capture_input_context(&ctx).focus_scope(),
        FocusScope::ErDiagram(ErDiagramFocusScope::Navigation)
    );
}

#[test]
fn opening_er_requests_fit_to_view_after_loading_starts() {
    let mut app = test_app();
    app.state.show_er_diagram = false;
    assert!(!app.state.show_er_diagram);
    assert!(!app.state.er_diagram_state.has_pending_fit_to_view());

    app.set_er_diagram_visible(true);

    assert!(app.state.show_er_diagram);
    assert!(app.state.er_diagram_state.has_pending_fit_to_view());
}
