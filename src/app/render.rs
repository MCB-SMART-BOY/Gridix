//! UI 渲染模块
//!
//! 将 `update()` 中的渲染逻辑拆分到此模块，提高代码可维护性。

use eframe::egui;

use crate::core::{constants, format_sql};
use crate::ui::{self, SqlEditorActions, TabBarActions, ToolbarActions};

use super::DbManagerApp;

impl DbManagerApp {
    /// 渲染 SQL 编辑器面板（在主内容区内部渲染，不遮挡侧边栏）
    pub(super) fn render_sql_editor_in_ui(
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
        let editor_height = self.sql_editor_height.clamp(100.0, available_height * 0.6);

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
                // 获取最新通知消息用于状态栏显示
                let latest_msg = self.notifications.latest_message().map(|s| s.to_string());
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
                    &mut self.focus_sql_editor,
                    is_editor_focused,
                    &mut self.editor_mode,
                );
            },
        );

        sql_editor_actions
    }

    /// 处理 SQL 编辑器操作
    pub(super) fn handle_sql_editor_actions(&mut self, actions: SqlEditorActions) {
        // 执行查询
        if actions.execute && !self.sql.is_empty() {
            let sql = self.sql.clone();
            let _ = self.execute(sql);
            self.sql.clear();
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
            self.notifications.dismiss_all();
            self.last_query_time_ms = None;
        }

        // 焦点转移到表格
        if actions.focus_to_grid {
            self.focus_area = ui::FocusArea::DataGrid;
            self.grid_state.focused = true;
        }

        // 编辑器请求焦点
        if actions.request_focus && self.focus_area != ui::FocusArea::SqlEditor {
            self.focus_area = ui::FocusArea::SqlEditor;
            self.grid_state.focused = false;
            self.focus_sql_editor = true;
        }
    }

    /// 处理工具栏操作
    pub(super) fn handle_toolbar_actions(&mut self, ctx: &egui::Context, actions: ToolbarActions) {
        if actions.toggle_sidebar {
            self.show_sidebar = !self.show_sidebar;
        }

        if actions.toggle_editor {
            self.show_sql_editor = !self.show_sql_editor;
        }

        if actions.refresh_tables {
            if let Some(name) = self.manager.active.clone() {
                self.connect(name);
            }
        }

        // 连接切换
        if let Some(conn_name) = actions.switch_connection {
            if self.manager.active.as_deref() != Some(&conn_name) {
                self.connect(conn_name);
                self.selected_table = None;
                self.result = None;
                self.autocomplete.clear();
                self.sidebar_panel_state.clear_triggers();
                self.sidebar_panel_state.clear_routines();
                self.sidebar_panel_state.loading_triggers = false;
                self.sidebar_panel_state.loading_routines = false;
            }
        }

        // 数据库切换
        if let Some(db_name) = actions.switch_database {
            self.select_database(db_name);
        }

        // 表切换
        if let Some(table_name) = actions.switch_table {
            self.selected_table = Some(table_name.clone());
            self.grid_state.primary_key_column = None;
            if let Ok(quoted_table) = ui::quote_identifier(&table_name, self.is_mysql()) {
                let query_sql = format!(
                    "SELECT * FROM {} LIMIT {};",
                    quoted_table,
                    constants::database::DEFAULT_QUERY_LIMIT
                );
                let _ = self.execute(query_sql);
            }
            self.fetch_primary_key(&table_name);
            self.sql.clear();
        }

        if actions.export {
            self.show_export_dialog = true;
            self.export_status = None;
        }

        if actions.import {
            self.handle_import();
        }

        if actions.create_table {
            let db_type = self
                .manager
                .get_active()
                .map(|c| c.config.db_type)
                .unwrap_or_default();
            self.ddl_dialog_state.open_create_table(db_type);
        }

        if actions.create_database {
            let db_type = self
                .manager
                .get_active()
                .map(|c| c.config.db_type)
                .unwrap_or_default();
            self.create_db_dialog_state.open(db_type);
        }

        if actions.create_user {
            self.handle_create_user_action();
        }

        if actions.toggle_er_diagram {
            self.show_er_diagram = !self.show_er_diagram;
            if self.show_er_diagram {
                self.load_er_diagram_data();
                self.notifications.info("ER 关系图已打开");
            } else {
                self.notifications.info("ER 关系图已关闭");
            }
        }

        if let Some(preset) = actions.theme_changed {
            if self.app_config.is_dark_mode {
                self.app_config.dark_theme = preset;
            } else {
                self.app_config.light_theme = preset;
            }
            self.set_theme(ctx, preset);
        }

        // 处理日/夜模式切换（来自工具栏按钮或 Ctrl+D 快捷键）
        if actions.toggle_dark_mode || self.pending_toggle_dark_mode {
            self.pending_toggle_dark_mode = false;
            self.app_config.is_dark_mode = !self.app_config.is_dark_mode;
            let new_theme = if self.app_config.is_dark_mode {
                self.app_config.dark_theme
            } else {
                self.app_config.light_theme
            };
            self.set_theme(ctx, new_theme);
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
            self.show_history_panel = true;
        }

        if actions.show_help {
            self.show_help = true;
        }

        if actions.show_about {
            self.show_about = true;
        }

        if actions.show_keybindings {
            self.keybindings_dialog_state.open(&self.keybindings);
        }
    }

    /// 处理创建用户操作
    fn handle_create_user_action(&mut self) {
        if let Some(conn) = self.manager.get_active() {
            let db_type = conn.config.db_type;
            if matches!(db_type, crate::database::DatabaseType::SQLite) {
                self.notifications.warning("SQLite 不支持用户管理");
            } else {
                let databases = conn.databases.clone();
                self.create_user_dialog_state.open(db_type, databases);
            }
        } else {
            self.notifications.warning("请先连接数据库");
        }
    }

    /// 处理侧边栏操作
    pub(super) fn handle_sidebar_actions(&mut self, actions: ui::SidebarActions) {
        if actions.filter_changed {
            self.grid_state.filter_cache.invalidate();
        }

        // 焦点转移
        if let Some(transfer) = actions.focus_transfer {
            match transfer {
                ui::SidebarFocusTransfer::ToDataGrid => {
                    self.focus_area = ui::FocusArea::DataGrid;
                    self.grid_state.focused = true;
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
        if let Some(name) = actions.delete {
            self.pending_delete_name = Some(name);
            self.show_delete_confirm = true;
        }

        // 查看表结构
        if let Some(table) = actions.show_table_schema {
            self.handle_show_table_schema(table);
        }

        // 查询表数据
        if let Some(table) = actions.query_table {
            self.handle_query_table(table);
        }

        if actions.add_filter {
            self.add_sidebar_filter();
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
            self.focus_sql_editor = true;
            self.notifications.info("触发器定义已加载到编辑器");
        }

        // 存储过程/函数定义
        if let Some(definition) = actions.show_routine_definition {
            self.sql = definition;
            self.show_sql_editor = true;
            self.focus_sql_editor = true;
            self.notifications.info("存储过程/函数定义已加载到编辑器");
        }
    }

    /// 处理查看表结构
    fn handle_show_table_schema(&mut self, table: String) {
        self.selected_table = Some(table.clone());
        if let Some(conn) = self.manager.get_active() {
            let schema_sql = match conn.config.db_type {
                crate::database::DatabaseType::SQLite => {
                    let escaped = table.replace('\'', "''");
                    format!("PRAGMA table_info('{}');", escaped)
                }
                crate::database::DatabaseType::PostgreSQL => {
                    let escaped = table.replace('\'', "''");
                    format!(
                        "SELECT column_name, data_type, is_nullable, column_default \
                         FROM information_schema.columns \
                         WHERE table_name = '{}' \
                         ORDER BY ordinal_position;",
                        escaped
                    )
                }
                crate::database::DatabaseType::MySQL => {
                    let escaped = table.replace('`', "``").replace('.', "_");
                    format!("DESCRIBE `{}`;", escaped)
                }
            };
            let _ = self.execute(schema_sql);
            self.sql.clear();
        }
    }

    /// 处理查询表数据
    fn handle_query_table(&mut self, table: String) {
        self.selected_table = Some(table.clone());
        self.grid_state.primary_key_column = None;
        if let Ok(quoted_table) = ui::quote_identifier(&table, self.is_mysql()) {
            let query_sql = format!(
                "SELECT * FROM {} LIMIT {};",
                quoted_table,
                constants::database::DEFAULT_QUERY_LIMIT
            );
            let _ = self.execute(query_sql);
        }
        self.fetch_primary_key(&table);
        self.sql.clear();
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
    fn add_sidebar_filter(&mut self) {
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

        self.grid_state
            .filters
            .push(ui::ColumnFilter::new(default_col));
        let new_index = self.grid_state.filters.len().saturating_sub(1);
        self.sidebar_panel_state.selection.filters = new_index;
        self.grid_state.filter_cache.invalidate();

        self.show_sidebar = true;
        self.sidebar_panel_state.show_filters = true;
        self.sidebar_section = ui::SidebarSection::Filters;
        self.focus_area = ui::FocusArea::Sidebar;
        self.pending_filter_input_focus = Some(new_index);
    }

    /// 侧边栏清空筛选条件
    fn clear_sidebar_filters(&mut self) {
        if self.grid_state.filters.is_empty() {
            return;
        }
        self.grid_state.filters.clear();
        self.sidebar_panel_state.selection.filters = 0;
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
        self.show_sidebar = true;
        self.sidebar_panel_state.show_filters = true;
        self.sidebar_section = ui::SidebarSection::Filters;
        self.focus_area = ui::FocusArea::Sidebar;
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
        self.focus_sql_editor = true;
        self.notifications
            .info("已生成重命名 SQL，请修改目标表名后执行");
    }

    /// 处理 Tab 栏操作
    pub(super) fn handle_tab_actions(&mut self, tab_actions: TabBarActions) {
        if tab_actions.new_tab {
            self.tab_manager.new_tab();
            self.sync_from_active_tab();
        }

        if let Some(idx) = tab_actions.switch_to {
            self.tab_manager.set_active(idx);
            self.sync_from_active_tab();
        }

        if let Some(idx) = tab_actions.close_tab {
            if self.tab_manager.tabs.len() > 1
                && let Some(request_id) = self
                    .tab_manager
                    .tabs
                    .get(idx)
                    .and_then(|tab| tab.pending_request_id)
            {
                self.cancel_query_request(request_id);
            }
            self.tab_manager.close_tab(idx);
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
            for request_id in request_ids {
                self.cancel_query_request(request_id);
            }
            self.tab_manager.close_other_tabs();
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
            for request_id in request_ids {
                self.cancel_query_request(request_id);
            }
            self.tab_manager.close_tabs_to_right();
            self.sync_from_active_tab();
        }
    }

    /// 检测并处理焦点切换快捷键
    pub(super) fn handle_focus_shortcuts(
        &mut self,
        ctx: &egui::Context,
        toolbar_actions: &mut ToolbarActions,
    ) {
        let has_dialog = self.has_modal_dialog_open();

        ctx.input(|i| {
            // Ctrl+Shift+T: 打开主题选择器
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                toolbar_actions.open_theme_selector = true;
            }

            if has_dialog {
                return;
            }

            // Ctrl+1~4: 聚焦侧边栏不同区域
            self.handle_sidebar_focus_shortcuts(i);

            // Ctrl+D: 切换日/夜模式
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::D) {
                toolbar_actions.toggle_dark_mode = true;
            }
        });
    }

    /// 处理侧边栏焦点快捷键
    fn handle_sidebar_focus_shortcuts(&mut self, input: &egui::InputState) {
        // 顺序：1连接 2数据库 3表 4筛选 5触发器 6存储过程
        let shortcuts = [
            (egui::Key::Num1, ui::SidebarSection::Connections, "连接列表"),
            (egui::Key::Num2, ui::SidebarSection::Databases, "数据库列表"),
            (egui::Key::Num3, ui::SidebarSection::Tables, "表列表"),
            (egui::Key::Num4, ui::SidebarSection::Filters, "筛选面板"),
            (egui::Key::Num5, ui::SidebarSection::Triggers, "触发器列表"),
            (
                egui::Key::Num6,
                ui::SidebarSection::Routines,
                "存储过程列表",
            ),
        ];

        for (key, section, name) in shortcuts {
            if input.modifiers.ctrl && input.key_pressed(key) {
                self.show_sidebar = true;
                self.focus_area = ui::FocusArea::Sidebar;
                self.sidebar_section = section;
                self.grid_state.focused = false;
                self.focus_sql_editor = false;
                self.notifications.info(format!("切换到: {}", name));
                break;
            }
        }
    }
}
