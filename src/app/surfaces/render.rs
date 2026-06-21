//! UI 渲染模块
//!
//! 将 `update()` 中的渲染逻辑拆分到此模块，提高代码可维护性。

use eframe::egui;

use crate::app::dialogs::host::DialogId;
use crate::core::{BottomPanelTab, constants, format_sql};
use crate::data::{ConnectionConfig, QueryResult};
use crate::state::WorkbenchSurfaceKind;
use crate::ui::{self, SqlEditorActions, TabBarActions, ToolbarActions};

use super::DbManagerApp;
use super::action_system::AppAction;

/// 主工作区当前应显示的内容表面。
///
/// 这是把散落在渲染分支里的"空状态/结果状态"判断收束成显式分类的第一步，
/// 后续可以继续扩展为导入预览、Explain 结果、命令面板等工作区表面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceSurface {
    Welcome,
    QueryOutputAvailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErDiagramSurfaceAction {
    FocusDiagram,
    RefreshData,
    Relayout,
    FitView,
}

fn classify_workspace_surface(
    result: Option<&QueryResult>,
    active_query_error: Option<&str>,
) -> WorkspaceSurface {
    if active_query_error.is_some() || result.is_some() {
        WorkspaceSurface::QueryOutputAvailable
    } else {
        WorkspaceSurface::Welcome
    }
}

fn select_sql_editor_status_message(
    active_tab_message: Option<&str>,
    latest_notification: Option<&str>,
) -> Option<String> {
    active_tab_message
        .map(ToOwned::to_owned)
        .or_else(|| latest_notification.map(ToOwned::to_owned))
}

fn workbench_main_width(
    available_width: f32,
    reserved_sidebar_width: f32,
    right_inspector_width: f32,
    right_inspector_divider_width: f32,
) -> f32 {
    (available_width
        - reserved_sidebar_width
        - right_inspector_width
        - right_inspector_divider_width)
        .max(0.0)
}

#[cfg(test)]
fn clamped_sql_editor_height(preferred_height: f32, available_height: f32) -> f32 {
    let max_height = (available_height * 0.6).max(0.0);
    if max_height <= 0.0 {
        return 0.0;
    }

    let min_height = 100.0_f32.min(max_height);
    preferred_height.clamp(min_height, max_height)
}

fn collect_er_diagram_surface_actions(
    response: &ui::ERDiagramResponse,
) -> Vec<ErDiagramSurfaceAction> {
    let mut actions = Vec::new();

    if response.request_focus {
        actions.push(ErDiagramSurfaceAction::FocusDiagram);
    }
    if response.refresh_requested {
        actions.push(ErDiagramSurfaceAction::RefreshData);
    }
    if response.layout_requested {
        actions.push(ErDiagramSurfaceAction::Relayout);
    }
    if response.fit_view_requested {
        actions.push(ErDiagramSurfaceAction::FitView);
    }

    actions
}

impl DbManagerApp {
    fn render_query_output_placeholder(
        &mut self,
        ui: &mut egui::Ui,
        active_query_error: Option<&str>,
    ) {
        let target_tab = if active_query_error.is_some() {
            BottomPanelTab::Messages
        } else {
            BottomPanelTab::Results
        };
        let (title, detail, action_label) = if active_query_error.is_some() {
            (
                "查询消息已移到底部面板",
                "错误详情会保留在 Messages，不再覆盖编辑工作区。",
                "打开 Messages",
            )
        } else {
            (
                "查询结果已移到底部面板",
                "结果表格会保留在 Results，编辑器和其它工作区仍可继续使用。",
                "打开 Results",
            )
        };

        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.heading(title);
            ui.add_space(8.0);
            ui.label(detail);
            ui.add_space(12.0);
            if ui.button(action_label).clicked() {
                let ctx = ui.ctx().clone();
                self.dispatch_app_action(&ctx, AppAction::SetBottomPanelTab(target_tab));
            }
        });
    }
}

impl DbManagerApp {
    /// 每帧主流程：消息处理、快捷键、对话框、中心区域渲染与动作落地。
    pub(in crate::app) fn run_frame(&mut self, root_ui: &mut egui::Ui) {
        let ctx = root_ui.ctx().clone();
        let mut toolbar_actions = ToolbarActions::default();

        self.reconcile_active_dialog_owner();
        self.handle_messages(&ctx);
        self.handle_input_router(&ctx, &mut toolbar_actions);
        self.handle_zoom_shortcuts(&ctx);

        // 清理过期通知，如果有通知被清理则请求重绘
        if self.session.notifications.tick() {
            ctx.request_repaint();
        }

        // 批量持久化配置（5秒节流）
        self.tick_config_save();

        // ===== 对话框 =====
        let was_connection_dialog_open = self.state.show_connection_dialog;
        let dialog_results = self.render_dialogs(&ctx);
        let save_connection = dialog_results.save_connection;
        self.handle_dialog_results(&ctx, dialog_results);

        if was_connection_dialog_open && !self.state.show_connection_dialog && !save_connection {
            self.state.editing_connection_name = None;
            self.state.new_config = ConnectionConfig::default();
        }

        // ===== Workbench shell =====
        let central_frame = egui::Frame::NONE
            .fill(ctx.global_style().visuals.panel_fill)
            .inner_margin(egui::Margin::same(0));

        // 侧边栏操作结果（在 CentralPanel 外声明）
        let mut sidebar_actions = ui::SidebarActions::default();
        self.sync_workbench_state_from_legacy_layout();
        let status_bar = self
            .state
            .workbench
            .status_bar
            .visible
            .then(|| self.workbench_status_bar_content());

        Self::render_workbench(root_ui, central_frame, status_bar, |ui| {
            let top_bar_actions = self.render_top_bar(ui);
            self.handle_toolbar_actions(ui.ctx(), top_bar_actions);
            ui.separator();

            // 准备连接列表（侧边栏使用）
            let mut connections: Vec<String> =
                self.session.manager.connections.keys().cloned().collect();
            connections.sort_unstable();

            // 使用 horizontal 布局：侧边栏 + 分割条 + 主内容区
            let available_width = ui.available_width();
            let available_height = ui.available_height();
            let divider_width = 8.0;

            // 计算侧边栏和主内容区的宽度
            let primary_sidebar_fallback_visible =
                self.state.show_sidebar && !self.active_activity_surface_is_docked();
            let sidebar_width = if primary_sidebar_fallback_visible {
                self.state.sidebar_width
            } else {
                0.0
            };
            let reserved_sidebar_width = if primary_sidebar_fallback_visible {
                sidebar_width + divider_width
            } else {
                0.0
            };
            let right_inspector_visible = self.state.workbench.right_inspector.visible
                && !self.active_right_inspector_surface_is_docked();
            let right_inspector_divider_width = if right_inspector_visible { 6.0 } else { 0.0 };
            let right_inspector_width = if right_inspector_visible {
                self.right_inspector_width_for_available(available_width)
            } else {
                0.0
            };
            let main_width = workbench_main_width(
                available_width,
                reserved_sidebar_width,
                right_inspector_width,
                right_inspector_divider_width,
            );

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                // ===== 侧边栏区域 =====
                if primary_sidebar_fallback_visible {
                    let mut sidebar_clicked = false;
                    ui.allocate_ui_with_layout(
                        egui::vec2(sidebar_width, available_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_size(egui::vec2(sidebar_width, available_height));

                            // 只有在没有对话框打开时，侧边栏才响应键盘
                            let is_sidebar_focused = self.state.focus_area
                                == ui::FocusArea::Sidebar
                                && !self.has_modal_dialog_open();

                            // 获取当前查询结果的列信息
                            let columns: Vec<String> = self
                                .state
                                .result
                                .as_ref()
                                .map(|r| r.columns.clone())
                                .unwrap_or_default();

                            if self.active_activity_uses_legacy_sidebar() {
                                // 同步表加载态，让侧栏区分"加载中"与"空 schema"（审计 SM-3）。
                                self.state.sidebar_panel_state.loading_tables =
                                    self.session.connecting;
                                let (actions, filter_changed) = ui::Sidebar::show_in_ui(
                                    ui,
                                    &mut self.session.manager,
                                    &mut self.state.selected_table,
                                    &mut self.state.show_connection_dialog,
                                    is_sidebar_focused,
                                    self.state.sidebar_section,
                                    &mut self.state.sidebar_panel_state,
                                    sidebar_width,
                                    &self.keybindings,
                                    &mut self.state.grid_state.filters,
                                    &columns,
                                    &mut self.state.pending_filter_input_focus,
                                );
                                sidebar_actions = actions;

                                // 如果筛选条件改变，使缓存失效
                                if filter_changed {
                                    self.state.grid_state.filter_cache.invalidate();
                                }
                            } else {
                                self.render_primary_sidebar_activity_placeholder(ui, sidebar_width);
                            }

                            if ui.ui_contains_pointer()
                                && ui.input(|input| input.pointer.primary_clicked())
                            {
                                sidebar_clicked = true;
                            }
                        },
                    );

                    if sidebar_clicked {
                        self.set_focus_area(ui::FocusArea::Sidebar);
                        let active_surface = WorkbenchSurfaceKind::from_activity(
                            self.state.workbench.active_activity,
                        );
                        self.state.workbench.set_focused_surface(&active_surface);
                    }

                    // 可拖动的垂直分割条（与 ER 图分割条相同风格）
                    let (divider_rect, divider_response) = ui.allocate_exact_size(
                        egui::vec2(divider_width, available_height),
                        egui::Sense::drag(),
                    );

                    // 绘制分割条
                    let divider_color = if divider_response.dragged() || divider_response.hovered()
                    {
                        egui::Color32::from_rgb(100, 150, 255)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(128, 128, 128, 80)
                    };

                    ui.painter().rect_filled(
                        divider_rect.shrink2(egui::vec2(2.0, 4.0)),
                        egui::CornerRadius::same(2),
                        divider_color,
                    );

                    // 中间的拖动指示器（三个小点）
                    let center = divider_rect.center();
                    for offset in [-10.0, 0.0, 10.0] {
                        ui.painter().circle_filled(
                            egui::pos2(center.x, center.y + offset),
                            2.0,
                            egui::Color32::from_gray(180),
                        );
                    }

                    // 处理拖动调整侧边栏宽度
                    if divider_response.dragged() {
                        let delta = divider_response.drag_delta().x;
                        self.state.sidebar_width = (self.state.sidebar_width + delta).clamp(
                            constants::ui::SIDEBAR_MIN_WIDTH_PX,
                            constants::ui::SIDEBAR_MAX_WIDTH_PX,
                        );
                    }
                    self.state.workbench.primary_sidebar.width = self.state.sidebar_width;
                    self.state.workbench.primary_sidebar.is_resizing = divider_response.dragged();

                    // 鼠标光标
                    if divider_response.hovered() || divider_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                }

                // ===== 主内容区 — egui_dock DockArea + BottomPanel =====
                ui.allocate_ui_with_layout(
                    egui::vec2(main_width, available_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_size(egui::vec2(main_width, available_height));
                        let bottom_panel_visible = self.state.workbench.bottom_panel.visible
                            && !self.active_bottom_panel_surface_is_docked();
                        let bottom_divider_height = if bottom_panel_visible { 6.0 } else { 0.0 };
                        let bottom_panel_height = if bottom_panel_visible {
                            self.bottom_panel_height_for_available(available_height)
                        } else {
                            0.0
                        };
                        let editor_area_height =
                            (available_height - bottom_panel_height - bottom_divider_height)
                                .max(0.0);

                        // Take dock_state to avoid borrow conflict with viewer
                        ui.allocate_ui_with_layout(
                            egui::vec2(main_width, editor_area_height),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                let fallback_dock = self.default_workbench_surface_layout();
                                let mut dock =
                                    std::mem::replace(&mut self.dock_state, fallback_dock);
                                ui::dock_tabs::sync_all(&mut dock, self);
                                let mut viewer = ui::dock_tabs::WorkspaceViewer { app: self };
                                egui_dock::DockArea::new(&mut dock).show_inside(ui, &mut viewer);
                                // Put dock_state back
                                self.dock_state = dock;
                            },
                        );

                        if bottom_panel_visible {
                            self.render_bottom_panel_resize_divider(ui, available_height);
                            ui.allocate_ui_with_layout(
                                egui::vec2(main_width, bottom_panel_height),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    self.render_bottom_panel_in_ui(ui);
                                },
                            );
                        }
                    },
                ); // allocate_ui_with_layout 主内容区结束

                if right_inspector_visible {
                    self.render_right_inspector_resize_divider(
                        ui,
                        available_width,
                        available_height,
                    );
                    ui.allocate_ui_with_layout(
                        egui::vec2(right_inspector_width, available_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_size(egui::vec2(right_inspector_width, available_height));
                            self.render_right_inspector_in_ui(ui);
                        },
                    );
                }
            }); // horizontal 布局结束
        }); // Workbench shell 闭包结束

        // ===== 处理各种操作 =====
        self.handle_toolbar_actions(&ctx, toolbar_actions);
        self.handle_sidebar_actions(&ctx, sidebar_actions);
        if matches!(self.active_dialog_id(), None | Some(DialogId::WelcomeSetup)) {
            self.show_welcome_setup_dialog_window(&ctx);
        }

        // 保存新连接
        if save_connection {
            self.save_connection_from_dialog();
        }

        if matches!(
            self.active_dialog_id(),
            None | Some(DialogId::CommandPalette)
        ) {
            self.render_command_palette(&ctx);
        }

        // 渲染通知 toast
        ui::NotificationToast::show(&ctx, &self.session.notifications);

        // 持续刷新（有活动任务或有通知时需要刷新）
        if self.session.connecting
            || self.session.executing
            || !self.session.notifications.is_empty()
        {
            ctx.request_repaint();
        }
    }

    /// 渲染非 SQL 文档占位内容，供 EditorArea dock tab 调用。
    pub(crate) fn render_workspace_content(&mut self, ui: &mut egui::Ui) {
        // Dock tabs 提供原生标签 UI，此处仅保留 QueryTabs 键盘快捷键处理。
        let mut tab_actions = ui::TabBarActions::default();
        if self.state.focus_area == ui::FocusArea::QueryTabs {
            ui::QueryTabBar::handle_keyboard(
                ui,
                self.session.tab_manager.tabs.len(),
                self.session.tab_manager.active_index,
                &mut tab_actions,
            );
        }
        self.handle_tab_actions(ui.ctx(), tab_actions);

        // 数据表格 / 欢迎页 / 错误
        let active_query_error = self
            .session
            .tab_manager
            .get_active()
            .and_then(|tab| tab.last_error.clone());
        let workspace_surface =
            classify_workspace_surface(self.state.result.as_ref(), active_query_error.as_deref());

        match workspace_surface {
            WorkspaceSurface::QueryOutputAvailable => {
                self.render_query_output_placeholder(ui, active_query_error.as_deref());
            }
            WorkspaceSurface::Welcome => {
                // Welcome surface — show when no table is selected and no result exists
                let action = ui::Welcome::show(ui, self.state.welcome_status, &self.keybindings);
                if let Some(action) = action {
                    let ctx = ui.ctx().clone();
                    self.handle_welcome_action(&ctx, action);
                }
            }
        }
    }

    pub(crate) fn render_result_grid_in_ui(&mut self, ui: &mut egui::Ui) {
        let Some(result) = &self.state.result else {
            return;
        };

        self.state.grid_state.focused =
            self.state.focus_area == ui::FocusArea::DataGrid && !self.has_modal_dialog_open();
        let table_name = self.state.selected_table.as_deref();
        let db_type = self
            .session
            .manager
            .active
            .as_ref()
            .and_then(|name| self.session.manager.connections.get(name))
            .map(|conn| conn.config.db_type);
        let (grid_actions, _) = ui::DataGrid::show_editable(
            ui,
            result,
            &self.state.search_text,
            &self.state.search_column,
            &mut self.state.selected_row,
            &mut self.state.selected_cell,
            &mut self.state.grid_state,
            table_name,
            &self.keybindings,
            db_type,
        );

        if grid_actions.open_filter_panel {
            let ctx = ui.ctx().clone();
            self.dispatch_app_action(&ctx, AppAction::OpenFilterWorkspace);
        }
        if grid_actions.request_focus {
            self.set_focus_area(ui::FocusArea::DataGrid);
        }
        if let Some(transfer) = grid_actions.focus_transfer {
            match transfer {
                ui::FocusTransfer::Sidebar => {
                    self.dispatch_app_action(ui.ctx(), AppAction::FocusSidebar);
                }
                ui::FocusTransfer::SqlEditor => {
                    self.dispatch_app_action(ui.ctx(), AppAction::FocusEditor);
                }
                ui::FocusTransfer::QueryTabs => {
                    self.dispatch_app_action(ui.ctx(), AppAction::FocusQueryTabs);
                }
            }
        }
        if grid_actions.refresh_requested {
            self.dispatch_app_action(ui.ctx(), AppAction::RefreshSelectedTable);
        }
        if let Some(message) = grid_actions.message {
            self.session.notifications.info(message);
        }
        if !grid_actions.sql_to_execute.is_empty() {
            // 网格保存走事务化批量通道（修复 B1/B2/B3）：整批原子提交，成功后清编辑并刷新。
            let save_table = self.state.selected_table.clone().unwrap_or_default();
            self.execute_grid_save(save_table, grid_actions.sql_to_execute);
        }
        if let Some(tab) = grid_actions.switch_to_tab {
            let index = tab.saturating_sub(1);
            self.dispatch_app_action(ui.ctx(), AppAction::SwitchToQueryTab(index));
        }
    }

    /// 渲染 ER 关系图，供 egui_dock TabViewer 调用
    pub(crate) fn render_er_diagram_in_ui(&mut self, ui: &mut egui::Ui) {
        let theme_preset = self.state.theme_manager.current;
        let er_is_focused =
            self.state.focus_area == ui::FocusArea::ErDiagram && !self.has_modal_dialog_open();
        let previous_selection = self
            .state
            .er_diagram_state
            .selected_table_name()
            .map(ToOwned::to_owned);
        let er_response = self
            .state
            .er_diagram_state
            .show(ui, &theme_preset, er_is_focused);
        let next_selection = self
            .state
            .er_diagram_state
            .selected_table_name()
            .map(ToOwned::to_owned);
        if next_selection.is_some() && next_selection != previous_selection {
            self.reveal_right_inspector_for_inspect(crate::core::RightInspectorTab::ErSelection);
        }
        for action in collect_er_diagram_surface_actions(&er_response) {
            match action {
                ErDiagramSurfaceAction::FocusDiagram => {
                    self.set_focus_area(ui::FocusArea::ErDiagram);
                }
                ErDiagramSurfaceAction::RefreshData => {
                    self.load_er_diagram_data();
                }
                ErDiagramSurfaceAction::Relayout => {
                    let summary = ui::analyze_er_graph(
                        &self.state.er_diagram_state.tables,
                        &self.state.er_diagram_state.relationships,
                    );
                    ui::apply_er_layout_strategy(
                        &mut self.state.er_diagram_state.tables,
                        &self.state.er_diagram_state.relationships,
                        summary.strategy,
                    );
                }
                ErDiagramSurfaceAction::FitView => {
                    let available_size = ui.available_size();
                    self.state.er_diagram_state.fit_to_view(available_size);
                }
            }
        }
    }

    /// 渲染 SQL 文档内容，供 EditorArea dock tab 调用。
    pub(crate) fn render_sql_document_in_ui(&mut self, ui: &mut egui::Ui) -> SqlEditorActions {
        let mut sql_editor_actions = SqlEditorActions::default();

        if !self.state.show_sql_editor {
            self.render_sql_document_hidden_placeholder(ui);
            return sql_editor_actions;
        }

        // 只有在没有对话框打开时，SQL 编辑器才响应快捷键
        let is_editor_focused =
            self.state.focus_area == ui::FocusArea::SqlEditor && !self.has_modal_dialog_open();

        // SQL 编辑器内容区域
        // 确保至少有一个 tab 用于 SQL 编辑
        if self.session.tab_manager.tabs.is_empty() {
            self.session.tab_manager.new_tab();
        }
        // 在获取 &mut sql 之前提取不可变值
        let active_tab_message = self
            .session
            .tab_manager
            .tabs
            .get(self.session.tab_manager.active_index)
            .and_then(|tab| tab.last_message.as_deref())
            .map(|s| s.to_owned());
        let tab_sql = &mut self.session.tab_manager.tabs[self.session.tab_manager.active_index].sql;

        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), ui.available_height()),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                let latest_msg = select_sql_editor_status_message(
                    active_tab_message.as_deref(),
                    self.session.notifications.latest_message(),
                );
                let mut request_editor_widget_focus =
                    is_editor_focused && self.state.editor_mode == ui::EditorMode::Insert;
                sql_editor_actions = ui::SqlEditor::show(
                    ui,
                    tab_sql,
                    &self.session.command_history,
                    &mut self.session.history_index,
                    self.session.executing,
                    &latest_msg,
                    &self.state.highlight_colors,
                    self.session.last_query_time_ms,
                    &self.session.autocomplete,
                    &mut self.state.show_autocomplete,
                    &mut self.state.selected_completion,
                    &mut request_editor_widget_focus,
                    is_editor_focused,
                    &mut self.state.editor_mode,
                );
            },
        );

        sql_editor_actions
    }

    fn render_sql_document_hidden_placeholder(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.heading("SQL 文档已隐藏");
            ui.add_space(8.0);
            ui.label("SQL 现在是 EditorArea 文档。旧的编辑器开关暂时只隐藏文档内容。");
            ui.add_space(12.0);
            if ui.button("显示 SQL 文档").clicked() {
                let ctx = ui.ctx().clone();
                self.dispatch_app_action(&ctx, AppAction::FocusEditor);
            }
        });
    }

    pub(crate) fn render_schema_object_placeholder(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.heading("Schema Object");
            ui.add_space(8.0);
            ui.label("SchemaObject 已作为 EditorArea 目标视图预留。具体内容会在 RightInspector/Schema 阶段接入。");
        });
    }

    /// 处理 SQL 编辑器操作
    pub(crate) fn handle_sql_editor_actions(&mut self, actions: SqlEditorActions) {
        // 执行查询
        let active_sql = self.active_sql().to_string();
        if actions.execute && !active_sql.is_empty() {
            let _ = self.execute(active_sql.clone());
        }

        // 取消正在执行的查询（修复审计 B4）
        if actions.cancel {
            self.cancel_active_query();
        }

        // EXPLAIN 分析
        if actions.explain && !active_sql.is_empty() {
            let sql = active_sql.trim();
            let explain_sql = if self.is_mysql() {
                format!("EXPLAIN FORMAT=TRADITIONAL {}", sql)
            } else if self
                .session
                .manager
                .get_active()
                .map(|c| c.config.db_type == crate::data::DatabaseType::PostgreSQL)
                .unwrap_or(false)
            {
                format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {}", sql)
            } else {
                format!("EXPLAIN QUERY PLAN {}", sql)
            };
            let _ = self.execute(explain_sql);
            self.session.notifications.info("正在分析执行计划...");
        }

        // 格式化
        if actions.format {
            let formatted = format_sql(&active_sql);
            self.set_active_sql(formatted);
        }

        // 清空
        if actions.clear {
            self.set_active_sql(String::new());
            if let Some(tab) = self.session.tab_manager.get_active_mut() {
                tab.sql.clear();
                tab.modified = false;
                tab.update_title();
            }
            self.session.notifications.dismiss_all();
            self.session.last_query_time_ms = None;
        }

        // 焦点转移到表格
        if actions.focus_to_grid {
            self.set_focus_area(ui::FocusArea::DataGrid);
        }

        // 编辑器请求焦点
        if actions.request_focus {
            self.set_focus_area(ui::FocusArea::SqlEditor);
        }

        if actions.text_changed {
            // SQL 直接写入 tab，无需反向同步
        }
    }

    /// 处理工具栏操作
    pub(in crate::app) fn handle_toolbar_actions(
        &mut self,
        ctx: &egui::Context,
        actions: ToolbarActions,
    ) {
        if actions.toggle_sidebar {
            self.dispatch_app_action(ctx, AppAction::ToggleSidebar);
        }

        if actions.toggle_editor {
            self.dispatch_app_action(ctx, AppAction::ToggleSqlEditor);
        }

        if actions.refresh_tables {
            self.dispatch_app_action(ctx, AppAction::RefreshActiveConnection);
        }

        if actions.open_actions_menu {
            self.open_dialog(DialogId::ToolbarActionsMenu);
        }

        if actions.open_create_menu {
            self.open_dialog(DialogId::ToolbarCreateMenu);
        }

        if actions.open_theme_selector {
            self.dispatch_app_action(ctx, AppAction::OpenThemeSelectorDialog);
        }

        // 连接切换
        if let Some(conn_name) = actions.switch_connection
            && self.session.manager.active.as_deref() != Some(&conn_name)
        {
            self.connect(conn_name);
            self.switch_grid_workspace(None);
            self.clear_result();
            self.session.autocomplete.clear();
            self.state.sidebar_panel_state.clear_triggers();
            self.state.sidebar_panel_state.clear_routines();
            self.state.sidebar_panel_state.loading_triggers = false;
            self.state.sidebar_panel_state.loading_routines = false;
            // 清除上一个连接的 ER 图（并使其在途回包失效）；若 ER 打开则会随新连接 reveal 重载（修复审计 CONN-F2）。
            self.state.er_diagram_state.clear();
            self.state.er_diagram_state.loading = false;
        }

        // 数据库切换
        if let Some(db_name) = actions.switch_database {
            self.select_database(db_name);
        }

        // 表切换
        if let Some(table_name) = actions.switch_table {
            self.handle_query_table(ctx, table_name);
        }

        if actions.export {
            self.dispatch_app_action(ctx, AppAction::OpenExportDialog);
        }

        if actions.import {
            self.dispatch_app_action(ctx, AppAction::OpenImportDialog);
        }

        if actions.create_table {
            self.dispatch_app_action(ctx, AppAction::NewTable);
        }

        if actions.create_database {
            self.dispatch_app_action(ctx, AppAction::NewDatabase);
        }

        if actions.create_user {
            self.dispatch_app_action(ctx, AppAction::NewUser);
        }

        if actions.toggle_er_diagram {
            self.dispatch_app_action(ctx, AppAction::ToggleErDiagram);
        }

        // 处理日/夜模式切换（来自工具栏按钮或 Ctrl+D 快捷键）
        if actions.toggle_dark_mode || self.pending_toggle_dark_mode {
            self.pending_toggle_dark_mode = false;
            self.dispatch_app_action(ctx, AppAction::ToggleDarkMode);
        }

        // 缩放操作
        if actions.zoom_in {
            self.set_ui_scale(ctx, self.state.ui_scale + 0.1);
        }
        if actions.zoom_out {
            self.set_ui_scale(ctx, self.state.ui_scale - 0.1);
        }
        if actions.zoom_reset {
            self.set_ui_scale(ctx, 1.0);
        }

        if actions.show_history {
            self.dispatch_app_action(ctx, AppAction::OpenHistoryPanel);
        }

        if actions.show_help {
            self.dispatch_app_action(ctx, AppAction::OpenHelpPanel);
        }

        if actions.show_about {
            self.dispatch_app_action(ctx, AppAction::OpenAboutDialog);
        }

        if actions.show_keybindings {
            self.dispatch_app_action(ctx, AppAction::OpenKeybindingsDialog);
        }

        if let Some(transfer) = actions.focus_transfer {
            match transfer {
                ui::ToolbarFocusTransfer::ToQueryTabs => {
                    self.set_focus_area(ui::FocusArea::QueryTabs);
                }
            }
        }
    }

    /// 处理创建用户操作
    pub(in crate::app) fn handle_create_user_action(&mut self) {
        if let Some(conn) = self.session.manager.get_active() {
            let db_type = conn.config.db_type;
            if matches!(db_type, crate::data::DatabaseType::SQLite) {
                self.session.notifications.warning("SQLite 不支持用户管理");
            } else {
                let databases = conn.databases.clone();
                self.open_dialog(DialogId::CreateUser);
                self.state.create_user_dialog_state.open(db_type, databases);
            }
        } else {
            self.session.notifications.warning("请先连接数据库");
        }
    }

    /// 处理侧边栏操作
    pub(in crate::app) fn handle_sidebar_actions(
        &mut self,
        ctx: &egui::Context,
        actions: ui::SidebarActions,
    ) {
        if actions.filter_changed {
            self.state.grid_state.filter_cache.invalidate();
        }

        // 焦点转移
        if let Some(transfer) = actions.focus_transfer {
            match transfer {
                ui::SidebarFocusTransfer::ToDataGrid => {
                    self.set_focus_area(ui::FocusArea::DataGrid);
                }
            }
        }

        // 层级导航
        if let Some(new_section) = actions.section_change {
            self.state.sidebar_section = new_section;
        }

        // 连接操作
        if let Some(name) = actions.connect {
            self.connect(name);
            // 连接后自动切换到数据库列表
            self.state.sidebar_section = ui::SidebarSection::Databases;
        }

        if let Some(name) = actions.disconnect {
            self.disconnect(name);
        }

        if let Some(name) = actions.edit_connection {
            self.open_connection_editor(&name);
        }

        if let Some((section, name)) = actions.rename_item {
            match section {
                ui::SidebarSection::Connections => {
                    self.open_connection_editor(&name);
                }
                ui::SidebarSection::Tables => {
                    self.prepare_table_rename_sql(&name);
                }
                _ => {
                    self.session.notifications.info("当前区域暂不支持重命名");
                }
            }
        }

        if actions.refresh {
            self.refresh_sidebar_section();
        }

        // 数据库选择
        if let Some(db_name) = actions.select_database {
            self.select_database(db_name);
        }

        // 删除请求
        if let Some(target) = actions.delete {
            self.state.pending_delete_target = Some(target);
            self.open_dialog(DialogId::DeleteConfirm);
        }

        // 查看表结构
        if let Some(table) = actions.show_table_schema {
            self.handle_show_table_schema(ctx, table);
        }

        // 查询表数据
        if let Some(table) = actions.query_table {
            self.handle_query_table(ctx, table);
        }

        if let Some(mode) = actions.insert_filter {
            self.insert_sidebar_filter(mode);
        }

        if actions.clear_filters {
            self.clear_sidebar_filters();
        }

        if let Some(index) = actions.toggle_filter_logic {
            self.toggle_sidebar_filter_logic(index);
        }

        if let Some((index, forward)) = actions.cycle_filter_column {
            self.cycle_sidebar_filter_column(index, forward);
        }

        if let Some(index) = actions.focus_filter_input {
            self.focus_sidebar_filter_input(index);
        }

        // 触发器定义
        if let Some(definition) = actions.show_trigger_definition {
            self.set_active_sql(definition);
            self.state.show_sql_editor = true;
            self.set_focus_area(ui::FocusArea::SqlEditor);
            self.session.notifications.info("触发器定义已加载到编辑器");
        }

        // 存储过程/函数定义
        if let Some(definition) = actions.show_routine_definition {
            self.set_active_sql(definition);
            self.state.show_sql_editor = true;
            self.set_focus_area(ui::FocusArea::SqlEditor);
            self.session
                .notifications
                .info("存储过程/函数定义已加载到编辑器");
        }
    }

    /// 处理查看表结构
    fn handle_show_table_schema(&mut self, ctx: &egui::Context, table: String) {
        self.reset_grid_workspace_for_transient_surface(Some(table));
        self.dispatch_app_action(ctx, AppAction::ShowSelectedTableSchema);
    }

    /// 处理查询表数据
    fn handle_query_table(&mut self, ctx: &egui::Context, table: String) {
        self.switch_grid_workspace(Some(table));
        self.dispatch_app_action(ctx, AppAction::QuerySelectedTable);
    }

    /// 刷新当前侧边栏区域数据
    fn refresh_sidebar_section(&mut self) {
        let active_name = self.session.manager.active.clone();
        let (db_type, selected_database) = self
            .session
            .manager
            .get_active()
            .map(|conn| (conn.config.db_type, conn.selected_database.clone()))
            .unwrap_or((crate::data::DatabaseType::SQLite, None));

        match self.state.sidebar_section {
            ui::SidebarSection::Connections | ui::SidebarSection::Databases => {
                if let Some(name) = active_name {
                    self.connect(name);
                } else {
                    self.session.notifications.info("当前没有活动连接可刷新");
                }
            }
            ui::SidebarSection::Tables => match db_type {
                crate::data::DatabaseType::SQLite => {
                    if let Some(name) = active_name {
                        self.connect(name);
                    } else {
                        self.session.notifications.info("当前没有活动连接可刷新");
                    }
                }
                crate::data::DatabaseType::PostgreSQL | crate::data::DatabaseType::MySQL => {
                    if let Some(db) = selected_database {
                        self.select_database(db);
                    } else {
                        self.session.notifications.info("请先选择数据库");
                    }
                }
            },
            ui::SidebarSection::Triggers => self.load_triggers(),
            ui::SidebarSection::Routines => self.load_routines(),
            ui::SidebarSection::Filters => {
                self.state.grid_state.filter_cache.invalidate();
            }
        }
    }

    /// 侧边栏添加筛选条件
    pub(in crate::app) fn add_sidebar_filter(&mut self) {
        self.insert_sidebar_filter(ui::SidebarFilterInsertMode::AppendEnd);
    }

    fn insert_sidebar_filter(&mut self, mode: ui::SidebarFilterInsertMode) {
        let Some(result) = &self.state.result else {
            self.session
                .notifications
                .warning("当前结果集为空，无法添加筛选条件");
            return;
        };
        let Some(default_col) = result
            .columns
            .get(self.state.selected_cell.map(|(_, col)| col).unwrap_or(0))
            .cloned()
            .or_else(|| result.columns.first().cloned())
        else {
            self.session
                .notifications
                .warning("当前结果集没有可筛选的列");
            return;
        };

        let insert_index = match mode {
            ui::SidebarFilterInsertMode::BelowSelection => {
                if self.state.grid_state.filters.is_empty() {
                    0
                } else {
                    (self.state.sidebar_panel_state.selection.filters + 1)
                        .min(self.state.grid_state.filters.len())
                }
            }
            ui::SidebarFilterInsertMode::AppendEnd => self.state.grid_state.filters.len(),
        };

        self.state
            .grid_state
            .filters
            .insert(insert_index, ui::ColumnFilter::new(default_col));
        self.state.sidebar_panel_state.selection.filters = insert_index;
        self.state.grid_state.filter_cache.invalidate();

        self.state.show_sidebar = true;
        self.state.sidebar_panel_state.show_filters = true;
        self.state.sidebar_section = ui::SidebarSection::Filters;
        self.set_focus_area(ui::FocusArea::Sidebar);
        self.state.sidebar_panel_state.workflow.filter_workspace =
            ui::SidebarFilterWorkspaceMode::Input;
        self.state.pending_filter_input_focus = Some(insert_index);
    }

    /// 侧边栏清空筛选条件
    pub(in crate::app) fn clear_sidebar_filters(&mut self) {
        if self.state.grid_state.filters.is_empty() {
            return;
        }
        self.state.grid_state.filters.clear();
        self.state.sidebar_panel_state.selection.filters = 0;
        self.state.sidebar_panel_state.exit_filter_input();
        self.state.pending_filter_input_focus = None;
        self.state.grid_state.filter_cache.invalidate();
    }

    /// 切换筛选条件逻辑（AND/OR）
    fn toggle_sidebar_filter_logic(&mut self, index: usize) {
        if let Some(filter) = self.state.grid_state.filters.get_mut(index) {
            filter.logic.toggle();
            self.state.grid_state.filter_cache.invalidate();
        }
    }

    /// 循环切换筛选列
    fn cycle_sidebar_filter_column(&mut self, index: usize, forward: bool) {
        let Some(columns) = self.state.result.as_ref().map(|r| r.columns.clone()) else {
            return;
        };
        if columns.is_empty() {
            return;
        }
        let Some(filter) = self.state.grid_state.filters.get_mut(index) else {
            return;
        };

        let current = columns
            .iter()
            .position(|c| c == &filter.column)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % columns.len()
        } else if current == 0 {
            columns.len() - 1
        } else {
            current - 1
        };

        if let Some(new_col) = columns.get(next) {
            filter.column = new_col.clone();
            self.state.grid_state.filter_cache.invalidate();
        }
    }

    /// 聚焦筛选输入框（i）
    fn focus_sidebar_filter_input(&mut self, index: usize) {
        let Some(filter) = self.state.grid_state.filters.get_mut(index) else {
            return;
        };
        if !filter.operator.needs_value() {
            self.session
                .notifications
                .info("当前筛选操作符不需要输入值");
            return;
        }
        filter.enabled = true;
        self.state.sidebar_panel_state.selection.filters = index;
        self.state.pending_filter_input_focus = Some(index);
        self.state.sidebar_panel_state.workflow.filter_workspace =
            ui::SidebarFilterWorkspaceMode::Input;
        self.state.show_sidebar = true;
        self.state.sidebar_panel_state.show_filters = true;
        self.state.sidebar_section = ui::SidebarSection::Filters;
        self.set_focus_area(ui::FocusArea::Sidebar);
    }

    /// 为表重命名生成 SQL 模板
    fn prepare_table_rename_sql(&mut self, table: &str) {
        let Some(conn) = self.session.manager.get_active() else {
            self.session.notifications.warning("请先连接数据库");
            return;
        };

        let db_type = conn.config.db_type;
        let use_backticks = matches!(db_type, crate::data::DatabaseType::MySQL);
        let quoted_old = match ui::quote_identifier(table, use_backticks) {
            Ok(name) => name,
            Err(e) => {
                self.session.notifications.error(format!("表名无效: {}", e));
                return;
            }
        };
        let quoted_new = match ui::quote_identifier("new_table_name", use_backticks) {
            Ok(name) => name,
            Err(e) => {
                self.session
                    .notifications
                    .error(format!("目标表名模板生成失败: {}", e));
                return;
            }
        };

        let rename_sql = match db_type {
            crate::data::DatabaseType::MySQL => {
                format!("RENAME TABLE {} TO {};", quoted_old, quoted_new)
            }
            crate::data::DatabaseType::PostgreSQL | crate::data::DatabaseType::SQLite => {
                format!("ALTER TABLE {} RENAME TO {};", quoted_old, quoted_new)
            }
        };
        self.set_active_sql(rename_sql);
        self.state.show_sql_editor = true;
        self.set_focus_area(ui::FocusArea::SqlEditor);
        self.session
            .notifications
            .info("已生成重命名 SQL，请修改目标表名后执行");
    }

    /// 处理 Tab 栏操作
    pub(in crate::app) fn handle_tab_actions(
        &mut self,
        ctx: &egui::Context,
        tab_actions: TabBarActions,
    ) {
        // 在切换/关闭标签前先保存当前草稿，避免 SQL 被覆盖
        self.persist_active_tab_state_for_navigation();

        if tab_actions.new_tab {
            self.dispatch_app_action(ctx, AppAction::NewQueryTab);
        }

        if let Some(idx) = tab_actions.switch_to {
            self.dispatch_app_action(ctx, AppAction::SwitchToQueryTab(idx));
        }

        if let Some(idx) = tab_actions.close_tab {
            let closing_tab_id = self
                .session
                .tab_manager
                .tabs
                .get(idx)
                .map(|tab| tab.id.clone());
            if self.session.tab_manager.tabs.len() > 1
                && let Some(request_id) = self
                    .session
                    .tab_manager
                    .tabs
                    .get(idx)
                    .and_then(|tab| tab.pending_request_id)
            {
                self.cancel_query_request_silently(request_id);
            }
            self.session.tab_manager.close_tab(idx);
            if let Some(tab_id) = closing_tab_id {
                self.warn_if_tab_has_unsaved_grid_edits(&tab_id);
                self.remove_grid_workspaces_for_tab(&tab_id);
            }
        }

        if tab_actions.close_others {
            let active_index = self.session.tab_manager.active_index;
            let request_ids: Vec<u64> = self
                .session
                .tab_manager
                .tabs
                .iter()
                .enumerate()
                .filter_map(|(idx, tab)| {
                    if idx != active_index {
                        tab.pending_request_id
                    } else {
                        None
                    }
                })
                .collect();
            let closing_tab_ids: Vec<String> = self
                .session
                .tab_manager
                .tabs
                .iter()
                .enumerate()
                .filter_map(|(idx, tab)| {
                    if idx != active_index {
                        Some(tab.id.clone())
                    } else {
                        None
                    }
                })
                .collect();
            for request_id in request_ids {
                self.cancel_query_request_silently(request_id);
            }
            self.session.tab_manager.close_other_tabs();
            for tab_id in closing_tab_ids {
                self.remove_grid_workspaces_for_tab(&tab_id);
            }
        }

        if tab_actions.close_right {
            let active_index = self.session.tab_manager.active_index;
            let request_ids: Vec<u64> = self
                .session
                .tab_manager
                .tabs
                .iter()
                .enumerate()
                .filter_map(|(idx, tab)| {
                    if idx > active_index {
                        tab.pending_request_id
                    } else {
                        None
                    }
                })
                .collect();
            let closing_tab_ids: Vec<String> = self
                .session
                .tab_manager
                .tabs
                .iter()
                .enumerate()
                .filter_map(|(idx, tab)| {
                    if idx > active_index {
                        Some(tab.id.clone())
                    } else {
                        None
                    }
                })
                .collect();
            for request_id in request_ids {
                self.cancel_query_request_silently(request_id);
            }
            self.session.tab_manager.close_tabs_to_right();
            for tab_id in closing_tab_ids {
                self.remove_grid_workspaces_for_tab(&tab_id);
            }
        }

        // 任一关闭路径后同步主视图到新的活动标签，避免底部面板/结果/搜索陈旧一帧（修复审计 Q2）。
        if tab_actions.close_tab.is_some() || tab_actions.close_others || tab_actions.close_right {
            self.sync_from_active_tab();
        }
    }

    /// 渲染辅助面板，供 egui_dock AuxPanel 调用
    pub(crate) fn render_aux_panel(
        &mut self,
        ui: &mut egui::Ui,
        kind: ui::dock_tabs::AuxPanelKind,
    ) {
        use ui::dock_tabs::AuxPanelKind;
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            match kind {
                AuxPanelKind::Help => {
                    ui.heading("帮助和学习");
                    ui.label("按 F1 或使用菜单打开帮助面板");
                }
                AuxPanelKind::Keybindings => {
                    ui.heading("快捷键设置");
                    ui.label("使用 设置 → 快捷键 菜单打开快捷键编辑器");
                }
                AuxPanelKind::History => {
                    ui.heading("查询历史");
                    ui.label("使用 工具栏 → 历史 菜单打开查询历史");
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErDiagramSurfaceAction, WorkspaceSurface, clamped_sql_editor_height,
        classify_workspace_surface, collect_er_diagram_surface_actions,
        select_sql_editor_status_message, workbench_main_width,
    };
    use crate::app::DbManagerApp;
    use crate::app::dialogs::host::DialogId;
    use crate::core::BottomPanelTab;
    use crate::data::{Connection, ConnectionConfig, DatabaseType, QueryResult};
    use crate::state::WorkbenchSurfaceKind;
    use crate::ui::{
        ERDiagramResponse, FocusArea, SidebarActions, SidebarDeleteTarget, ToolbarActions,
    };
    use eframe::egui::{self, Event, Key, Modifiers, RawInput};

    fn run_frame_with_event(app: &mut DbManagerApp, ctx: &egui::Context, event: Event) {
        let modifiers = match &event {
            Event::Key { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        };
        let raw_input = RawInput {
            events: vec![event],
            modifiers,
            ..Default::default()
        };

        ctx.begin_pass(raw_input);
        egui::Area::new(egui::Id::new("render_frame_test_area")).show(ctx, |ui| app.run_frame(ui));
        let _ = ctx.end_pass();
    }

    fn prime_active_connection_with_tables(app: &mut DbManagerApp, tables: &[&str]) {
        let mut connection = Connection::new(ConnectionConfig::new("demo", DatabaseType::SQLite));
        connection.connected = true;
        connection.selected_database = Some("main".to_string());
        connection.tables = tables.iter().map(|name| (*name).to_string()).collect();
        app.session
            .manager
            .connections
            .insert("demo".to_string(), connection);
        app.session.manager.active = Some("demo".to_string());
    }

    #[test]
    fn workspace_surface_routes_query_output_to_bottom_panel() {
        assert_eq!(
            classify_workspace_surface(None, None),
            WorkspaceSurface::Welcome
        );
        assert_eq!(
            classify_workspace_surface(
                Some(&QueryResult {
                    affected_rows: 3,
                    ..QueryResult::default()
                }),
                None,
            ),
            WorkspaceSurface::QueryOutputAvailable
        );
        assert_eq!(
            classify_workspace_surface(
                Some(&QueryResult::with_rows(vec!["id".to_string()], Vec::new())),
                None,
            ),
            WorkspaceSurface::QueryOutputAvailable
        );
    }

    #[test]
    fn workspace_surface_routes_query_error_to_bottom_panel() {
        assert_eq!(
            classify_workspace_surface(
                Some(&QueryResult::with_rows(vec!["id".to_string()], Vec::new())),
                Some("错误: syntax error"),
            ),
            WorkspaceSurface::QueryOutputAvailable
        );
        assert_eq!(
            classify_workspace_surface(None, Some("错误: syntax error")),
            WorkspaceSurface::QueryOutputAvailable
        );
    }

    #[test]
    fn sql_editor_status_message_prefers_active_tab_message() {
        assert_eq!(
            select_sql_editor_status_message(Some("查询完成，返回 1 行"), Some("已连接到 demo")),
            Some("查询完成，返回 1 行".to_string())
        );
        assert_eq!(
            select_sql_editor_status_message(None, Some("已连接到 demo")),
            Some("已连接到 demo".to_string())
        );
    }

    #[test]
    fn workbench_main_width_does_not_reserve_duplicate_activity_rail() {
        assert_eq!(workbench_main_width(1200.0, 0.0, 0.0, 0.0), 1200.0);
        assert_eq!(workbench_main_width(1200.0, 288.0, 326.0, 6.0), 580.0);
        assert_eq!(workbench_main_width(400.0, 288.0, 326.0, 6.0), 0.0);
    }

    #[test]
    fn er_diagram_surface_actions_follow_response_flags_in_stable_order() {
        let response = ERDiagramResponse {
            request_focus: true,
            refresh_requested: true,
            layout_requested: true,
            fit_view_requested: true,
        };

        assert_eq!(
            collect_er_diagram_surface_actions(&response),
            vec![
                ErDiagramSurfaceAction::FocusDiagram,
                ErDiagramSurfaceAction::RefreshData,
                ErDiagramSurfaceAction::Relayout,
                ErDiagramSurfaceAction::FitView,
            ]
        );
    }

    #[test]
    fn er_diagram_surface_actions_skip_unset_flags() {
        let response = ERDiagramResponse {
            refresh_requested: true,
            ..Default::default()
        };

        assert_eq!(
            collect_er_diagram_surface_actions(&response),
            vec![ErDiagramSurfaceAction::RefreshData]
        );
        assert!(collect_er_diagram_surface_actions(&ERDiagramResponse::default()).is_empty());
    }

    #[test]
    fn sql_editor_height_keeps_min_height_when_viewport_allows_it() {
        assert_eq!(clamped_sql_editor_height(80.0, 500.0), 100.0);
        assert_eq!(clamped_sql_editor_height(260.0, 500.0), 260.0);
    }

    #[test]
    fn sql_editor_height_shrinks_in_tiny_viewport_without_panicking() {
        let height = clamped_sql_editor_height(240.0, 47.343753);
        assert!((height - 28.406252).abs() < 0.001);

        let zero_height = clamped_sql_editor_height(240.0, 0.0);
        assert_eq!(zero_height, 0.0);
    }

    #[test]
    fn sidebar_delete_action_opens_delete_confirm_with_saved_target() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        let target = SidebarDeleteTarget::connection("primary".to_string());
        let actions = SidebarActions {
            delete: Some(target.clone()),
            ..Default::default()
        };

        app.handle_sidebar_actions(&ctx, actions);

        assert_eq!(app.state.pending_delete_target, Some(target));
        assert!(app.state.show_delete_confirm);
        assert_eq!(app.active_dialog_id(), Some(DialogId::DeleteConfirm));
    }

    #[test]
    fn toolbar_toggle_sidebar_does_not_mutate_dock_navigation_tabs() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        let result_surface = app.active_bottom_panel_surface_kind(BottomPanelTab::Results);

        assert!(app.state.show_sidebar);
        assert!(!app.has_workbench_surface_tab(&WorkbenchSurfaceKind::Explorer));
        assert!(app.has_workbench_surface_tab(&result_surface));

        app.handle_toolbar_actions(
            &ctx,
            ToolbarActions {
                toggle_sidebar: true,
                ..Default::default()
            },
        );

        assert!(!app.state.show_sidebar);
        assert!(!app.has_workbench_surface_tab(&WorkbenchSurfaceKind::Explorer));
        assert!(app.has_workbench_surface_tab(&result_surface));

        app.handle_toolbar_actions(
            &ctx,
            ToolbarActions {
                toggle_sidebar: true,
                ..Default::default()
            },
        );

        assert!(app.state.show_sidebar);
        assert!(!app.has_workbench_surface_tab(&WorkbenchSurfaceKind::Explorer));
        assert!(app.has_workbench_surface_tab(&result_surface));
    }

    #[test]
    fn ctrl_r_opening_er_moves_focus_off_data_grid_before_grid_can_consume_j() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.state.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        app.state.selected_table = Some("customers".to_string());
        app.state.show_er_diagram = false;
        app.set_focus_area(FocusArea::DataGrid);
        app.state.grid_state.cursor = (0, 0);
        app.state.grid_state.focused = true;

        run_frame_with_event(
            &mut app,
            &ctx,
            Event::Key {
                key: Key::R,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::CTRL,
            },
        );

        assert!(app.state.show_er_diagram);
        assert_eq!(app.state.focus_area, FocusArea::ErDiagram);
        assert!(!app.state.grid_state.focused);
        assert_eq!(
            app.state.er_diagram_state.selected_table_name(),
            Some("customers")
        );

        run_frame_with_event(
            &mut app,
            &ctx,
            Event::Key {
                key: Key::J,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        );

        assert_eq!(app.state.focus_area, FocusArea::ErDiagram);
        assert!(!app.state.grid_state.focused);
        assert_eq!(app.state.grid_state.cursor, (0, 0));
        assert_eq!(
            app.state.er_diagram_state.selected_table_name(),
            Some("orders")
        );
    }

    #[test]
    fn toolbar_toggle_er_diagram_action_focuses_er_from_data_grid() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.state.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        app.state.selected_table = Some("customers".to_string());
        app.state.show_er_diagram = false;
        app.set_focus_area(FocusArea::DataGrid);

        let actions = ToolbarActions {
            toggle_er_diagram: true,
            ..Default::default()
        };
        app.handle_toolbar_actions(&ctx, actions);

        assert!(app.state.show_er_diagram);
        assert_eq!(app.state.focus_area, FocusArea::ErDiagram);
        assert!(!app.state.grid_state.focused);
        assert_eq!(
            app.state.er_diagram_state.selected_table_name(),
            Some("customers")
        );
    }

    #[test]
    fn er_diagram_v_shortcut_toggles_viewport_mode_once_per_frame() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.state.show_er_diagram = true;
        app.set_focus_area(FocusArea::ErDiagram);

        run_frame_with_event(
            &mut app,
            &ctx,
            Event::Key {
                key: Key::V,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        );
        assert!(app.state.er_diagram_state.is_viewport_mode());

        run_frame_with_event(
            &mut app,
            &ctx,
            Event::Key {
                key: Key::V,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            },
        );
        assert!(!app.state.er_diagram_state.is_viewport_mode());
    }
}
