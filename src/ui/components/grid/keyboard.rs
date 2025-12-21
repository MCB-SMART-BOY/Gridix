//! 键盘输入处理（Helix 风格）
//!
//! ## Normal 模式键位
//! - `h/j/k/l`: 移动光标（支持数字前缀，如 5j）
//! - `w`: 右移一列
//! - `b`: 左移一列
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
//! - `d`: 删除当前单元格
//! - `dd`: 标记删除当前行
//! - `y`: 复制当前单元格
//! - `yy`: 复制整行
//! - `p`: 粘贴
//! - `u/U`: 撤销
//! - `/`: 添加筛选
//! - `f`: 为当前列添加筛选
//! - `o/O`: 添加新行
//! - `:w`: 保存修改
//! - `q`: 放弃修改
//! - `Ctrl+R`: 刷新表格数据
//! - `Space+d`: 标记删除行
//! - `Ctrl+S`: 保存修改
//!
//! ## 视图模式 (z 前缀)
//! - `zz`: 将当前行滚动到屏幕中央
//! - `zt`: 将当前行滚动到屏幕顶部
//! - `zb`: 将当前行滚动到屏幕底部
//! - `zc`: 居中当前行（同 zz）
//!
//! ## 数字计数
//! - `1-9`: 输入计数前缀（如 10j 向下移动10行）
//! - `0`: 追加到已有计数（如 10 中的 0）
//! - `Backspace`: 回退计数数字

#![allow(clippy::too_many_arguments)]

use super::actions::DataGridActions;
use super::filter::ColumnFilter;
use super::mode::GridMode;
use super::state::DataGridState;
use crate::database::QueryResult;
use egui::{self, Key};

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
        return;
    }

    let half_page = (max_row / 2).max(1);

    ui.input(|i| {
        // 数字前缀（支持 0-9，用于 10j 等命令）
        // 注意：0 只有在已有计数时才追加，否则 0 可能用于其他功能
        for digit in 0..=9 {
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
            if i.key_pressed(key) && !i.modifiers.ctrl {
                // 0 只有在已有计数时才追加（避免单独按 0 误触发）
                if digit == 0 && state.count.is_none() {
                    continue;
                }
                let current = state.count.unwrap_or(0);
                state.count = Some(current * 10 + digit);
                return;
            }
        }
        
        // 数字 + Enter: 切换到指定的查询Tab
        if i.key_pressed(Key::Enter) && state.count.is_some() && state.command_buffer.is_empty() {
            let tab_number = state.count.unwrap();
            if tab_number > 0 {
                // 转换为 0-indexed
                actions.switch_to_tab = Some(tab_number - 1);
                actions.message = Some(format!("切换到查询 {}", tab_number));
            }
            state.count = None;
            return;
        }
        
        // Backspace 回退数字计数
        if i.key_pressed(Key::Backspace) && state.count.is_some() {
            let current = state.count.unwrap_or(0);
            if current < 10 {
                state.count = None;
            } else {
                state.count = Some(current / 10);
            }
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
                );
            }
            GridMode::Select => {
                handle_select_mode(i, state, filtered_rows, actions, max_row, max_col);
            }
            GridMode::Insert => {}
        }
    });
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
) {
    // === 基本移动（仅在无命令前缀时生效） ===
    if state.command_buffer.is_empty() {
        // h/左箭头：向左移动，如果已在最左边则转移焦点到侧边栏
        if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
            if state.cursor.1 == 0 {
                // 已在最左边，转移焦点到侧边栏
                actions.focus_transfer = Some(super::actions::FocusTransfer::ToSidebar);
            } else {
                state.move_cursor(0, -1, max_row, max_col);
            }
        }
        // j/下箭头：向下移动，如果已在最下面则转移焦点到编辑区
        if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
            if state.cursor.0 >= max_row.saturating_sub(1) {
                // 已在最下面，转移焦点到 SQL 编辑器
                actions.focus_transfer = Some(super::actions::FocusTransfer::ToSqlEditor);
            } else {
                state.move_cursor(1, 0, max_row, max_col);
            }
        }
        // k/上箭头：向上移动，如果已在第一行则转移焦点到查询Tab栏
        if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
            if state.cursor.0 == 0 {
                // 已在第一行，转移焦点到查询Tab栏
                actions.focus_transfer = Some(super::actions::FocusTransfer::ToQueryTabs);
            } else {
                state.move_cursor(-1, 0, max_row, max_col);
            }
        }
        if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
            state.move_cursor(0, 1, max_row, max_col);
        }
    }

    // w 移动：向右移动一列（Helix 风格，类似于下一个单词）
    if i.key_pressed(Key::W) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        if state.cursor.1 >= max_col.saturating_sub(1) {
            // 已在最右边，转移焦点到数据表格外
            actions.focus_transfer = Some(super::actions::FocusTransfer::ToSqlEditor);
        } else {
            state.move_cursor(0, 1, max_row, max_col);
        }
    }

    // b 移动：向左，如果已在最左边则转移焦点到侧边栏
    if i.key_pressed(Key::B) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        if state.cursor.1 == 0 {
            actions.focus_transfer = Some(super::actions::FocusTransfer::ToSidebar);
        } else {
            state.move_cursor(0, -1, max_row, max_col);
        }
    }

    // e 移动：跳转到行尾（Helix 风格，类似于单词末尾）
    if i.key_pressed(Key::E) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        state.goto_line_end(max_col);
    }

    // Home/End
    if i.key_pressed(Key::Home) {
        state.goto_line_start();
    }
    if i.key_pressed(Key::End) {
        state.goto_line_end(max_col);
    }

    // 0: 跳转到行首（当没有数字计数时）
    if i.key_pressed(Key::Num0) && state.count.is_none() && state.command_buffer.is_empty() {
        state.goto_line_start();
    }

    // $: 跳转到行尾（Shift+4）
    if i.key_pressed(Key::Num4) && i.modifiers.shift && state.command_buffer.is_empty() {
        state.goto_line_end(max_col);
    }

    // ^: 跳转到行首（Shift+6）
    if i.key_pressed(Key::Num6) && i.modifiers.shift && state.command_buffer.is_empty() {
        state.goto_line_start();
    }

    // Ctrl+U 向上翻半页（Ctrl+D 已用于切换日夜主题，使用 PageUp/PageDown 代替）
    if i.modifiers.ctrl && i.key_pressed(Key::U) {
        let count = state.count.unwrap_or(1);
        let delta = half_page * count;
        state.cursor.0 = state.cursor.0.saturating_sub(delta);
        state.scroll_to_row = Some(state.cursor.0);
        state.count = None;
    }
    // PageUp/PageDown 翻页
    if i.key_pressed(Key::PageUp) {
        let count = state.count.unwrap_or(1);
        let delta = half_page * count;
        state.cursor.0 = state.cursor.0.saturating_sub(delta);
        state.scroll_to_row = Some(state.cursor.0);
        state.count = None;
    }
    if i.key_pressed(Key::PageDown) {
        let count = state.count.unwrap_or(1);
        let delta = half_page * count;
        state.cursor.0 = (state.cursor.0 + delta).min(max_row.saturating_sub(1));
        state.scroll_to_row = Some(state.cursor.0);
        state.count = None;
    }

    // === goto 模式 (g 前缀) ===
    if i.key_pressed(Key::G) && !i.modifiers.shift {
        if state.command_buffer == "g" {
            // gg: 跳转到文件首
            state.goto_file_start();
        } else if state.command_buffer.is_empty() {
            state.command_buffer = "g".to_string();
        }
    }
    if i.key_pressed(Key::G) && i.modifiers.shift {
        // G: 跳转到文件尾
        state.goto_file_end(max_row);
        state.command_buffer.clear();
    }
    if i.key_pressed(Key::E) && state.command_buffer == "g" {
        // ge: 跳转到文件尾
        state.goto_file_end(max_row);
        state.command_buffer.clear();
    }
    if i.key_pressed(Key::H) && state.command_buffer == "g" {
        // gh: 跳转到行首
        state.goto_line_start();
        state.command_buffer.clear();
    }
    if i.key_pressed(Key::L) && state.command_buffer == "g" {
        // gl: 跳转到行尾
        state.goto_line_end(max_col);
        state.command_buffer.clear();
    }

    // === 视图模式 (z 前缀) ===
    if i.key_pressed(Key::Z) && !i.modifiers.shift && state.command_buffer.is_empty() {
        state.command_buffer = "z".to_string();
    }
    // zz/zc: 将当前行滚动到屏幕中央
    if (i.key_pressed(Key::Z) || i.key_pressed(Key::C)) && state.command_buffer == "z" {
        state.scroll_to_row = Some(state.cursor.0);
        actions.scroll_to_center = true;
        actions.message = Some("滚动到中央 (zz)".to_string());
        state.command_buffer.clear();
    }
    // zt: 将当前行滚动到屏幕顶部
    if i.key_pressed(Key::T) && state.command_buffer == "z" {
        state.scroll_to_row = Some(state.cursor.0);
        actions.scroll_to_top = true;
        actions.message = Some("滚动到顶部 (zt)".to_string());
        state.command_buffer.clear();
    }
    // zb: 将当前行滚动到屏幕底部
    if i.key_pressed(Key::B) && state.command_buffer == "z" {
        state.scroll_to_row = Some(state.cursor.0);
        actions.scroll_to_bottom = true;
        actions.message = Some("滚动到底部 (zb)".to_string());
        state.command_buffer.clear();
    }

    // === Space 模式 ===
    if i.key_pressed(Key::Space) && state.command_buffer.is_empty() {
        state.command_buffer = " ".to_string();
    }
    if i.key_pressed(Key::D) && state.command_buffer == " " {
        let row_idx = state.cursor.0;
        if !state.rows_to_delete.contains(&row_idx) {
            state.rows_to_delete.push(row_idx);
            actions.message = Some(format!("标记删除第 {} 行 (Space+d)", row_idx + 1));
        }
        state.command_buffer.clear();
    }
    // === 命令模式 (: 前缀) ===
    // 注意：egui 的 Key::Semicolon 加 Shift 是冒号
    if i.key_pressed(Key::Semicolon) && i.modifiers.shift && state.command_buffer.is_empty() {
        state.command_buffer = ":".to_string();
    }
    // :w 保存修改
    if i.key_pressed(Key::W) && state.command_buffer == ":" {
        state.pending_save = true;
        state.command_buffer.clear();
        actions.message = Some("保存修改 (:w)".to_string());
    }
    // :q 放弃修改
    if i.key_pressed(Key::Q) && state.command_buffer == ":" {
        if state.has_changes() {
            state.clear_edits();
            actions.message = Some("已放弃所有修改 (:q)".to_string());
        }
        state.command_buffer.clear();
    }
    // :wq 保存并退出编辑
    // (在表格中，退出编辑意味着返回 Normal 模式)
    
    // Ctrl+S 保存（兼容快捷键）
    if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(Key::S) {
        state.pending_save = true;
        actions.message = Some("保存修改 (Ctrl+S)".to_string());
    }

    // q 放弃修改（兼容旧快捷键，不需要冒号前缀）
    if i.key_pressed(Key::Q) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        if state.has_changes() {
            state.clear_edits();
            actions.message = Some("已放弃所有修改 (q)".to_string());
        }
    }

    // === 模式切换 ===
    if i.key_pressed(Key::I) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        enter_insert_mode(state, filtered_rows);
    }
    if i.key_pressed(Key::A) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        enter_insert_mode(state, filtered_rows);
    }
    if i.key_pressed(Key::C) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        state.mode = GridMode::Insert;
        state.editing_cell = Some(state.cursor);
        state.edit_text.clear();
        if let Some((_, row_data)) = filtered_rows.get(state.cursor.0)
            && let Some(cell) = row_data.get(state.cursor.1) {
                state.original_value = cell.to_string();
            }
        actions.message = Some("修改单元格 (c)".to_string());
    }
    if i.key_pressed(Key::V) && state.command_buffer.is_empty() {
        state.mode = GridMode::Select;
        state.select_anchor = Some(state.cursor);
    }
    if i.key_pressed(Key::X) && !i.modifiers.shift && state.command_buffer.is_empty() {
        state.mode = GridMode::Select;
        state.select_anchor = Some((state.cursor.0, 0));
        state.cursor.1 = max_col.saturating_sub(1);
        actions.message = Some("选择整行 (x)".to_string());
    }
    // %: 选择全部
    if i.key_pressed(Key::Num5) && i.modifiers.shift && state.command_buffer.is_empty() {
        state.mode = GridMode::Select;
        state.select_anchor = Some((0, 0));
        state.cursor = (max_row.saturating_sub(1), max_col.saturating_sub(1));
        actions.message = Some("选择全部 (%)".to_string());
    }
    // ;: 折叠选择到单个光标
    if i.key_pressed(Key::Semicolon) && !i.modifiers.shift && state.command_buffer.is_empty() {
        if state.mode == GridMode::Select {
            state.mode = GridMode::Normal;
            state.select_anchor = None;
            actions.message = Some("折叠选择 (;)".to_string());
        }
    }

    // === 操作 ===
    // 'd' 命令前缀
    if i.key_pressed(Key::D) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        state.command_buffer = "d".to_string();
    }
    // 'dd' 标记删除当前行
    if i.key_pressed(Key::D) && state.command_buffer == "d" {
        let row_idx = state.cursor.0;
        if !state.rows_to_delete.contains(&row_idx) {
            state.rows_to_delete.push(row_idx);
            actions.message = Some(format!("已标记删除第 {} 行 (dd)", row_idx + 1));
        }
        state.command_buffer.clear();
    }
    // 单独的 'd' 在超时后删除单元格内容（通过 Escape 或其他键取消）
    
    // 'y' 命令前缀
    if i.key_pressed(Key::Y) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        state.command_buffer = "y".to_string();
    }
    // 'yy' 复制整行
    if i.key_pressed(Key::Y) && state.command_buffer == "y" {
        if let Some((_, row_data)) = filtered_rows.get(state.cursor.0) {
            let row_text = row_data.join("\t");
            state.clipboard = Some(row_text);
            actions.message = Some(format!("已复制第 {} 行 (yy)", state.cursor.0 + 1));
        }
        state.command_buffer.clear();
    }
    if i.key_pressed(Key::P) && state.command_buffer.is_empty()
        && let Some(text) = &state.clipboard {
            state.modified_cells.insert(state.cursor, text.clone());
            actions.message = Some("已粘贴 (p)".to_string());
        }
    if i.key_pressed(Key::U)
        && !i.modifiers.shift
        && !i.modifiers.ctrl
        && state.command_buffer.is_empty()
        && state.modified_cells.remove(&state.cursor).is_some()
    {
        actions.message = Some("已撤销修改 (u)".to_string());
    }
    if i.key_pressed(Key::U)
        && i.modifiers.shift
        && state.command_buffer.is_empty()
        && state.rows_to_delete.contains(&state.cursor.0)
    {
        state.rows_to_delete.retain(|&x| x != state.cursor.0);
        actions.message = Some("已取消删除标记 (U)".to_string());
    }
    if i.key_pressed(Key::R) && !i.modifiers.ctrl && state.command_buffer.is_empty() {
        state.mode = GridMode::Insert;
        state.editing_cell = Some(state.cursor);
        state.edit_text.clear();
        state.original_value.clear();
    }

    // === 筛选 ===
    // / 打开左侧栏筛选面板
    if i.key_pressed(Key::Slash) && state.command_buffer.is_empty() {
        actions.open_filter_panel = true;
    }
    // f 为当前列快速添加筛选（鼠标用户也可用）
    if i.key_pressed(Key::F) && !i.modifiers.ctrl && state.command_buffer.is_empty()
        && let Some(col_name) = result.columns.get(state.cursor.1)
            && !state.filters.iter().any(|f| &f.column == col_name) {
                state.filters.push(ColumnFilter::new(col_name.clone()));
                actions.message = Some(format!("为列 {} 添加筛选 (f)", col_name));
            }

    // === 新增行 ===
    // o: 在末尾添加新行并移动光标到新行
    if i.key_pressed(Key::O) && !i.modifiers.shift && state.command_buffer.is_empty() {
        let new_row = vec!["".to_string(); result.columns.len()];
        state.new_rows.push(new_row);
        // 移动光标到新增行（虚拟索引 = 原始行数 + 新增行索引）
        let new_row_idx = result.rows.len() + state.new_rows.len() - 1;
        state.cursor = (new_row_idx, 0);
        state.scroll_to_row = Some(new_row_idx);
        actions.message = Some("已添加新行 (o)".to_string());
    }
    // O: 在开头添加新行并移动光标到新行
    if i.key_pressed(Key::O) && i.modifiers.shift && state.command_buffer.is_empty() {
        let new_row = vec!["".to_string(); result.columns.len()];
        state.new_rows.insert(0, new_row);
        // 移动光标到新增行（虚拟索引 = 原始行数，因为是第一个新增行）
        let new_row_idx = result.rows.len();
        state.cursor = (new_row_idx, 0);
        state.scroll_to_row = Some(new_row_idx);
        actions.message = Some("已在开头添加新行 (O)".to_string());
    }

    // === 刷新 ===
    // Ctrl+R 刷新表格数据
    if i.modifiers.ctrl && i.key_pressed(Key::R) {
        actions.refresh_requested = true;
        actions.message = Some("刷新表格数据 (Ctrl+R)".to_string());
    }

    // Escape
    if i.key_pressed(Key::Escape) {
        if !state.command_buffer.is_empty() {
            state.command_buffer.clear();
            state.count = None;
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
) {
    // 移动扩展选择
    if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
        state.move_cursor(0, -1, max_row, max_col);
    }
    if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
        state.move_cursor(1, 0, max_row, max_col);
    }
    if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
        state.move_cursor(-1, 0, max_row, max_col);
    }
    if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
        state.move_cursor(0, 1, max_row, max_col);
    }
    if i.key_pressed(Key::W) {
        state.move_cursor(0, 1, max_row, max_col);
    }
    if i.key_pressed(Key::B) {
        state.move_cursor(0, -1, max_row, max_col);
    }

    // d 删除选中
    if i.key_pressed(Key::D) {
        if let Some(((min_r, min_c), (max_r, max_c))) = state.get_selection() {
            for r in min_r..=max_r {
                for c in min_c..=max_c {
                    state.modified_cells.insert((r, c), String::new());
                }
            }
            actions.message = Some(format!(
                "已清空 {} 个单元格 (d)",
                (max_r - min_r + 1) * (max_c - min_c + 1)
            ));
        }
        state.mode = GridMode::Normal;
        state.select_anchor = None;
    }

    // c 清空选中并进入插入
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
    }

    // y 复制选中
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
    }

    // x 选择整行
    if i.key_pressed(Key::X) {
        state.select_anchor = Some((state.cursor.0, 0));
        state.cursor.1 = max_col.saturating_sub(1);
    }

    // Esc 退出
    if i.key_pressed(Key::Escape) {
        state.mode = GridMode::Normal;
        state.select_anchor = None;
    }
}

fn enter_insert_mode(state: &mut DataGridState, filtered_rows: &[(usize, &Vec<String>)]) {
    state.mode = GridMode::Insert;
    state.editing_cell = Some(state.cursor);
    if let Some((_, row_data)) = filtered_rows.get(state.cursor.0)
        && let Some(cell) = row_data.get(state.cursor.1) {
            state.edit_text = state
                .modified_cells
                .get(&state.cursor)
                .cloned()
                .unwrap_or_else(|| cell.to_string());
            state.original_value = cell.to_string();
        }
}
