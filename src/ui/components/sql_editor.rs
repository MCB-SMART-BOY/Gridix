//! SQL 编辑器组件 - Helix 风格双模式
//!
//! 特性：
//! - Normal 模式：hjkl 移动，w/b 词跳转，Helix 风格导航
//! - Insert 模式：双击进入，正常输入
//! - Ctrl+Enter 执行 SQL
//! - 语法高亮 + 自动补全

#![allow(clippy::too_many_arguments)]

use crate::core::{highlight_sql, AutoComplete, CompletionKind, HighlightColors};
use crate::ui::styles::GRAY;
use egui::{self, Align, Color32, Key, Layout, PopupCloseBehavior, RichText, ScrollArea, TextEdit, Vec2};

/// 行号区域宽度
const LINE_NUMBER_WIDTH: f32 = 45.0;

/// 编辑器模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditorMode {
    #[default]
    Normal,
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
            EditorMode::Normal => Color32::from_rgb(130, 170, 255),  // 蓝色
            EditorMode::Insert => Color32::from_rgb(180, 230, 140),  // 绿色
        }
    }
}

pub struct SqlEditor;

/// 应用自动补全（在光标位置插入）
fn apply_completion_at_cursor(text: &mut String, cursor_pos: usize, insert_text: &str) -> usize {
    let pos = cursor_pos.min(text.len());
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
    
    // 返回新的光标位置
    word_start + insert_text.len() + 1
}

/// 获取当前正在输入的单词
fn get_current_word(text: &str) -> &str {
    let mut word_start = text.len();
    for (i, c) in text.char_indices().rev() {
        if c.is_alphanumeric() || c == '_' {
            word_start = i;
        } else {
            break;
        }
    }
    &text[word_start..]
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
    let text_before_cursor = &text[..cursor_pos.min(text.len())];
    let line = text_before_cursor.matches('\n').count() + 1;
    let last_newline = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = text_before_cursor[last_newline..].chars().count() + 1;
    (line, column)
}

/// 获取行的起始和结束位置
fn get_line_bounds(text: &str, cursor_pos: usize) -> (usize, usize) {
    let pos = cursor_pos.min(text.len());
    let start = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let end = text[pos..].find('\n').map(|i| pos + i).unwrap_or(text.len());
    (start, end)
}

/// 移动到下一个单词（预留 Helix w 键）
#[allow(dead_code)]
fn next_word_pos(text: &str, cursor_pos: usize) -> usize {
    let pos = cursor_pos.min(text.len());
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
    let pos = cursor_pos.min(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = pos;
    
    // 跳过前面的空白
    while i > 0 && chars[i.saturating_sub(1)].is_whitespace() {
        i -= 1;
    }
    // 跳过单词
    while i > 0 && (chars[i.saturating_sub(1)].is_alphanumeric() || chars[i.saturating_sub(1)] == '_') {
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
    pub focus_to_grid: bool,
    pub request_focus: bool,
    /// Escape 键已被编辑器消费（用于退出 Insert 模式）
    pub escape_consumed: bool,
}

impl SqlEditor {
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

        let available_height = ui.available_height();
        let available_width = ui.available_width();
        
        let status_bar_height = 20.0;
        let toolbar_height = 26.0;
        let editor_height = (available_height - status_bar_height - toolbar_height - 8.0).max(60.0);

        // ========== 工具栏 ==========
        Self::show_toolbar(ui, sql_input, is_executing, &mut actions, toolbar_height, *editor_mode);
        
        ui.add_space(2.0);

        // ========== 编辑器主体 ==========
        let line_count = count_lines(sql_input);
        let line_height = 16.0;
        
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
                    
                    let scroll_offset = ui.ctx().data(|d| {
                        d.get_temp::<f32>(scroll_id).unwrap_or(0.0)
                    });
                    
                    ScrollArea::vertical()
                        .id_salt("line_numbers")
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .vertical_scroll_offset(scroll_offset)
                        .show(ui, |ui| {
                            ui.set_width(LINE_NUMBER_WIDTH);
                            let display_lines = line_count.max((editor_height / line_height) as usize);
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
            let editor_width = available_width - LINE_NUMBER_WIDTH - 8.0;
            
            egui::Frame::NONE
                .fill(ui.style().visuals.extreme_bg_color)
                .show(ui, |ui| {
                    ui.set_width(editor_width);
                    ui.set_height(editor_height);
                    
                    let colors = highlight_colors.clone();
                    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
                        let mut job = highlight_sql(text.as_str(), &colors);
                        job.wrap.max_width = wrap_width;
                        ui.ctx().fonts_mut(|f| f.layout_job(job))
                    };

                    let scroll_output = ScrollArea::vertical()
                        .id_salt("sql_editor_content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            // Insert 模式：可编辑；Normal 模式：只读显示
                            let is_insert_mode = *editor_mode == EditorMode::Insert;
                            
                            let output = TextEdit::multiline(sql_input)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(editor_width - 16.0)
                                .desired_rows(((editor_height / line_height) as usize).max(4))
                                .hint_text(if is_insert_mode { 
                                    "输入 SQL... (Esc 退出编辑, Ctrl+Enter 执行)" 
                                } else { 
                                    "双击进入编辑模式, Ctrl+Enter 执行" 
                                })
                                .frame(false)
                                .margin(Vec2::new(8.0, 0.0))
                                .interactive(is_insert_mode)
                                .layouter(&mut layouter)
                                .show(ui);
                            
                            let response = &output.response;

                            // 双击进入 Insert 模式
                            if response.double_clicked() {
                                *editor_mode = EditorMode::Insert;
                                response.request_focus();
                            }
                            
                            // i 键也可进入 Insert 模式（在 Normal 模式下）
                            if *editor_mode == EditorMode::Normal && is_focused {
                                let enter_insert = ui.input(|i| {
                                    i.key_pressed(Key::I) || i.key_pressed(Key::A) || i.key_pressed(Key::O)
                                });
                                if enter_insert {
                                    *editor_mode = EditorMode::Insert;
                                    // o 键在当前行后插入新行
                                    if ui.input(|i| i.key_pressed(Key::O)) {
                                        let cursor_pos = output.cursor_range
                                            .map(|r| r.primary.index)
                                            .unwrap_or(sql_input.len());
                                        let (_, line_end) = get_line_bounds(sql_input, cursor_pos);
                                        sql_input.insert(line_end, '\n');
                                    }
                                }
                            }

                            if *request_focus && is_insert_mode {
                                response.request_focus();
                                *request_focus = false;
                            }

                            if response.clicked() || response.has_focus() {
                                actions.request_focus = true;
                            }

                            // 获取光标位置
                            let cursor_pos = output.cursor_range
                                .map(|range| range.primary.index)
                                .unwrap_or(sql_input.len());

                            // Insert 模式下：在检查焦点之前先处理 Escape 键
                            // 因为 egui 会在 Escape 时自动让 TextEdit 失焦
                            if is_focused && is_insert_mode {
                                let esc_pressed = ui.input(|i| i.key_pressed(Key::Escape));
                                if esc_pressed {
                                    if *show_autocomplete {
                                        *show_autocomplete = false;
                                    } else {
                                        *editor_mode = EditorMode::Normal;
                                    }
                                    actions.escape_consumed = true;
                                    // 保持焦点在编辑器上
                                    response.request_focus();
                                }
                            }
                            
                            // Insert 模式下的其他快捷键处理
                            if response.has_focus() && is_focused && is_insert_mode {
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
                                );
                                
                                // 输入改变时自动触发补全
                                if response.changed() {
                                    let text_before_cursor = &sql_input[..cursor_pos.min(sql_input.len())];
                                    let current_word = get_current_word(text_before_cursor);
                                    if !current_word.is_empty() {
                                        let completions = autocomplete.get_completions(sql_input, cursor_pos);
                                        if !completions.is_empty() {
                                            *show_autocomplete = true;
                                            *selected_completion = 0;
                                        } else {
                                            *show_autocomplete = false;
                                        }
                                    } else {
                                        *show_autocomplete = false;
                                    }
                                }
                                
                                Self::show_autocomplete_popup(
                                    ui,
                                    response,
                                    sql_input,
                                    autocomplete,
                                    show_autocomplete,
                                    selected_completion,
                                    highlight_colors,
                                    cursor_pos,
                                );
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
            
            let icon_btn = |ui: &mut egui::Ui, icon: &str, enabled: bool, tooltip: &str| -> bool {
                let color = if enabled { Color32::LIGHT_GRAY } else { Color32::from_gray(60) };
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
            } else if icon_btn(ui, "▶", !sql_input.trim().is_empty(), "执行 (Ctrl+Enter)") {
                actions.execute = true;
            }
            
            if icon_btn(ui, "📊", !is_executing && !sql_input.trim().is_empty(), "分析 (F6)") {
                actions.explain = true;
            }
            
            if icon_btn(ui, "🗑", !sql_input.is_empty(), "清空") {
                actions.clear = true;
            }
            
            ui.add_space(8.0);
            ui.label(RichText::new("|").small().color(Color32::from_gray(60)));
            ui.add_space(8.0);
            
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if mode == EditorMode::Normal {
                    ui.label(RichText::new("双击/i 编辑").small().color(GRAY));
                    ui.label(RichText::new("hjkl 移动").small().color(GRAY));
                } else {
                    ui.label(RichText::new("Esc 退出编辑").small().color(GRAY));
                    ui.label(RichText::new("Tab 补全").small().color(GRAY));
                }
                ui.label(RichText::new("Ctrl+Enter 执行").small().color(GRAY));
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
    ) {
        let (line, column) = get_cursor_position(sql_input, sql_input.len());
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
                            ui.label(RichText::new(mode.label()).small().strong().color(mode.color()));
                        });
                    
                    if is_executing {
                        ui.spinner();
                        ui.label(RichText::new("执行中...").small().color(highlight_colors.keyword));
                    } else if let Some(msg) = last_message {
                        let is_error = msg.contains("错误") || msg.contains("Error") || msg.contains("失败");
                        let color = if is_error { highlight_colors.operator } else { highlight_colors.string };
                        let icon = if is_error { "✗" } else { "✓" };
                        ui.label(RichText::new(icon).color(color));
                        let display_msg = if msg.len() > 50 { format!("{}...", &msg[..47]) } else { msg.clone() };
                        ui.label(RichText::new(display_msg).small().color(color));
                    } else {
                        ui.label(RichText::new("就绪").small().color(GRAY));
                    }

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        
                        ui.label(RichText::new(format!("历史 {}", history_count)).small().color(GRAY));
                        ui.label(RichText::new(format!("{}行 {}字符", line_count, char_count)).small().color(GRAY));
                        ui.label(RichText::new(format!("Ln {} Col {}", line, column)).small().color(GRAY));
                        
                        if let Some(ms) = query_time_ms {
                            let time_text = if ms >= 1000 {
                                format!("{:.1}s", ms as f64 / 1000.0)
                            } else {
                                format!("{}ms", ms)
                            };
                            ui.label(RichText::new(time_text).small().color(
                                if ms > 1000 { highlight_colors.operator } else { highlight_colors.string }
                            ));
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
            let text_before_cursor = &sql_input[..cursor_pos.min(sql_input.len())];
            !text_before_cursor.contains('\n')
        };
        
        ui.input(|i| {
            // k/上箭头: 如果在第一行，转移焦点到数据表格
            if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp)) && !i.modifiers.shift && is_at_first_line {
                actions.focus_to_grid = true;
                return;
            }
            
            // Ctrl+Enter 执行
            if i.modifiers.ctrl && i.key_pressed(Key::Enter) && !sql_input.trim().is_empty() {
                actions.execute = true;
            }
            
            // F5 执行
            if i.key_pressed(Key::F5) && !sql_input.trim().is_empty() {
                actions.execute = true;
            }
            
            // F6 EXPLAIN
            if i.key_pressed(Key::F6) && !sql_input.trim().is_empty() {
                actions.explain = true;
            }
            
            // Escape 切换焦点到 Grid
            if i.key_pressed(Key::Escape) {
                actions.focus_to_grid = true;
            }
            
            // Shift+↑↓ 或 K/J 历史导航
            let history_up = (i.modifiers.shift && i.key_pressed(Key::ArrowUp)) 
                || (i.key_pressed(Key::K) && i.modifiers.shift);
            let history_down = (i.modifiers.shift && i.key_pressed(Key::ArrowDown))
                || (i.key_pressed(Key::J) && i.modifiers.shift);
            
            if history_up && !command_history.is_empty() {
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
            
            if history_down {
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
            
            // dd 清空（模拟 Helix 删除行）
            if i.key_pressed(Key::D) && i.modifiers.shift {
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
    ) {
        let completions = autocomplete.get_completions(sql_input, cursor_pos);
        let has_completions = !completions.is_empty();

        ui.input(|i| {
            // 注意：Escape 键在外层已经处理，这里不再处理
            
            // Ctrl+Enter 执行
            if i.modifiers.ctrl && i.key_pressed(Key::Enter) && !sql_input.trim().is_empty() {
                actions.execute = true;
                *editor_mode = EditorMode::Normal;
            }
            
            // F5 执行
            if i.key_pressed(Key::F5) && !sql_input.trim().is_empty() {
                actions.execute = true;
                *editor_mode = EditorMode::Normal;
            }
            
            // F6 EXPLAIN
            if i.key_pressed(Key::F6) && !sql_input.trim().is_empty() {
                actions.explain = true;
            }
            
            // Ctrl+Space 或 Alt+L 触发补全
            if (i.modifiers.ctrl && i.key_pressed(Key::Space)) || (i.modifiers.alt && i.key_pressed(Key::L)) {
                if has_completions {
                    *show_autocomplete = true;
                    *selected_completion = 0;
                }
            }

            // 补全菜单导航
            if *show_autocomplete && has_completions {
                if i.key_pressed(Key::ArrowDown) {
                    *selected_completion = (*selected_completion + 1) % completions.len();
                }
                if i.key_pressed(Key::ArrowUp) {
                    if *selected_completion == 0 {
                        *selected_completion = completions.len().saturating_sub(1);
                    } else {
                        *selected_completion -= 1;
                    }
                }
                if i.key_pressed(Key::Tab) || i.key_pressed(Key::Enter) {
                    if *selected_completion < completions.len() {
                        apply_completion_at_cursor(sql_input, cursor_pos, &completions[*selected_completion].insert_text);
                        *show_autocomplete = false;
                    }
                }
            }
            
            // Shift+↑↓ 历史
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
        cursor_pos: usize,
    ) {
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
                                            CompletionKind::Keyword => ("K", highlight_colors.keyword),
                                            CompletionKind::Function => ("F", highlight_colors.function),
                                            CompletionKind::Table => ("T", highlight_colors.string),
                                            CompletionKind::Column => ("C", highlight_colors.identifier),
                                        };
                                        
                                        egui::Frame::NONE
                                            .fill(icon_color.gamma_multiply(0.3))
                                            .corner_radius(2.0)
                                            .inner_margin(egui::Margin::symmetric(3, 0))
                                            .show(ui, |ui| {
                                                ui.label(RichText::new(icon).color(icon_color).monospace().size(10.0));
                                            });

                                        ui.label(RichText::new(&item.label).monospace().color(
                                            if is_selected { Color32::WHITE } else { Color32::LIGHT_GRAY }
                                        ));
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());
                            
                            if frame_response.clicked() {
                                apply_completion_at_cursor(sql_input, cursor_pos, &item.insert_text);
                                *show_autocomplete = false;
                            }
                            
                            if is_selected {
                                frame_response.scroll_to_me(Some(Align::Center));
                            }
                        }
                    });
                });
        }
    }
}
