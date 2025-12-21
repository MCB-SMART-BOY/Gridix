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
        let is_editor_focused = self.focus_area == ui::FocusArea::SqlEditor
            && !self.has_modal_dialog_open();

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
            }
        );

        sql_editor_actions
    }

    /// 处理 SQL 编辑器操作
    pub(super) fn handle_sql_editor_actions(&mut self, actions: SqlEditorActions) {
        // 执行查询
        if actions.execute && !self.sql.is_empty() {
            let sql = self.sql.clone();
            self.execute(sql);
            self.sql.clear();
        }

        // EXPLAIN 分析
        if actions.explain && !self.sql.is_empty() {
            let sql = self.sql.trim();
            let explain_sql = if self.is_mysql() {
                format!("EXPLAIN FORMAT=TRADITIONAL {}", sql)
            } else if self.manager.get_active()
                .map(|c| c.config.db_type == crate::database::DatabaseType::PostgreSQL)
                .unwrap_or(false)
            {
                format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT TEXT) {}", sql)
            } else {
                format!("EXPLAIN QUERY PLAN {}", sql)
            };
            self.execute(explain_sql);
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
                self.execute(query_sql);
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
            let db_type = self.manager.get_active()
                .map(|c| c.config.db_type)
                .unwrap_or_default();
            self.ddl_dialog_state.open_create_table(db_type);
        }

        if actions.create_database {
            let db_type = self.manager.get_active()
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
            self.execute(schema_sql);
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
            self.execute(query_sql);
        }
        self.fetch_primary_key(&table);
        self.sql.clear();
    }

    /// 处理 Tab 栏操作
    pub(super) fn handle_tab_actions(&mut self, tab_actions: TabBarActions) {
        if tab_actions.new_tab {
            self.tab_manager.new_tab();
        }

        if let Some(idx) = tab_actions.switch_to {
            self.tab_manager.set_active(idx);
            if let Some(tab) = self.tab_manager.get_active() {
                self.sql = tab.sql.clone();
                self.result = tab.result.clone();
            }
        }

        if let Some(idx) = tab_actions.close_tab {
            self.tab_manager.close_tab(idx);
            if let Some(tab) = self.tab_manager.get_active() {
                self.sql = tab.sql.clone();
                self.result = tab.result.clone();
            }
        }

        if tab_actions.close_others {
            self.tab_manager.close_other_tabs();
        }

        if tab_actions.close_right {
            self.tab_manager.close_tabs_to_right();
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

            // Ctrl+T: 新建查询标签页
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::T) {
                self.tab_manager.new_tab();
                if let Some(tab) = self.tab_manager.get_active() {
                    self.sql = tab.sql.clone();
                    self.result = tab.result.clone();
                }
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
            (egui::Key::Num6, ui::SidebarSection::Routines, "存储过程列表"),
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
