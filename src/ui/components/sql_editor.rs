//! SQL 编辑器组件
//! 左侧：SQL 输入区域（支持语法高亮、自动补全）
//! 右侧：历史记录列表
//! 底部：执行状态栏（显示成功/失败消息和耗时）

#![allow(clippy::too_many_arguments)]

use crate::core::{highlight_sql, AutoComplete, CompletionKind, HighlightColors};
use crate::ui::styles::GRAY;
use egui::{self, Align, Color32, Key, Layout, PopupCloseBehavior, RichText, ScrollArea, TextEdit};

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
    /// 请求焦点转移到数据表格（Escape 键或向上移动时）
    pub focus_to_grid: bool,
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

        // 预先获取并固定可用空间，防止后续布局改变它
        let total_rect = ui.available_rect_before_wrap();
        let available_width = total_rect.width();
        let available_height = total_rect.height();
        
        // 预分配整个空间，防止面板自动增长
        let (_id, allocated_rect) = ui.allocate_space(egui::vec2(available_width, available_height));
        
        // 在分配的空间内绘制
        let mut child_ui = ui.new_child(egui::UiBuilder::new().max_rect(allocated_rect));
        let ui = &mut child_ui;

        let frame = egui::Frame::none().inner_margin(egui::Margin::symmetric(8.0, 6.0));

        frame.show(ui, |ui| {
            // 状态栏高度
            let status_bar_height = 24.0;
            // 左右分栏：70% SQL编辑器，30% 历史记录
            let inner_width = available_width - 16.0; // 减去 margin
            let inner_height = available_height - 12.0; // 减去 margin
            let editor_width = inner_width * 0.70 - 4.0;
            let history_width = inner_width * 0.30 - 4.0;
            // 主内容高度（减去状态栏）
            let content_height = (inner_height - status_bar_height - 2.0).max(50.0);

            // ========== 主内容区域 ==========
            ui.allocate_ui_with_layout(
                egui::vec2(available_width, content_height),
                egui::Layout::left_to_right(egui::Align::TOP),
                |ui| {
                    // ========== 左侧：SQL 编辑区域 ==========
                    ui.allocate_ui_with_layout(
                        egui::vec2(editor_width, content_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            // 工具栏
                            ui.horizontal(|ui| {
                                // 简洁的标签
                                ui.label(
                                    RichText::new("SQL").small().color(highlight_colors.comment),
                                );

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
                                    },
                                );
                            });

                            ui.add_space(2.0);

                        // SQL 编辑区域 - 不显示行号，简化布局
                        let editor_height = ui.available_height();
                        
                        // SQL 输入框的 layouter
                        let mut layouter = |ui: &egui::Ui, text: &str, wrap_width: f32| {
                            let mut job = highlight_sql(text, highlight_colors);
                            job.wrap.max_width = wrap_width;
                            ui.fonts(|f| f.layout_job(job))
                        };

                        // 编辑器滚动区域
                        ScrollArea::vertical()
                            .id_salt("sql_editor_scroll")
                            .auto_shrink([false, false])
                            .max_height(editor_height)
                            .show(ui, |ui| {
                                    // SQL 编辑器 - 占满宽度
                                    let response = ui.add(
                                        TextEdit::multiline(sql_input)
                                            .font(egui::TextStyle::Monospace)
                                            .desired_width(ui.available_width())
                                            .desired_rows(8)
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
                    },
                );

                    ui.separator();

                    // ========== 右侧：历史记录 ==========
                    ui.allocate_ui_with_layout(
                        egui::vec2(history_width, content_height),
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
                            .id_salt("history_list")
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
                },
            );

            // ========== 底部：状态栏 ==========
            Self::show_status_bar(
                ui,
                is_executing,
                last_message,
                query_time_ms,
                highlight_colors,
            );
        });

        actions
    }

    /// 显示状态栏 - SQL 执行结果和耗时
    fn show_status_bar(
        ui: &mut egui::Ui,
        is_executing: bool,
        last_message: &Option<String>,
        query_time_ms: Option<u64>,
        highlight_colors: &HighlightColors,
    ) {
        egui::Frame::none()
            .fill(ui.style().visuals.extreme_bg_color)
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.set_height(18.0);
                ui.horizontal(|ui| {
                    if is_executing {
                        ui.spinner();
                        ui.label(RichText::new("正在执行...").small());
                    } else if let Some(msg) = last_message {
                        let is_error = msg.contains("错误")
                            || msg.contains("Error")
                            || msg.contains("失败")
                            || msg.contains("failed");
                        let (icon, color) = if is_error {
                            ("✗", highlight_colors.operator)
                        } else {
                            ("✓", highlight_colors.string)
                        };
                        ui.label(RichText::new(icon).color(color).size(14.0));
                        ui.label(RichText::new(msg).color(color).small());
                    } else {
                        ui.label(RichText::new("就绪").small().color(GRAY));
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if let Some(ms) = query_time_ms {
                            ui.label(
                                RichText::new(format!("耗时: {}ms", ms))
                                    .small()
                                    .color(GRAY),
                            );
                        }
                    });
                });
            });
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
            } else if i.key_pressed(Key::Escape) {
                // 没有补全菜单时，Escape 返回数据表格
                actions.focus_to_grid = true;
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
            egui::popup::popup_below_widget(ui, popup_id, response, PopupCloseBehavior::CloseOnClickOutside, |ui| {
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
