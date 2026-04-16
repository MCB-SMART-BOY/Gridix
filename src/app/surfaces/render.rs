//! UI 渲染模块
//!
//! 将 `update()` 中的渲染逻辑拆分到此模块，提高代码可维护性。

use eframe::egui;

use crate::app::dialogs::host::DialogId;
use crate::core::{constants, format_sql};
use crate::database::{ConnectionConfig, QueryResult};
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
    QueryError,
    TabularResult,
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
    if active_query_error.is_some() {
        WorkspaceSurface::QueryError
    } else if result.is_some_and(|result| !result.columns.is_empty()) {
        WorkspaceSurface::TabularResult
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
    fn render_query_error_surface(ui: &mut egui::Ui, error: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(24.0);
            ui.heading("查询执行失败");
            ui.add_space(8.0);
            ui.label("结果表格未更新。请修正 SQL 后重新执行。");
            ui.add_space(12.0);
            ui.group(|ui| {
                ui.set_width(ui.available_width().min(720.0));
                ui.label(egui::RichText::new(error).monospace());
            });
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
        if self.notifications.tick() {
            ctx.request_repaint();
        }

        // ===== 对话框 =====
        let was_connection_dialog_open = self.show_connection_dialog;
        let dialog_results = self.render_dialogs(&ctx);
        let save_connection = dialog_results.save_connection;
        self.handle_dialog_results(&ctx, dialog_results);

        if was_connection_dialog_open && !self.show_connection_dialog && !save_connection {
            self.editing_connection_name = None;
            self.new_config = ConnectionConfig::default();
        }

        // SQL 编辑器操作（将在主内容区内部渲染）
        let mut sql_editor_actions = SqlEditorActions::default();
        let mut welcome_action = None;

        // ===== 中心面板 =====
        let central_frame = egui::Frame::NONE
            .fill(ctx.global_style().visuals.panel_fill)
            .inner_margin(egui::Margin::same(0));

        // 侧边栏操作结果（在 CentralPanel 外声明）
        let mut sidebar_actions = ui::SidebarActions::default();

        egui::CentralPanel::default()
            .frame(central_frame)
            .show_inside(root_ui, |ui| {
                // 准备连接、数据库和表列表数据（提前克隆以避免借用冲突）
                let mut connections: Vec<String> =
                    self.manager.connections.keys().cloned().collect();
                connections.sort_unstable();
                let active_connection = self.manager.active.clone();
                let (databases, selected_database, tables): (
                    Vec<String>,
                    Option<String>,
                    Vec<String>,
                ) = self
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
                let selected_table_for_toolbar = self.selected_table.clone();

                // 使用 horizontal 布局：侧边栏 + 分割条 + 主内容区
                let available_width = ui.available_width();
                let available_height = ui.available_height();
                let divider_width = 8.0;

                // 计算侧边栏和主内容区的宽度
                let sidebar_width = if self.show_sidebar {
                    self.sidebar_width
                } else {
                    0.0
                };
                let main_width = if self.show_sidebar {
                    available_width - sidebar_width - divider_width
                } else {
                    available_width
                };

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                    // ===== 侧边栏区域 =====
                    if self.show_sidebar {
                        let mut sidebar_clicked = false;
                        ui.allocate_ui_with_layout(
                            egui::vec2(sidebar_width, available_height),
                            egui::Layout::top_down(egui::Align::LEFT),
                            |ui| {
                                ui.set_min_size(egui::vec2(sidebar_width, available_height));

                                // 只有在没有对话框打开时，侧边栏才响应键盘
                                let is_sidebar_focused = self.focus_area == ui::FocusArea::Sidebar
                                    && !self.has_modal_dialog_open();

                                // 获取当前查询结果的列信息
                                let columns: Vec<String> = self
                                    .result
                                    .as_ref()
                                    .map(|r| r.columns.clone())
                                    .unwrap_or_default();

                                let (actions, filter_changed) = ui::Sidebar::show_in_ui(
                                    ui,
                                    &mut self.manager,
                                    &mut self.selected_table,
                                    &mut self.show_connection_dialog,
                                    is_sidebar_focused,
                                    self.sidebar_section,
                                    &mut self.sidebar_panel_state,
                                    sidebar_width,
                                    &self.keybindings,
                                    &mut self.grid_state.filters,
                                    &columns,
                                    &mut self.pending_filter_input_focus,
                                );
                                sidebar_actions = actions;

                                // 如果筛选条件改变，使缓存失效
                                if filter_changed {
                                    self.grid_state.filter_cache.invalidate();
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
                        }

                        // 可拖动的垂直分割条（与 ER 图分割条相同风格）
                        let (divider_rect, divider_response) = ui.allocate_exact_size(
                            egui::vec2(divider_width, available_height),
                            egui::Sense::drag(),
                        );

                        // 绘制分割条
                        let divider_color =
                            if divider_response.dragged() || divider_response.hovered() {
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
                            self.sidebar_width = (self.sidebar_width + delta).clamp(
                                constants::ui::SIDEBAR_MIN_WIDTH_PX,
                                constants::ui::SIDEBAR_MAX_WIDTH_PX,
                            );
                        }

                        // 鼠标光标
                        if divider_response.hovered() || divider_response.dragged() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                    }

                    // ===== 主内容区 =====
                    ui.allocate_ui_with_layout(
                        egui::vec2(main_width, available_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_size(egui::vec2(main_width, available_height));

                            // 添加左边距（仅当侧边栏显示时不需要，否则需要）
                            let content_margin = if self.show_sidebar { 0 } else { 8 };
                            egui::Frame::NONE
                                .inner_margin(egui::Margin {
                                    left: content_margin,
                                    right: 8,
                                    top: 8,
                                    bottom: 8,
                                })
                                .show(ui, |ui| {
                                    // 工具栏
                                    let is_toolbar_focused =
                                        self.focus_area == ui::FocusArea::Toolbar;
                                    let mut toolbar_clicked = false;
                                    let cancel_task_id = ui
                                        .scope(|ui| {
                                            let cancel_task_id = ui::Toolbar::show_with_focus(
                                                ui,
                                                &self.theme_manager,
                                                &self.keybindings,
                                                self.result.is_some(),
                                                self.show_sidebar,
                                                self.show_sql_editor,
                                                self.app_config.is_dark_mode,
                                                &mut toolbar_actions,
                                                &connections,
                                                active_connection.as_deref(),
                                                &databases,
                                                selected_database.as_deref(),
                                                &tables,
                                                selected_table_for_toolbar.as_deref(),
                                                self.ui_scale,
                                                &self.progress,
                                                is_toolbar_focused,
                                                self.toolbar_index,
                                            );
                                            if ui.ui_contains_pointer()
                                                && ui.input(|input| input.pointer.primary_clicked())
                                            {
                                                toolbar_clicked = true;
                                            }
                                            cancel_task_id
                                        })
                                        .inner;

                                    if toolbar_clicked {
                                        self.set_focus_area(ui::FocusArea::Toolbar);
                                    }

                                    // 处理进度任务取消
                                    if let Some(id) = cancel_task_id {
                                        self.progress.cancel(id);
                                    }

                                    // 如果焦点在工具栏，处理键盘输入
                                    if self.focus_area == ui::FocusArea::Toolbar {
                                        ui::Toolbar::handle_keyboard(
                                            ui,
                                            &mut self.toolbar_index,
                                            &mut toolbar_actions,
                                        );

                                        // 显示焦点提示
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("工具栏焦点")
                                                    .small()
                                                    .color(self.highlight_colors.keyword),
                                            );
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    " {}:移动  {}:Tab栏  {}:选择",
                                                    ui::local_shortcuts_text(&[
                                                        ui::LocalShortcut::ToolbarPrev,
                                                        ui::LocalShortcut::ToolbarNext,
                                                    ]),
                                                    ui::local_shortcut_text(
                                                        ui::LocalShortcut::ToolbarToQueryTabs,
                                                    ),
                                                    ui::local_shortcut_text(
                                                        ui::LocalShortcut::ToolbarActivate,
                                                    ),
                                                ))
                                                    .small()
                                                    .color(egui::Color32::GRAY),
                                            );
                                        });

                                        // 处理焦点转移
                                        if let Some(transfer) = toolbar_actions.focus_transfer {
                                            match transfer {
                                                ui::ToolbarFocusTransfer::ToQueryTabs => {
                                                    self.set_focus_area(ui::FocusArea::QueryTabs);
                                                }
                                            }
                                        }
                                    }

                                    ui.separator();

                                    // Tab 栏（多查询窗口）
                                    let mut tab_clicked = false;
                                    let mut tab_actions = ui
                                        .scope(|ui| {
                                            let actions = ui::QueryTabBar::show(
                                                ui,
                                                &self.tab_manager.tabs,
                                                self.tab_manager.active_index,
                                                &self.highlight_colors,
                                                &self.keybindings,
                                            );
                                            if ui.ui_contains_pointer()
                                                && ui.input(|input| input.pointer.primary_clicked())
                                            {
                                                tab_clicked = true;
                                            }
                                            actions
                                        })
                                        .inner;

                                    if tab_clicked {
                                        self.set_focus_area(ui::FocusArea::QueryTabs);
                                    }

                                    // 如果焦点在Tab栏，处理键盘输入
                                    if self.focus_area == ui::FocusArea::QueryTabs {
                                        ui::QueryTabBar::handle_keyboard(
                                            ui,
                                            self.tab_manager.tabs.len(),
                                            self.tab_manager.active_index,
                                            &mut tab_actions,
                                        );

                                        // 显示焦点提示
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new("TAB焦点")
                                                    .small()
                                                    .color(self.highlight_colors.keyword),
                                            );
                                            ui.label(
                                                egui::RichText::new(
                                                    format!(
                                                        " {}:切换  {}:表格  {}:工具栏  {}:删除",
                                                        ui::local_shortcuts_text(&[
                                                            ui::LocalShortcut::QueryTabPrev,
                                                            ui::LocalShortcut::QueryTabNext,
                                                        ]),
                                                        ui::local_shortcut_text(
                                                            ui::LocalShortcut::QueryTabToDataGrid,
                                                        ),
                                                        ui::local_shortcut_text(
                                                            ui::LocalShortcut::QueryTabToToolbar,
                                                        ),
                                                        ui::local_shortcut_text(
                                                            ui::LocalShortcut::QueryTabClose,
                                                        ),
                                                    ),
                                                )
                                                .small()
                                                .color(egui::Color32::GRAY),
                                            );
                                        });
                                    }

                                    // 处理Tab栏焦点转移
                                    if let Some(transfer) = tab_actions.focus_transfer {
                                        match transfer {
                                            ui::TabBarFocusTransfer::ToToolbar => {
                                                self.set_focus_area(ui::FocusArea::Toolbar);
                                            }
                                            ui::TabBarFocusTransfer::ToDataGrid => {
                                                self.set_focus_area(ui::FocusArea::DataGrid);
                                            }
                                        }
                                    }

                                    self.handle_tab_actions(&ctx, tab_actions);

                                    ui.separator();

                                    // 计算数据表格和 SQL 编辑器的高度分配
                                    let total_content_height = ui.available_height();
                                    let sql_editor_height = if self.show_sql_editor {
                                        clamped_sql_editor_height(
                                            self.sql_editor_height,
                                            total_content_height,
                                        )
                                    } else {
                                        0.0
                                    };
                                    let divider_height =
                                        if self.show_sql_editor { 6.0 } else { 0.0 };
                                    let data_grid_height =
                                        (total_content_height - sql_editor_height - divider_height)
                                            .max(0.0);

                                    // 数据表格区域（支持左右分割显示 ER 图）
                                    ui.allocate_ui_with_layout(
                                        egui::vec2(ui.available_width(), data_grid_height),
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| {
                                            let active_query_error = self
                                                .tab_manager
                                                .get_active()
                                                .and_then(|tab| tab.last_error.clone());
                                            let workspace_surface =
                                                classify_workspace_surface(
                                                    self.result.as_ref(),
                                                    active_query_error.as_deref(),
                                                );

                                            if self.show_er_diagram {
                                                // 左右分割布局 - 使用 horizontal 和固定宽度的子区域
                                                let available_width = ui.available_width();
                                                let available_height = data_grid_height;
                                                let divider_width = 8.0;
                                                let left_width = (available_width - divider_width)
                                                    * self.central_panel_ratio;
                                                let right_width =
                                                    available_width - left_width - divider_width;
                                                let theme_preset = self.theme_manager.current;

                                                ui.horizontal(|ui| {
                                                    // 左侧：数据表格
                                                    ui.allocate_ui_with_layout(
                                                        egui::vec2(left_width, available_height),
                                                        egui::Layout::top_down(egui::Align::LEFT),
                                                        |ui| {
                                                            ui.set_min_size(egui::vec2(
                                                                left_width,
                                                                available_height,
                                                            ));

                                                            if workspace_surface
                                                                == WorkspaceSurface::TabularResult
                                                            {
                                                                if let Some(result) = &self.result {
                                                                    self.grid_state.focused = self
                                                                        .focus_area
                                                                        == ui::FocusArea::DataGrid
                                                                        && !self
                                                                            .has_modal_dialog_open(
                                                                            );

                                                                    let table_name = self
                                                                        .selected_table
                                                                        .as_deref();
                                                                    let (grid_actions, _) =
                                                                        ui::DataGrid::show_editable(
                                                                            ui,
                                                                            result,
                                                                            &self.search_text,
                                                                            &self.search_column,
                                                                            &mut self.selected_row,
                                                                            &mut self.selected_cell,
                                                                            &mut self.grid_state,
                                                                            table_name,
                                                                            &self.keybindings,
                                                                        );

                                                                    if grid_actions
                                                                        .open_filter_panel
                                                                    {
                                                                        self.dispatch_app_action(
                                                                            &ctx,
                                                                            AppAction::OpenFilterWorkspace,
                                                                        );
                                                                    }
                                                                }
                                                            } else if workspace_surface
                                                                == WorkspaceSurface::QueryError
                                                            {
                                                                Self::render_query_error_surface(
                                                                    ui,
                                                                    active_query_error
                                                                        .as_deref()
                                                                        .unwrap_or("查询失败"),
                                                                );
                                                            } else {
                                                                welcome_action = ui::Welcome::show(
                                                                    ui,
                                                                    self.welcome_status,
                                                                    &self.keybindings,
                                                                );
                                                            }
                                                        },
                                                    );

                                                    // 可拖动的垂直分割条
                                                    let (divider_rect, divider_response) = ui
                                                        .allocate_exact_size(
                                                            egui::vec2(
                                                                divider_width,
                                                                available_height,
                                                            ),
                                                            egui::Sense::drag(),
                                                        );

                                                    // 绘制分割条
                                                    let divider_color = if divider_response
                                                        .dragged()
                                                        || divider_response.hovered()
                                                    {
                                                        egui::Color32::from_rgb(100, 150, 255)
                                                    } else {
                                                        egui::Color32::from_rgba_unmultiplied(
                                                            128, 128, 128, 80,
                                                        )
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

                                                    // 处理拖动
                                                    if divider_response.dragged() {
                                                        let delta = divider_response.drag_delta().x;
                                                        let delta_ratio = delta / available_width;
                                                        self.central_panel_ratio = (self
                                                            .central_panel_ratio
                                                            + delta_ratio)
                                                            .clamp(0.2, 0.8);
                                                    }

                                                    // 鼠标光标
                                                    if divider_response.hovered()
                                                        || divider_response.dragged()
                                                    {
                                                        ui.ctx().set_cursor_icon(
                                                            egui::CursorIcon::ResizeHorizontal,
                                                        );
                                                    }

                                                    // 右侧：ER 关系图
                                                    ui.allocate_ui_with_layout(
                                                        egui::vec2(right_width, available_height),
                                                        egui::Layout::top_down(egui::Align::LEFT),
                                                        |ui| {
                                                            ui.set_min_size(egui::vec2(
                                                                right_width,
                                                                available_height,
                                                            ));

                                                            let er_is_focused = self.focus_area
                                                                == ui::FocusArea::ErDiagram
                                                                && !self.has_modal_dialog_open();
                                                            let er_response = self
                                                                .er_diagram_state
                                                                .show(
                                                                    ui,
                                                                    &theme_preset,
                                                                    er_is_focused,
                                                                );

                                                            for action in collect_er_diagram_surface_actions(&er_response) {
                                                                match action {
                                                                    ErDiagramSurfaceAction::FocusDiagram => {
                                                                        self.set_focus_area(
                                                                            ui::FocusArea::ErDiagram,
                                                                        );
                                                                    }
                                                                    ErDiagramSurfaceAction::RefreshData => {
                                                                        self.load_er_diagram_data();
                                                                    }
                                                                    ErDiagramSurfaceAction::Relayout => {
                                                                        let summary = ui::analyze_er_graph(
                                                                            &self.er_diagram_state.tables,
                                                                            &self.er_diagram_state.relationships,
                                                                        );
                                                                        ui::apply_er_layout_strategy(
                                                                            &mut self.er_diagram_state.tables,
                                                                            &self.er_diagram_state.relationships,
                                                                            summary.strategy,
                                                                        );
                                                                    }
                                                                    ErDiagramSurfaceAction::FitView => {
                                                                        self.er_diagram_state
                                                                            .fit_to_view(
                                                                                ui.available_size(),
                                                                            );
                                                                    }
                                                                }
                                                            }
                                                        },
                                                    );
                                                });
                                            } else if workspace_surface
                                                == WorkspaceSurface::TabularResult
                                            {
                                                if let Some(result) = &self.result {
                                                    // 同步焦点状态：只有当全局焦点在 DataGrid 且没有对话框打开时才响应键盘
                                                    self.grid_state.focused = self.focus_area
                                                        == ui::FocusArea::DataGrid
                                                        && !self.has_modal_dialog_open();

                                                    let table_name = self.selected_table.as_deref();
                                                    let (grid_actions, _) =
                                                        ui::DataGrid::show_editable(
                                                            ui,
                                                            result,
                                                            &self.search_text,
                                                            &self.search_column,
                                                            &mut self.selected_row,
                                                            &mut self.selected_cell,
                                                            &mut self.grid_state,
                                                            table_name,
                                                            &self.keybindings,
                                                        );

                                                    if let Some(msg) = grid_actions.message {
                                                        self.notifications.info(msg);
                                                    }

                                                    if !grid_actions.sql_to_execute.is_empty() {
                                                        let _ = self.execute_grid_save(
                                                            grid_actions.sql_to_execute,
                                                        );
                                                    }

                                                    if grid_actions.refresh_requested {
                                                        self.dispatch_app_action(
                                                            &ctx,
                                                            AppAction::RefreshSelectedTable,
                                                        );
                                                    }

                                                    if let Some(transfer) =
                                                        grid_actions.focus_transfer
                                                    {
                                                        match transfer {
                                                            ui::FocusTransfer::Sidebar => {
                                                                self.show_sidebar = true;
                                                                self.set_focus_area(
                                                                    ui::FocusArea::Sidebar,
                                                                );
                                                            }
                                                            ui::FocusTransfer::SqlEditor => {
                                                                self.show_sql_editor = true;
                                                                self.set_focus_area(
                                                                    ui::FocusArea::SqlEditor,
                                                                );
                                                            }
                                                            ui::FocusTransfer::QueryTabs => {
                                                                self.set_focus_area(
                                                                    ui::FocusArea::QueryTabs,
                                                                );
                                                            }
                                                        }
                                                    }

                                                    if grid_actions.request_focus {
                                                        self.set_focus_area(
                                                            ui::FocusArea::DataGrid,
                                                        );
                                                    }

                                                    if grid_actions.open_filter_panel {
                                                        self.dispatch_app_action(
                                                            &ctx,
                                                            AppAction::OpenFilterWorkspace,
                                                        );
                                                    }

                                                    if let Some(tab_idx) =
                                                        grid_actions.switch_to_tab
                                                    {
                                                        self.dispatch_app_action(
                                                            &ctx,
                                                            AppAction::SwitchToQueryTab(tab_idx),
                                                        );
                                                    }
                                                }
                                            } else if workspace_surface
                                                == WorkspaceSurface::QueryError
                                            {
                                                Self::render_query_error_surface(
                                                    ui,
                                                    active_query_error
                                                        .as_deref()
                                                        .unwrap_or("查询失败"),
                                                );
                                            } else {
                                                welcome_action = ui::Welcome::show(
                                                    ui,
                                                    self.welcome_status,
                                                    &self.keybindings,
                                                );
                                            }
                                        },
                                    ); // allocate_ui_with_layout 数据表格区域结束

                                    // ===== SQL 编辑器 =====
                                    sql_editor_actions =
                                        self.render_sql_editor_in_ui(ui, total_content_height);
                                }); // Frame 闭包结束
                        },
                    ); // allocate_ui_with_layout 主内容区结束
                }); // horizontal 布局结束
            }); // CentralPanel 闭包结束

        // ===== 处理各种操作 =====
        self.handle_toolbar_actions(&ctx, toolbar_actions);
        self.handle_sidebar_actions(&ctx, sidebar_actions);
        self.handle_sql_editor_actions(sql_editor_actions);
        if let Some(action) = welcome_action {
            self.handle_welcome_action(&ctx, action);
        }
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
        ui::NotificationToast::show(&ctx, &self.notifications);

        // 持续刷新（有活动任务或有通知时需要刷新）
        if self.connecting || self.executing || !self.notifications.is_empty() {
            ctx.request_repaint();
        }
    }

    /// 渲染 SQL 编辑器面板（在主内容区内部渲染，不遮挡侧边栏）
    pub(in crate::app) fn render_sql_editor_in_ui(
        &mut self,
        ui: &mut egui::Ui,
        available_height: f32,
    ) -> SqlEditorActions {
        let mut sql_editor_actions = SqlEditorActions::default();

        if !self.show_sql_editor {
            return sql_editor_actions;
        }

        // 只有在没有对话框打开时，SQL 编辑器才响应快捷键
        let is_editor_focused =
            self.focus_area == ui::FocusArea::SqlEditor && !self.has_modal_dialog_open();

        // 计算编辑器高度（使用 sql_editor_height 字段或默认值）
        let editor_height = clamped_sql_editor_height(self.sql_editor_height, available_height);

        // 可拖动的水平分割条
        let divider_height = 6.0;
        let (divider_rect, divider_response) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), divider_height),
            egui::Sense::drag(),
        );

        // 绘制分割条
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

        // 中间的拖动指示器（三个小点水平排列）
        let center = divider_rect.center();
        for offset in [-15.0, 0.0, 15.0] {
            ui.painter().circle_filled(
                egui::pos2(center.x + offset, center.y),
                2.0,
                egui::Color32::from_gray(160),
            );
        }

        // 处理拖动调整高度
        if divider_response.dragged() {
            let delta = -divider_response.drag_delta().y; // 向上拖动增加高度
            self.sql_editor_height = (self.sql_editor_height + delta).clamp(100.0, 500.0);
        }

        // 鼠标光标
        if divider_response.hovered() || divider_response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }

        // SQL 编辑器内容区域
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), editor_height),
            egui::Layout::top_down(egui::Align::LEFT),
            |ui| {
                let active_tab_message = self
                    .tab_manager
                    .get_active()
                    .and_then(|tab| tab.last_message.as_deref());
                let latest_msg = select_sql_editor_status_message(
                    active_tab_message,
                    self.notifications.latest_message(),
                );
                let mut request_editor_widget_focus =
                    is_editor_focused && self.editor_mode == ui::EditorMode::Insert;
                sql_editor_actions = ui::SqlEditor::show(
                    ui,
                    &mut self.sql,
                    &self.command_history,
                    &mut self.history_index,
                    self.executing,
                    &latest_msg,
                    &self.highlight_colors,
                    self.last_query_time_ms,
                    &self.autocomplete,
                    &mut self.show_autocomplete,
                    &mut self.selected_completion,
                    &mut request_editor_widget_focus,
                    is_editor_focused,
                    &mut self.editor_mode,
                );
            },
        );

        sql_editor_actions
    }

    /// 处理 SQL 编辑器操作
    pub(in crate::app) fn handle_sql_editor_actions(&mut self, actions: SqlEditorActions) {
        // 执行查询
        if actions.execute && !self.sql.is_empty() {
            let sql = self.sql.clone();
            let _ = self.execute(sql);
        }

        // EXPLAIN 分析
        if actions.explain && !self.sql.is_empty() {
            let sql = self.sql.trim();
            let explain_sql = if self.is_mysql() {
                format!("EXPLAIN FORMAT=TRADITIONAL {}", sql)
            } else if self
                .manager
                .get_active()
                .map(|c| c.config.db_type == crate::database::DatabaseType::PostgreSQL)
                .unwrap_or(false)
            {
                format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {}", sql)
            } else {
                format!("EXPLAIN QUERY PLAN {}", sql)
            };
            let _ = self.execute(explain_sql);
            self.notifications.info("正在分析执行计划...");
        }

        // 格式化
        if actions.format {
            self.sql = format_sql(&self.sql);
        }

        // 清空
        if actions.clear {
            self.sql.clear();
            if let Some(tab) = self.tab_manager.get_active_mut() {
                tab.sql.clear();
                tab.modified = false;
                tab.update_title();
            }
            self.notifications.dismiss_all();
            self.last_query_time_ms = None;
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
            self.sync_sql_to_active_tab();
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
            && self.manager.active.as_deref() != Some(&conn_name)
        {
            self.connect(conn_name);
            self.switch_grid_workspace(None);
            self.result = None;
            self.autocomplete.clear();
            self.sidebar_panel_state.clear_triggers();
            self.sidebar_panel_state.clear_routines();
            self.sidebar_panel_state.loading_triggers = false;
            self.sidebar_panel_state.loading_routines = false;
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
            self.set_ui_scale(ctx, self.ui_scale + 0.1);
        }
        if actions.zoom_out {
            self.set_ui_scale(ctx, self.ui_scale - 0.1);
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
    }

    /// 处理创建用户操作
    pub(in crate::app) fn handle_create_user_action(&mut self) {
        if let Some(conn) = self.manager.get_active() {
            let db_type = conn.config.db_type;
            if matches!(db_type, crate::database::DatabaseType::SQLite) {
                self.notifications.warning("SQLite 不支持用户管理");
            } else {
                let databases = conn.databases.clone();
                self.create_user_dialog_state.open(db_type, databases);
                self.mark_dialog_owner(DialogId::CreateUser);
            }
        } else {
            self.notifications.warning("请先连接数据库");
        }
    }

    /// 处理侧边栏操作
    pub(in crate::app) fn handle_sidebar_actions(
        &mut self,
        ctx: &egui::Context,
        actions: ui::SidebarActions,
    ) {
        if actions.filter_changed {
            self.grid_state.filter_cache.invalidate();
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
            self.sidebar_section = new_section;
        }

        // 连接操作
        if let Some(name) = actions.connect {
            self.connect(name);
            // 连接后自动切换到数据库列表
            self.sidebar_section = ui::SidebarSection::Databases;
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
                    self.notifications.info("当前区域暂不支持重命名");
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
            self.pending_delete_target = Some(target);
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
            self.sql = definition;
            self.show_sql_editor = true;
            self.set_focus_area(ui::FocusArea::SqlEditor);
            self.notifications.info("触发器定义已加载到编辑器");
        }

        // 存储过程/函数定义
        if let Some(definition) = actions.show_routine_definition {
            self.sql = definition;
            self.show_sql_editor = true;
            self.set_focus_area(ui::FocusArea::SqlEditor);
            self.notifications.info("存储过程/函数定义已加载到编辑器");
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
        let active_name = self.manager.active.clone();
        let (db_type, selected_database) = self
            .manager
            .get_active()
            .map(|conn| (conn.config.db_type, conn.selected_database.clone()))
            .unwrap_or((crate::database::DatabaseType::SQLite, None));

        match self.sidebar_section {
            ui::SidebarSection::Connections | ui::SidebarSection::Databases => {
                if let Some(name) = active_name {
                    self.connect(name);
                } else {
                    self.notifications.info("当前没有活动连接可刷新");
                }
            }
            ui::SidebarSection::Tables => match db_type {
                crate::database::DatabaseType::SQLite => {
                    if let Some(name) = active_name {
                        self.connect(name);
                    } else {
                        self.notifications.info("当前没有活动连接可刷新");
                    }
                }
                crate::database::DatabaseType::PostgreSQL
                | crate::database::DatabaseType::MySQL => {
                    if let Some(db) = selected_database {
                        self.select_database(db);
                    } else {
                        self.notifications.info("请先选择数据库");
                    }
                }
            },
            ui::SidebarSection::Triggers => self.load_triggers(),
            ui::SidebarSection::Routines => self.load_routines(),
            ui::SidebarSection::Filters => {
                self.grid_state.filter_cache.invalidate();
            }
        }
    }

    /// 侧边栏添加筛选条件
    pub(in crate::app) fn add_sidebar_filter(&mut self) {
        self.insert_sidebar_filter(ui::SidebarFilterInsertMode::AppendEnd);
    }

    fn insert_sidebar_filter(&mut self, mode: ui::SidebarFilterInsertMode) {
        let Some(result) = &self.result else {
            self.notifications
                .warning("当前结果集为空，无法添加筛选条件");
            return;
        };
        let Some(default_col) = result
            .columns
            .get(self.selected_cell.map(|(_, col)| col).unwrap_or(0))
            .cloned()
            .or_else(|| result.columns.first().cloned())
        else {
            self.notifications.warning("当前结果集没有可筛选的列");
            return;
        };

        let insert_index = match mode {
            ui::SidebarFilterInsertMode::BelowSelection => {
                if self.grid_state.filters.is_empty() {
                    0
                } else {
                    (self.sidebar_panel_state.selection.filters + 1)
                        .min(self.grid_state.filters.len())
                }
            }
            ui::SidebarFilterInsertMode::AppendEnd => self.grid_state.filters.len(),
        };

        self.grid_state
            .filters
            .insert(insert_index, ui::ColumnFilter::new(default_col));
        self.sidebar_panel_state.selection.filters = insert_index;
        self.grid_state.filter_cache.invalidate();

        self.show_sidebar = true;
        self.sidebar_panel_state.show_filters = true;
        self.sidebar_section = ui::SidebarSection::Filters;
        self.set_focus_area(ui::FocusArea::Sidebar);
        self.sidebar_panel_state.workflow.filter_workspace = ui::SidebarFilterWorkspaceMode::Input;
        self.pending_filter_input_focus = Some(insert_index);
    }

    /// 侧边栏清空筛选条件
    pub(in crate::app) fn clear_sidebar_filters(&mut self) {
        if self.grid_state.filters.is_empty() {
            return;
        }
        self.grid_state.filters.clear();
        self.sidebar_panel_state.selection.filters = 0;
        self.sidebar_panel_state.exit_filter_input();
        self.pending_filter_input_focus = None;
        self.grid_state.filter_cache.invalidate();
    }

    /// 切换筛选条件逻辑（AND/OR）
    fn toggle_sidebar_filter_logic(&mut self, index: usize) {
        if let Some(filter) = self.grid_state.filters.get_mut(index) {
            filter.logic.toggle();
            self.grid_state.filter_cache.invalidate();
        }
    }

    /// 循环切换筛选列
    fn cycle_sidebar_filter_column(&mut self, index: usize, forward: bool) {
        let Some(columns) = self.result.as_ref().map(|r| r.columns.clone()) else {
            return;
        };
        if columns.is_empty() {
            return;
        }
        let Some(filter) = self.grid_state.filters.get_mut(index) else {
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
            self.grid_state.filter_cache.invalidate();
        }
    }

    /// 聚焦筛选输入框（i）
    fn focus_sidebar_filter_input(&mut self, index: usize) {
        let Some(filter) = self.grid_state.filters.get_mut(index) else {
            return;
        };
        if !filter.operator.needs_value() {
            self.notifications.info("当前筛选操作符不需要输入值");
            return;
        }
        filter.enabled = true;
        self.sidebar_panel_state.selection.filters = index;
        self.pending_filter_input_focus = Some(index);
        self.sidebar_panel_state.workflow.filter_workspace = ui::SidebarFilterWorkspaceMode::Input;
        self.show_sidebar = true;
        self.sidebar_panel_state.show_filters = true;
        self.sidebar_section = ui::SidebarSection::Filters;
        self.set_focus_area(ui::FocusArea::Sidebar);
    }

    /// 为表重命名生成 SQL 模板
    fn prepare_table_rename_sql(&mut self, table: &str) {
        let Some(conn) = self.manager.get_active() else {
            self.notifications.warning("请先连接数据库");
            return;
        };

        let db_type = conn.config.db_type;
        let use_backticks = matches!(db_type, crate::database::DatabaseType::MySQL);
        let quoted_old = match ui::quote_identifier(table, use_backticks) {
            Ok(name) => name,
            Err(e) => {
                self.notifications.error(format!("表名无效: {}", e));
                return;
            }
        };
        let quoted_new = match ui::quote_identifier("new_table_name", use_backticks) {
            Ok(name) => name,
            Err(e) => {
                self.notifications
                    .error(format!("目标表名模板生成失败: {}", e));
                return;
            }
        };

        self.sql = match db_type {
            crate::database::DatabaseType::MySQL => {
                format!("RENAME TABLE {} TO {};", quoted_old, quoted_new)
            }
            crate::database::DatabaseType::PostgreSQL | crate::database::DatabaseType::SQLite => {
                format!("ALTER TABLE {} RENAME TO {};", quoted_old, quoted_new)
            }
        };
        self.show_sql_editor = true;
        self.set_focus_area(ui::FocusArea::SqlEditor);
        self.notifications
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
            let closing_tab_id = self.tab_manager.tabs.get(idx).map(|tab| tab.id.clone());
            if self.tab_manager.tabs.len() > 1
                && let Some(request_id) = self
                    .tab_manager
                    .tabs
                    .get(idx)
                    .and_then(|tab| tab.pending_request_id)
            {
                self.cancel_query_request_silently(request_id);
            }
            self.tab_manager.close_tab(idx);
            if let Some(tab_id) = closing_tab_id {
                self.remove_grid_workspaces_for_tab(&tab_id);
            }
            self.sync_from_active_tab();
        }

        if tab_actions.close_others {
            let active_index = self.tab_manager.active_index;
            let request_ids: Vec<u64> = self
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
            self.tab_manager.close_other_tabs();
            for tab_id in closing_tab_ids {
                self.remove_grid_workspaces_for_tab(&tab_id);
            }
            self.sync_from_active_tab();
        }

        if tab_actions.close_right {
            let active_index = self.tab_manager.active_index;
            let request_ids: Vec<u64> = self
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
            self.tab_manager.close_tabs_to_right();
            for tab_id in closing_tab_ids {
                self.remove_grid_workspaces_for_tab(&tab_id);
            }
            self.sync_from_active_tab();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErDiagramSurfaceAction, WorkspaceSurface, clamped_sql_editor_height,
        classify_workspace_surface, collect_er_diagram_surface_actions,
        select_sql_editor_status_message,
    };
    use crate::app::DbManagerApp;
    use crate::app::dialogs::host::DialogId;
    use crate::database::{Connection, ConnectionConfig, DatabaseType, QueryResult};
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
        app.manager
            .connections
            .insert("demo".to_string(), connection);
        app.manager.active = Some("demo".to_string());
    }

    #[test]
    fn workspace_surface_requires_columns_for_tabular_view() {
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
            WorkspaceSurface::Welcome
        );
        assert_eq!(
            classify_workspace_surface(
                Some(&QueryResult::with_rows(vec!["id".to_string()], Vec::new())),
                None,
            ),
            WorkspaceSurface::TabularResult
        );
    }

    #[test]
    fn workspace_surface_prioritizes_explicit_query_error() {
        assert_eq!(
            classify_workspace_surface(
                Some(&QueryResult::with_rows(vec!["id".to_string()], Vec::new())),
                Some("错误: syntax error"),
            ),
            WorkspaceSurface::QueryError
        );
        assert_eq!(
            classify_workspace_surface(None, Some("错误: syntax error")),
            WorkspaceSurface::QueryError
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

        assert_eq!(app.pending_delete_target, Some(target));
        assert!(app.show_delete_confirm);
        assert_eq!(app.active_dialog_id(), Some(DialogId::DeleteConfirm));
    }

    #[test]
    fn ctrl_r_opening_er_moves_focus_off_data_grid_before_grid_can_consume_j() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        app.selected_table = Some("customers".to_string());
        app.set_focus_area(FocusArea::DataGrid);
        app.grid_state.cursor = (0, 0);
        app.grid_state.focused = true;

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

        assert!(app.show_er_diagram);
        assert_eq!(app.focus_area, FocusArea::ErDiagram);
        assert!(!app.grid_state.focused);
        assert_eq!(
            app.er_diagram_state.selected_table_name(),
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

        assert_eq!(app.focus_area, FocusArea::ErDiagram);
        assert!(!app.grid_state.focused);
        assert_eq!(app.grid_state.cursor, (0, 0));
        assert_eq!(app.er_diagram_state.selected_table_name(), Some("orders"));
    }

    #[test]
    fn toolbar_toggle_er_diagram_action_focuses_er_from_data_grid() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        app.selected_table = Some("customers".to_string());
        app.set_focus_area(FocusArea::DataGrid);

        let actions = ToolbarActions {
            toggle_er_diagram: true,
            ..Default::default()
        };
        app.handle_toolbar_actions(&ctx, actions);

        assert!(app.show_er_diagram);
        assert_eq!(app.focus_area, FocusArea::ErDiagram);
        assert!(!app.grid_state.focused);
        assert_eq!(
            app.er_diagram_state.selected_table_name(),
            Some("customers")
        );
    }

    #[test]
    fn er_diagram_v_shortcut_toggles_viewport_mode_once_per_frame() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.show_er_diagram = true;
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
        assert!(app.er_diagram_state.is_viewport_mode());

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
        assert!(!app.er_diagram_state.is_viewport_mode());
    }
}
