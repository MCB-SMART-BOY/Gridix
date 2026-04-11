//! 键盘兼容层
//!
//! app-level 快捷键解析已收束到 `input_router.rs`；这里暂时保留焦点循环辅助函数
//! 和 zoom 这个 true-global 兼容路径。

use crate::core::Action;
use crate::ui;
use eframe::egui;

use super::DbManagerApp;

impl DbManagerApp {
    /// 焦点循环导航
    pub(in crate::app) fn cycle_focus(&mut self, reverse: bool) {
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
    pub(in crate::app) fn handle_zoom_shortcuts(&mut self, ctx: &egui::Context) {
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
