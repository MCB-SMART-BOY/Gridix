//! Workbench shell orchestration.
//!
//! This module is the app-bound adapter between legacy rendering state and the
//! new editor-style workbench shell. It deliberately keeps existing regions in
//! place while Phase 2 establishes state and shell boundaries.

use eframe::egui;

use crate::core::{BottomPanelTab, RightInspectorTab, WorkbenchActivity, constants};
use crate::data::Connection;
use crate::state::{WorkbenchFocus, WorkbenchSurfaceKind};
use crate::types::QueryResult;
use crate::ui::{self, ToolbarActions, WorkbenchStatusBarContent};

use super::DbManagerApp;
use super::action_system::AppAction;

pub(in crate::app) fn clamped_bottom_panel_height(
    preferred_height: f32,
    available_height: f32,
    min_height: f32,
    max_height_ratio: f32,
) -> f32 {
    let available_height = available_height.max(0.0);
    let ratio = if max_height_ratio.is_finite() {
        max_height_ratio.clamp(0.1, 0.9)
    } else {
        constants::ui::workbench::BOTTOM_PANEL_MAX_HEIGHT_RATIO
    };
    let max_height = available_height * ratio;
    if max_height <= 0.0 {
        return 0.0;
    }

    let min_height = min_height.max(0.0).min(max_height);
    preferred_height.clamp(min_height, max_height)
}

pub(in crate::app) fn clamped_right_inspector_width(
    preferred_width: f32,
    available_width: f32,
    min_width: f32,
    max_width: f32,
) -> f32 {
    let available_width = available_width.max(0.0);
    if available_width <= 0.0 {
        return 0.0;
    }

    let max_width = max_width.max(0.0).min(available_width);
    if max_width <= 0.0 {
        return 0.0;
    }

    let min_width = min_width.max(0.0).min(max_width);
    preferred_width.clamp(min_width, max_width)
}

impl DbManagerApp {
    pub(in crate::app) fn default_workbench_surface_layout(
        &self,
    ) -> egui_dock::DockState<ui::dock_tabs::DockTab> {
        let active_query_tab_id = self
            .session
            .tab_manager
            .get_active()
            .map(|tab| tab.id.clone())
            .unwrap_or_else(|| "query-0".to_string());
        ui::dock_tabs::default_surface_layout(
            active_query_tab_id,
            self.state.workbench.right_inspector.active_tab,
        )
    }

    pub(in crate::app) fn reveal_workbench_surface(
        &mut self,
        surface: WorkbenchSurfaceKind,
    ) -> bool {
        let inserted = ui::dock_tabs::ensure_surface_tab(&mut self.dock_state, surface.clone());
        self.state.workbench.set_focused_surface(&surface);
        inserted
    }

    pub(in crate::app) fn has_workbench_surface_tab(&self, surface: &WorkbenchSurfaceKind) -> bool {
        ui::dock_tabs::has_surface_tab(&self.dock_state, surface)
    }

    pub(in crate::app) fn active_activity_surface_is_docked(&self) -> bool {
        self.has_workbench_surface_tab(&WorkbenchSurfaceKind::from_activity(
            self.state.workbench.active_activity,
        ))
    }

    pub(in crate::app) fn active_bottom_panel_surface_is_docked(&self) -> bool {
        self.has_workbench_surface_tab(
            &self.active_bottom_panel_surface_kind(self.state.workbench.bottom_panel.active_tab),
        )
    }

    pub(in crate::app) fn active_right_inspector_surface_is_docked(&self) -> bool {
        self.has_workbench_surface_tab(&WorkbenchSurfaceKind::from_right_inspector_tab(
            self.state.workbench.right_inspector.active_tab,
        ))
    }

    pub(in crate::app) fn render_activity_bar(
        &mut self,
        ui: &mut egui::Ui,
    ) -> ui::WorkbenchActivityBarResponse {
        let response = ui::WorkbenchActivityBar::show(
            ui,
            self.state.workbench.active_activity,
            self.state.show_sidebar,
        );
        if response.selected_activity.is_some() || response.toggle_sidebar {
            self.state.workbench.focus = WorkbenchFocus::ActivityBar;
        }
        response
    }

    pub(in crate::app) fn set_workbench_activity(&mut self, activity: WorkbenchActivity) {
        self.state.workbench.active_activity = activity;
        self.app_config.workbench.activity = activity;
        self.reveal_workbench_surface(WorkbenchSurfaceKind::from_activity(activity));
        self.set_primary_sidebar_visible(true);
        self.apply_workbench_activity_to_sidebar_panels();
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_primary_sidebar_visible(&mut self, visible: bool) {
        self.set_sidebar_visible(visible);
        self.app_config.workbench.sidebar.visible = visible;
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_bottom_panel_visible(&mut self, visible: bool) {
        self.state.workbench.bottom_panel.visible = visible;
        self.app_config.workbench.bottom_panel.visible = visible;
        if visible {
            let surface =
                self.active_bottom_panel_surface_kind(self.state.workbench.bottom_panel.active_tab);
            self.reveal_workbench_surface(surface);
        }
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_bottom_panel_tab(&mut self, tab: BottomPanelTab) {
        self.state.workbench.bottom_panel.active_tab = tab;
        self.app_config.workbench.bottom_panel.active_tab = tab;
        let surface = self.active_bottom_panel_surface_kind(tab);
        self.reveal_workbench_surface(surface);
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_right_inspector_visible(&mut self, visible: bool) {
        self.state.workbench.right_inspector.visible = visible;
        self.app_config.workbench.right_inspector.visible = visible;
        if visible {
            self.reveal_workbench_surface(WorkbenchSurfaceKind::from_right_inspector_tab(
                self.state.workbench.right_inspector.active_tab,
            ));
        }
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_right_inspector_tab(&mut self, tab: RightInspectorTab) {
        self.state.workbench.right_inspector.active_tab = tab;
        self.app_config.workbench.right_inspector.active_tab = tab;
        self.reveal_workbench_surface(WorkbenchSurfaceKind::from_right_inspector_tab(tab));
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_right_inspector_width(&mut self, width: f32, available_width: f32) {
        let config = &self.app_config.workbench.right_inspector;
        let width = clamped_right_inspector_width(
            width,
            available_width,
            config.min_width,
            config.max_width,
        );
        self.state.workbench.right_inspector.width = width;
        self.app_config.workbench.right_inspector.width = width;
    }

    pub(in crate::app) fn reveal_right_inspector_for_inspect(&mut self, tab: RightInspectorTab) {
        self.state.workbench.right_inspector.active_tab = tab;
        self.app_config.workbench.right_inspector.active_tab = tab;
        if self
            .app_config
            .workbench
            .right_inspector
            .auto_open_on_inspect
        {
            self.state.workbench.right_inspector.visible = true;
            self.app_config.workbench.right_inspector.visible = true;
        }
        self.reveal_workbench_surface(WorkbenchSurfaceKind::from_right_inspector_tab(tab));
        self.save_config_debounced();
    }

    pub(in crate::app) fn set_bottom_panel_height(&mut self, height: f32, available_height: f32) {
        let config = &self.app_config.workbench.bottom_panel;
        let height = clamped_bottom_panel_height(
            height,
            available_height,
            config.min_height,
            config.max_height_ratio,
        );
        self.state.workbench.bottom_panel.height = height;
        self.app_config.workbench.bottom_panel.height = height;
    }

    pub(in crate::app) fn reveal_bottom_panel_for_query(&mut self, tab: BottomPanelTab) {
        if !self.app_config.workbench.bottom_panel.auto_open_on_query {
            return;
        }

        self.state.workbench.bottom_panel.visible = true;
        self.state.workbench.bottom_panel.active_tab = tab;
        self.app_config.workbench.bottom_panel.visible = true;
        self.app_config.workbench.bottom_panel.active_tab = tab;
        let surface = self.active_bottom_panel_surface_kind(tab);
        self.reveal_workbench_surface(surface);
        self.save_config_debounced();
    }

    pub(in crate::app) fn bottom_panel_height_for_available(&self, available_height: f32) -> f32 {
        let config = &self.app_config.workbench.bottom_panel;
        clamped_bottom_panel_height(
            self.state.workbench.bottom_panel.height,
            available_height,
            config.min_height,
            config.max_height_ratio,
        )
    }

    pub(in crate::app) fn right_inspector_width_for_available(&self, available_width: f32) -> f32 {
        let config = &self.app_config.workbench.right_inspector;
        clamped_right_inspector_width(
            self.state.workbench.right_inspector.width,
            available_width,
            config.min_width,
            config.max_width,
        )
    }

    pub(in crate::app) fn render_bottom_panel_resize_divider(
        &mut self,
        ui: &mut egui::Ui,
        available_height: f32,
    ) {
        let divider_height = 6.0;
        let (divider_rect, divider_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), divider_height),
            egui::Sense::drag(),
        );

        let divider_color = if divider_response.dragged() || divider_response.hovered() {
            egui::Color32::from_rgb(100, 150, 255)
        } else {
            egui::Color32::from_rgba_unmultiplied(128, 128, 128, 80)
        };
        ui.painter().rect_filled(
            divider_rect.shrink2(egui::vec2(4.0, 1.0)),
            egui::CornerRadius::same(2),
            divider_color,
        );

        let center = divider_rect.center();
        for offset in [-15.0, 0.0, 15.0] {
            ui.painter().circle_filled(
                egui::pos2(center.x + offset, center.y),
                2.0,
                egui::Color32::from_gray(160),
            );
        }

        if divider_response.dragged() {
            self.state.workbench.bottom_panel.is_resizing = true;
            let next_height =
                self.state.workbench.bottom_panel.height - divider_response.drag_delta().y;
            self.set_bottom_panel_height(next_height, available_height);
        } else if divider_response.drag_stopped() {
            self.state.workbench.bottom_panel.is_resizing = false;
            self.save_config_debounced();
        }

        if divider_response.hovered() || divider_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
    }

    pub(in crate::app) fn render_right_inspector_resize_divider(
        &mut self,
        ui: &mut egui::Ui,
        available_width: f32,
        available_height: f32,
    ) {
        let divider_width = 6.0;
        let (divider_rect, divider_response) = ui.allocate_exact_size(
            egui::vec2(divider_width, available_height),
            egui::Sense::drag(),
        );

        let divider_color = if divider_response.dragged() || divider_response.hovered() {
            egui::Color32::from_rgb(100, 150, 255)
        } else {
            egui::Color32::from_rgba_unmultiplied(128, 128, 128, 80)
        };
        ui.painter().rect_filled(
            divider_rect.shrink2(egui::vec2(1.0, 4.0)),
            egui::CornerRadius::same(2),
            divider_color,
        );

        let center = divider_rect.center();
        for offset in [-10.0, 0.0, 10.0] {
            ui.painter().circle_filled(
                egui::pos2(center.x, center.y + offset),
                2.0,
                egui::Color32::from_gray(180),
            );
        }

        if divider_response.dragged() {
            self.state.workbench.right_inspector.is_resizing = true;
            let next_width =
                self.state.workbench.right_inspector.width - divider_response.drag_delta().x;
            self.set_right_inspector_width(next_width, available_width);
        } else if divider_response.drag_stopped() {
            self.state.workbench.right_inspector.is_resizing = false;
            self.save_config_debounced();
        }

        if divider_response.hovered() || divider_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }

    pub(in crate::app) fn apply_workbench_activity_to_sidebar_panels(&mut self) {
        let panel_state = &mut self.state.sidebar_panel_state;
        match self.state.workbench.active_activity {
            WorkbenchActivity::Explorer => {
                panel_state.show_connections = true;
                panel_state.show_filters = false;
                panel_state.show_triggers = false;
                panel_state.show_routines = false;
                self.state.sidebar_section = ui::SidebarSection::Connections;
            }
            WorkbenchActivity::Filters => {
                panel_state.show_connections = false;
                panel_state.show_filters = true;
                panel_state.show_triggers = false;
                panel_state.show_routines = false;
                self.state.sidebar_section = ui::SidebarSection::Filters;
            }
            WorkbenchActivity::Objects => {
                panel_state.show_connections = false;
                panel_state.show_filters = false;
                panel_state.show_triggers = true;
                panel_state.show_routines = true;
                self.state.sidebar_section = ui::SidebarSection::Triggers;
            }
            WorkbenchActivity::History | WorkbenchActivity::Help | WorkbenchActivity::Settings => {
                panel_state.show_connections = false;
                panel_state.show_filters = false;
                panel_state.show_triggers = false;
                panel_state.show_routines = false;
            }
        }
    }

    pub(in crate::app) fn active_activity_uses_legacy_sidebar(&self) -> bool {
        matches!(
            self.state.workbench.active_activity,
            WorkbenchActivity::Explorer | WorkbenchActivity::Filters | WorkbenchActivity::Objects
        )
    }

    pub(in crate::app) fn render_primary_sidebar_activity_placeholder(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
    ) {
        ui.set_min_width(width);
        ui.set_max_width(width);
        let (title, message, action_label, action) = match self.state.workbench.active_activity {
            WorkbenchActivity::History => (
                "历史",
                "查询历史仍以浮动面板承载。下一步可将 HistoryPanel 适配进 PrimarySidebar。",
                "打开历史记录",
                Some(AppAction::OpenHistoryPanel),
            ),
            WorkbenchActivity::Help => (
                "帮助",
                "帮助内容仍以对话框承载。后续会把导航入口迁入 Workbench。",
                "打开帮助",
                Some(AppAction::OpenHelpPanel),
            ),
            WorkbenchActivity::Settings => (
                "设置",
                "设置活动先提供快捷入口，完整 Preferences 面板会在后续阶段收束。",
                "打开快捷键设置",
                Some(AppAction::OpenKeybindingsDialog),
            ),
            _ => ("", "", "", None),
        };

        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.heading(title);
            ui.add_space(8.0);
            ui.label(message);
            if let Some(action) = action {
                ui.add_space(12.0);
                if ui.button(action_label).clicked() {
                    let ctx = ui.ctx().clone();
                    self.dispatch_app_action(&ctx, action);
                }
            }
        });
    }

    pub(in crate::app) fn render_top_bar(&mut self, ui: &mut egui::Ui) -> ToolbarActions {
        let is_toolbar_focused = self.state.focus_area == ui::FocusArea::Toolbar;
        let mut toolbar_actions = ToolbarActions::default();
        let cancel_task_id = ui
            .scope(|ui| {
                let connections: Vec<String> =
                    self.session.manager.connections.keys().cloned().collect();
                let (databases, selected_database, tables) = self
                    .session
                    .manager
                    .get_active()
                    .map(|c| {
                        (
                            c.databases.clone(),
                            c.selected_database.clone(),
                            c.tables.clone(),
                        )
                    })
                    .unwrap_or_default();
                let active_connection = self.session.manager.active.clone();
                let selected_table = self.state.selected_table.clone();

                let cid = ui::Toolbar::show_with_focus(
                    ui,
                    &self.state.theme_manager,
                    &self.keybindings,
                    self.state.result.is_some(),
                    self.state.show_sidebar,
                    self.state.show_sql_editor,
                    self.app_config.is_dark_mode,
                    &mut toolbar_actions,
                    &connections,
                    active_connection.as_deref(),
                    &databases,
                    selected_database.as_deref(),
                    &tables,
                    selected_table.as_deref(),
                    self.state.ui_scale,
                    &self.session.progress,
                    is_toolbar_focused,
                    self.state.toolbar_index,
                );
                if ui.ui_contains_pointer() && ui.input(|input| input.pointer.primary_clicked()) {
                    self.set_focus_area(ui::FocusArea::Toolbar);
                }
                cid
            })
            .inner;

        if let Some(id) = cancel_task_id {
            self.session.progress.cancel(id);
        }
        if self.state.focus_area == ui::FocusArea::Toolbar {
            ui::Toolbar::handle_keyboard(ui, &mut self.state.toolbar_index, &mut toolbar_actions);
        }

        toolbar_actions
    }

    pub(in crate::app) fn sync_workbench_state_from_legacy_layout(&mut self) {
        self.state.workbench.primary_sidebar.visible = self.state.show_sidebar;
        self.state.workbench.primary_sidebar.width = self.state.sidebar_width;
        self.state.workbench.bottom_panel.visible = self.app_config.workbench.bottom_panel.visible;
        self.state.workbench.bottom_panel.active_tab =
            self.app_config.workbench.bottom_panel.active_tab;
        self.state.workbench.right_inspector.visible =
            self.app_config.workbench.right_inspector.visible;
        self.state.workbench.right_inspector.active_tab =
            self.app_config.workbench.right_inspector.active_tab;
        self.state.workbench.status_bar.visible = self.app_config.workbench.status_bar.visible;
        self.state.workbench.set_focus_area(self.state.focus_area);
    }

    pub(in crate::app) fn render_bottom_panel_in_ui(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                let mut active_tab = self.state.workbench.bottom_panel.active_tab;
                let header = ui::WorkbenchBottomPanel::show_header(ui, active_tab);
                if let Some(tab) = header.selected_tab {
                    self.set_bottom_panel_tab(tab);
                    active_tab = tab;
                }
                if header.close_requested {
                    self.set_bottom_panel_visible(false);
                }

                ui.separator();

                let active_surface = self.active_bottom_panel_surface_kind(active_tab);
                self.render_bottom_panel_surface_body(ui, active_tab);
                if ui.ui_contains_pointer() && ui.input(|input| input.pointer.primary_clicked()) {
                    self.state.workbench.set_focused_surface(&active_surface);
                }
            });
    }

    pub(in crate::app) fn render_right_inspector_in_ui(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(ui.visuals().faint_bg_color)
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| {
                let mut active_tab = self.state.workbench.right_inspector.active_tab;
                let header = ui::WorkbenchRightInspector::show_header(ui, active_tab);
                if let Some(tab) = header.selected_tab {
                    let ctx = ui.ctx().clone();
                    self.dispatch_app_action(&ctx, AppAction::SetRightInspectorTab(tab));
                    active_tab = tab;
                }
                if header.close_requested {
                    let ctx = ui.ctx().clone();
                    self.dispatch_app_action(&ctx, AppAction::SetRightInspectorVisible(false));
                }

                ui.separator();

                let active_surface = WorkbenchSurfaceKind::from_right_inspector_tab(active_tab);
                self.render_right_inspector_surface_body(ui, active_tab);
                if ui.ui_contains_pointer() && ui.input(|input| input.pointer.primary_clicked()) {
                    self.state.workbench.set_focused_surface(&active_surface);
                }
            });
    }

    pub(crate) fn render_workbench_surface_in_ui(
        &mut self,
        ui: &mut egui::Ui,
        surface: WorkbenchSurfaceKind,
    ) {
        let surface_id = surface.surface_id();

        match surface {
            WorkbenchSurfaceKind::SqlDocument { index } => {
                self.render_sql_document_surface_in_ui(ui, index);
            }
            WorkbenchSurfaceKind::QueryResult { .. } => {
                self.render_bottom_panel_surface_body(ui, BottomPanelTab::Results);
            }
            WorkbenchSurfaceKind::Explain { .. } => {
                self.render_bottom_panel_surface_body(ui, BottomPanelTab::Explain);
            }
            WorkbenchSurfaceKind::TableData { .. } | WorkbenchSurfaceKind::Welcome => {
                self.render_workspace_content(ui);
            }
            WorkbenchSurfaceKind::ErDiagram => {
                self.render_er_diagram_in_ui(ui);
            }
            WorkbenchSurfaceKind::SchemaObject { .. } => {
                self.render_schema_object_placeholder(ui);
            }
            WorkbenchSurfaceKind::Explorer
            | WorkbenchSurfaceKind::Filters
            | WorkbenchSurfaceKind::Objects => {
                self.render_navigation_surface_placeholder(ui, &surface);
            }
            WorkbenchSurfaceKind::History => {
                self.render_bottom_panel_surface_body(ui, BottomPanelTab::History);
            }
            WorkbenchSurfaceKind::Messages => {
                self.render_bottom_panel_surface_body(ui, BottomPanelTab::Messages);
            }
            WorkbenchSurfaceKind::Tasks => {
                self.render_bottom_panel_surface_body(ui, BottomPanelTab::Tasks);
            }
            WorkbenchSurfaceKind::Inspector { mode } => {
                self.render_right_inspector_surface_body(ui, mode);
            }
            WorkbenchSurfaceKind::Settings => {
                self.render_aux_panel(ui, ui::dock_tabs::AuxPanelKind::Keybindings);
            }
            WorkbenchSurfaceKind::Help => {
                self.render_aux_panel(ui, ui::dock_tabs::AuxPanelKind::Help);
            }
        }

        if ui.ui_contains_pointer() && ui.input(|input| input.pointer.primary_clicked()) {
            self.state.workbench.focus = WorkbenchFocus::Surface(surface_id);
        }
    }

    fn render_sql_document_surface_in_ui(&mut self, ui: &mut egui::Ui, index: usize) {
        if index >= self.tab_manager().tabs.len() {
            ui::WorkbenchBottomPanel::show_empty_state(
                ui,
                "SQL 文档不存在",
                "该 surface 指向的查询文档已经关闭。",
            );
            return;
        }

        if self.tab_manager().active_index != index {
            self.tab_manager_mut().active_index = index;
            self.sync_from_active_tab();
        }
        let actions = self.render_sql_document_in_ui(ui);
        self.handle_sql_editor_actions(actions);
    }

    pub(in crate::app) fn active_bottom_panel_surface_kind(
        &self,
        tab: BottomPanelTab,
    ) -> WorkbenchSurfaceKind {
        WorkbenchSurfaceKind::from_bottom_panel_tab(tab, self.active_query_tab_id())
    }

    fn active_query_tab_id(&self) -> String {
        self.session
            .tab_manager
            .get_active()
            .map(|tab| tab.id.clone())
            .unwrap_or_else(|| format!("query-index-{}", self.session.tab_manager.active_index))
    }

    fn render_bottom_panel_surface_body(&mut self, ui: &mut egui::Ui, active_tab: BottomPanelTab) {
        match active_tab {
            BottomPanelTab::Results => self.render_bottom_panel_results(ui),
            BottomPanelTab::Messages => self.render_bottom_panel_messages(ui),
            BottomPanelTab::Explain => self.render_bottom_panel_explain(ui),
            BottomPanelTab::History => self.render_bottom_panel_history(ui),
            BottomPanelTab::Tasks => self.render_bottom_panel_tasks(ui),
        }
    }

    fn render_right_inspector_surface_body(
        &self,
        ui: &mut egui::Ui,
        active_tab: RightInspectorTab,
    ) {
        match active_tab {
            RightInspectorTab::Properties => self.render_right_inspector_properties(ui),
            RightInspectorTab::Schema => self.render_right_inspector_schema(ui),
            RightInspectorTab::Row => self.render_right_inspector_row(ui),
            RightInspectorTab::Cell => self.render_right_inspector_cell(ui),
            RightInspectorTab::ErSelection => self.render_right_inspector_er_selection(ui),
            RightInspectorTab::Connection => self.render_right_inspector_connection(ui),
        }
    }

    fn render_navigation_surface_placeholder(
        &self,
        ui: &mut egui::Ui,
        surface: &WorkbenchSurfaceKind,
    ) {
        let descriptor = surface.descriptor();

        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.heading(descriptor.title);
            ui.add_space(8.0);
            ui.label(descriptor.description);
            ui.add_space(8.0);
            ui.label("该 surface 已有稳定身份；内容仍由 PrimarySidebar 兼容适配器承载。");
        });
    }

    fn render_right_inspector_properties(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("当前上下文");
            ui.add_space(8.0);
            property_row(
                ui,
                "连接",
                self.session.manager.active.as_deref().unwrap_or("无"),
            );
            property_row(
                ui,
                "数据库",
                self.session
                    .manager
                    .get_active()
                    .and_then(|conn| conn.selected_database.as_deref())
                    .filter(|database| !database.is_empty())
                    .unwrap_or("无"),
            );
            property_row(
                ui,
                "表",
                self.state.selected_table.as_deref().unwrap_or("未选择"),
            );
            property_row(
                ui,
                "结果行",
                &self
                    .state
                    .result
                    .as_ref()
                    .map(|result| result.rows.len().to_string())
                    .unwrap_or_else(|| "无结果".to_string()),
            );
            property_row(
                ui,
                "选中单元格",
                &selected_cell_label(self.state.selected_cell),
            );
        });
    }

    fn render_right_inspector_schema(&self, ui: &mut egui::Ui) {
        let target_table = self
            .state
            .workbench
            .right_inspector
            .schema_table
            .as_deref()
            .or_else(|| self.state.er_diagram_state.selected_table_name())
            .or(self.state.selected_table.as_deref());

        let Some(table_name) = target_table else {
            ui::WorkbenchRightInspector::show_empty_state(
                ui,
                "未选择表",
                "在 Explorer 中选择一张表，或在 ER 图中选择一个表节点。",
            );
            return;
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(table_name);
            ui.add_space(8.0);

            if let Some(er_table) = self
                .state
                .er_diagram_state
                .tables
                .iter()
                .find(|table| table.name == table_name)
            {
                ui.label(egui::RichText::new("ER 元数据").strong());
                for column in &er_table.columns {
                    ui.horizontal_wrapped(|ui| {
                        ui.monospace(&column.name);
                        ui.label(&column.data_type);
                        if column.is_primary_key {
                            ui.label("PK");
                        }
                        if column.is_foreign_key {
                            ui.label("FK");
                        }
                        if !column.nullable {
                            ui.label("NOT NULL");
                        }
                    });
                }
                ui.add_space(8.0);
            }

            if let Some(result) = self.state.result.as_ref().filter(|result| {
                self.state.workbench.right_inspector.schema_table.is_some()
                    && !result.columns.is_empty()
            }) {
                ui.label(egui::RichText::new("最近结构查询结果").strong());
                render_compact_result_rows(ui, result, 24);
            } else if self.state.er_diagram_state.tables.is_empty() {
                ui.label(
                    "结构查询结果会显示在 BottomPanel::Results；加载 ER 图后这里会显示列元数据。",
                );
            }
        });
    }

    fn render_right_inspector_row(&self, ui: &mut egui::Ui) {
        let Some((result, row_index, row)) =
            selected_result_row(self.state.result.as_ref(), self.state.selected_row)
        else {
            ui::WorkbenchRightInspector::show_empty_state(
                ui,
                "未选择行",
                "在 Results 表格中移动光标或点击一行后，这里会显示行详情。",
            );
            return;
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(format!("Row {}", row_index + 1));
            ui.add_space(8.0);
            for (col_index, column) in result.columns.iter().enumerate() {
                let value = row.get(col_index).map(String::as_str).unwrap_or("");
                property_row(ui, column, value);
            }
        });
    }

    fn render_right_inspector_cell(&self, ui: &mut egui::Ui) {
        let Some((result, row_index, col_index, value)) =
            selected_result_cell(self.state.result.as_ref(), self.state.selected_cell)
        else {
            ui::WorkbenchRightInspector::show_empty_state(
                ui,
                "未选择单元格",
                "在 Results 表格中选择单元格后，这里会显示完整值。",
            );
            return;
        };

        let column = result
            .columns
            .get(col_index)
            .map(String::as_str)
            .unwrap_or("<unknown>");
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("Cell");
            ui.add_space(8.0);
            property_row(ui, "行", &(row_index + 1).to_string());
            property_row(ui, "列", column);
            ui.separator();
            ui.label(egui::RichText::new("完整值").strong());
            ui.add_space(4.0);
            ui.add(
                egui::Label::new(egui::RichText::new(value).monospace())
                    .wrap()
                    .selectable(true),
            );
        });
    }

    fn render_right_inspector_er_selection(&self, ui: &mut egui::Ui) {
        let Some(table) = self
            .state
            .er_diagram_state
            .selected_table
            .and_then(|index| self.state.er_diagram_state.tables.get(index))
        else {
            ui::WorkbenchRightInspector::show_empty_state(
                ui,
                "未选择 ER 节点",
                "打开 ER 图并选择表节点后，这里会显示节点详情。",
            );
            return;
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading(&table.name);
            property_row(ui, "列数量", &table.columns.len().to_string());
            property_row(
                ui,
                "关联边",
                &self
                    .state
                    .er_diagram_state
                    .relationships
                    .iter()
                    .filter(|rel| rel.from_table == table.name || rel.to_table == table.name)
                    .count()
                    .to_string(),
            );
            ui.separator();
            for column in &table.columns {
                ui.horizontal_wrapped(|ui| {
                    ui.monospace(&column.name);
                    ui.label(&column.data_type);
                    if column.is_primary_key {
                        ui.label("PK");
                    }
                    if column.is_foreign_key {
                        ui.label("FK");
                    }
                });
            }
        });
    }

    fn render_right_inspector_connection(&self, ui: &mut egui::Ui) {
        let Some(connection) = self.session.manager.get_active() else {
            ui::WorkbenchRightInspector::show_empty_state(
                ui,
                "无活动连接",
                "连接数据库后，这里会显示当前连接的非敏感属性。",
            );
            return;
        };

        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.heading("连接");
            ui.add_space(8.0);
            render_connection_properties(ui, connection);
        });
    }

    fn render_bottom_panel_results(&mut self, ui: &mut egui::Ui) {
        let Some(result) = self.state.result.as_ref() else {
            ui::WorkbenchBottomPanel::show_empty_state(
                ui,
                "暂无结果",
                "执行查询后，结果集会显示在这里。",
            );
            return;
        };

        if result.columns.is_empty() {
            ui::WorkbenchBottomPanel::show_empty_state(
                ui,
                "语句执行成功",
                &format!("没有返回列，影响 {} 行。", result.affected_rows),
            );
            return;
        }

        self.render_result_grid_in_ui(ui);
    }

    fn render_bottom_panel_messages(&self, ui: &mut egui::Ui) {
        if let Some(error) = self.active_query_error_message() {
            ui.vertical(|ui| {
                ui.heading("查询执行失败");
                ui.add_space(6.0);
                ui.label("结果表格未更新。请修正 SQL 后重新执行。");
                ui.add_space(8.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.group(|ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(error).monospace());
                    });
                });
            });
            return;
        }

        if let Some(message) = self.active_query_status_message().or_else(|| {
            self.session
                .notifications
                .latest_message()
                .map(ToOwned::to_owned)
        }) {
            ui.vertical(|ui| {
                ui.heading("最近消息");
                ui.add_space(6.0);
                ui.label(message);
            });
            return;
        }

        ui::WorkbenchBottomPanel::show_empty_state(
            ui,
            "暂无消息",
            "查询状态、错误和系统提示会显示在这里。",
        );
    }

    fn render_bottom_panel_explain(&self, ui: &mut egui::Ui) {
        ui::WorkbenchBottomPanel::show_empty_state(
            ui,
            "Explain 暂未独立接入",
            "当前 EXPLAIN 查询仍作为普通结果显示在 Results。后续阶段会加入结构化执行计划视图。",
        );
    }

    fn render_bottom_panel_history(&self, ui: &mut egui::Ui) {
        if self.session.query_history.is_empty() {
            ui::WorkbenchBottomPanel::show_empty_state(
                ui,
                "暂无历史",
                "执行 SQL 后，最近查询会显示在这里。",
            );
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in self.session.query_history.items().iter().take(12) {
                ui.horizontal(|ui| {
                    let status = if item.success { "OK" } else { "ERR" };
                    ui.label(egui::RichText::new(status).monospace().strong());
                    ui.label(item.timestamp.format("%H:%M:%S").to_string());
                    ui.label(&item.database_type);
                    ui.monospace(item.sql.lines().next().unwrap_or(""));
                });
            }
        });
    }

    fn render_bottom_panel_tasks(&self, ui: &mut egui::Ui) {
        let tasks = self.session.progress.active_tasks();
        if tasks.is_empty() {
            ui::WorkbenchBottomPanel::show_empty_state(
                ui,
                "暂无任务",
                "导入、导出、连接或长查询任务会显示在这里。",
            );
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for task in tasks {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(format!("#{}", task.id)).monospace());
                    ui.label(&task.description);
                    if let Some(progress) = task.progress {
                        ui.add(
                            egui::ProgressBar::new(progress)
                                .desired_width(120.0)
                                .show_percentage(),
                        );
                    } else {
                        ui.label("进行中");
                    }
                    ui.label(format!("{}ms", task.elapsed_ms()));
                });
            }
        });
    }

    fn active_query_error_message(&self) -> Option<String> {
        self.session
            .tab_manager
            .get_active()
            .and_then(|tab| tab.last_error.clone())
    }

    fn active_query_status_message(&self) -> Option<String> {
        self.session
            .tab_manager
            .get_active()
            .and_then(|tab| tab.last_message.clone())
    }

    pub(in crate::app) fn workbench_status_bar_content(&self) -> WorkbenchStatusBarContent {
        let context = self.action_context();
        let status_config = &self.app_config.workbench.status_bar;
        let status_line = if status_config.show_focus_area {
            context.status_line()
        } else {
            let connection = if context.has_active_connection {
                "已连接"
            } else if context.has_any_connection {
                "未激活连接"
            } else {
                "无连接"
            };
            let table = context.selected_table.as_deref().unwrap_or("无表");
            format!("{} · {}", connection, table)
        };

        let mut content = WorkbenchStatusBarContent::new(status_line);
        if status_config.show_query_time {
            content.query_time_ms = self.session.last_query_time_ms;
        }
        if status_config.show_row_count {
            content.row_count = self.state.result.as_ref().map(|result| result.rows.len());
        }
        content
    }

    pub(in crate::app) fn render_workbench(
        root_ui: &mut egui::Ui,
        frame: egui::Frame,
        status_bar: Option<WorkbenchStatusBarContent>,
        add_content: impl FnOnce(&mut egui::Ui),
    ) {
        ui::WorkbenchShell::new()
            .status_bar(status_bar)
            .show_inside(root_ui, frame, add_content);
    }
}

fn property_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.set_min_height(22.0);
        ui.label(egui::RichText::new(label).color(ui.visuals().weak_text_color()));
        ui.label(":");
        ui.add(
            egui::Label::new(egui::RichText::new(value).monospace())
                .wrap()
                .selectable(true),
        );
    });
}

fn selected_cell_label(selected_cell: Option<(usize, usize)>) -> String {
    selected_cell
        .map(|(row, col)| format!("row {}, col {}", row + 1, col + 1))
        .unwrap_or_else(|| "未选择".to_string())
}

fn selected_result_row(
    result: Option<&QueryResult>,
    selected_row: Option<usize>,
) -> Option<(&QueryResult, usize, &Vec<String>)> {
    let result = result?;
    let row_index = selected_row?;
    let row = result.rows.get(row_index)?;
    Some((result, row_index, row))
}

fn selected_result_cell(
    result: Option<&QueryResult>,
    selected_cell: Option<(usize, usize)>,
) -> Option<(&QueryResult, usize, usize, &str)> {
    let result = result?;
    let (row_index, col_index) = selected_cell?;
    let row = result.rows.get(row_index)?;
    let value = row.get(col_index)?;
    Some((result, row_index, col_index, value.as_str()))
}

fn render_compact_result_rows(ui: &mut egui::Ui, result: &QueryResult, max_rows: usize) {
    for row in result.rows.iter().take(max_rows) {
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            for (index, column) in result.columns.iter().enumerate() {
                let value = row.get(index).map(String::as_str).unwrap_or("");
                property_row(ui, column, value);
            }
        });
    }

    if result.rows.len() > max_rows {
        ui.label(format!("另有 {} 行未显示", result.rows.len() - max_rows));
    }
}

fn render_connection_properties(ui: &mut egui::Ui, connection: &Connection) {
    let config = &connection.config;
    property_row(ui, "名称", &config.name);
    property_row(ui, "类型", config.db_type.display_name());
    property_row(
        ui,
        "状态",
        if connection.connected {
            "已连接"
        } else {
            "未连接"
        },
    );
    if config.db_type.requires_network() {
        property_row(ui, "主机", &config.host);
        property_row(ui, "端口", &config.port.to_string());
        property_row(ui, "用户", &config.username);
    }
    if !config.database.is_empty() {
        property_row(ui, "数据库", &config.database);
    }
    if let Some(database) = connection.selected_database.as_deref() {
        property_row(ui, "当前数据库", database);
    }
    property_row(ui, "表数量", &connection.tables.len().to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dialogs::host::DialogId;
    use crate::state::WorkbenchSurfaceId;
    use crate::ui::FocusArea;
    use egui_dock::SurfaceIndex;

    fn count_surface_tabs(app: &DbManagerApp, surface: &WorkbenchSurfaceKind) -> usize {
        let target = surface.surface_id();
        app.dock_state
            .get_surface(SurfaceIndex::main())
            .map(|surface| {
                surface
                    .iter_all_tabs()
                    .filter(|(_, tab)| tab.surface_kind().surface_id() == target)
                    .count()
            })
            .unwrap_or(0)
    }

    #[test]
    fn status_bar_content_handles_empty_app() {
        let app = DbManagerApp::new_for_test();

        let content = app.workbench_status_bar_content();

        assert!(content.status_line.contains("无连接"));
        assert!(content.status_line.contains("无表"));
        assert_eq!(content.query_time_ms, None);
        assert_eq!(content.row_count, None);
    }

    #[test]
    fn bottom_panel_height_clamps_to_min_and_viewport_ratio() {
        assert_eq!(clamped_bottom_panel_height(80.0, 600.0, 140.0, 0.55), 140.0);
        assert_eq!(
            clamped_bottom_panel_height(500.0, 600.0, 140.0, 0.55),
            330.0
        );
        assert_eq!(clamped_bottom_panel_height(240.0, 0.0, 140.0, 0.55), 0.0);
    }

    #[test]
    fn right_inspector_width_clamps_to_config_and_available_width() {
        assert_eq!(
            clamped_right_inspector_width(120.0, 900.0, 260.0, 480.0),
            260.0
        );
        assert_eq!(
            clamped_right_inspector_width(600.0, 900.0, 260.0, 480.0),
            480.0
        );
        assert_eq!(
            clamped_right_inspector_width(400.0, 300.0, 260.0, 480.0),
            300.0
        );
        assert_eq!(clamped_right_inspector_width(320.0, 0.0, 260.0, 480.0), 0.0);
    }

    #[test]
    fn bottom_panel_visibility_and_tab_sync_to_config() {
        let mut app = DbManagerApp::new_for_test();

        app.set_bottom_panel_visible(false);
        app.set_bottom_panel_tab(BottomPanelTab::Messages);

        assert!(!app.state.workbench.bottom_panel.visible);
        assert!(!app.app_config.workbench.bottom_panel.visible);
        assert_eq!(
            app.state.workbench.bottom_panel.active_tab,
            BottomPanelTab::Messages
        );
        assert_eq!(
            app.app_config.workbench.bottom_panel.active_tab,
            BottomPanelTab::Messages
        );
        assert_eq!(count_surface_tabs(&app, &WorkbenchSurfaceKind::Messages), 1);
    }

    #[test]
    fn right_inspector_visibility_and_tab_sync_to_config() {
        let mut app = DbManagerApp::new_for_test();

        app.set_right_inspector_visible(true);
        app.set_right_inspector_tab(RightInspectorTab::Cell);

        assert!(app.state.workbench.right_inspector.visible);
        assert!(app.app_config.workbench.right_inspector.visible);
        assert_eq!(
            app.state.workbench.right_inspector.active_tab,
            RightInspectorTab::Cell
        );
        assert_eq!(
            app.app_config.workbench.right_inspector.active_tab,
            RightInspectorTab::Cell
        );
        assert_eq!(
            count_surface_tabs(
                &app,
                &WorkbenchSurfaceKind::Inspector {
                    mode: RightInspectorTab::Cell
                }
            ),
            1
        );
    }

    #[test]
    fn reveal_right_inspector_respects_auto_open_config() {
        let mut app = DbManagerApp::new_for_test();

        app.reveal_right_inspector_for_inspect(RightInspectorTab::Schema);

        assert!(app.state.workbench.right_inspector.visible);
        assert_eq!(
            app.state.workbench.right_inspector.active_tab,
            RightInspectorTab::Schema
        );
        assert_eq!(
            count_surface_tabs(
                &app,
                &WorkbenchSurfaceKind::Inspector {
                    mode: RightInspectorTab::Schema
                }
            ),
            1
        );

        app.app_config
            .workbench
            .right_inspector
            .auto_open_on_inspect = false;
        app.set_right_inspector_visible(false);
        app.reveal_right_inspector_for_inspect(RightInspectorTab::Cell);

        assert!(!app.state.workbench.right_inspector.visible);
        assert_eq!(
            app.state.workbench.right_inspector.active_tab,
            RightInspectorTab::Cell
        );
        assert_eq!(
            count_surface_tabs(
                &app,
                &WorkbenchSurfaceKind::Inspector {
                    mode: RightInspectorTab::Cell
                }
            ),
            1
        );
    }

    #[test]
    fn reveal_bottom_panel_for_query_respects_auto_open_config() {
        let mut app = DbManagerApp::new_for_test();
        app.set_bottom_panel_visible(false);

        app.reveal_bottom_panel_for_query(BottomPanelTab::Results);

        assert!(app.state.workbench.bottom_panel.visible);
        assert_eq!(
            app.state.workbench.bottom_panel.active_tab,
            BottomPanelTab::Results
        );
        let result_surface = app.active_bottom_panel_surface_kind(BottomPanelTab::Results);
        assert_eq!(count_surface_tabs(&app, &result_surface), 1);

        app.app_config.workbench.bottom_panel.auto_open_on_query = false;
        app.set_bottom_panel_visible(false);
        app.set_bottom_panel_tab(BottomPanelTab::History);

        app.reveal_bottom_panel_for_query(BottomPanelTab::Messages);

        assert!(!app.state.workbench.bottom_panel.visible);
        assert_eq!(
            app.state.workbench.bottom_panel.active_tab,
            BottomPanelTab::History
        );
        assert_eq!(count_surface_tabs(&app, &WorkbenchSurfaceKind::Messages), 0);
    }

    #[test]
    fn reveal_workbench_surface_is_idempotent_and_updates_surface_focus() {
        let mut app = DbManagerApp::new_for_test();
        let surface = WorkbenchSurfaceKind::Filters;

        assert!(app.reveal_workbench_surface(surface.clone()));
        assert!(!app.reveal_workbench_surface(surface.clone()));

        assert_eq!(count_surface_tabs(&app, &surface), 1);
        assert_eq!(
            app.state.workbench.focus,
            WorkbenchFocus::Surface(WorkbenchSurfaceId::new("filters"))
        );
    }

    #[test]
    fn workbench_activity_reveals_matching_surface_tab() {
        let mut app = DbManagerApp::new_for_test();

        assert!(app.active_activity_surface_is_docked());

        app.set_workbench_activity(WorkbenchActivity::Filters);

        assert!(app.active_activity_surface_is_docked());
        assert_eq!(count_surface_tabs(&app, &WorkbenchSurfaceKind::Filters), 1);
        assert_eq!(
            app.state.workbench.focus,
            WorkbenchFocus::Surface(WorkbenchSurfaceId::new("filters"))
        );
    }

    #[test]
    fn fixed_region_fallback_detection_tracks_docked_equivalent_surfaces() {
        let mut app = DbManagerApp::new_for_test();

        assert!(app.active_bottom_panel_surface_is_docked());
        assert!(app.active_right_inspector_surface_is_docked());

        app.set_bottom_panel_tab(BottomPanelTab::Messages);
        app.set_right_inspector_tab(RightInspectorTab::Cell);

        assert!(app.active_bottom_panel_surface_is_docked());
        assert!(app.active_right_inspector_surface_is_docked());
        assert!(app.state.workbench.bottom_panel.visible);
        assert!(!app.state.workbench.right_inspector.visible);
    }

    #[test]
    fn new_app_starts_with_surface_dock_seed() {
        let app = DbManagerApp::new_for_test();
        let result_surface = app.active_bottom_panel_surface_kind(BottomPanelTab::Results);

        assert_eq!(count_surface_tabs(&app, &WorkbenchSurfaceKind::Explorer), 1);
        assert_eq!(count_surface_tabs(&app, &result_surface), 1);
        assert_eq!(
            count_surface_tabs(
                &app,
                &WorkbenchSurfaceKind::Inspector {
                    mode: app.state.workbench.right_inspector.active_tab
                }
            ),
            1
        );
    }

    #[test]
    fn er_visibility_reveals_er_surface_tab() {
        let mut app = DbManagerApp::new_for_test();

        app.set_er_diagram_visible(true);

        assert!(app.state.show_er_diagram);
        assert_eq!(
            count_surface_tabs(&app, &WorkbenchSurfaceKind::ErDiagram),
            1
        );
    }

    #[test]
    fn active_bottom_panel_surface_kind_uses_active_query_tab_id() {
        let app = DbManagerApp::new_for_test();
        let active_tab_id = app
            .session
            .tab_manager
            .get_active()
            .expect("test app should have an active query tab")
            .id
            .clone();

        assert_eq!(
            app.active_bottom_panel_surface_kind(BottomPanelTab::Results),
            WorkbenchSurfaceKind::QueryResult {
                query_tab_id: active_tab_id.clone()
            }
        );
        assert_eq!(
            app.active_bottom_panel_surface_kind(BottomPanelTab::Explain),
            WorkbenchSurfaceKind::Explain {
                query_tab_id: active_tab_id
            }
        );
        assert_eq!(
            app.active_bottom_panel_surface_kind(BottomPanelTab::Messages),
            WorkbenchSurfaceKind::Messages
        );
    }

    #[test]
    fn workbench_state_syncs_legacy_sidebar_and_focus() {
        let mut app = DbManagerApp::new_for_test();
        app.state.show_sidebar = false;
        app.state.sidebar_width = 333.0;
        app.set_focus_area(FocusArea::SqlEditor);

        app.sync_workbench_state_from_legacy_layout();

        assert!(!app.state.workbench.primary_sidebar.visible);
        assert_eq!(app.state.workbench.primary_sidebar.width, 333.0);
        assert_eq!(
            app.state.workbench.focus,
            crate::state::WorkbenchFocus::EditorArea
        );
    }

    #[test]
    fn workbench_activity_maps_to_sidebar_panel_groups() {
        let mut app = DbManagerApp::new_for_test();

        app.set_workbench_activity(WorkbenchActivity::Explorer);
        assert_eq!(
            app.state.workbench.active_activity,
            WorkbenchActivity::Explorer
        );
        assert!(app.state.show_sidebar);
        assert!(app.state.sidebar_panel_state.show_connections);
        assert!(!app.state.sidebar_panel_state.show_filters);
        assert!(!app.state.sidebar_panel_state.show_triggers);
        assert!(!app.state.sidebar_panel_state.show_routines);

        app.set_workbench_activity(WorkbenchActivity::Filters);
        assert_eq!(
            app.state.workbench.active_activity,
            WorkbenchActivity::Filters
        );
        assert!(!app.state.sidebar_panel_state.show_connections);
        assert!(app.state.sidebar_panel_state.show_filters);
        assert!(!app.state.sidebar_panel_state.show_triggers);
        assert!(!app.state.sidebar_panel_state.show_routines);

        app.set_workbench_activity(WorkbenchActivity::Objects);
        assert_eq!(
            app.state.workbench.active_activity,
            WorkbenchActivity::Objects
        );
        assert!(!app.state.sidebar_panel_state.show_connections);
        assert!(!app.state.sidebar_panel_state.show_filters);
        assert!(app.state.sidebar_panel_state.show_triggers);
        assert!(app.state.sidebar_panel_state.show_routines);
    }

    #[test]
    fn primary_sidebar_visibility_syncs_legacy_state_and_config() {
        let mut app = DbManagerApp::new_for_test();

        app.set_primary_sidebar_visible(false);

        assert!(!app.state.show_sidebar);
        assert!(!app.state.workbench.primary_sidebar.visible);
        assert!(!app.app_config.workbench.sidebar.visible);
    }

    #[test]
    fn render_top_bar_handles_keyboard_activation() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        app.set_focus_area(FocusArea::Toolbar);
        app.state.toolbar_index = 3;

        ctx.begin_pass(egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers::NONE,
            }],
            modifiers: egui::Modifiers::NONE,
            ..Default::default()
        });
        egui::Area::new(egui::Id::new("top_bar_keyboard_test")).show(&ctx, |ui| {
            let actions = app.render_top_bar(ui);
            app.handle_toolbar_actions(ui.ctx(), actions);
        });
        let _ = ctx.end_pass();

        assert_eq!(app.active_dialog_id(), Some(DialogId::ToolbarActionsMenu));
    }
}
