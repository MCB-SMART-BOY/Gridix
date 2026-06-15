//! 多 Tab 查询窗口组件
//!
//! Tab 栏的 UI 渲染。纯数据类型（QueryTab, QueryTabManager）在 `src/session/tab.rs`。

// 向后兼容重导出
pub use crate::session::tab::{QueryTab, QueryTabManager};

use crate::core::HighlightColors;
use crate::core::{Action, KeyBindings};
use crate::ui::styles::theme_text;
use crate::ui::{LocalShortcut, action_tooltip, consume_local_shortcut};
use egui::{self, Color32, RichText, Ui, Vec2};

// ============================================================================
// Tab 栏 UI 组件
// ============================================================================

/// Tab栏焦点转移方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabBarFocusTransfer {
    /// 转移到工具栏
    ToToolbar,
    /// 转移到数据表格
    ToDataGrid,
}

/// Tab 栏操作
#[derive(Default)]
pub struct TabBarActions {
    /// 新建 Tab
    pub new_tab: bool,
    /// 关闭指定 Tab
    pub close_tab: Option<usize>,
    /// 切换到指定 Tab
    pub switch_to: Option<usize>,
    /// 关闭其他
    pub close_others: bool,
    /// 关闭右侧
    pub close_right: bool,
    /// 焦点转移
    pub focus_transfer: Option<TabBarFocusTransfer>,
}

/// Tab 栏 UI
pub struct QueryTabBar;

impl QueryTabBar {
    /// 显示 Tab 栏
    pub fn show(
        ui: &mut Ui,
        tabs: &[QueryTab],
        active_index: usize,
        highlight_colors: &HighlightColors,
        keybindings: &KeyBindings,
    ) -> TabBarActions {
        let mut actions = TabBarActions::default();

        ui.horizontal(|ui| {
            // Tab 按钮
            for (idx, tab) in tabs.iter().enumerate() {
                let is_active = idx == active_index;

                // Tab 背景色
                let bg_color = if is_active {
                    Color32::from_rgba_unmultiplied(
                        highlight_colors.keyword.r(),
                        highlight_colors.keyword.g(),
                        highlight_colors.keyword.b(),
                        40,
                    )
                } else {
                    Color32::TRANSPARENT
                };

                let frame = egui::Frame::NONE
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .corner_radius(egui::CornerRadius::same(4));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // 状态图标
                        if tab.executing {
                            ui.spinner();
                        } else if tab.modified {
                            ui.label(RichText::new("*").color(highlight_colors.number).small());
                        }

                        // Tab 标题
                        let title_color = if is_active {
                            highlight_colors.keyword
                        } else {
                            highlight_colors.default
                        };

                        let title_response = ui.add(
                            egui::Label::new(RichText::new(&tab.title).color(title_color).small())
                                .sense(egui::Sense::click()),
                        );

                        if title_response.clicked() {
                            actions.switch_to = Some(idx);
                        }

                        // 右键菜单
                        title_response.context_menu(|ui| {
                            let menu_text_color = theme_text(ui.visuals());
                            let menu_btn = |ui: &mut Ui, text: &str, tooltip: &str| -> bool {
                                ui.add(
                                    egui::Button::new(
                                        RichText::new(text).size(13.0).color(menu_text_color),
                                    )
                                    .frame(false)
                                    .min_size(Vec2::new(0.0, 24.0)),
                                )
                                .on_hover_text(tooltip)
                                .clicked()
                            };

                            if menu_btn(ui, "✕ 关闭", "关闭此标签") {
                                actions.close_tab = Some(idx);
                                ui.close();
                            }
                            if menu_btn(ui, "◎ 关闭其他", "关闭其他标签") {
                                actions.close_others = true;
                                ui.close();
                            }
                            if menu_btn(ui, "▷ 关闭右侧", "关闭右侧标签") {
                                actions.close_right = true;
                                ui.close();
                            }
                        });

                        // 关闭按钮 - 无边框图标
                        if tabs.len() > 1 {
                            let close_response = ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("×")
                                            .size(12.0)
                                            .color(highlight_colors.comment),
                                    )
                                    .frame(false)
                                    .min_size(Vec2::new(18.0, 18.0)),
                                )
                                .on_hover_text(action_tooltip(keybindings, Action::CloseTab));

                            if close_response.clicked() {
                                actions.close_tab = Some(idx);
                            }
                        }
                    });
                });

                // Tab 分隔符
                if idx < tabs.len() - 1 {
                    ui.separator();
                }
            }

            // 新建 Tab 按钮 - 无边框图标
            ui.add_space(4.0);
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+")
                            .size(14.0)
                            .color(theme_text(ui.visuals())),
                    )
                    .frame(false)
                    .min_size(Vec2::new(22.0, 22.0)),
                )
                .on_hover_text(action_tooltip(keybindings, Action::NewTab))
                .clicked()
            {
                actions.new_tab = true;
            }
        });

        actions
    }

    /// 处理Tab栏键盘输入 (Helix风格)
    ///
    /// - h/l: 左右切换Tab
    /// - j: 向下进入数据表格
    /// - k: 向上进入工具栏
    /// - d: 删除当前Tab
    /// - Enter: 确认选择当前Tab（进入数据表格）
    pub fn handle_keyboard(
        ui: &mut Ui,
        tab_count: usize,
        active_index: usize,
        actions: &mut TabBarActions,
    ) {
        if tab_count == 0 {
            return;
        }

        ui.input_mut(|i| {
            if consume_local_shortcut(i, LocalShortcut::QueryTabPrev) && active_index > 0 {
                actions.switch_to = Some(active_index - 1);
            } else if consume_local_shortcut(i, LocalShortcut::QueryTabNext)
                && active_index < tab_count - 1
            {
                actions.switch_to = Some(active_index + 1);
            } else if consume_local_shortcut(i, LocalShortcut::QueryTabToDataGrid) {
                actions.focus_transfer = Some(TabBarFocusTransfer::ToDataGrid);
            } else if consume_local_shortcut(i, LocalShortcut::QueryTabToToolbar) {
                actions.focus_transfer = Some(TabBarFocusTransfer::ToToolbar);
            } else if consume_local_shortcut(i, LocalShortcut::QueryTabActivate) {
                actions.focus_transfer = Some(TabBarFocusTransfer::ToDataGrid);
            } else if consume_local_shortcut(i, LocalShortcut::QueryTabClose) && tab_count > 1 {
                actions.close_tab = Some(active_index);
            } else if consume_local_shortcut(i, LocalShortcut::QueryTabDismiss) {
                actions.focus_transfer = Some(TabBarFocusTransfer::ToDataGrid);
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

    fn run_tab_bar_key(key: Key, tab_count: usize, active_index: usize) -> TabBarActions {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            events: vec![key_event(key)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
        let mut actions = TabBarActions::default();
        Area::new(Id::new("query_tab_keyboard_test")).show(&ctx, |ui| {
            QueryTabBar::handle_keyboard(ui, tab_count, active_index, &mut actions);
        });
        let _ = ctx.end_pass();
        actions
    }

    #[test]
    fn query_tab_keyboard_uses_local_shortcuts_for_focus_transfer() {
        let actions = run_tab_bar_key(Key::J, 3, 1);
        assert_eq!(
            actions.focus_transfer,
            Some(TabBarFocusTransfer::ToDataGrid)
        );

        let actions = run_tab_bar_key(Key::K, 3, 1);
        assert_eq!(actions.focus_transfer, Some(TabBarFocusTransfer::ToToolbar));
    }

    #[test]
    fn query_tab_keyboard_close_is_local_shortcut_driven() {
        let actions = run_tab_bar_key(Key::D, 3, 1);
        assert_eq!(actions.close_tab, Some(1));
    }
}
