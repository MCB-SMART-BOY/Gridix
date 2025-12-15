//! 单元格渲染

#![allow(clippy::too_many_arguments)]

use super::mode::GridMode;
use super::state::DataGridState;
use super::{
    CELL_TRUNCATE_LEN, COLOR_CELL_EDITING, COLOR_CELL_MODIFIED, COLOR_CELL_SELECTED,
    COLOR_VISUAL_SELECT,
};
use crate::ui::styles::GRAY;
use egui::{self, Color32, Key, RichText, Sense, TextEdit};


// NULL 值颜色
const COLOR_NULL: Color32 = Color32::from_rgb(120, 120, 140);

/// 渲染列头
pub fn render_column_header(
    ui: &mut egui::Ui,
    col_name: &str,
    col_idx: usize,
    state: &DataGridState,
    columns_to_filter: &mut Vec<String>,
) {
    ui.horizontal(|ui| {
        let is_cursor_col = state.cursor.1 == col_idx;
        let has_filter = state.filters.iter().any(|f| f.column == col_name);

        // 列名文字 - 确保在所有主题下都清晰可见
        let text = if is_cursor_col {
            RichText::new(col_name).strong().color(state.mode.color())
        } else if has_filter {
            RichText::new(col_name)
                .strong()
                .color(Color32::from_rgb(150, 200, 100))
        } else {
            // 使用默认文字颜色（由主题控制），不单独设置颜色
            RichText::new(col_name).strong()
        };
        ui.label(text);

        let filter_btn = if has_filter { "▼" } else { "▽" };
        let btn_color = if has_filter {
            Color32::from_rgb(150, 200, 100)
        } else {
            GRAY
        };
        if ui
            .add(egui::Button::new(RichText::new(filter_btn).small().color(btn_color)).small())
            .on_hover_text(format!("为 {} 列添加/编辑筛选条件", col_name))
            .clicked()
        {
            columns_to_filter.push(col_name.to_string());
        }
    });
}

/// 渲染行号单元格
pub fn render_row_number(
    ui: &mut egui::Ui,
    row_idx: usize,
    is_cursor_row: bool,
    is_deleted: bool,
    state: &mut DataGridState,
) {
    // 只在删除状态时设置背景色，普通行由表格的 set_selected 和 striped 效果处理
    let bg = if is_deleted {
        Color32::from_rgb(150, 50, 50)
    } else {
        Color32::TRANSPARENT
    };

    egui::Frame::none()
        .fill(bg)
        .inner_margin(4.0)
        .show(ui, |ui| {
            let text = if is_deleted {
                RichText::new(format!("✕{}", row_idx + 1))
                    .color(Color32::WHITE)
                    .small()
            } else if is_cursor_row {
                RichText::new(format!("{}", row_idx + 1))
                    .color(state.mode.color())
                    .small()
            } else {
                RichText::new(format!("{}", row_idx + 1))
                    .color(GRAY)
                    .small()
            };

            let response = ui.add(egui::Label::new(text).sense(Sense::click()));

            if response.clicked() {
                state.cursor.0 = row_idx;
                state.focused = true;
            }

            response.context_menu(|ui| {
                if is_deleted {
                    if ui.button("取消删除 [u]").clicked() {
                        state.rows_to_delete.retain(|&x| x != row_idx);
                        ui.close_menu();
                    }
                } else if ui.button("标记删除 [Space+d]").clicked() {
                    if !state.rows_to_delete.contains(&row_idx) {
                        state.rows_to_delete.push(row_idx);
                    }
                    ui.close_menu();
                }
            });
        });
}

/// 渲染可编辑的数据单元格
pub fn render_editable_cell(
    ui: &mut egui::Ui,
    cell: &str,
    row_idx: usize,
    col_idx: usize,
    _is_cursor_row: bool, // 行级别高亮由 set_selected 处理
    is_row_deleted: bool,
    state: &mut DataGridState,
) {
    let is_cursor = state.cursor == (row_idx, col_idx);
    let is_editing = state.editing_cell == Some((row_idx, col_idx));
    let is_modified = state.modified_cells.contains_key(&(row_idx, col_idx));
    let is_selected = state.mode == GridMode::Select && state.is_in_selection(row_idx, col_idx);

    let display_value = state
        .modified_cells
        .get(&(row_idx, col_idx))
        .cloned()
        .unwrap_or_else(|| cell.to_string());

    // 只在特殊单元格状态时设置背景色，行级别高亮由表格的 set_selected 处理
    let bg_color = if is_row_deleted {
        Color32::from_rgba_unmultiplied(150, 50, 50, 100)
    } else if is_editing {
        COLOR_CELL_EDITING
    } else if is_selected {
        COLOR_VISUAL_SELECT
    } else if is_modified {
        COLOR_CELL_MODIFIED
    } else if is_cursor {
        COLOR_CELL_SELECTED
    } else {
        Color32::TRANSPARENT
    };

    egui::Frame::none()
        .fill(bg_color)
        .inner_margin(4.0)
        .show(ui, |ui| {
            if is_editing && state.mode == GridMode::Insert {
                render_editing_cell(ui, state, row_idx, col_idx);
            } else {
                render_display_cell(
                    ui,
                    state,
                    cell,
                    &display_value,
                    row_idx,
                    col_idx,
                    is_cursor,
                    is_row_deleted,
                );
            }
        });
}

fn render_editing_cell(
    ui: &mut egui::Ui,
    state: &mut DataGridState,
    row_idx: usize,
    col_idx: usize,
) {
    let response = ui.add(
        TextEdit::singleline(&mut state.edit_text)
            .desired_width(ui.available_width() - 8.0)
            .font(egui::TextStyle::Monospace),
    );

    let should_exit = ui.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Enter));

    if should_exit || response.lost_focus() {
        if state.edit_text != state.original_value {
            state
                .modified_cells
                .insert((row_idx, col_idx), state.edit_text.clone());
        }
        state.editing_cell = None;
        state.mode = GridMode::Normal;
    }

    response.request_focus();
}

fn render_display_cell(
    ui: &mut egui::Ui,
    state: &mut DataGridState,
    cell: &str,
    display_value: &str,
    row_idx: usize,
    col_idx: usize,
    is_cursor: bool,
    is_row_deleted: bool,
) {
    let cell_text = format_cell_text(display_value, is_cursor);
    let response = ui.add(egui::Label::new(cell_text).sense(Sense::click()));

    if response.clicked() {
        state.cursor = (row_idx, col_idx);
        state.focused = true;
    }

    if response.double_clicked() && !is_row_deleted {
        state.mode = GridMode::Insert;
        state.editing_cell = Some((row_idx, col_idx));
        state.edit_text = display_value.to_string();
        state.original_value = cell.to_string();
    }

    let show_hover = display_value.len() > CELL_TRUNCATE_LEN;

    response.context_menu(|ui| {
        if ui.button("编辑 [i]").clicked() {
            state.mode = GridMode::Insert;
            state.editing_cell = Some((row_idx, col_idx));
            state.edit_text = display_value.to_string();
            state.original_value = cell.to_string();
            ui.close_menu();
        }
        if ui.button("复制 [y]").clicked() {
            state.clipboard = Some(display_value.to_string());
            ui.ctx().copy_text(display_value.to_string());
            ui.close_menu();
        }
        if ui.button("粘贴 [p]").clicked() {
            if let Some(text) = &state.clipboard {
                state
                    .modified_cells
                    .insert((row_idx, col_idx), text.clone());
            }
            ui.close_menu();
        }
        if state.modified_cells.contains_key(&(row_idx, col_idx)) && ui.button("还原 [u]").clicked()
        {
            state.modified_cells.remove(&(row_idx, col_idx));
            ui.close_menu();
        }
    });

    if show_hover {
        response.on_hover_text(display_value);
    }
}

fn format_cell_text(cell: &str, is_cursor: bool) -> RichText {
    let text = if cell == "NULL" {
        // NULL 值使用斜体、特殊颜色和背景标记
        RichText::new("∅ NULL").italics().color(COLOR_NULL)
    } else if cell.len() > CELL_TRUNCATE_LEN {
        RichText::new(format!("{}...", &cell[..CELL_TRUNCATE_LEN - 3]))
    } else {
        RichText::new(cell)
    };

    if is_cursor {
        text.underline()
    } else {
        text
    }
}

// 新增行的背景色 - 浅绿色表示待保存
const COLOR_NEW_ROW: Color32 = Color32::from_rgba_premultiplied(48, 96, 48, 60);

/// 渲染新增行的单元格
pub fn render_new_row_cell(
    ui: &mut egui::Ui,
    cell: &str,
    row_idx: usize,
    col_idx: usize,
    _is_cursor_row: bool,
    state: &mut DataGridState,
) {
    let is_cursor = state.cursor == (row_idx, col_idx);
    let is_editing = state.editing_cell == Some((row_idx, col_idx));

    // 新增行使用特殊背景色
    let bg_color = if is_editing {
        COLOR_CELL_EDITING
    } else if is_cursor {
        COLOR_CELL_SELECTED
    } else {
        COLOR_NEW_ROW
    };

    egui::Frame::none()
        .fill(bg_color)
        .inner_margin(4.0)
        .show(ui, |ui| {
            if is_editing && state.mode == GridMode::Insert {
                render_new_row_editing_cell(ui, state, row_idx, col_idx);
            } else {
                render_new_row_display_cell(ui, state, cell, row_idx, col_idx, is_cursor);
            }
        });
}

fn render_new_row_editing_cell(
    ui: &mut egui::Ui,
    state: &mut DataGridState,
    row_idx: usize,
    col_idx: usize,
) {
    let response = ui.add(
        TextEdit::singleline(&mut state.edit_text)
            .desired_width(ui.available_width() - 8.0)
            .font(egui::TextStyle::Monospace),
    );

    let should_exit = ui.input(|i| i.key_pressed(Key::Escape) || i.key_pressed(Key::Enter));

    if should_exit || response.lost_focus() {
        // 计算新增行的索引（row_idx - 原始结果行数）
        // 由于 new_rows 的修改需要通过特殊方式处理，这里直接保存到 edit_text
        state.editing_cell = None;
        state.mode = GridMode::Normal;
        // 新增行的编辑会通过 pending_new_row_edit 处理
        state.pending_new_row_edit = Some((row_idx, col_idx, state.edit_text.clone()));
    }

    response.request_focus();
}

fn render_new_row_display_cell(
    ui: &mut egui::Ui,
    state: &mut DataGridState,
    cell: &str,
    row_idx: usize,
    col_idx: usize,
    is_cursor: bool,
) {
    let display_value = if cell.is_empty() {
        "(空)"
    } else {
        cell
    };

    let cell_text = if cell.is_empty() {
        RichText::new(display_value).italics().color(GRAY)
    } else if is_cursor {
        RichText::new(display_value).underline()
    } else {
        RichText::new(display_value)
    };

    let response = ui.add(egui::Label::new(cell_text).sense(Sense::click()));

    if response.clicked() {
        state.cursor = (row_idx, col_idx);
        state.focused = true;
    }

    if response.double_clicked() {
        state.mode = GridMode::Insert;
        state.editing_cell = Some((row_idx, col_idx));
        state.edit_text = cell.to_string();
        state.original_value = cell.to_string();
    }

    response.context_menu(|ui| {
        if ui.button("编辑 [i]").clicked() {
            state.mode = GridMode::Insert;
            state.editing_cell = Some((row_idx, col_idx));
            state.edit_text = cell.to_string();
            state.original_value = cell.to_string();
            ui.close_menu();
        }
        if ui.button("粘贴 [p]").clicked() {
            if let Some(text) = &state.clipboard {
                state.pending_new_row_edit = Some((row_idx, col_idx, text.clone()));
            }
            ui.close_menu();
        }
    });
}
