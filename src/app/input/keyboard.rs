//! 键盘兼容层
//!
//! app-level 快捷键解析已收束到 `input_router.rs`；这里暂时保留焦点循环辅助函数
//! 和 zoom 这个 true-global 兼容路径。

use crate::core::Action;
use crate::ui;
use eframe::egui;

use super::DbManagerApp;

fn focus_cycle_areas(
    show_sidebar: bool,
    show_er_diagram: bool,
    show_sql_editor: bool,
) -> Vec<ui::FocusArea> {
    let mut areas = Vec::new();
    if show_sidebar {
        areas.push(ui::FocusArea::Sidebar);
    }
    areas.push(ui::FocusArea::DataGrid);
    if show_er_diagram {
        areas.push(ui::FocusArea::ErDiagram);
    }
    if show_sql_editor {
        areas.push(ui::FocusArea::SqlEditor);
    }
    areas
}

impl DbManagerApp {
    /// 焦点循环导航
    pub(in crate::app) fn cycle_focus(&mut self, reverse: bool) {
        // 焦点循环顺序: Sidebar -> DataGrid -> ErDiagram -> SqlEditor -> Sidebar
        let areas = focus_cycle_areas(
            self.show_sidebar,
            self.show_er_diagram,
            self.show_sql_editor,
        );

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

#[cfg(test)]
mod tests {
    use super::focus_cycle_areas;
    use crate::ui::FocusArea;

    #[test]
    fn focus_cycle_areas_include_er_diagram_between_grid_and_editor() {
        assert_eq!(
            focus_cycle_areas(true, true, true),
            vec![
                FocusArea::Sidebar,
                FocusArea::DataGrid,
                FocusArea::ErDiagram,
                FocusArea::SqlEditor,
            ]
        );
    }

    #[test]
    fn focus_cycle_areas_skip_er_diagram_when_hidden() {
        assert_eq!(
            focus_cycle_areas(true, false, true),
            vec![
                FocusArea::Sidebar,
                FocusArea::DataGrid,
                FocusArea::SqlEditor
            ]
        );
    }
}
