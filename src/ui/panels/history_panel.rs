use crate::core::QueryHistory;
use crate::ui::dialogs::keyboard;
use crate::ui::styles::{DANGER, GRAY, SUCCESS};
use egui::{self, Key, RichText};

pub struct HistoryPanelState {
    pub selected_index: usize,
}

impl Default for HistoryPanelState {
    fn default() -> Self {
        Self { selected_index: 0 }
    }
}

pub struct HistoryPanel;

impl HistoryPanel {
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

        // Helix 键盘导航
        if !keyboard::has_text_focus(ctx) {
            let len = history.len();
            
            // Esc/q 关闭
            if keyboard::handle_close_keys(ctx) {
                *show = false;
                return;
            }
            
            // Ctrl+Delete 清空历史
            if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Delete)) {
                *clear_history = true;
            }
            
            if len > 0 {
                // j/k 或 ↑/↓ 导航
                if ctx.input(|i| i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown)) {
                    state.selected_index = (state.selected_index + 1).min(len - 1);
                }
                if ctx.input(|i| i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp)) {
                    state.selected_index = state.selected_index.saturating_sub(1);
                }
                
                // g/G 跳转到首/尾
                if ctx.input(|i| i.key_pressed(Key::G) && !i.modifiers.shift) {
                    state.selected_index = 0;
                }
                if ctx.input(|i| i.key_pressed(Key::G) && i.modifiers.shift) {
                    state.selected_index = len - 1;
                }
                
                // Ctrl+u/d 翻页
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::U)) {
                    state.selected_index = state.selected_index.saturating_sub(10);
                }
                if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::D)) {
                    state.selected_index = (state.selected_index + 10).min(len - 1);
                }
                
                // Enter/l 选择当前项
                if ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::L)) {
                    if let Some(item) = history.items().get(state.selected_index) {
                        *selected_sql = Some(item.sql.clone());
                        *show = false;
                        return;
                    }
                }
            }
        }

        egui::Window::new("查询历史 [j/k 导航, Enter 选择, Esc 关闭]")
            .collapsible(true)
            .resizable(true)
            .default_size([500.0, 400.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(format!("{} 条记录", history.len()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("关闭 [Esc]").clicked() {
                            *show = false;
                        }
                        if ui
                            .add_enabled(!history.is_empty(), egui::Button::new("清空 [Ctrl+Del]"))
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

                            response.on_hover_text("点击使用此查询");
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
