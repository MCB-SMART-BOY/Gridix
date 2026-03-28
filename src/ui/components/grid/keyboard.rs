//! 键盘输入处理（Helix 风格）
//!
//! ## Normal 模式键位
//! - `h/j/k/l`: 移动光标（Grid 上下文：四向移动）
//! - `w/b`: 右/左移一列
//! - `e`: 跳转到行尾
//! - `gh/gl`: 行首/行尾
//! - `gg/G`: 文件首/尾
//! - `Ctrl+u`: 向上翻半页
//! - `PageUp/PageDown`: 翻页
//! - `i/a/c`: 进入插入模式
//! - `v`: 进入选择模式
//! - `x`: 选择整行
//! - `%`: 选择全部
//! - `;`: 折叠选择到单个光标
//! - `dd`: 标记删除当前行
//! - `yy`: 复制整行
//! - `p`: 粘贴
//! - `u/U`: 撤销/取消删除标记
//! - `/`: 打开筛选面板
//! - `f`: 为当前列添加筛选
//! - `o/O`: 添加新行
//! - `:w`: 保存修改
//! - `q`: 放弃修改
//! - `Ctrl+R`: 刷新表格数据
//! - `Space+d`: 标记删除行
//! - `Ctrl+S`: 保存修改
//!
//! ## 视图模式 (z 前缀)
//! - `zz/zc`: 将当前行滚动到屏幕中央
//! - `zt`: 将当前行滚动到屏幕顶部
//! - `zb`: 将当前行滚动到屏幕底部
//!
//! ## 数字计数
//! - `1-9`: 输入计数前缀（如 10j 向下移动10行）
//! - `0`: 追加到已有计数
//! - `Backspace`: 回退计数数字

#![allow(clippy::too_many_arguments)]

use super::actions::DataGridActions;
use super::filter::ColumnFilter;
use super::mode::GridMode;
use super::state::DataGridState;
use crate::database::QueryResult;
use egui::{self, Key};
use tracing::debug;

/// 命令缓冲区状态
#[derive(Default)]
struct CmdBuffer {
    keys: String,
    count: Option<usize>,
}

impl CmdBuffer {
    fn clear(&mut self) {
        self.keys.clear();
        self.count = None;
    }

    fn get_count(&self) -> usize {
        self.count.unwrap_or(1)
    }
}

pub fn handle_keyboard(
    ui: &mut egui::Ui,
    state: &mut DataGridState,
    result: &QueryResult,
    filtered_rows: &[(usize, &Vec<String>)],
    actions: &mut DataGridActions,
) {
    // 如果表格未聚焦或处于编辑模式，不处理表格快捷键
    if !state.focused || state.mode == GridMode::Insert {
        return;
    }

    let max_row = filtered_rows.len();
    let max_col = result.columns.len();

    if max_row == 0 || max_col == 0 {
        debug!(max_row, max_col, "DataGrid 为空，跳过键盘处理");
        return;
    }

    let half_page = (max_row / 2).max(1);

    // 从 state 同步命令缓冲区
    let mut cmd = CmdBuffer {
        keys: state.command_buffer.clone(),
        count: state.count,
    };

    ui.input(|i| {
        // === 数字前缀处理 ===
        if handle_number_input(i, &mut cmd) {
            state.count = cmd.count;
            return;
        }

        // 数字 + Enter: 切换到指定的查询Tab
        if i.key_pressed(Key::Enter)
            && cmd.keys.is_empty()
            && let Some(tab_number) = cmd.count
        {
            if tab_number > 0 {
                actions.switch_to_tab = Some(tab_number - 1);
                actions.message = Some(format!("切换到查询 {}", tab_number));
            }
            cmd.clear();
            return;
        }

        // Backspace 回退数字计数
        if i.key_pressed(Key::Backspace)
            && let Some(current) = cmd.count
        {
            cmd.count = if current < 10 {
                None
            } else {
                Some(current / 10)
            };
            return;
        }

        match state.mode {
            GridMode::Normal => {
                handle_normal_mode(
                    i,
                    state,
                    result,
                    filtered_rows,
                    actions,
                    max_row,
                    max_col,
                    half_page,
                    &mut cmd,
                );
            }
            GridMode::Select => {
                handle_select_mode(i, state, filtered_rows, actions, max_row, max_col, &mut cmd);
            }
            GridMode::Insert => {}
        }
    });

    // 同步命令缓冲区回 state
    state.command_buffer = cmd.keys;
    state.count = cmd.count;
}

/// 处理数字输入，返回 true 表示已处理
fn handle_number_input(i: &egui::InputState, cmd: &mut CmdBuffer) -> bool {
    // 检查修饰键，有修饰键时不处理数字
    if i.modifiers.ctrl || i.modifiers.alt || i.modifiers.shift {
        return false;
    }

    for digit in 0..=9u32 {
        let key = match digit {
            0 => Key::Num0,
            1 => Key::Num1,
            2 => Key::Num2,
            3 => Key::Num3,
            4 => Key::Num4,
            5 => Key::Num5,
            6 => Key::Num6,
            7 => Key::Num7,
            8 => Key::Num8,
            9 => Key::Num9,
            _ => continue,
        };

        if i.key_pressed(key) {
            // 0 只有在已有计数时才追加，否则作为跳转到行首
            if digit == 0 && cmd.count.is_none() {
                return false;
            }
            let current = cmd.count.unwrap_or(0);
            // 防止溢出，限制最大计数为 99999
            if current <= 9999 {
                cmd.count = Some(current * 10 + digit as usize);
            }
            return true;
        }
    }
    false
}

fn handle_normal_mode(
    i: &egui::InputState,
    state: &mut DataGridState,
    result: &QueryResult,
    filtered_rows: &[(usize, &Vec<String>)],
    actions: &mut DataGridActions,
    max_row: usize,
    max_col: usize,
    half_page: usize,
    cmd: &mut CmdBuffer,
) {
    let repeat = cmd.get_count();

    // === 基础导航 ===

    // h / 左箭头: 向左
    if (i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft)) && cmd.keys.is_empty() {
        if state.cursor.1 == 0 {
            actions.focus_transfer = Some(super::actions::FocusTransfer::Sidebar);
        } else {
            for _ in 0..repeat {
                state.move_cursor(0, -1, max_row, max_col);
            }
        }
        cmd.clear();
        return;
    }

    // j / 下箭头: 向下
    if (i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown)) && cmd.keys.is_empty() {
        if state.cursor.0 >= max_row.saturating_sub(1) {
            actions.focus_transfer = Some(super::actions::FocusTransfer::SqlEditor);
        } else {
            for _ in 0..repeat {
                state.move_cursor(1, 0, max_row, max_col);
            }
        }
        cmd.clear();
        return;
    }

    // k / 上箭头: 向上
    if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp)) && cmd.keys.is_empty() {
        if state.cursor.0 == 0 {
            actions.focus_transfer = Some(super::actions::FocusTransfer::QueryTabs);
        } else {
            for _ in 0..repeat {
                state.move_cursor(-1, 0, max_row, max_col);
            }
        }
        cmd.clear();
        return;
    }

    // l / 右箭头: 向右
    if (i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight)) && cmd.keys.is_empty() {
        for _ in 0..repeat {
            state.move_cursor(0, 1, max_row, max_col);
        }
        cmd.clear();
        return;
    }

    // w: 向右移动一列（带焦点转移）
    if i.key_pressed(Key::W) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        if state.cursor.1 >= max_col.saturating_sub(1) {
            actions.focus_transfer = Some(super::actions::FocusTransfer::SqlEditor);
        } else {
            state.move_cursor(0, 1, max_row, max_col);
        }
        return;
    }

    // b: 向左移动一列（带焦点转移）
    if i.key_pressed(Key::B) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        if state.cursor.1 == 0 {
            actions.focus_transfer = Some(super::actions::FocusTransfer::Sidebar);
        } else {
            state.move_cursor(0, -1, max_row, max_col);
        }
        return;
    }

    // e: 跳转到行尾
    if i.key_pressed(Key::E) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        state.goto_line_end(max_col);
        return;
    }

    // 0: 跳转到行首（无计数时）
    if i.key_pressed(Key::Num0) && cmd.count.is_none() && cmd.keys.is_empty() {
        state.goto_line_start();
        return;
    }

    // $: 跳转到行尾 (Shift+4)
    if i.key_pressed(Key::Num4) && i.modifiers.shift && cmd.keys.is_empty() {
        state.goto_line_end(max_col);
        return;
    }

    // ^: 跳转到行首 (Shift+6)
    if i.key_pressed(Key::Num6) && i.modifiers.shift && cmd.keys.is_empty() {
        state.goto_line_start();
        return;
    }

    // Home/End
    if i.key_pressed(Key::Home) {
        if i.modifiers.ctrl {
            state.goto_file_start();
        } else {
            state.goto_line_start();
        }
        return;
    }
    if i.key_pressed(Key::End) {
        if i.modifiers.ctrl {
            state.goto_file_end(max_row);
        } else {
            state.goto_line_end(max_col);
        }
        return;
    }

    // === 翻页 ===

    // Ctrl+U: 向上翻半页
    if i.modifiers.ctrl && i.key_pressed(Key::U) {
        let delta = half_page * repeat;
        state.cursor.0 = state.cursor.0.saturating_sub(delta);
        state.scroll_to_row = Some(state.cursor.0);
        cmd.clear();
        return;
    }

    // Ctrl+D: 向下翻半页
    if i.modifiers.ctrl && i.key_pressed(Key::D) {
        let delta = half_page * repeat;
        state.cursor.0 = (state.cursor.0 + delta).min(max_row.saturating_sub(1));
        state.scroll_to_row = Some(state.cursor.0);
        cmd.clear();
        return;
    }

    // PageUp/PageDown
    if i.key_pressed(Key::PageUp) {
        let delta = half_page * repeat;
        state.cursor.0 = state.cursor.0.saturating_sub(delta);
        state.scroll_to_row = Some(state.cursor.0);
        cmd.clear();
        return;
    }
    if i.key_pressed(Key::PageDown) {
        let delta = half_page * repeat;
        state.cursor.0 = (state.cursor.0 + delta).min(max_row.saturating_sub(1));
        state.scroll_to_row = Some(state.cursor.0);
        cmd.clear();
        return;
    }

    // === g 前缀命令 ===
    if i.key_pressed(Key::G) && !i.modifiers.shift {
        if cmd.keys == "g" {
            // gg: 跳到开头
            state.goto_file_start();
            cmd.clear();
            return;
        } else if cmd.keys.is_empty() {
            cmd.keys = "g".to_string();
            return;
        }
    }

    // G (Shift+g): 跳到结尾
    if i.key_pressed(Key::G) && i.modifiers.shift {
        state.goto_file_end(max_row);
        cmd.clear();
        return;
    }

    // ge: 跳到结尾
    if i.key_pressed(Key::E) && cmd.keys == "g" {
        state.goto_file_end(max_row);
        cmd.clear();
        return;
    }

    // gh: 行首
    if i.key_pressed(Key::H) && cmd.keys == "g" {
        state.goto_line_start();
        cmd.clear();
        return;
    }

    // gl: 行尾
    if i.key_pressed(Key::L) && cmd.keys == "g" {
        state.goto_line_end(max_col);
        cmd.clear();
        return;
    }

    // === z 前缀命令（视图控制）===
    if i.key_pressed(Key::Z) && !i.modifiers.shift && cmd.keys.is_empty() {
        cmd.keys = "z".to_string();
        return;
    }

    if cmd.keys == "z" {
        if i.key_pressed(Key::Z) || i.key_pressed(Key::C) {
            // zz/zc: 居中
            state.scroll_to_row = Some(state.cursor.0);
            actions.scroll_to_center = true;
            actions.message = Some("滚动到中央 (zz)".to_string());
            cmd.clear();
            return;
        }
        if i.key_pressed(Key::T) {
            // zt: 置顶
            state.scroll_to_row = Some(state.cursor.0);
            actions.scroll_to_top = true;
            actions.message = Some("滚动到顶部 (zt)".to_string());
            cmd.clear();
            return;
        }
        if i.key_pressed(Key::B) {
            // zb: 置底
            state.scroll_to_row = Some(state.cursor.0);
            actions.scroll_to_bottom = true;
            actions.message = Some("滚动到底部 (zb)".to_string());
            cmd.clear();
            return;
        }
    }

    // === Space 前缀命令 ===
    if i.key_pressed(Key::Space) && cmd.keys.is_empty() {
        cmd.keys = " ".to_string();
        return;
    }

    if cmd.keys == " " && i.key_pressed(Key::D) {
        // Space+d: 标记删除
        let row_idx = state.cursor.0;
        if !state.rows_to_delete.contains(&row_idx) {
            state.rows_to_delete.push(row_idx);
            actions.message = Some(format!("标记删除第 {} 行 (Space+d)", row_idx + 1));
        }
        cmd.clear();
        return;
    }

    // === : 前缀命令 ===
    if i.key_pressed(Key::Semicolon) && i.modifiers.shift && cmd.keys.is_empty() {
        cmd.keys = ":".to_string();
        return;
    }

    if cmd.keys == ":" {
        if i.key_pressed(Key::W) {
            // :w 保存
            state.pending_save = true;
            actions.message = Some("保存修改 (:w)".to_string());
            cmd.clear();
            return;
        }
        if i.key_pressed(Key::Q) {
            // :q 放弃
            if state.has_changes() {
                state.clear_edits();
                actions.message = Some("已放弃所有修改 (:q)".to_string());
            }
            cmd.clear();
            return;
        }
    }

    // === d 前缀命令 ===
    if i.key_pressed(Key::D) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        cmd.keys = "d".to_string();
        return;
    }

    if cmd.keys == "d" && i.key_pressed(Key::D) {
        // dd: 标记删除当前行
        let row_idx = state.cursor.0;
        if !state.rows_to_delete.contains(&row_idx) {
            state.rows_to_delete.push(row_idx);
            actions.message = Some(format!("已标记删除第 {} 行 (dd)", row_idx + 1));
        }
        cmd.clear();
        return;
    }

    // === y 前缀命令 ===
    if i.key_pressed(Key::Y) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        cmd.keys = "y".to_string();
        return;
    }

    if cmd.keys == "y" && i.key_pressed(Key::Y) {
        // yy: 复制整行
        if let Some((_, row_data)) = filtered_rows.get(state.cursor.0) {
            let row_text = row_data.join("\t");
            state.clipboard = Some(row_text);
            actions.message = Some(format!("已复制第 {} 行 (yy)", state.cursor.0 + 1));
        }
        cmd.clear();
        return;
    }

    // === 单键命令 ===

    // p: 粘贴
    if i.key_pressed(Key::P) && cmd.keys.is_empty() {
        if let Some(text) = &state.clipboard {
            state.modified_cells.insert(state.cursor, text.clone());
            actions.message = Some("已粘贴 (p)".to_string());
        }
        return;
    }

    // u: 撤销修改
    if i.key_pressed(Key::U) && !i.modifiers.shift && !i.modifiers.ctrl && cmd.keys.is_empty() {
        if state.modified_cells.remove(&state.cursor).is_some() {
            actions.message = Some("已撤销修改 (u)".to_string());
        }
        return;
    }

    // U: 取消删除标记
    if i.key_pressed(Key::U) && i.modifiers.shift && cmd.keys.is_empty() {
        if state.rows_to_delete.contains(&state.cursor.0) {
            state.rows_to_delete.retain(|&x| x != state.cursor.0);
            actions.message = Some("已取消删除标记 (U)".to_string());
        }
        return;
    }

    // q: 放弃所有修改
    if i.key_pressed(Key::Q) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        if state.has_changes() {
            state.clear_edits();
            actions.message = Some("已放弃所有修改 (q)".to_string());
        }
        return;
    }

    // Ctrl+S: 保存
    if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(Key::S) {
        state.pending_save = true;
        actions.message = Some("保存修改 (Ctrl+S)".to_string());
        return;
    }

    // Ctrl+R: 刷新
    if i.modifiers.ctrl && i.key_pressed(Key::R) {
        actions.refresh_requested = true;
        actions.message = Some("刷新表格数据 (Ctrl+R)".to_string());
        return;
    }

    // === 模式切换 ===

    // i: 进入插入模式
    if i.key_pressed(Key::I) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        enter_insert_mode(state, filtered_rows);
        return;
    }

    // a: 进入插入模式（追加）
    if i.key_pressed(Key::A) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        enter_insert_mode(state, filtered_rows);
        return;
    }

    // c: 清空并进入插入模式
    if i.key_pressed(Key::C) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        state.mode = GridMode::Insert;
        state.editing_cell = Some(state.cursor);
        state.edit_text.clear();
        if let Some((_, row_data)) = filtered_rows.get(state.cursor.0)
            && let Some(cell) = row_data.get(state.cursor.1)
        {
            state.original_value = cell.to_string();
        }
        actions.message = Some("修改单元格 (c)".to_string());
        return;
    }

    // r: 替换模式
    if i.key_pressed(Key::R) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        state.mode = GridMode::Insert;
        state.editing_cell = Some(state.cursor);
        state.edit_text.clear();
        state.original_value.clear();
        return;
    }

    // v: 进入选择模式
    if i.key_pressed(Key::V) && !i.modifiers.shift && cmd.keys.is_empty() {
        state.mode = GridMode::Select;
        state.select_anchor = Some(state.cursor);
        return;
    }

    // x: 选择整行
    if i.key_pressed(Key::X) && !i.modifiers.shift && cmd.keys.is_empty() {
        state.mode = GridMode::Select;
        state.select_anchor = Some((state.cursor.0, 0));
        state.cursor.1 = max_col.saturating_sub(1);
        actions.message = Some("选择整行 (x)".to_string());
        return;
    }

    // %: 选择全部 (Shift+5)
    if i.key_pressed(Key::Num5) && i.modifiers.shift && cmd.keys.is_empty() {
        state.mode = GridMode::Select;
        state.select_anchor = Some((0, 0));
        state.cursor = (max_row.saturating_sub(1), max_col.saturating_sub(1));
        actions.message = Some("选择全部 (%)".to_string());
        return;
    }

    // ;: 折叠选择（从选择模式退出）
    if i.key_pressed(Key::Semicolon) && !i.modifiers.shift && cmd.keys.is_empty() {
        if state.mode == GridMode::Select {
            state.mode = GridMode::Normal;
            state.select_anchor = None;
            actions.message = Some("折叠选择 (;)".to_string());
        }
        return;
    }

    // === 筛选 ===

    // /: 打开筛选面板
    if i.key_pressed(Key::Slash) && cmd.keys.is_empty() {
        actions.open_filter_panel = true;
        return;
    }

    // f: 为当前列添加筛选
    if i.key_pressed(Key::F) && !i.modifiers.ctrl && cmd.keys.is_empty() {
        if let Some(col_name) = result.columns.get(state.cursor.1)
            && !state.filters.iter().any(|f| &f.column == col_name)
        {
            state.filters.push(ColumnFilter::new(col_name.clone()));
            actions.message = Some(format!("为列 {} 添加筛选 (f)", col_name));
        }
        return;
    }

    // === 新增行 ===

    // o: 在末尾添加新行
    if i.key_pressed(Key::O) && !i.modifiers.shift && cmd.keys.is_empty() {
        let new_row = vec!["".to_string(); result.columns.len()];
        state.new_rows.push(new_row);
        let new_row_idx = result.rows.len() + state.new_rows.len() - 1;
        state.cursor = (new_row_idx, 0);
        state.scroll_to_row = Some(new_row_idx);
        actions.message = Some("已添加新行 (o)".to_string());
        return;
    }

    // O: 在开头添加新行
    if i.key_pressed(Key::O) && i.modifiers.shift && cmd.keys.is_empty() {
        let new_row = vec!["".to_string(); result.columns.len()];
        state.new_rows.insert(0, new_row);
        let new_row_idx = result.rows.len();
        state.cursor = (new_row_idx, 0);
        state.scroll_to_row = Some(new_row_idx);
        actions.message = Some("已在开头添加新行 (O)".to_string());
        return;
    }

    // === Escape ===
    if i.key_pressed(Key::Escape) {
        if !cmd.keys.is_empty() || cmd.count.is_some() {
            cmd.clear();
        } else if !state.filters.is_empty() {
            state.filters.clear();
            actions.message = Some("已清空筛选条件 (Esc)".to_string());
        }
    }
}

fn handle_select_mode(
    i: &egui::InputState,
    state: &mut DataGridState,
    filtered_rows: &[(usize, &Vec<String>)],
    actions: &mut DataGridActions,
    max_row: usize,
    max_col: usize,
    cmd: &mut CmdBuffer,
) {
    // === 导航（扩展选区）===

    if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
        state.move_cursor(0, -1, max_row, max_col);
        return;
    }
    if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
        state.move_cursor(1, 0, max_row, max_col);
        return;
    }
    if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
        state.move_cursor(-1, 0, max_row, max_col);
        return;
    }
    if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
        state.move_cursor(0, 1, max_row, max_col);
        return;
    }
    if i.key_pressed(Key::W) {
        state.move_cursor(0, 1, max_row, max_col);
        return;
    }
    if i.key_pressed(Key::B) {
        state.move_cursor(0, -1, max_row, max_col);
        return;
    }

    // d: 删除选中内容
    if i.key_pressed(Key::D) {
        if let Some(((min_r, min_c), (max_r, max_c))) = state.get_selection() {
            for r in min_r..=max_r {
                for c in min_c..=max_c {
                    state.modified_cells.insert((r, c), String::new());
                }
            }
            let cell_count = (max_r - min_r + 1) * (max_c - min_c + 1);
            actions.message = Some(format!("已清空 {} 个单元格 (d)", cell_count));
        }
        state.mode = GridMode::Normal;
        state.select_anchor = None;
        return;
    }

    // c: 清空选中并进入插入模式
    if i.key_pressed(Key::C) {
        if let Some(((min_r, min_c), (max_r, max_c))) = state.get_selection() {
            for r in min_r..=max_r {
                for c in min_c..=max_c {
                    state.modified_cells.insert((r, c), String::new());
                }
            }
        }
        state.mode = GridMode::Insert;
        state.editing_cell = Some(state.cursor);
        state.edit_text.clear();
        state.original_value.clear();
        state.select_anchor = None;
        return;
    }

    // y: 复制选中内容
    if i.key_pressed(Key::Y) {
        if let Some(((min_r, min_c), (max_r, max_c))) = state.get_selection() {
            let mut text = String::new();
            for r in min_r..=max_r {
                if let Some((_, row_data)) = filtered_rows.get(r) {
                    let row_text: Vec<&str> = (min_c..=max_c)
                        .filter_map(|c| row_data.get(c).map(|s| s.as_str()))
                        .collect();
                    if !text.is_empty() {
                        text.push('\n');
                    }
                    text.push_str(&row_text.join("\t"));
                }
            }
            state.clipboard = Some(text);
            actions.message = Some("已复制选中内容 (y)".to_string());
        }
        state.mode = GridMode::Normal;
        state.select_anchor = None;
        return;
    }

    // x: 选择整行
    if i.key_pressed(Key::X) {
        state.select_anchor = Some((state.cursor.0, 0));
        state.cursor.1 = max_col.saturating_sub(1);
        return;
    }

    // Esc/;: 退出选择模式
    if i.key_pressed(Key::Escape) || (i.key_pressed(Key::Semicolon) && !i.modifiers.shift) {
        state.mode = GridMode::Normal;
        state.select_anchor = None;
        cmd.clear();
    }
}

fn enter_insert_mode(state: &mut DataGridState, filtered_rows: &[(usize, &Vec<String>)]) {
    state.mode = GridMode::Insert;
    state.editing_cell = Some(state.cursor);

    if let Some((_, row_data)) = filtered_rows.get(state.cursor.0)
        && let Some(cell) = row_data.get(state.cursor.1)
    {
        state.edit_text = state
            .modified_cells
            .get(&state.cursor)
            .cloned()
            .unwrap_or_else(|| cell.to_string());
        state.original_value = cell.to_string();
    }
}
