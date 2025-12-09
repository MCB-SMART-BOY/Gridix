//! SQL 编辑器组件
//! 左侧：SQL 输入区域（支持语法高亮、自动补全）
//! 右侧：历史记录列表

#![allow(clippy::too_many_arguments)]

use crate::core::{highlight_sql, AutoComplete, CompletionKind, HighlightColors};
use crate::ui::styles::GRAY;
use egui::{self, Color32, Key, RichText, ScrollArea, TextEdit};

pub struct SqlEditor;

/// 应用自动补全 - 替换当前正在输入的单词
fn apply_completion(text: &mut String, insert_text: &str) {
    let mut word_start = text.len();
    for (i, c) in text.char_indices().rev() {
        if c.is_alphanumeric() || c == '_' {
            word_start = i;
        } else {
            break;
        }
    }
    text.truncate(word_start);
    text.push_str(insert_text);
    text.push(' ');
}

/// SQL 编辑器操作
#[derive(Default)]
pub struct SqlEditorActions {
    pub execute: bool,
    pub format: bool,
    pub clear: bool,
}

impl SqlEditor {
    /// 显示 SQL 编辑器（左侧输入，右侧历史）
    pub fn show(
        ui: &mut egui::Ui,
        sql_input: &mut String,
        command_history: &[String],
        history_index: &mut Option<usize>,
        is_executing: bool,
        last_message: &Option<String>,
        highlight_colors: &HighlightColors,
        query_time_ms: Option<u64>,
        autocomplete: &AutoComplete,
        show_autocomplete: &mut bool,
        selected_completion: &mut usize,
        request_focus: &mut bool,
    ) -> SqlEditorActions {
        let mut actions = SqlEditorActions::default();

        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 6.0));

        frame.show(ui, |ui| {
            let available_width = ui.available_width();
            let available_height = ui.available_height();

            // 左右分栏：60% SQL编辑器，40% 历史记录
            let editor_width = available_width * 0.6 - 8.0;
            let history_width = available_width * 0.4 - 8.0;

            ui.horizontal(|ui| {
                // ========== 左侧：SQL 编辑区域 ==========
                ui.allocate_ui_with_layout(
                    egui::vec2(editor_width, available_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        // 工具栏
                        ui.horizontal(|ui| {
                            // 状态信息
                            if let Some(msg) = last_message {
                                let (icon, color) = if msg.contains("错误") || msg.contains("Error")
                                {
                                    ("✗", highlight_colors.operator)
                                } else {
                                    ("✓", highlight_colors.string)
                                };
                                ui.label(RichText::new(icon).color(color));
                                let display_msg = if msg.len() > 40 {
                                    format!("{}...", &msg[..40])
                                } else {
                                    msg.clone()
                                };
                                ui.label(RichText::new(display_msg).color(color).small());
                            } else {
                                ui.label(
                                    RichText::new("SQL").small().color(highlight_colors.comment),
                                );
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // 执行按钮
                                    let execute_btn = ui.add_enabled(
                                        !is_executing && !sql_input.trim().is_empty(),
                                        egui::Button::new(if is_executing {
                                            "..."
                                        } else {
                                            "▶ 执行 [Enter]"
                                        }),
                                    );
                                    if execute_btn.clicked() {
                                        actions.execute = true;
                                    }

                                    // 清空按钮
                                    if ui
                                        .add_enabled(
                                            !sql_input.is_empty(),
                                            egui::Button::new("清空 [Ctrl+L]"),
                                        )
                                        .clicked()
                                    {
                                        actions.clear = true;
                                    }

                                    // 耗时
                                    if is_executing {
                                        ui.spinner();
                                    } else if let Some(ms) = query_time_ms {
                                        ui.label(
                                            RichText::new(format!("{}ms", ms))
                                                .small()
                                                .color(highlight_colors.comment),
                                        );
                                    }
                                },
                            );
                        });

                        ui.add_space(2.0);

                        // SQL 编辑区域
                        let editor_height = ui.available_height();

                        ui.horizontal(|ui| {
                            // 行号
                            let line_count = sql_input.lines().count().max(1);
                            let line_numbers: String = (1..=line_count)
                                .map(|n| format!("{:2}", n))
                                .collect::<Vec<_>>()
                                .join("\n");

                            ui.allocate_ui_with_layout(
                                egui::vec2(24.0, editor_height),
                                egui::Layout::top_down(egui::Align::RIGHT),
                                |ui| {
                                    ScrollArea::vertical()
                                        .id_source("line_numbers")
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            ui.label(
                                                RichText::new(&line_numbers)
                                                    .monospace()
                                                    .color(highlight_colors.comment),
                                            );
                                        });
                                },
                            );

                            // SQL 输入框
                            let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                                let mut job = highlight_sql(text, highlight_colors);
                                job.wrap.max_width = wrap_width;
                                ui.fonts(|f| f.layout_job(job))
                            };

                            ScrollArea::vertical()
                                .id_source("sql_editor")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let response = ui.add_sized(
                                        [ui.available_width(), editor_height - 4.0],
                                        TextEdit::multiline(sql_input)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(f32::INFINITY)
                                            .hint_text("输入 SQL... (Enter 执行)")
                                            .layouter(&mut layouter),
                                    );

                                    // 如果请求聚焦，则聚焦到编辑器
                                    if *request_focus {
                                        response.request_focus();
                                        *request_focus = false;
                                    }

                                    // 处理快捷键
                                    if response.has_focus() {
                                        Self::handle_shortcuts(
                                            ui,
                                            sql_input,
                                            command_history,
                                            history_index,
                                            &mut actions,
                                            autocomplete,
                                            show_autocomplete,
                                            selected_completion,
                                            highlight_colors,
                                        );
                                    } else {
                                        *show_autocomplete = false;
                                    }

                                    // 自动补全弹窗
                                    Self::show_autocomplete_popup(
                                        ui,
                                        &response,
                                        sql_input,
                                        autocomplete,
                                        show_autocomplete,
                                        selected_completion,
                                        highlight_colors,
                                    );
                                });
                        });
                    },
                );

                ui.separator();

                // ========== 右侧：历史记录 ==========
                ui.allocate_ui_with_layout(
                    egui::vec2(history_width, available_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        // 标题栏
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("历史记录").small().strong());
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        RichText::new(format!("{} 条", command_history.len()))
                                            .small()
                                            .color(GRAY),
                                    );
                                },
                            );
                        });

                        ui.add_space(2.0);

                        // 历史列表
                        let history_height = ui.available_height();
                        ScrollArea::vertical()
                            .id_source("history_list")
                            .auto_shrink([false, false])
                            .max_height(history_height)
                            .show(ui, |ui| {
                                if command_history.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(20.0);
                                        ui.label(RichText::new("暂无历史记录").small().color(GRAY));
                                    });
                                } else {
                                    for (idx, sql) in command_history.iter().enumerate() {
                                        let is_selected = *history_index == Some(idx);
                                        let bg_color = if is_selected {
                                            Color32::from_rgba_unmultiplied(100, 100, 150, 60)
                                        } else {
                                            Color32::TRANSPARENT
                                        };

                                        egui::Frame::none()
                                            .fill(bg_color)
                                            .inner_margin(4.0)
                                            .rounding(2.0)
                                            .show(ui, |ui| {
                                                // 截断显示
                                                let display_sql = if sql.len() > 60 {
                                                    format!(
                                                        "{}...",
                                                        sql.chars().take(60).collect::<String>()
                                                    )
                                                } else {
                                                    sql.replace('\n', " ")
                                                };

                                                let response = ui.add(
                                                    egui::Label::new(
                                                        RichText::new(&display_sql)
                                                            .small()
                                                            .monospace()
                                                            .color(if is_selected {
                                                                highlight_colors.keyword
                                                            } else {
                                                                Color32::LIGHT_GRAY
                                                            }),
                                                    )
                                                    .sense(egui::Sense::click()),
                                                );

                                                if response.clicked() {
                                                    *sql_input = sql.clone();
                                                    *history_index = Some(idx);
                                                }

                                                // 双击执行
                                                if response.double_clicked() {
                                                    *sql_input = sql.clone();
                                                    actions.execute = true;
                                                }

                                                response.on_hover_text(sql);
                                            });

                                        ui.add_space(1.0);
                                    }
                                }
                            });
                    },
                );
            });
        });

        actions
    }

    /// 处理快捷键
    fn handle_shortcuts(
        ui: &mut egui::Ui,
        sql_input: &mut String,
        command_history: &[String],
        history_index: &mut Option<usize>,
        actions: &mut SqlEditorActions,
        autocomplete: &AutoComplete,
        show_autocomplete: &mut bool,
        selected_completion: &mut usize,
        _highlight_colors: &HighlightColors,
    ) {
        let completions = autocomplete.get_completions(sql_input, sql_input.len());
        let has_completions = !completions.is_empty();

        // Enter 执行
        let enter_to_execute =
            ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.ctrl && !i.modifiers.shift);

        if enter_to_execute && !sql_input.trim().is_empty() {
            while sql_input.ends_with('\n') {
                sql_input.pop();
            }
            actions.execute = true;
            *show_autocomplete = false;
        }

        ui.input(|i| {
            // Ctrl+Space 触发补全
            if i.modifiers.ctrl && i.key_pressed(Key::Space) {
                *show_autocomplete = true;
                *selected_completion = 0;
            }

            // 补全菜单导航
            if *show_autocomplete && has_completions {
                if i.key_pressed(Key::ArrowDown) {
                    *selected_completion =
                        (*selected_completion + 1).min(completions.len().saturating_sub(1));
                }
                if i.key_pressed(Key::ArrowUp) {
                    *selected_completion = selected_completion.saturating_sub(1);
                }
                if i.key_pressed(Key::Escape) {
                    *show_autocomplete = false;
                }
            }

            // F5 执行
            if i.key_pressed(Key::F5) && !sql_input.trim().is_empty() {
                actions.execute = true;
                *show_autocomplete = false;
            }

            // Ctrl+L 清空
            if i.modifiers.ctrl && i.key_pressed(Key::L) {
                actions.clear = true;
                *show_autocomplete = false;
            }

            // Shift+↑↓ 历史导航
            if i.modifiers.shift && !*show_autocomplete {
                if i.key_pressed(Key::ArrowUp) && !command_history.is_empty() {
                    let new_idx = match *history_index {
                        None => Some(0),
                        Some(idx) if idx + 1 < command_history.len() => Some(idx + 1),
                        Some(idx) => Some(idx),
                    };
                    if let Some(idx) = new_idx {
                        *history_index = Some(idx);
                        *sql_input = command_history[idx].clone();
                    }
                }

                if i.key_pressed(Key::ArrowDown) {
                    match *history_index {
                        Some(0) => {
                            *history_index = None;
                            sql_input.clear();
                        }
                        Some(idx) => {
                            *history_index = Some(idx - 1);
                            *sql_input = command_history[idx - 1].clone();
                        }
                        None => {}
                    }
                }
            }
        });
    }

    /// 显示自动补全弹窗
    fn show_autocomplete_popup(
        ui: &mut egui::Ui,
        response: &egui::Response,
        sql_input: &mut String,
        autocomplete: &AutoComplete,
        show_autocomplete: &mut bool,
        selected_completion: &mut usize,
        highlight_colors: &HighlightColors,
    ) {
        let completions = autocomplete.get_completions(sql_input, sql_input.len());

        if *show_autocomplete && !completions.is_empty() {
            let popup_id = ui.make_persistent_id("autocomplete_popup");
            egui::popup::popup_below_widget(ui, popup_id, response, |ui| {
                ui.set_min_width(200.0);
                ui.set_max_height(150.0);

                ScrollArea::vertical().show(ui, |ui| {
                    for (idx, item) in completions.iter().enumerate() {
                        let is_selected = idx == *selected_completion;
                        let bg = if is_selected {
                            Color32::from_rgba_unmultiplied(
                                highlight_colors.keyword.r(),
                                highlight_colors.keyword.g(),
                                highlight_colors.keyword.b(),
                                60,
                            )
                        } else {
                            Color32::TRANSPARENT
                        };

                        egui::Frame::none()
                            .fill(bg)
                            .inner_margin(2.0)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let icon_color = match item.kind {
                                        CompletionKind::Keyword => highlight_colors.keyword,
                                        CompletionKind::Function => highlight_colors.function,
                                        CompletionKind::Table => highlight_colors.string,
                                        CompletionKind::Column => highlight_colors.identifier,
                                    };
                                    ui.label(
                                        RichText::new(item.kind.icon())
                                            .color(icon_color)
                                            .monospace()
                                            .small(),
                                    );

                                    let resp = ui.selectable_label(
                                        is_selected,
                                        RichText::new(&item.label).small(),
                                    );
                                    if resp.clicked() {
                                        apply_completion(sql_input, &item.insert_text);
                                        *show_autocomplete = false;
                                    }
                                });
                            });
                    }
                });
            });
            ui.memory_mut(|m| m.open_popup(popup_id));
        }
    }
}
