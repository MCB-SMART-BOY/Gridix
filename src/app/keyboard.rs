//! 键盘快捷键处理模块
//!
//! 集中管理所有键盘快捷键的处理逻辑。

use crate::core::Action;
use crate::ui;
use eframe::egui;

use super::DbManagerApp;

impl DbManagerApp {
    fn action_triggered_in_input(
        keybindings: &crate::core::KeyBindings,
        action: Action,
        input: &egui::InputState,
    ) -> bool {
        keybindings.get(action).is_some_and(|binding| {
            binding.modifiers.matches(&input.modifiers)
                && input.key_pressed(binding.key.to_egui_key())
        })
    }

    fn consume_action_binding(&self, action: Action, input: &mut egui::InputState) -> bool {
        self.keybindings.get(action).is_some_and(|binding| {
            input.consume_key(binding.modifiers.to_egui(), binding.key.to_egui_key())
        })
    }

    fn handle_focus_fallback_shortcuts(&mut self, input: &mut egui::InputState) -> bool {
        if self.focus_area == ui::FocusArea::Sidebar
            && !(self.sidebar_section == ui::SidebarSection::Filters
                && self.sidebar_panel_state.filter_input_has_focus)
        {
            let move_right = input.consume_key(egui::Modifiers::NONE, egui::Key::L)
                || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight);
            if move_right {
                match self.sidebar_section {
                    ui::SidebarSection::Connections => {
                        if self
                            .manager
                            .get_active()
                            .is_some_and(|connection| !connection.databases.is_empty())
                        {
                            self.sidebar_section = ui::SidebarSection::Databases;
                        } else {
                            self.set_focus_area(ui::FocusArea::DataGrid);
                        }
                    }
                    ui::SidebarSection::Databases => {
                        if self
                            .manager
                            .get_active()
                            .is_some_and(|connection| !connection.tables.is_empty())
                        {
                            self.sidebar_section = ui::SidebarSection::Tables;
                        } else {
                            self.set_focus_area(ui::FocusArea::DataGrid);
                        }
                    }
                    ui::SidebarSection::Tables
                    | ui::SidebarSection::Filters
                    | ui::SidebarSection::Triggers
                    | ui::SidebarSection::Routines => {
                        self.set_focus_area(ui::FocusArea::DataGrid);
                    }
                }
                return true;
            }
        }

        if self.focus_area == ui::FocusArea::SqlEditor && self.show_sql_editor {
            if ui::consume_local_shortcut(input, ui::LocalShortcut::Cancel) {
                if self.show_autocomplete {
                    self.show_autocomplete = false;
                } else if self.editor_mode == ui::EditorMode::Insert {
                    self.editor_mode = ui::EditorMode::Normal;
                    self.set_focus_area(ui::FocusArea::SqlEditor);
                } else {
                    self.set_focus_area(ui::FocusArea::DataGrid);
                }
                return true;
            }

            if self.consume_action_binding(Action::ToggleEditor, input) {
                self.toggle_sql_editor_visibility();
                return true;
            }
        }

        if self.focus_area == ui::FocusArea::DataGrid
            && self.grid_state.focused
            && self.grid_state.editing_cell.is_none()
            && let Some(result) = &self.result
        {
            let max_row = result.rows.len();
            let max_col = result.columns.len();

            if max_row > 0 && max_col > 0 {
                if input.consume_key(egui::Modifiers::NONE, egui::Key::H)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft)
                {
                    if self.grid_state.cursor.1 == 0 {
                        self.show_sidebar = true;
                        self.set_focus_area(ui::FocusArea::Sidebar);
                    } else {
                        self.grid_state.move_cursor(0, -1, max_row, max_col);
                    }
                    return true;
                }

                if input.consume_key(egui::Modifiers::NONE, egui::Key::L)
                    || input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight)
                {
                    self.grid_state.move_cursor(0, 1, max_row, max_col);
                    return true;
                }
            }
        }

        false
    }

    /// 处理键盘快捷键
    pub(super) fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        // 检查是否有模态对话框打开
        let input_context = self.capture_input_context(ctx);
        let has_dialog = input_context.has_modal_dialog;
        let keybindings = self.keybindings.clone();

        ctx.input_mut(|i| {
            if self.handle_focus_fallback_shortcuts(i) {
                return;
            }

            // ===== 对话框打开时跳过以下快捷键 =====
            if has_dialog {
                return;
            }

            // Ctrl+Shift+N: 新建表
            if Self::action_triggered_in_input(&keybindings, Action::NewTable, i)
                && input_context.allows_workspace_creation_shortcuts()
            {
                self.open_create_table_dialog();
            }

            // Ctrl+Shift+D: 新建数据库
            if Self::action_triggered_in_input(&keybindings, Action::NewDatabase, i)
                && input_context.allows_workspace_creation_shortcuts()
            {
                self.open_create_database_dialog();
            }

            // Ctrl+Shift+U: 新建用户
            if Self::action_triggered_in_input(&keybindings, Action::NewUser, i)
                && input_context.allows_workspace_creation_shortcuts()
            {
                self.open_create_user_dialog();
            }

            // Ctrl+E: 导出
            if Self::action_triggered_in_input(&keybindings, Action::Export, i)
                && input_context.allows_import_export_shortcuts()
            {
                self.open_export_dialog();
            }

            // Ctrl+I: 导入
            if Self::action_triggered_in_input(&keybindings, Action::Import, i)
                && input_context.allows_import_export_shortcuts()
            {
                self.open_import_dialog();
            }

            // Ctrl+H: 历史记录
            if Self::action_triggered_in_input(&keybindings, Action::ShowHistory, i)
                && input_context.allows_workspace_overlay_shortcuts()
            {
                self.toggle_history_panel();
            }

            // Ctrl+R: 切换 ER 关系图
            if Self::action_triggered_in_input(&keybindings, Action::ToggleErDiagram, i)
                && input_context.allows_workspace_overlay_shortcuts()
            {
                self.toggle_er_diagram_visibility();
            }

            // F5: 刷新表列表
            if Self::action_triggered_in_input(&keybindings, Action::Refresh, i)
                && input_context.allows_refresh()
                && let Some(name) = self.manager.active.clone()
            {
                self.connect(name);
            }

            // Ctrl+L: 清空命令行
            if Self::action_triggered_in_input(&keybindings, Action::ClearCommandLine, i)
                && input_context.allows_clear_command_line()
            {
                self.sql.clear();
                self.notifications.dismiss_all();
            }

            // Ctrl+J: 切换 SQL 编辑器显示
            if Self::action_triggered_in_input(&keybindings, Action::ToggleEditor, i)
                && input_context.allows_editor_visibility_toggle()
            {
                self.toggle_sql_editor_visibility();
            }

            // Ctrl+B: 切换侧边栏显示
            if Self::action_triggered_in_input(&keybindings, Action::ToggleSidebar, i)
                && input_context.allows_panel_visibility_toggle()
            {
                self.toggle_sidebar_visibility();
            }

            // Ctrl+K: 清空搜索
            if Self::action_triggered_in_input(&keybindings, Action::ClearSearch, i)
                && input_context.allows_search_shortcuts()
            {
                self.search_text.clear();
            }

            // Ctrl+F: 添加筛选条件
            if Self::action_triggered_in_input(&keybindings, Action::AddFilter, i)
                && input_context.allows_filter_shortcuts()
            {
                self.add_sidebar_filter();
            }

            // Ctrl+Shift+F: 清空筛选条件
            if Self::action_triggered_in_input(&keybindings, Action::ClearFilters, i)
                && input_context.allows_filter_shortcuts()
            {
                self.clear_sidebar_filters();
            }

            // Ctrl+S: 触发保存表格修改
            if Self::action_triggered_in_input(&keybindings, Action::Save, i)
                && input_context.allows_data_grid_shortcuts()
            {
                self.grid_state.pending_save = true;
            }

            // Ctrl+G: 跳转到行
            if Self::action_triggered_in_input(&keybindings, Action::GotoLine, i)
                && input_context.allows_data_grid_shortcuts()
            {
                self.grid_state.show_goto_dialog = true;
            }

            // 用户可选：新建查询标签页（默认未绑定）
            if Self::action_triggered_in_input(&keybindings, Action::NewTab, i)
                && input_context.allows_tab_management_shortcuts()
            {
                self.open_new_query_tab();
            }

            // Ctrl+Tab: 下一个查询标签页
            if Self::action_triggered_in_input(&keybindings, Action::NextTab, i)
                && input_context.allows_tab_management_shortcuts()
            {
                self.select_next_query_tab();
            }

            // Ctrl+Shift+Tab: 上一个查询标签页
            if Self::action_triggered_in_input(&keybindings, Action::PrevTab, i)
                && input_context.allows_tab_management_shortcuts()
            {
                self.select_previous_query_tab();
            }

            // Ctrl+W: 关闭当前查询标签页
            if Self::action_triggered_in_input(&keybindings, Action::CloseTab, i)
                && input_context.allows_tab_management_shortcuts()
            {
                self.close_active_query_tab();
            }

            // Escape: 取消当前操作/关闭面板
            if i.key_pressed(egui::Key::Escape) && !input_context.is_text_entry_scope() {
                // 优先关闭帮助面板
                if self.show_help {
                    self.show_help = false;
                } else if self.show_history_panel {
                    self.show_history_panel = false;
                } else if self.show_er_diagram {
                    self.show_er_diagram = false;
                }
            }
        });
    }

    /// 焦点循环导航
    pub(super) fn cycle_focus(&mut self, reverse: bool) {
        // 焦点循环顺序: Sidebar -> DataGrid -> SqlEditor -> Sidebar
        let areas = if self.show_sidebar && self.show_sql_editor {
            vec![
                ui::FocusArea::Sidebar,
                ui::FocusArea::DataGrid,
                ui::FocusArea::SqlEditor,
            ]
        } else if self.show_sidebar {
            vec![ui::FocusArea::Sidebar, ui::FocusArea::DataGrid]
        } else if self.show_sql_editor {
            vec![ui::FocusArea::DataGrid, ui::FocusArea::SqlEditor]
        } else {
            vec![ui::FocusArea::DataGrid]
        };

        if areas.len() <= 1 {
            return;
        }

        let current_idx = areas
            .iter()
            .position(|&a| a == self.focus_area)
            .unwrap_or(0);
        let next_idx = if reverse {
            if current_idx == 0 {
                areas.len() - 1
            } else {
                current_idx - 1
            }
        } else {
            (current_idx + 1) % areas.len()
        };

        let new_focus = areas[next_idx];
        self.set_focus_area(new_focus);
    }

    /// 处理缩放快捷键
    pub(super) fn handle_zoom_shortcuts(&mut self, ctx: &egui::Context) {
        let keybindings = self.keybindings.clone();
        let zoom_delta = ctx.input(|i| {
            let mut delta = 0.0f32;
            let action_triggered = |action: Action| {
                keybindings.get(action).is_some_and(|binding| {
                    binding.modifiers.matches(&i.modifiers)
                        && i.key_pressed(binding.key.to_egui_key())
                })
            };

            // Ctrl++ 或 Ctrl+= 放大
            if action_triggered(Action::ZoomIn) {
                delta = 0.1;
            }

            // Ctrl+- 缩小
            if action_triggered(Action::ZoomOut) {
                delta = -0.1;
            }

            // Ctrl+0 重置缩放
            if action_triggered(Action::ZoomReset) {
                return Some(-999.0); // 特殊值表示重置
            }

            // Ctrl+滚轮缩放
            if i.modifiers.ctrl && i.smooth_scroll_delta.y != 0.0 {
                delta = i.smooth_scroll_delta.y * 0.001;
            }

            if delta != 0.0 { Some(delta) } else { None }
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
