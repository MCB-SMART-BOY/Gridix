//! 键盘快捷键处理模块
//!
//! 集中管理所有键盘快捷键的处理逻辑。

use eframe::egui;
use crate::ui;

use super::DbManagerApp;

impl DbManagerApp {
    /// 处理键盘快捷键
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // 检查是否有模态对话框打开
        let has_dialog = self.has_modal_dialog_open();

        ctx.input(|i| {
            // ===== 始终可用的快捷键（即使对话框打开） =====
            
            // F1: 帮助（切换）
            if i.key_pressed(egui::Key::F1) {
                self.show_help = !self.show_help;
            }

            // ===== 对话框打开时跳过以下快捷键 =====
            if has_dialog {
                return;
            }

            // Ctrl+N: 新建连接
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::N) {
                self.show_connection_dialog = true;
            }
            
            // Ctrl+Shift+N: 新建表
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::N) {
                if let Some(conn) = self.manager.get_active() {
                    if conn.selected_database.is_some() {
                        let db_type = conn.config.db_type.clone();
                        self.ddl_dialog_state.open_create_table(db_type);
                    }
                }
            }

            // Ctrl+Shift+D: 新建数据库
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::D) {
                let db_type = self.manager.get_active()
                    .map(|c| c.config.db_type.clone())
                    .unwrap_or_default();
                self.create_db_dialog_state.open(db_type);
            }

            // Ctrl+Shift+U: 新建用户
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::U) {
                if let Some(conn) = self.manager.get_active() {
                    let db_type = conn.config.db_type.clone();
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

            // Ctrl+E: 导出
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) && self.result.is_some() {
                self.show_export_dialog = true;
                self.export_status = None;
            }

            // Ctrl+I: 导入
            if i.modifiers.ctrl && i.key_pressed(egui::Key::I) {
                self.handle_import();
            }

            // Ctrl+H: 历史记录
            if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                self.show_history_panel = !self.show_history_panel;
            }

            // Ctrl+R: 切换 ER 关系图
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::R) {
                self.show_er_diagram = !self.show_er_diagram;
                if self.show_er_diagram {
                    self.load_er_diagram_data();
                    self.notifications.info("ER 关系图已打开");
                } else {
                    self.notifications.info("ER 关系图已关闭");
                }
            }

            // F5: 刷新表列表
            if i.key_pressed(egui::Key::F5) {
                if let Some(name) = self.manager.active.clone() {
                    self.connect(name);
                }
            }

            // Ctrl+L: 清空命令行
            if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                self.sql.clear();
                self.notifications.dismiss_all();
            }

            // Ctrl+J: 切换 SQL 编辑器显示
            if i.modifiers.ctrl && i.key_pressed(egui::Key::J) {
                self.show_sql_editor = !self.show_sql_editor;
                if self.show_sql_editor {
                    // 打开时自动聚焦到编辑器
                    self.focus_area = ui::FocusArea::SqlEditor;
                    self.focus_sql_editor = true;
                    self.grid_state.focused = false;
                } else {
                    // 关闭时将焦点还给数据表格
                    self.focus_area = ui::FocusArea::DataGrid;
                    self.grid_state.focused = true;
                }
            }

            // Ctrl+B: 切换侧边栏显示
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                self.show_sidebar = !self.show_sidebar;
                if self.show_sidebar {
                    // 打开侧边栏时聚焦到侧边栏
                    self.focus_area = ui::FocusArea::Sidebar;
                    self.grid_state.focused = false;
                } else if self.focus_area == ui::FocusArea::Sidebar {
                    // 关闭侧边栏时，如果焦点在侧边栏，则转移到数据表格
                    self.focus_area = ui::FocusArea::DataGrid;
                    self.grid_state.focused = true;
                }
            }

            // Ctrl+K: 清空搜索
            if i.modifiers.ctrl && i.key_pressed(egui::Key::K) {
                self.search_text.clear();
            }

            // Ctrl+F: 添加筛选条件
            if i.modifiers.ctrl && i.key_pressed(egui::Key::F) && !i.modifiers.shift {
                if let Some(result) = &self.result {
                    if let Some(col) = result.columns.first() {
                        self.grid_state.filters.push(ui::components::ColumnFilter::new(col.clone()));
                    }
                }
            }

            // Ctrl+Shift+F: 清空筛选条件
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::F) {
                self.grid_state.filters.clear();
            }

            // Ctrl+S: 触发保存表格修改
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.grid_state.pending_save = true;
            }

            // Ctrl+G: 跳转到行
            if i.modifiers.ctrl && i.key_pressed(egui::Key::G) {
                self.grid_state.show_goto_dialog = true;
            }

            // Ctrl+Tab: 下一个查询标签页
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::Tab) {
                self.tab_manager.next_tab();
            }

            // Ctrl+Shift+Tab: 上一个查询标签页
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::Tab) {
                self.tab_manager.prev_tab();
            }

            // Ctrl+W: 关闭当前查询标签页
            if i.modifiers.ctrl && i.key_pressed(egui::Key::W) {
                self.tab_manager.close_active_tab();
            }
        });
    }

    /// 处理缩放快捷键
    pub(super) fn handle_zoom_shortcuts(&mut self, ctx: &egui::Context) {
        let zoom_delta = ctx.input(|i| {
            let mut delta = 0.0f32;

            // Ctrl++ 或 Ctrl+= 放大
            if i.modifiers.ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
                delta = 0.1;
            }

            // Ctrl+- 缩小
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Minus) {
                delta = -0.1;
            }

            // Ctrl+0 重置缩放
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num0) {
                return Some(-999.0); // 特殊值表示重置
            }

            // Ctrl+滚轮缩放
            if i.modifiers.ctrl && i.raw_scroll_delta.y != 0.0 {
                delta = i.raw_scroll_delta.y * 0.001;
            }

            if delta != 0.0 {
                Some(delta)
            } else {
                None
            }
        });

        if let Some(delta) = zoom_delta {
            if delta == -999.0 {
                // 重置为 1.0
                self.set_ui_scale(ctx, 1.0);
            } else {
                let new_scale = self.ui_scale + delta;
                self.set_ui_scale(ctx, new_scale);
            }
        }
    }
}
