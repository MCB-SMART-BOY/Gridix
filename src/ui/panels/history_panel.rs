use crate::core::QueryHistory;
use crate::ui::styles::{DANGER, GRAY, SUCCESS};
use crate::ui::{
    LocalShortcut, consume_local_shortcut, local_shortcut_text, local_shortcut_tooltip,
    local_shortcuts_text, local_shortcuts_tooltip,
};
use egui::{self, RichText};

#[derive(Default)]
pub struct HistoryPanelState {
    pub selected_index: usize,
}

pub struct HistoryPanel;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HistoryKeyAction {
    Close,
    Clear,
    Prev,
    Next,
    Start,
    End,
    PageUp,
    PageDown,
    UseSelected,
}

impl HistoryPanel {
    fn detect_key_action(ctx: &egui::Context) -> Option<HistoryKeyAction> {
        ctx.input_mut(|i| {
            if consume_local_shortcut(i, LocalShortcut::Dismiss) {
                return Some(HistoryKeyAction::Close);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryClear) {
                return Some(HistoryKeyAction::Clear);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryPrev) {
                return Some(HistoryKeyAction::Prev);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryNext) {
                return Some(HistoryKeyAction::Next);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryStart) {
                return Some(HistoryKeyAction::Start);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryEnd) {
                return Some(HistoryKeyAction::End);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryPageUp) {
                return Some(HistoryKeyAction::PageUp);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryPageDown) {
                return Some(HistoryKeyAction::PageDown);
            }
            if consume_local_shortcut(i, LocalShortcut::HistoryUse) {
                return Some(HistoryKeyAction::UseSelected);
            }
            None
        })
    }

    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        history: &QueryHistory,
        selected_sql: &mut Option<String>,
        clear_history: &mut bool,
        state: &mut HistoryPanelState,
    ) {
        if !*show {
            return;
        }

        let len = history.len();
        if len == 0 {
            state.selected_index = 0;
        } else if state.selected_index >= len {
            state.selected_index = len - 1;
        }

        match Self::detect_key_action(ctx) {
            Some(HistoryKeyAction::Close) => {
                *show = false;
                return;
            }
            Some(HistoryKeyAction::Clear) => {
                *clear_history = true;
            }
            Some(HistoryKeyAction::Prev) if len > 0 => {
                state.selected_index = state.selected_index.saturating_sub(1);
            }
            Some(HistoryKeyAction::Next) if len > 0 => {
                state.selected_index = (state.selected_index + 1).min(len - 1);
            }
            Some(HistoryKeyAction::Start) if len > 0 => {
                state.selected_index = 0;
            }
            Some(HistoryKeyAction::End) if len > 0 => {
                state.selected_index = len - 1;
            }
            Some(HistoryKeyAction::PageUp) if len > 0 => {
                state.selected_index = state.selected_index.saturating_sub(10);
            }
            Some(HistoryKeyAction::PageDown) if len > 0 => {
                state.selected_index = (state.selected_index + 10).min(len - 1);
            }
            Some(HistoryKeyAction::UseSelected) if len > 0 => {
                if let Some(item) = history.items().get(state.selected_index) {
                    *selected_sql = Some(item.sql.clone());
                    *show = false;
                    return;
                }
            }
            _ => {}
        }

        egui::Window::new("查询历史")
            .collapsible(true)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} 条记录", history.len()));
                    ui.label(
                        RichText::new(format!(
                            "{} 导航 | {} 使用 | {} 关闭",
                            local_shortcuts_text(&[
                                LocalShortcut::HistoryPrev,
                                LocalShortcut::HistoryNext,
                            ]),
                            local_shortcuts_text(&[LocalShortcut::HistoryUse]),
                            local_shortcuts_text(&[LocalShortcut::Dismiss]),
                        ))
                        .small()
                        .color(GRAY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(format!(
                                "关闭 [{}]",
                                local_shortcut_text(LocalShortcut::Dismiss)
                            ))
                            .on_hover_text(local_shortcut_tooltip(
                                "关闭历史面板",
                                LocalShortcut::Dismiss,
                            ))
                            .clicked()
                        {
                            *show = false;
                        }
                        if ui
                            .add_enabled(
                                !history.is_empty(),
                                egui::Button::new(format!(
                                    "清空 [{}]",
                                    local_shortcut_text(LocalShortcut::HistoryClear)
                                )),
                            )
                            .on_hover_text(local_shortcut_tooltip(
                                "清空查询历史",
                                LocalShortcut::HistoryClear,
                            ))
                            .clicked()
                        {
                            *clear_history = true;
                        }
                    });
                });

                ui.separator();

                if history.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(20.0);
                        ui.label(RichText::new("暂无查询历史").italics().color(GRAY));
                    });
                    return;
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (idx, item) in history.items().iter().enumerate() {
                        let is_selected = idx == state.selected_index;
                        let bg_color = if is_selected {
                            ui.visuals().selection.bg_fill
                        } else {
                            ui.visuals().extreme_bg_color
                        };

                        let frame = egui::Frame::NONE
                            .inner_margin(8.0)
                            .corner_radius(4.0)
                            .fill(bg_color);

                        let response = frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                // 状态图标 - 使用图标+文字双重指示，对色盲友好
                                if item.success {
                                    ui.colored_label(SUCCESS, "[OK] 成功");
                                } else {
                                    ui.colored_label(DANGER, "[X] 失败");
                                }

                                ui.separator();

                                // 数据库类型
                                ui.label(RichText::new(&item.database_type).small());

                                ui.separator();

                                // 时间戳
                                ui.label(
                                    RichText::new(item.timestamp.format("%H:%M:%S").to_string())
                                        .small()
                                        .color(GRAY),
                                );

                                // 影响行数
                                if let Some(rows) = item.rows_affected {
                                    ui.separator();
                                    ui.label(RichText::new(format!("{} 行", rows)).small());
                                }
                            });

                            // SQL 预览
                            let sql_preview = if item.sql.len() > 100 {
                                format!("{}...", &item.sql[..100])
                            } else {
                                item.sql.clone()
                            };

                            ui.add_space(4.0);
                            let response = ui.add(
                                egui::Label::new(
                                    RichText::new(&sql_preview).monospace().size(12.0),
                                )
                                .sense(egui::Sense::click()),
                            );

                            if response.clicked() {
                                *selected_sql = Some(item.sql.clone());
                                *show = false;
                            }

                            if response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }

                            response.on_hover_text(local_shortcuts_tooltip(
                                "使用这条查询",
                                &[LocalShortcut::HistoryUse],
                            ));
                        });

                        // 点击整个条目也可以选择
                        if response.response.clicked() {
                            state.selected_index = idx;
                        }

                        // 双击执行
                        if response.response.double_clicked() {
                            *selected_sql = Some(item.sql.clone());
                            *show = false;
                        }

                        if idx < history.len() - 1 {
                            ui.add_space(4.0);
                        }
                    }
                });
            });
    }
}
