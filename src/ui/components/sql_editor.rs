//! SQL 编辑器组件 - Helix 风格双模式
//!
//! 特性：
//! - Normal 模式：hjkl 移动，w/b 词跳转，Helix 风格导航
//! - Insert 模式：双击进入，正常输入
//! - Ctrl+Enter / F5 执行 SQL
//! - 语法高亮 + 自动补全

#![allow(clippy::too_many_arguments)]

use crate::core::{AutoComplete, CompletionKind, HighlightColors, highlight_sql};
use crate::ui::styles::{
    GRAY, theme_disabled_text, theme_muted_text, theme_subtle_stroke, theme_text,
};
use crate::ui::{
    LocalShortcut, consume_local_shortcut, local_shortcut_text, local_shortcut_tooltip,
};
use egui::{
    self, Align, Color32, Key, Layout, Modifiers, PopupCloseBehavior, RichText, ScrollArea,
    TextEdit, Vec2,
};

/// 行号区域宽度
const LINE_NUMBER_WIDTH: f32 = 45.0;

/// 编辑器模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    Normal,
    #[default]
    Insert,
}

impl EditorMode {
    pub fn label(&self) -> &'static str {
        match self {
            EditorMode::Normal => "NOR",
            EditorMode::Insert => "INS",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            EditorMode::Normal => Color32::from_rgb(130, 170, 255), // 蓝色
            EditorMode::Insert => Color32::from_rgb(180, 230, 140), // 绿色
        }
    }
}

pub struct SqlEditor;

#[derive(Debug, Default, Clone, Copy)]
struct CompletionKeyInput {
    up: bool,
    down: bool,
    confirm_tab: bool,
    confirm_enter: bool,
}

impl CompletionKeyInput {
    fn any(self) -> bool {
        self.up || self.down || self.confirm_tab || self.confirm_enter
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InsertCancelOutcome {
    next_mode: EditorMode,
    show_autocomplete: bool,
    keep_editor_focus: bool,
    escape_consumed: bool,
}

/// 将字符索引（egui 光标）转换为字符串字节索引
fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    if char_index == 0 {
        return 0;
    }
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}

/// 应用自动补全（在光标位置插入）
fn apply_completion_at_cursor(text: &mut String, cursor_pos: usize, insert_text: &str) -> usize {
    let pos = char_to_byte_index(text, cursor_pos);
    let text_before = &text[..pos];

    // 找到当前单词的开始位置
    let mut word_start = pos;
    for (i, c) in text_before.char_indices().rev() {
        if c.is_alphanumeric() || c == '_' {
            word_start = i;
        } else {
            break;
        }
    }

    // 替换当前单词
    let text_after = &text[pos..];
    let new_text = format!("{}{} {}", &text[..word_start], insert_text, text_after);
    *text = new_text;

    // 返回新的光标位置（字符索引）
    let new_cursor_byte = (word_start + insert_text.len() + 1).min(text.len());
    text[..new_cursor_byte].chars().count()
}

/// 计算文本的行数
fn count_lines(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.lines().count().max(1) + if text.ends_with('\n') { 1 } else { 0 }
    }
}

/// 计算光标所在的行和列
fn get_cursor_position(text: &str, cursor_pos: usize) -> (usize, usize) {
    let byte_pos = char_to_byte_index(text, cursor_pos);
    let text_before_cursor = &text[..byte_pos];
    let line = text_before_cursor.matches('\n').count() + 1;
    let last_newline = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = text_before_cursor[last_newline..].chars().count() + 1;
    (line, column)
}

/// 获取行的起始和结束位置
fn get_line_bounds(text: &str, cursor_pos: usize) -> (usize, usize) {
    let pos = char_to_byte_index(text, cursor_pos);
    let start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[pos..]
        .find('\n')
        .map(|i| pos + i)
        .unwrap_or(text.len());
    (start, end)
}

/// 移动到下一个单词（预留 Helix w 键）
#[allow(dead_code)]
fn next_word_pos(text: &str, cursor_pos: usize) -> usize {
    let pos = cursor_pos.min(text.chars().count());
    let chars: Vec<char> = text.chars().collect();
    let mut i = pos;

    // 跳过当前单词
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    // 跳过空白
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

/// 移动到上一个单词（预留 Helix b 键）
#[allow(dead_code)]
fn prev_word_pos(text: &str, cursor_pos: usize) -> usize {
    let pos = cursor_pos.min(text.chars().count());
    let chars: Vec<char> = text.chars().collect();
    let mut i = pos;

    // 跳过前面的空白
    while i > 0 && chars[i.saturating_sub(1)].is_whitespace() {
        i -= 1;
    }
    // 跳过单词
    while i > 0
        && (chars[i.saturating_sub(1)].is_alphanumeric() || chars[i.saturating_sub(1)] == '_')
    {
        i -= 1;
    }
    i
}

/// SQL 编辑器操作
#[derive(Default)]
pub struct SqlEditorActions {
    pub execute: bool,
    pub format: bool,
    pub clear: bool,
    pub explain: bool,
    pub text_changed: bool,
    pub focus_to_grid: bool,
    pub request_focus: bool,
    /// Escape 键已被编辑器消费（用于退出 Insert 模式）
    pub escape_consumed: bool,
}

impl SqlEditor {
    fn text_edit_id() -> egui::Id {
        egui::Id::new("gridix.sql_editor.text_edit")
    }

    fn apply_insert_mode_cancel(
        show_autocomplete: bool,
        editor_mode: EditorMode,
    ) -> InsertCancelOutcome {
        if show_autocomplete {
            InsertCancelOutcome {
                next_mode: editor_mode,
                show_autocomplete: false,
                keep_editor_focus: true,
                escape_consumed: true,
            }
        } else {
            InsertCancelOutcome {
                next_mode: EditorMode::Normal,
                show_autocomplete,
                keep_editor_focus: true,
                escape_consumed: true,
            }
        }
    }

    fn set_editor_cursor(
        ui: &egui::Ui,
        output: &mut egui::text_edit::TextEditOutput,
        cursor_char_index: usize,
    ) {
        let range = egui::text::CCursorRange::one(egui::text::CCursor::new(cursor_char_index));
        output.state.cursor.set_char_range(Some(range));
        output.state.clone().store(ui.ctx(), output.response.id);
    }

    /// 显示 SQL 编辑器
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
        is_focused: bool,
        editor_mode: &mut EditorMode,
    ) -> SqlEditorActions {
        let mut actions = SqlEditorActions::default();
        let execute_shortcut = local_shortcut_text(LocalShortcut::SqlExecute);
        let cancel_shortcut = local_shortcut_text(LocalShortcut::Cancel);

        let available_height = ui.available_height();
        let available_width = ui.available_width();

        let status_bar_height = 20.0;
        let toolbar_height = 26.0;
        let editor_height = (available_height - status_bar_height - toolbar_height - 8.0).max(60.0);

        // ========== 工具栏 ==========
        Self::show_toolbar(
            ui,
            sql_input,
            is_executing,
            &mut actions,
            toolbar_height,
            *editor_mode,
        );

        ui.add_space(2.0);

        // ========== 编辑器主体 ==========
        let line_count = count_lines(sql_input);
        let line_height = 16.0;
        let mut status_cursor_pos = sql_input.chars().count();
        let text_edit_id = Self::text_edit_id();

        if !is_focused {
            ui.memory_mut(|mem| mem.surrender_focus(text_edit_id));
            *request_focus = false;
            *show_autocomplete = false;
        }

        // 共享滚动状态 ID
        let scroll_id = ui.id().with("editor_scroll");

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            // ===== 行号区域 =====
            egui::Frame::NONE
                .fill(ui.style().visuals.faint_bg_color)
                .show(ui, |ui| {
                    ui.set_width(LINE_NUMBER_WIDTH);
                    ui.set_height(editor_height);

                    let scroll_offset = ui
                        .ctx()
                        .data(|d| d.get_temp::<f32>(scroll_id).unwrap_or(0.0));

                    ScrollArea::vertical()
                        .id_salt("line_numbers")
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .vertical_scroll_offset(scroll_offset)
                        .show(ui, |ui| {
                            ui.set_width(LINE_NUMBER_WIDTH);
                            let display_lines =
                                line_count.max((editor_height / line_height) as usize);
                            for line_num in 1..=display_lines {
                                ui.horizontal(|ui| {
                                    ui.set_height(line_height);
                                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                        ui.add_space(8.0);
                                        ui.label(
                                            RichText::new(format!("{}", line_num))
                                                .monospace()
                                                .size(13.0)
                                                .color(highlight_colors.comment),
                                        );
                                    });
                                });
                            }
                        });
                });

            // ===== 编辑器区域 =====
            let editor_width = (available_width - LINE_NUMBER_WIDTH - 8.0).max(120.0);

            egui::Frame::NONE
                .fill(ui.style().visuals.extreme_bg_color)
                .show(ui, |ui| {
                    ui.set_width(editor_width);
                    ui.set_height(editor_height);

                    let colors = highlight_colors.clone();
                    let mut layouter =
                        |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                            let mut job = highlight_sql(text.as_str(), &colors);
                            job.wrap.max_width = wrap_width;
                            ui.ctx().fonts_mut(|f| f.layout_job(job))
                        };

                    let scroll_output = ScrollArea::vertical()
                        .id_salt("sql_editor_content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Insert 模式：可编辑；Normal 模式：只读显示
                            let mut is_insert_mode = *editor_mode == EditorMode::Insert;

                            if is_focused && is_insert_mode {
                                let cancel_pressed = ui.input_mut(|i| {
                                    consume_local_shortcut(i, LocalShortcut::Cancel)
                                });
                                if cancel_pressed {
                                    let outcome =
                                        Self::apply_insert_mode_cancel(*show_autocomplete, *editor_mode);
                                    *show_autocomplete = outcome.show_autocomplete;
                                    *editor_mode = outcome.next_mode;
                                    is_insert_mode = *editor_mode == EditorMode::Insert;
                                    actions.escape_consumed = outcome.escape_consumed;
                                    actions.request_focus = outcome.keep_editor_focus;
                                    *request_focus = outcome.keep_editor_focus;
                                }
                            }

                            let mut completion_key_input = CompletionKeyInput::default();
                            if is_insert_mode && is_focused {
                                ui.input_mut(|i| {
                                    // Tab 在 Insert 模式下始终由编辑器优先消费，防止焦点循环抢走
                                    completion_key_input.confirm_tab =
                                        i.consume_key(Modifiers::NONE, Key::Tab);
                                    // 仅在补全弹窗打开时，消费上下和回车用于补全导航/确认
                                    if *show_autocomplete {
                                        completion_key_input.down =
                                            i.consume_key(Modifiers::NONE, Key::ArrowDown);
                                        completion_key_input.up =
                                            i.consume_key(Modifiers::NONE, Key::ArrowUp);
                                        completion_key_input.confirm_enter =
                                            i.consume_key(Modifiers::NONE, Key::Enter);
                                    }
                                });
                            }

                            let mut output = TextEdit::multiline(sql_input)
                                .id(text_edit_id)
                                .font(egui::TextStyle::Monospace)
                                .desired_width((editor_width - 16.0).max(80.0))
                                .desired_rows(((editor_height / line_height) as usize).max(4))
                                .hint_text(if is_insert_mode {
                                    format!(
                                        "输入 SQL... ({cancel_shortcut} 退出编辑, {execute_shortcut} 执行)"
                                    )
                                } else {
                                    format!("双击进入编辑模式, {execute_shortcut} 执行")
                                })
                                .frame(egui::Frame::NONE)
                                .margin(Vec2::new(8.0, 0.0))
                                .interactive(is_insert_mode && is_focused)
                                .lock_focus(is_insert_mode && is_focused)
                                .layouter(&mut layouter)
                                .show(ui);

                            let response = output.response.clone();
                            let click_response = if !is_insert_mode || !is_focused {
                                Some(ui.interact(
                                    response.rect,
                                    text_edit_id.with("click_layer"),
                                    egui::Sense::click(),
                                ))
                            } else {
                                None
                            };

                            // 双击进入 Insert 模式
                            if response.double_clicked()
                                || click_response
                                    .as_ref()
                                    .is_some_and(egui::Response::double_clicked)
                            {
                                *editor_mode = EditorMode::Insert;
                                actions.request_focus = true;
                                *request_focus = true;
                            }
                            // 单击也进入 Insert 模式，降低新手上手门槛
                            if (response.clicked()
                                || click_response.as_ref().is_some_and(egui::Response::clicked))
                                && *editor_mode == EditorMode::Normal
                            {
                                *editor_mode = EditorMode::Insert;
                                actions.request_focus = true;
                                *request_focus = true;
                            }

                            // i 键也可进入 Insert 模式（在 Normal 模式下）
                            if *editor_mode == EditorMode::Normal && is_focused {
                                let enter_insert = ui.input(|i| {
                                    i.key_pressed(Key::I)
                                        || i.key_pressed(Key::A)
                                        || i.key_pressed(Key::O)
                                });
                                if enter_insert {
                                    *editor_mode = EditorMode::Insert;
                                    // o 键在当前行后插入新行
                                    if ui.input(|i| i.key_pressed(Key::O)) {
                                        let cursor_pos = output
                                            .cursor_range
                                            .map(|r| r.primary.index)
                                            .unwrap_or(sql_input.chars().count());
                                        let (_, line_end) = get_line_bounds(sql_input, cursor_pos);
                                        sql_input.insert(line_end, '\n');
                                    }
                                }
                            }

                            if *request_focus && is_insert_mode {
                                response.request_focus();
                                *request_focus = false;
                            }

                            if response.clicked()
                                || click_response.as_ref().is_some_and(egui::Response::clicked)
                            {
                                actions.request_focus = true;
                            }

                            // 获取光标位置
                            let cursor_pos = output
                                .cursor_range
                                .map(|range| range.primary.index)
                                .unwrap_or(sql_input.chars().count());
                            status_cursor_pos = cursor_pos;

                            // Insert 模式下的快捷键处理（焦点由应用区块管理，避免 Tab 被全局循环抢走）
                            if is_focused && is_insert_mode {
                                let mut completion_cursor_target = None;
                                Self::handle_insert_mode(
                                    ui,
                                    sql_input,
                                    command_history,
                                    history_index,
                                    &mut actions,
                                    autocomplete,
                                    show_autocomplete,
                                    selected_completion,
                                    cursor_pos,
                                    editor_mode,
                                    completion_key_input,
                                    &mut completion_cursor_target,
                                );

                                if let Some(new_cursor) = completion_cursor_target {
                                    Self::set_editor_cursor(ui, &mut output, new_cursor);
                                    response.request_focus();
                                    *request_focus = true;
                                    actions.request_focus = true;
                                    status_cursor_pos = new_cursor;
                                }

                                if *show_autocomplete || completion_key_input.any() {
                                    response.request_focus();
                                }

                                // 输入改变时自动触发补全
                                if response.changed() {
                                    actions.text_changed = true;
                                    let completions =
                                        autocomplete.get_completions(sql_input, cursor_pos);
                                    if !completions.is_empty() {
                                        *show_autocomplete = true;
                                        *selected_completion = 0;
                                    } else {
                                        *show_autocomplete = false;
                                    }
                                }

                                if let Some(new_cursor) = Self::show_autocomplete_popup(
                                    ui,
                                    &response,
                                    sql_input,
                                    autocomplete,
                                    show_autocomplete,
                                    selected_completion,
                                    highlight_colors,
                                    cursor_pos,
                                ) {
                                    actions.text_changed = true;
                                    Self::set_editor_cursor(ui, &mut output, new_cursor);
                                    response.request_focus();
                                    *request_focus = true;
                                    actions.request_focus = true;
                                    status_cursor_pos = new_cursor;
                                }
                            } else if is_focused && *editor_mode == EditorMode::Normal {
                                // Normal 模式下的 Helix 风格导航
                                Self::handle_normal_mode(
                                    ui,
                                    sql_input,
                                    command_history,
                                    history_index,
                                    &mut actions,
                                    cursor_pos,
                                );
                                *show_autocomplete = false;
                            } else {
                                *show_autocomplete = false;
                            }
                        });

                    ui.ctx().data_mut(|d| {
                        d.insert_temp(scroll_id, scroll_output.state.offset.y);
                    });
                });
        });

        ui.add_space(2.0);

        // ========== 状态栏 ==========
        Self::show_status_bar(
            ui,
            sql_input,
            is_executing,
            last_message,
            query_time_ms,
            highlight_colors,
            command_history.len(),
            *editor_mode,
            status_cursor_pos,
        );

        actions
    }

    /// 显示工具栏
    fn show_toolbar(
        ui: &mut egui::Ui,
        sql_input: &str,
        is_executing: bool,
        actions: &mut SqlEditorActions,
        height: f32,
        mode: EditorMode,
    ) {
        ui.horizontal(|ui| {
            ui.set_height(height);
            ui.spacing_mut().item_spacing.x = 2.0;
            let toolbar_text_color = theme_text(ui.visuals());
            let toolbar_disabled_color = theme_disabled_text(ui.visuals());
            let toolbar_muted_color = theme_muted_text(ui.visuals());
            let toolbar_separator_color = theme_subtle_stroke(ui.visuals());

            let icon_btn = |ui: &mut egui::Ui, icon: &str, enabled: bool, tooltip: &str| -> bool {
                let color = if enabled {
                    toolbar_text_color
                } else {
                    toolbar_disabled_color
                };
                ui.add_enabled(
                    enabled,
                    egui::Button::new(RichText::new(icon).size(16.0).color(color))
                        .frame(false)
                        .min_size(Vec2::new(24.0, 24.0)),
                )
                .on_hover_text(tooltip)
                .clicked()
            };

            // 模式指示
            ui.label(RichText::new(mode.label()).monospace().color(mode.color()));
            ui.add_space(8.0);

            if is_executing {
                ui.spinner();
            } else if icon_btn(
                ui,
                "▶",
                !sql_input.trim().is_empty(),
                &local_shortcut_tooltip("执行当前 SQL", LocalShortcut::SqlExecute),
            ) {
                actions.execute = true;
            }

            if icon_btn(
                ui,
                "📊",
                !is_executing && !sql_input.trim().is_empty(),
                &local_shortcut_tooltip("分析执行计划", LocalShortcut::SqlExplain),
            ) {
                actions.explain = true;
            }

            if icon_btn(
                ui,
                "🗑",
                !sql_input.is_empty(),
                &local_shortcut_tooltip("清空 SQL", LocalShortcut::SqlClear),
            ) {
                actions.clear = true;
            }

            ui.add_space(8.0);
            ui.label(RichText::new("|").small().color(toolbar_separator_color));
            ui.add_space(8.0);

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mode == EditorMode::Normal {
                    ui.label(
                        RichText::new("双击/i 编辑")
                            .small()
                            .color(toolbar_muted_color),
                    );
                    ui.label(
                        RichText::new("hjkl 移动")
                            .small()
                            .color(toolbar_muted_color),
                    );
                } else {
                    ui.label(
                        RichText::new(format!(
                            "{} 退出编辑",
                            local_shortcut_text(LocalShortcut::Cancel)
                        ))
                        .small()
                        .color(toolbar_muted_color),
                    );
                    ui.label(
                        RichText::new(format!(
                            "{} 触发补全",
                            local_shortcut_text(LocalShortcut::SqlAutocompleteTrigger)
                        ))
                        .small()
                        .color(toolbar_muted_color),
                    );
                }
                ui.label(
                    RichText::new(format!(
                        "{} 执行",
                        local_shortcut_text(LocalShortcut::SqlExecute)
                    ))
                    .small()
                    .color(toolbar_muted_color),
                );
            });
        });
    }

    /// 显示状态栏
    fn show_status_bar(
        ui: &mut egui::Ui,
        sql_input: &str,
        is_executing: bool,
        last_message: &Option<String>,
        query_time_ms: Option<u64>,
        highlight_colors: &HighlightColors,
        history_count: usize,
        mode: EditorMode,
        cursor_pos: usize,
    ) {
        let (line, column) = get_cursor_position(sql_input, cursor_pos);
        let char_count = sql_input.chars().count();
        let line_count = count_lines(sql_input);

        egui::Frame::NONE
            .fill(ui.style().visuals.faint_bg_color)
            .inner_margin(egui::Margin::symmetric(8, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;

                    // 模式指示
                    egui::Frame::NONE
                        .fill(mode.color().gamma_multiply(0.3))
                        .corner_radius(2.0)
                        .inner_margin(egui::Margin::symmetric(4, 1))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(mode.label())
                                    .small()
                                    .strong()
                                    .color(mode.color()),
                            );
                        });

                    if is_executing {
                        ui.spinner();
                        ui.label(
                            RichText::new("执行中...")
                                .small()
                                .color(highlight_colors.keyword),
                        );
                    } else if let Some(msg) = last_message {
                        let is_error =
                            msg.contains("错误") || msg.contains("Error") || msg.contains("失败");
                        let color = if is_error {
                            highlight_colors.operator
                        } else {
                            highlight_colors.string
                        };
                        let icon = if is_error { "✗" } else { "✓" };
                        ui.label(RichText::new(icon).color(color));
                        let display_msg = if msg.len() > 50 {
                            format!("{}...", &msg[..47])
                        } else {
                            msg.clone()
                        };
                        ui.label(RichText::new(display_msg).small().color(color));
                    } else {
                        ui.label(RichText::new("就绪").small().color(GRAY));
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;

                        ui.label(
                            RichText::new(format!("历史 {}", history_count))
                                .small()
                                .color(GRAY),
                        );
                        ui.label(
                            RichText::new(format!("{}行 {}字符", line_count, char_count))
                                .small()
                                .color(GRAY),
                        );
                        ui.label(
                            RichText::new(format!("Ln {} Col {}", line, column))
                                .small()
                                .color(GRAY),
                        );

                        if let Some(ms) = query_time_ms {
                            let time_text = if ms >= 1000 {
                                format!("{:.1}s", ms as f64 / 1000.0)
                            } else {
                                format!("{}ms", ms)
                            };
                            ui.label(RichText::new(time_text).small().color(if ms > 1000 {
                                highlight_colors.operator
                            } else {
                                highlight_colors.string
                            }));
                        }
                    });
                });
            });
    }

    /// Normal 模式：Helix 风格导航
    fn handle_normal_mode(
        ui: &mut egui::Ui,
        sql_input: &mut String,
        command_history: &[String],
        history_index: &mut Option<usize>,
        actions: &mut SqlEditorActions,
        cursor_pos: usize,
    ) {
        // 检查光标是否在第一行
        let is_at_first_line = {
            let text_before_cursor = &sql_input[..char_to_byte_index(sql_input, cursor_pos)];
            !text_before_cursor.contains('\n')
        };

        ui.input(|i| {
            // k/上箭头: 如果在第一行，转移焦点到数据表格
            if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp))
                && !i.modifiers.shift
                && is_at_first_line
            {
                actions.focus_to_grid = true;
                return;
            }

            // Escape 切换焦点到 Grid
            if i.key_pressed(Key::Escape) {
                actions.focus_to_grid = true;
            }
        });

        ui.input_mut(|i| {
            if !sql_input.trim().is_empty() && consume_local_shortcut(i, LocalShortcut::SqlExecute)
            {
                actions.execute = true;
            }

            if !sql_input.trim().is_empty() && consume_local_shortcut(i, LocalShortcut::SqlExplain)
            {
                actions.explain = true;
            }

            if consume_local_shortcut(i, LocalShortcut::SqlHistoryPrev)
                && !command_history.is_empty()
            {
                let new_idx = match *history_index {
                    None => Some(0),
                    Some(idx) if idx + 1 < command_history.len() => Some(idx + 1),
                    Some(idx) => Some(idx),
                };
                if let Some(idx) = new_idx {
                    *history_index = Some(idx);
                    *sql_input = command_history[idx].clone();
                    actions.text_changed = true;
                }
            }

            if consume_local_shortcut(i, LocalShortcut::SqlHistoryNext) {
                match *history_index {
                    Some(0) => {
                        *history_index = None;
                        sql_input.clear();
                        actions.text_changed = true;
                    }
                    Some(idx) => {
                        *history_index = Some(idx - 1);
                        *sql_input = command_history[idx - 1].clone();
                        actions.text_changed = true;
                    }
                    None => {}
                }
            }

            if consume_local_shortcut(i, LocalShortcut::SqlClear) {
                actions.clear = true;
            }
        });
    }

    /// Insert 模式：编辑和补全
    fn handle_insert_mode(
        ui: &mut egui::Ui,
        sql_input: &mut String,
        command_history: &[String],
        history_index: &mut Option<usize>,
        actions: &mut SqlEditorActions,
        autocomplete: &AutoComplete,
        show_autocomplete: &mut bool,
        selected_completion: &mut usize,
        cursor_pos: usize,
        editor_mode: &mut EditorMode,
        completion_key_input: CompletionKeyInput,
        completion_cursor_target: &mut Option<usize>,
    ) {
        let completions = autocomplete.get_completions(sql_input, cursor_pos);
        let has_completions = !completions.is_empty();

        ui.input_mut(|i| {
            // 注意：Escape 键在外层已经处理，这里不再处理

            if !sql_input.trim().is_empty() && consume_local_shortcut(i, LocalShortcut::SqlExecute)
            {
                actions.execute = true;
                *editor_mode = EditorMode::Normal;
            }

            if !sql_input.trim().is_empty() && consume_local_shortcut(i, LocalShortcut::SqlExplain)
            {
                actions.explain = true;
            }

            if consume_local_shortcut(i, LocalShortcut::SqlAutocompleteTrigger) && has_completions {
                *show_autocomplete = true;
                *selected_completion = 0;
            }

            // Tab: 无补全弹窗时优先打开补全，避免触发焦点转移
            if completion_key_input.confirm_tab && !*show_autocomplete && has_completions {
                *show_autocomplete = true;
                *selected_completion = 0;
            }

            // 补全菜单导航
            if *show_autocomplete && has_completions {
                let down_pressed =
                    completion_key_input.down || i.consume_key(Modifiers::NONE, Key::ArrowDown);
                if down_pressed {
                    *selected_completion = (*selected_completion + 1) % completions.len();
                }
                let up_pressed =
                    completion_key_input.up || i.consume_key(Modifiers::NONE, Key::ArrowUp);
                if up_pressed {
                    if *selected_completion == 0 {
                        *selected_completion = completions.len().saturating_sub(1);
                    } else {
                        *selected_completion -= 1;
                    }
                }
                let tab_confirm =
                    completion_key_input.confirm_tab || i.consume_key(Modifiers::NONE, Key::Tab);
                let enter_confirm = completion_key_input.confirm_enter
                    || i.consume_key(Modifiers::NONE, Key::Enter);
                if (tab_confirm || enter_confirm) && *selected_completion < completions.len() {
                    let new_cursor = apply_completion_at_cursor(
                        sql_input,
                        cursor_pos,
                        &completions[*selected_completion].insert_text,
                    );
                    *completion_cursor_target = Some(new_cursor);
                    actions.text_changed = true;
                    *show_autocomplete = false;
                }
            }

            if !*show_autocomplete
                && consume_local_shortcut(i, LocalShortcut::SqlHistoryPrev)
                && !command_history.is_empty()
            {
                let new_idx = match *history_index {
                    None => Some(0),
                    Some(idx) if idx + 1 < command_history.len() => Some(idx + 1),
                    Some(idx) => Some(idx),
                };
                if let Some(idx) = new_idx {
                    *history_index = Some(idx);
                    *sql_input = command_history[idx].clone();
                    actions.text_changed = true;
                }
            }
            if !*show_autocomplete && consume_local_shortcut(i, LocalShortcut::SqlHistoryNext) {
                match *history_index {
                    Some(0) => {
                        *history_index = None;
                        sql_input.clear();
                        actions.text_changed = true;
                    }
                    Some(idx) => {
                        *history_index = Some(idx - 1);
                        *sql_input = command_history[idx - 1].clone();
                        actions.text_changed = true;
                    }
                    None => {}
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
        cursor_pos: usize,
    ) -> Option<usize> {
        let mut new_cursor = None;
        let completions = autocomplete.get_completions(sql_input, cursor_pos);

        if *show_autocomplete && !completions.is_empty() {
            egui::Popup::open_id(ui.ctx(), response.id);
            egui::Popup::from_response(response)
                .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                .show(|ui| {
                    ui.set_min_width(220.0);
                    ui.set_max_height(180.0);

                    ScrollArea::vertical().show(ui, |ui| {
                        for (idx, item) in completions.iter().enumerate() {
                            let is_selected = idx == *selected_completion;
                            let bg = if is_selected {
                                highlight_colors.keyword.gamma_multiply(0.25)
                            } else {
                                Color32::TRANSPARENT
                            };

                            let frame_response = egui::Frame::NONE
                                .fill(bg)
                                .inner_margin(egui::Margin::symmetric(4, 2))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let (icon, icon_color) = match item.kind {
                                            CompletionKind::Keyword => {
                                                ("K", highlight_colors.keyword)
                                            }
                                            CompletionKind::Function => {
                                                ("F", highlight_colors.function)
                                            }
                                            CompletionKind::Table => ("T", highlight_colors.string),
                                            CompletionKind::Column => {
                                                ("C", highlight_colors.identifier)
                                            }
                                        };

                                        egui::Frame::NONE
                                            .fill(icon_color.gamma_multiply(0.3))
                                            .corner_radius(2.0)
                                            .inner_margin(egui::Margin::symmetric(3, 0))
                                            .show(ui, |ui| {
                                                ui.label(
                                                    RichText::new(icon)
                                                        .color(icon_color)
                                                        .monospace()
                                                        .size(10.0),
                                                );
                                            });

                                        ui.label(RichText::new(&item.label).monospace().color(
                                            if is_selected {
                                                theme_text(ui.visuals())
                                            } else {
                                                theme_muted_text(ui.visuals())
                                            },
                                        ));
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());

                            if frame_response.clicked() {
                                let cursor_after = apply_completion_at_cursor(
                                    sql_input,
                                    cursor_pos,
                                    &item.insert_text,
                                );
                                *show_autocomplete = false;
                                new_cursor = Some(cursor_after);
                            }

                            if is_selected {
                                frame_response.scroll_to_me(Some(Align::Center));
                            }
                        }
                    });
                });
        }
        new_cursor
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CompletionKeyInput, EditorMode, SqlEditor, SqlEditorActions, apply_completion_at_cursor,
    };
    use crate::core::AutoComplete;
    use egui::Key;

    #[test]
    fn insert_mode_cancel_closes_autocomplete_without_leaving_insert_mode() {
        let outcome = SqlEditor::apply_insert_mode_cancel(true, EditorMode::Insert);

        assert_eq!(outcome.next_mode, EditorMode::Insert);
        assert!(!outcome.show_autocomplete);
        assert!(outcome.keep_editor_focus);
        assert!(outcome.escape_consumed);
    }

    #[test]
    fn insert_mode_cancel_without_autocomplete_returns_to_normal_mode() {
        let outcome = SqlEditor::apply_insert_mode_cancel(false, EditorMode::Insert);

        assert_eq!(outcome.next_mode, EditorMode::Normal);
        assert!(!outcome.show_autocomplete);
        assert!(outcome.keep_editor_focus);
        assert!(outcome.escape_consumed);
    }

    #[test]
    fn apply_completion_at_cursor_returns_cursor_after_inserted_text() {
        let mut sql = "sel".to_string();

        let cursor = apply_completion_at_cursor(&mut sql, 3, "SELECT");

        assert_eq!(sql, "SELECT ");
        assert_eq!(cursor, "SELECT ".chars().count());
    }

    #[test]
    fn insert_mode_tab_accepts_completion_and_reports_cursor_target() {
        let ctx = egui::Context::default();
        let mut sql = "sel".to_string();
        let mut history_index = None;
        let mut actions = SqlEditorActions::default();
        let autocomplete = AutoComplete::default();
        let mut show_autocomplete = true;
        let mut selected_completion = 0usize;
        let mut editor_mode = EditorMode::Insert;
        let mut completion_cursor_target = None;

        ctx.begin_pass(egui::RawInput::default());
        egui::Area::new(egui::Id::new("sql_editor_insert_mode_tab_accept")).show(&ctx, |ui| {
            SqlEditor::handle_insert_mode(
                ui,
                &mut sql,
                &[],
                &mut history_index,
                &mut actions,
                &autocomplete,
                &mut show_autocomplete,
                &mut selected_completion,
                3,
                &mut editor_mode,
                CompletionKeyInput {
                    confirm_tab: true,
                    ..Default::default()
                },
                &mut completion_cursor_target,
            );
        });
        let _ = ctx.end_pass();

        assert_eq!(sql, "SELECT ");
        assert_eq!(completion_cursor_target, Some("SELECT ".chars().count()));
        assert!(!show_autocomplete);
        assert!(actions.text_changed);
        assert_eq!(editor_mode, EditorMode::Insert);
    }

    #[test]
    fn unfocused_editor_does_not_execute_on_editor_shortcuts() {
        let ctx = egui::Context::default();
        let mut sql = "select 1".to_string();
        let mut history_index = None;
        let mut show_autocomplete = false;
        let mut selected_completion = 0usize;
        let mut request_focus = false;
        let mut editor_mode = EditorMode::Insert;

        let raw_input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..egui::Modifiers::NONE
                },
            }],
            modifiers: egui::Modifiers {
                ctrl: true,
                command: true,
                ..egui::Modifiers::NONE
            },
            ..Default::default()
        };

        ctx.begin_pass(raw_input);
        let actions = egui::Area::new(egui::Id::new("sql_editor_unfocused_shortcut_guard"))
            .show(&ctx, |ui| {
                SqlEditor::show(
                    ui,
                    &mut sql,
                    &[],
                    &mut history_index,
                    false,
                    &None,
                    &Default::default(),
                    None,
                    &AutoComplete::default(),
                    &mut show_autocomplete,
                    &mut selected_completion,
                    &mut request_focus,
                    false,
                    &mut editor_mode,
                )
            })
            .inner;
        let _ = ctx.end_pass();

        assert!(!actions.execute);
        assert!(!actions.explain);
    }
}
