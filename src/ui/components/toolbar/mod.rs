#![allow(clippy::too_many_arguments)]

mod actions;
mod dropdowns;
mod theme_combo;
mod utils;

pub use actions::{ToolbarActions, ToolbarFocusTransfer};

use crate::core::{ProgressManager, ThemeManager};
use crate::ui::styles::{MARGIN_MD, MARGIN_SM};
use egui::{Color32, Vec2};

use super::ProgressIndicator;
use dropdowns::{show_actions_dropdown, show_create_dropdown};
use theme_combo::{helix_theme_combo_simple, DARK_THEMES, LIGHT_THEMES};
use utils::{separator, icon_button, icon_button_with_focus, text_button};

pub struct Toolbar;

impl Toolbar {
    /// 显示工具栏（无焦点状态）
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        theme_manager: &ThemeManager,
        has_result: bool,
        show_sidebar: bool,
        show_editor: bool,
        is_dark_mode: bool,
        actions: &mut ToolbarActions,
        connections: &[String],
        active_connection: Option<&str>,
        databases: &[String],
        selected_database: Option<&str>,
        tables: &[String],
        selected_table: Option<&str>,
        ui_scale: f32,
        progress: &ProgressManager,
    ) -> Option<u64> {
        Self::show_with_focus(
            ui, theme_manager, has_result, show_sidebar, show_editor,
            is_dark_mode, actions, connections, active_connection,
            databases, selected_database, tables, selected_table,
            ui_scale, progress, false, 0,
        )
    }

    /// 显示工具栏（带焦点状态）
    #[allow(clippy::too_many_arguments)]
    pub fn show_with_focus(
        ui: &mut egui::Ui,
        theme_manager: &ThemeManager,
        has_result: bool,
        show_sidebar: bool,
        show_editor: bool,
        is_dark_mode: bool,
        actions: &mut ToolbarActions,
        connections: &[String],
        active_connection: Option<&str>,
        databases: &[String],
        selected_database: Option<&str>,
        tables: &[String],
        selected_table: Option<&str>,
        ui_scale: f32,
        progress: &ProgressManager,
        is_focused: bool,
        selected_index: usize,
    ) -> Option<u64> {
        let mut cancel_task_id = None;
        actions.show_editor = show_editor;

        // 工具栏容器
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(MARGIN_MD, MARGIN_SM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                    // 左侧按钮组
                    Self::show_left_buttons(ui, show_sidebar, show_editor, actions, is_focused, selected_index);

                    ui.add_space(8.0);
                    separator(ui);
                    ui.add_space(8.0);

                    // 操作按钮（移除了连接/库/表选择器，这些在左侧栏中已有）
                    Self::show_action_buttons(ui, has_result, actions, is_focused, selected_index);
                    
                    // 保留快捷键功能但不显示选择器
                    // 快捷键 Ctrl+1/2/3 仍可在 app 中触发侧边栏操作
                    let _ = (connections, active_connection, databases, selected_database, tables, selected_table);

                    // 右侧区域
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 圆形头像按钮
                        let avatar_size = 24.0;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::splat(avatar_size),
                            egui::Sense::click(),
                        );
                        
                        // 绘制圆形背景
                        let center = rect.center();
                        let radius = avatar_size / 2.0;
                        let bg_color = if response.hovered() {
                            Color32::from_rgb(100, 149, 237)  // 悬停时更亮
                        } else {
                            Color32::from_rgb(70, 130, 180)   // 钢蓝色
                        };
                        
                        ui.painter().circle_filled(center, radius, bg_color);
                        
                        // 绘制笑脸图标
                        let text = "😊";
                        let font_id = egui::FontId::proportional(14.0);
                        let text_color = Color32::WHITE;
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            text,
                            font_id,
                            text_color,
                        );
                        
                        if response.clicked() {
                            actions.show_about = true;
                        }
                        
                        response.on_hover_text("关于我们");
                        
                        ui.add_space(8.0);
                        separator(ui);
                        ui.add_space(8.0);

                        // 缩放控制
                        Self::show_zoom_controls(ui, ui_scale, actions);

                        ui.add_space(4.0);
                        separator(ui);
                        ui.add_space(4.0);

                        // 主题选择器 - 根据当前模式显示对应主题列表
                        let themes = if is_dark_mode { DARK_THEMES } else { LIGHT_THEMES };
                        let current_theme_idx = themes
                            .iter()
                            .position(|&t| t == theme_manager.current)
                            .unwrap_or(0);

                        if let Some(new_idx) = helix_theme_combo_simple(
                            ui,
                            "theme_selector",
                            theme_manager.current,
                            current_theme_idx,
                            themes,
                            200.0,
                            actions.open_theme_selector,
                        )
                            && let Some(&preset) = themes.get(new_idx) {
                                actions.theme_changed = Some(preset);
                            }
                        actions.open_theme_selector = false;

                        ui.add_space(4.0);

                        // 日/夜模式切换按钮
                        let mode_icon = if is_dark_mode { "🌙" } else { "☀" };
                        let mode_tooltip = if is_dark_mode {
                            "切换到日间模式 (Ctrl+D)"
                        } else {
                            "切换到夜间模式 (Ctrl+D)"
                        };

                        if icon_button(ui, mode_icon, mode_tooltip, true) {
                            actions.toggle_dark_mode = true;
                        }

                        // 进度指示器（如果有活跃任务）
                        if progress.has_active_tasks() {
                            ui.add_space(8.0);
                            separator(ui);
                            ui.add_space(4.0);
                            
                            if let Some(id) = ProgressIndicator::show_in_toolbar(ui, progress) {
                                cancel_task_id = Some(id);
                            }
                        }
                    });
                });
            });
        
        cancel_task_id
    }

    /// 显示左侧按钮
    fn show_left_buttons(
        ui: &mut egui::Ui,
        show_sidebar: bool,
        show_editor: bool,
        actions: &mut ToolbarActions,
        is_focused: bool,
        selected_index: usize,
    ) {
        // 侧边栏切换 (索引 0)
        let sidebar_icon = if show_sidebar { "◀" } else { "▶" };
        if icon_button_with_focus(ui, sidebar_icon, "侧边栏 (Ctrl+B)", true, is_focused && selected_index == 0) {
            actions.toggle_sidebar = true;
        }

        // 编辑器切换 (索引 1)
        let editor_icon = if show_editor { "▼" } else { "▲" };
        if icon_button_with_focus(ui, editor_icon, "编辑器 (Ctrl+J)", true, is_focused && selected_index == 1) {
            actions.toggle_editor = true;
        }
    }

    /// 显示缩放控制
    fn show_zoom_controls(ui: &mut egui::Ui, ui_scale: f32, actions: &mut ToolbarActions) {
        // 缩小按钮
        if icon_button(ui, "−", "缩小 (Ctrl+-)", true) {
            actions.zoom_out = true;
        }

        // 缩放比例显示（可点击重置）
        let scale_text = format!("{}%", (ui_scale * 100.0).round() as i32);
        if text_button(ui, &scale_text, "重置缩放 (Ctrl+0)", true) {
            actions.zoom_reset = true;
        }

        // 放大按钮
        if icon_button(ui, "+", "放大 (Ctrl++)", true) {
            actions.zoom_in = true;
        }
    }

    /// 显示操作按钮
    fn show_action_buttons(
        ui: &mut egui::Ui,
        has_result: bool,
        actions: &mut ToolbarActions,
        is_focused: bool,
        selected_index: usize,
    ) {
        // 刷新 (索引 2)
        if icon_button_with_focus(ui, "🔄", "刷新 (F5)", true, is_focused && selected_index == 2) {
            actions.refresh_tables = true;
        }

        ui.add_space(4.0);
        separator(ui);
        ui.add_space(4.0);

        // 操作下拉菜单 (索引 3)
        show_actions_dropdown(ui, has_result, actions);

        ui.add_space(4.0);

        // 新建下拉菜单 (索引 4)
        show_create_dropdown(ui, actions);

        ui.add_space(4.0);
        separator(ui);
        ui.add_space(4.0);

        // 快捷键设置 (索引 5)
        if icon_button_with_focus(ui, "⌨", "快捷键设置", true, is_focused && selected_index == 5) {
            actions.show_keybindings = true;
        }

        // 帮助 (索引 6)
        if icon_button_with_focus(ui, "?", "帮助 (F1)", true, is_focused && selected_index == 6) {
            actions.show_help = true;
        }
    }
    
    /// 处理工具栏键盘输入 (Helix风格)
    /// 
    /// - h/l: 左右移动选中项
    /// - j: 向下进入Tab栏
    /// - Enter: 激活当前选中项
    pub fn handle_keyboard(
        ui: &mut egui::Ui,
        toolbar_index: &mut usize,
        actions: &mut ToolbarActions,
    ) {
        // 工具栏项目列表 (简化版本，主要支持导航)
        const TOOLBAR_ITEMS: usize = 7; // 侧边栏、编辑器、刷新、操作、新建、快捷键、帮助
        
        ui.input(|i| {
            // h/左箭头: 向左移动
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::ArrowLeft) {
                if *toolbar_index > 0 {
                    *toolbar_index -= 1;
                }
            }
            
            // l/右箭头: 向右移动
            if i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight) {
                if *toolbar_index < TOOLBAR_ITEMS - 1 {
                    *toolbar_index += 1;
                }
            }
            
            // j/下箭头: 向下进入Tab栏
            if i.key_pressed(egui::Key::J) || i.key_pressed(egui::Key::ArrowDown) {
                actions.focus_transfer = Some(actions::ToolbarFocusTransfer::ToQueryTabs);
            }
            
            // Enter: 激活当前选中项
            if i.key_pressed(egui::Key::Enter) {
                match *toolbar_index {
                    0 => actions.toggle_sidebar = true,
                    1 => actions.toggle_editor = true,
                    2 => actions.refresh_tables = true,
                    3 => { /* 操作下拉菜单 - 暂不支持 */ }
                    4 => { /* 新建下拉菜单 - 暂不支持 */ }
                    5 => actions.show_keybindings = true,
                    6 => actions.show_help = true,
                    _ => {}
                }
            }
            
            // Escape: 返回Tab栏
            if i.key_pressed(egui::Key::Escape) {
                actions.focus_transfer = Some(actions::ToolbarFocusTransfer::ToQueryTabs);
            }
        });
    }
}
