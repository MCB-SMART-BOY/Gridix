#![allow(clippy::too_many_arguments)]

mod actions;
mod dropdowns;
mod theme_combo;
mod utils;

pub use actions::{ToolbarActions, ToolbarFocusTransfer};

use crate::core::{Action, KeyBindings, ProgressManager, ThemeManager};
use crate::ui::styles::{MARGIN_MD, MARGIN_SM};
use crate::ui::{
    LocalShortcut, action_tooltip, action_tooltip_with_extras, consume_local_shortcut,
    shortcut_tooltip,
};
use egui::{Color32, Vec2};

use super::ProgressIndicator;
use dropdowns::{show_actions_dropdown, show_create_dropdown};
use theme_combo::{DARK_THEMES, LIGHT_THEMES, helix_theme_combo_simple};
use utils::{icon_button, icon_button_with_focus, separator, text_button};

pub struct Toolbar;

impl Toolbar {
    /// 显示工具栏（无焦点状态）
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        theme_manager: &ThemeManager,
        keybindings: &KeyBindings,
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
            ui,
            theme_manager,
            keybindings,
            has_result,
            show_sidebar,
            show_editor,
            is_dark_mode,
            actions,
            connections,
            active_connection,
            databases,
            selected_database,
            tables,
            selected_table,
            ui_scale,
            progress,
            false,
            0,
        )
    }

    /// 显示工具栏（带焦点状态）
    #[allow(clippy::too_many_arguments)]
    pub fn show_with_focus(
        ui: &mut egui::Ui,
        theme_manager: &ThemeManager,
        keybindings: &KeyBindings,
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
                    Self::show_left_buttons(
                        ui,
                        keybindings,
                        show_sidebar,
                        show_editor,
                        actions,
                        is_focused,
                        selected_index,
                    );

                    ui.add_space(8.0);
                    separator(ui);
                    ui.add_space(8.0);

                    // 操作按钮（移除了连接/库/表选择器，这些在左侧栏中已有）
                    Self::show_action_buttons(
                        ui,
                        keybindings,
                        has_result,
                        actions,
                        is_focused,
                        selected_index,
                    );

                    // 保留快捷键功能但不显示选择器
                    // 快捷键 Ctrl+1/2/3 仍可在 app 中触发侧边栏操作
                    let _ = (
                        connections,
                        active_connection,
                        databases,
                        selected_database,
                        tables,
                        selected_table,
                    );

                    // 右侧区域
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 圆形头像按钮
                        let avatar_size = 24.0;
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::splat(avatar_size), egui::Sense::click());

                        // 绘制圆形背景
                        let center = rect.center();
                        let radius = avatar_size / 2.0;
                        let bg_color = if response.hovered() {
                            Color32::from_rgb(100, 149, 237) // 悬停时更亮
                        } else {
                            Color32::from_rgb(70, 130, 180) // 钢蓝色
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
                        Self::show_zoom_controls(ui, keybindings, ui_scale, actions);

                        ui.add_space(4.0);
                        separator(ui);
                        ui.add_space(4.0);

                        // 主题选择器 - 根据当前模式显示对应主题列表
                        let themes = if is_dark_mode {
                            DARK_THEMES
                        } else {
                            LIGHT_THEMES
                        };
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
                        ) && let Some(&preset) = themes.get(new_idx)
                        {
                            actions.theme_changed = Some(preset);
                        }
                        actions.open_theme_selector = false;

                        ui.add_space(4.0);

                        // 日/夜模式切换按钮
                        let mode_icon = if is_dark_mode { "🌙" } else { "☀" };
                        let mode_tooltip = if is_dark_mode {
                            shortcut_tooltip("切换到日间模式", &["Ctrl+D"])
                        } else {
                            shortcut_tooltip("切换到夜间模式", &["Ctrl+D"])
                        };

                        if icon_button(ui, mode_icon, &mode_tooltip, true) {
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
        keybindings: &KeyBindings,
        show_sidebar: bool,
        show_editor: bool,
        actions: &mut ToolbarActions,
        is_focused: bool,
        selected_index: usize,
    ) {
        // 侧边栏切换 (索引 0)
        let sidebar_icon = if show_sidebar { "◀" } else { "▶" };
        if icon_button_with_focus(
            ui,
            sidebar_icon,
            &action_tooltip_with_extras(keybindings, Action::ToggleSidebar, "切换侧边栏", &[]),
            true,
            is_focused && selected_index == 0,
        ) {
            actions.toggle_sidebar = true;
        }

        // 编辑器切换 (索引 1)
        let editor_icon = if show_editor { "▼" } else { "▲" };
        if icon_button_with_focus(
            ui,
            editor_icon,
            &action_tooltip_with_extras(keybindings, Action::ToggleEditor, "切换 SQL 编辑器", &[]),
            true,
            is_focused && selected_index == 1,
        ) {
            actions.toggle_editor = true;
        }
    }

    /// 显示缩放控制
    fn show_zoom_controls(
        ui: &mut egui::Ui,
        keybindings: &KeyBindings,
        ui_scale: f32,
        actions: &mut ToolbarActions,
    ) {
        // 缩小按钮
        if icon_button(
            ui,
            "−",
            &action_tooltip_with_extras(keybindings, Action::ZoomOut, "缩小界面", &[]),
            true,
        ) {
            actions.zoom_out = true;
        }

        // 缩放比例显示（可点击重置）
        let scale_text = format!("{}%", (ui_scale * 100.0).round() as i32);
        if text_button(
            ui,
            &scale_text,
            &action_tooltip_with_extras(keybindings, Action::ZoomReset, "重置界面缩放", &[]),
            true,
        ) {
            actions.zoom_reset = true;
        }

        // 放大按钮
        if icon_button(
            ui,
            "+",
            &action_tooltip_with_extras(keybindings, Action::ZoomIn, "放大界面", &[]),
            true,
        ) {
            actions.zoom_in = true;
        }
    }

    /// 显示操作按钮
    fn show_action_buttons(
        ui: &mut egui::Ui,
        keybindings: &KeyBindings,
        has_result: bool,
        actions: &mut ToolbarActions,
        is_focused: bool,
        selected_index: usize,
    ) {
        // 刷新 (索引 2)
        if icon_button_with_focus(
            ui,
            "🔄",
            &action_tooltip(keybindings, Action::Refresh),
            true,
            is_focused && selected_index == 2,
        ) {
            actions.refresh_tables = true;
        }

        ui.add_space(4.0);
        separator(ui);
        ui.add_space(4.0);

        // 操作下拉菜单 (索引 3)
        let force_open = actions.open_actions_dropdown;
        show_actions_dropdown(ui, keybindings, has_result, force_open, actions);
        actions.open_actions_dropdown = false;

        ui.add_space(4.0);

        // 新建下拉菜单 (索引 4)
        let force_open = actions.open_create_dropdown;
        show_create_dropdown(ui, keybindings, force_open, actions);
        actions.open_create_dropdown = false;

        ui.add_space(4.0);
        separator(ui);
        ui.add_space(4.0);

        // 快捷键设置 (索引 5)
        if icon_button_with_focus(
            ui,
            "⌨",
            &action_tooltip(keybindings, Action::OpenKeybindingsDialog),
            true,
            is_focused && selected_index == 5,
        ) {
            actions.show_keybindings = true;
        }

        // 帮助 (索引 6)
        if icon_button_with_focus(
            ui,
            "?",
            &action_tooltip(keybindings, Action::ShowHelp),
            true,
            is_focused && selected_index == 6,
        ) {
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

        ui.input_mut(|i| {
            if consume_local_shortcut(i, LocalShortcut::ToolbarPrev) && *toolbar_index > 0 {
                *toolbar_index -= 1;
            } else if consume_local_shortcut(i, LocalShortcut::ToolbarNext)
                && *toolbar_index < TOOLBAR_ITEMS - 1
            {
                *toolbar_index += 1;
            } else if consume_local_shortcut(i, LocalShortcut::ToolbarToQueryTabs) {
                actions.focus_transfer = Some(actions::ToolbarFocusTransfer::ToQueryTabs);
            } else if consume_local_shortcut(i, LocalShortcut::ToolbarActivate) {
                match *toolbar_index {
                    0 => actions.toggle_sidebar = true,
                    1 => actions.toggle_editor = true,
                    2 => actions.refresh_tables = true,
                    3 => actions.open_actions_dropdown = true,
                    4 => actions.open_create_dropdown = true,
                    5 => actions.show_keybindings = true,
                    6 => actions.show_help = true,
                    _ => {}
                }
            } else if consume_local_shortcut(i, LocalShortcut::ToolbarDismiss) {
                actions.focus_transfer = Some(actions::ToolbarFocusTransfer::ToQueryTabs);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Area, Context, Event, Id, Key, Modifiers, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn run_toolbar_key(key: Key, toolbar_index: &mut usize, actions: &mut ToolbarActions) {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            events: vec![key_event(key)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
        Area::new(Id::new("toolbar_keyboard_test")).show(&ctx, |ui| {
            Toolbar::handle_keyboard(ui, toolbar_index, actions);
        });
        let _ = ctx.end_pass();
    }

    #[test]
    fn toolbar_keyboard_activate_opens_action_dropdown_when_selected() {
        let mut index = 3;
        let mut actions = ToolbarActions::default();

        run_toolbar_key(Key::Enter, &mut index, &mut actions);

        assert!(actions.open_actions_dropdown);
    }

    #[test]
    fn toolbar_keyboard_uses_local_shortcuts_for_navigation() {
        let mut index = 1;
        let mut actions = ToolbarActions::default();

        run_toolbar_key(Key::L, &mut index, &mut actions);
        assert_eq!(index, 2);

        run_toolbar_key(Key::J, &mut index, &mut actions);
        assert_eq!(
            actions.focus_transfer,
            Some(actions::ToolbarFocusTransfer::ToQueryTabs)
        );
    }
}
