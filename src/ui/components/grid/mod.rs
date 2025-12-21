//! 数据表格组件模块
//!
//! 提供 Helix 编辑器风格的模态操作数据表格。
//!
//! ## 模块结构
//! - `state`: 表格状态管理
//! - `mode`: 编辑模式定义
//! - `filter`: 筛选条件（拆分为多个子模块）
//! - `keyboard`: 键盘输入处理
//! - `render`: 单元格渲染
//! - `actions`: 操作和 SQL 生成

#![allow(clippy::too_many_arguments)]

mod actions;
pub mod filter;
mod keyboard;
mod mode;
mod render;
mod state;

pub use actions::{escape_identifier, escape_value, quote_identifier, DataGridActions, FocusTransfer};
pub use filter::{
    check_filter_match, filter_rows_cached,
    ColumnFilter, FilterCache, FilterLogic, FilterOperator,
};
pub use mode::GridMode;
pub use state::DataGridState;

use crate::core::constants;
use crate::database::QueryResult;
use crate::ui::styles::GRAY;
use egui::{self, RichText, Vec2};
use egui_extras::{Column, TableBuilder};

// 使用集中管理的常量
use constants::grid::{HEADER_HEIGHT, MAX_COL_WIDTH, MIN_COL_WIDTH, ROW_HEIGHT};
pub(crate) const ROW_NUM_WIDTH: f32 = 50.0;
pub(crate) const CELL_TRUNCATE_LEN: usize = 50;
/// 每个字符的估计宽度（像素）
pub(crate) const CHAR_WIDTH: f32 = 8.0;

use egui::Color32;
pub(crate) const COLOR_CELL_SELECTED: Color32 = Color32::from_rgb(60, 100, 180);
pub(crate) const COLOR_CELL_EDITING: Color32 = Color32::from_rgb(80, 120, 200);
pub(crate) const COLOR_CELL_MODIFIED: Color32 = Color32::from_rgb(100, 150, 80);
pub(crate) const COLOR_VISUAL_SELECT: Color32 = Color32::from_rgb(120, 80, 160);

// ============================================================================
// 数据表格组件
// ============================================================================

pub struct DataGrid;

impl DataGrid {
    /// 显示可编辑的数据表格（Helix 风格）
    pub fn show_editable(
        ui: &mut egui::Ui,
        result: &QueryResult,
        search_text: &str,
        search_column: &Option<String>,
        selected_row: &mut Option<usize>,
        selected_cell: &mut Option<(usize, usize)>,
        state: &mut DataGridState,
        table_name: Option<&str>,
    ) -> (DataGridActions, (usize, usize)) {
        let mut actions = DataGridActions::default();

        if result.columns.is_empty() {
            Self::show_empty(ui);
            return (actions, (0, 0));
        }

        // 显示模式状态栏和操作按钮
        Self::show_mode_bar(ui, state, result, table_name, &mut actions);

        ui.add_space(2.0);

        // 显示跳转对话框
        Self::show_goto_dialog(ui.ctx(), state, result.rows.len());

        // 显示保存确认对话框
        Self::show_save_confirm_dialog(ui.ctx(), state, &mut actions);

        // 显示筛选状态栏（简洁版）
        let filter_changed = filter::show_filter_bar(ui, result, &mut state.filters);
        if filter_changed {
            state.filter_cache.invalidate();
        }

        ui.add_space(4.0);

        // 过滤行（使用缓存）
        let filtered_rows = filter::filter_rows_cached(
            result,
            search_text,
            search_column,
            &state.filters,
            &mut state.filter_cache,
        );
        // 总显示行数 = 筛选后的行 + 新增行
        let new_rows_count = state.new_rows.len();
        let filtered_count = filtered_rows.len() + new_rows_count;
        let total_count = result.rows.len() + new_rows_count;

        // 处理键盘输入
        keyboard::handle_keyboard(ui, state, result, &filtered_rows, &mut actions);

        // 处理新增行的编辑
        if let Some((virtual_idx, col_idx, new_value)) = state.pending_new_row_edit.take() {
            // 计算新增行在 new_rows 中的索引
            let new_row_idx = virtual_idx.saturating_sub(result.rows.len());
            if let Some(row_data) = state.new_rows.get_mut(new_row_idx)
                && col_idx < row_data.len() {
                    row_data[col_idx] = new_value;
                }
        }

        // 处理 Ctrl+S 保存请求
        if state.pending_save && state.has_changes() {
            if let Some(table) = table_name {
                actions::generate_save_sql(result, state, table, &mut actions);
            }
            state.pending_save = false;
        } else if state.pending_save {
            state.pending_save = false;
        }

        // 同步选择状态
        *selected_row = Some(state.cursor.0);
        *selected_cell = Some(state.cursor);

        // 获取每列的最佳宽度（使用缓存优化）
        let col_widths = Self::get_column_widths(result, &filtered_rows, &mut state.column_width_cache);

        // 收集需要添加筛选的列
        let mut columns_to_filter: Vec<String> = Vec::new();

        // 获取需要滚动到的行（表格内部处理垂直滚动）
        let scroll_to_row = state.scroll_to_row.take();
        let _ = state.scroll_to_col.take();
        
        // 获取可用宽度
        let available_width = ui.available_width();
        
        // 计算目标列的位置信息
        let current_col = state.cursor.1;
        let mut col_left = ROW_NUM_WIDTH;
        for i in 0..current_col {
            if let Some(&w) = col_widths.get(i) {
                col_left += w;
            }
        }
        let col_width = col_widths.get(current_col).copied().unwrap_or(MIN_COL_WIDTH);
        let col_right = col_left + col_width;
        
        // 检测光标列是否改变
        let col_changed = current_col != state.last_cursor_col;
        state.last_cursor_col = current_col;
        
        // 计算水平滚动偏移
        let mut target_h_offset = state.h_scroll_offset;
        if col_changed {
            // 向左移动时：确保列的左边缘可见
            if col_left < state.h_scroll_offset + ROW_NUM_WIDTH {
                target_h_offset = (col_left - ROW_NUM_WIDTH).max(0.0);
            }
            // 向右移动时：确保列的右边缘完全可见（预留100像素边距）
            else if col_right > state.h_scroll_offset + available_width - 100.0 {
                target_h_offset = col_right - available_width + 100.0;
            }
        }

        // 创建表格
        let table_response = egui::Frame::NONE.show(ui, |ui| {
            let scroll_output = egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .scroll_offset(egui::vec2(target_h_offset, 0.0))
                .show(ui, |ui| {
                    // 构建表格，保留内部垂直滚动
                    let mut table_builder = TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Column::exact(ROW_NUM_WIDTH));

                    // 为每列设置基于内容的初始宽度
                    for &width in &col_widths {
                        table_builder = table_builder.column(
                            Column::initial(width)
                                .at_least(MIN_COL_WIDTH)
                                .clip(true),
                        );
                    }

                    // 使用表格内部的垂直滚动
                    if let Some(target_row) = scroll_to_row {
                        table_builder = table_builder.scroll_to_row(target_row, Some(egui::Align::Center));
                    }


                    table_builder
                        .header(HEADER_HEIGHT, |mut header| {
                            // 行号列头
                            header.col(|ui| {
                                ui.label(RichText::new("#").strong().color(GRAY));
                            });
                            // 数据列头
                            for (col_idx, col_name) in result.columns.iter().enumerate() {
                                header.col(|ui| {
                                    render::render_column_header(
                                        ui,
                                        col_name,
                                        col_idx,
                                        state,
                                        &mut columns_to_filter,
                                    );
                                });
                            }
                        })
                        .body(|body| {
                            let filtered_rows_len = filtered_rows.len();
                            body.rows(ROW_HEIGHT, filtered_count, |mut row| {
                                let display_idx = row.index();
                                
                                // 判断是显示已有数据还是新增行
                                if display_idx < filtered_rows_len {
                                    // 显示已有数据行
                                    if let Some((original_idx, row_data)) =
                                        filtered_rows.get(display_idx)
                                    {
                                        let is_cursor_row = state.cursor.0 == *original_idx;
                                        let is_row_deleted =
                                            state.rows_to_delete.contains(original_idx);

                                        row.set_selected(is_cursor_row || is_row_deleted);

                                        // 行号列
                                        row.col(|ui| {
                                            render::render_row_number(
                                                ui,
                                                *original_idx,
                                                is_cursor_row,
                                                is_row_deleted,
                                                state,
                                            );
                                        });

                                        // 数据列
                                        for (col_idx, cell) in row_data.iter().enumerate() {
                                            row.col(|ui| {
                                                render::render_editable_cell(
                                                    ui,
                                                    cell,
                                                    *original_idx,
                                                    col_idx,
                                                    is_cursor_row,
                                                    is_row_deleted,
                                                    state,
                                                );
                                            });
                                        }
                                    }
                                } else {
                                    // 显示新增行（pending rows）
                                    let new_row_idx = display_idx - filtered_rows_len;
                                    // 新增行的虚拟原始索引 = 结果行数 + 新增行索引
                                    let virtual_idx = result.rows.len() + new_row_idx;
                                    let is_cursor_row = state.cursor.0 == virtual_idx;

                                    // 新增行使用特殊高亮
                                    row.set_selected(is_cursor_row);

                                    // 行号列 - 显示 "+" 标记表示新增行
                                    row.col(|ui| {
                                        let text = RichText::new(format!("{}+", virtual_idx + 1))
                                            .monospace()
                                            .color(Color32::from_rgb(100, 200, 100));
                                        ui.label(text);
                                    });

                                    // 数据列 - 显示新增行的内容
                                    // 先克隆数据避免借用冲突
                                    let new_row_data: Vec<String> = state
                                        .new_rows
                                        .get(new_row_idx)
                                        .cloned()
                                        .unwrap_or_default();
                                    for (col_idx, cell) in new_row_data.iter().enumerate() {
                                        row.col(|ui| {
                                            render::render_new_row_cell(
                                                ui,
                                                cell,
                                                virtual_idx,
                                                col_idx,
                                                is_cursor_row,
                                                state,
                                            );
                                        });
                                    }
                                }
                            });
                        });
                });
            // 更新保存的水平滚动偏移量
            state.h_scroll_offset = scroll_output.state.offset.x;
        });

        // 处理列筛选点击
        for col_name in columns_to_filter {
            if !state.filters.iter().any(|f| f.column == col_name) {
                state.filters.push(ColumnFilter::new(col_name));
            }
        }

        // 点击表格区域聚焦
        if table_response.response.clicked() {
            state.focused = true;
            actions.request_focus = true;
        }

        (actions, (filtered_count, total_count))
    }

    /// 显示模式状态栏和操作按钮
    fn show_mode_bar(
        ui: &mut egui::Ui,
        state: &mut DataGridState,
        result: &QueryResult,
        table_name: Option<&str>,
        actions: &mut DataGridActions,
    ) {
        ui.horizontal(|ui| {
            // 模式指示器
            let mode_text = format!("-- {} --", state.mode.display_name());
            ui.label(RichText::new(mode_text).strong().color(state.mode.color()));

            ui.separator();

            // 光标位置
            let pos_text = format!("{}:{}", state.cursor.0 + 1, state.cursor.1 + 1);
            ui.label(RichText::new(pos_text).monospace().color(GRAY));

            // 选择范围
            if let Some(((min_r, min_c), (max_r, max_c))) = state.get_selection() {
                let sel_text = format!("选择: {}x{}", max_r - min_r + 1, max_c - min_c + 1);
                ui.separator();
                ui.label(RichText::new(sel_text).small().color(COLOR_VISUAL_SELECT));
            }

            // 命令缓冲
            if !state.command_buffer.is_empty() {
                ui.separator();
                ui.label(
                    RichText::new(&state.command_buffer)
                        .monospace()
                        .color(Color32::YELLOW),
                );
            }

            // 计数
            if let Some(count) = state.count {
                ui.separator();
                ui.label(
                    RichText::new(format!("{}", count))
                        .monospace()
                        .color(Color32::YELLOW),
                );
            }

            // 截断警告
            if result.truncated {
                ui.separator();
                let truncated_msg = if let Some(original) = result.original_row_count {
                    format!("! 已截断 (原{}行)", original)
                } else {
                    "! 已截断".to_string()
                };
                ui.label(
                    RichText::new(truncated_msg)
                        .small()
                        .color(Color32::from_rgb(255, 165, 0)), // 橙色警告
                ).on_hover_text("结果集过大已被截断。建议在 SQL 中添加 LIMIT 子句限制返回行数。");
            }

            ui.separator();

            // 筛选 - 可点击文字，打开左侧栏筛选面板
            let filter_text = if state.filters.is_empty() {
                "+ 筛选".to_string()
            } else {
                format!("筛选({})", state.filters.iter().filter(|f| f.enabled).count())
            };
            if ui
                .add(egui::Label::new(RichText::new(filter_text).size(12.0).color(Color32::from_rgb(130, 160, 200))).sense(egui::Sense::click()))
                .on_hover_text("打开筛选面板 [/]")
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .clicked()
            {
                // 设置标记，让 app 层处理打开侧边栏筛选面板
                actions.open_filter_panel = true;
            }

            // 操作按钮
            if table_name.is_some() {
                ui.add_space(16.0);
                
                // 新增行 - 可点击文字
                if ui
                    .add(egui::Label::new(RichText::new("+ 行").size(12.0).color(Color32::from_rgb(130, 160, 200))).sense(egui::Sense::click()))
                    .on_hover_text("添加新行 [o]")
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .clicked()
                {
                    let new_row = vec!["".to_string(); result.columns.len()];
                    state.new_rows.push(new_row);
                    // 移动光标到新增行
                    let new_row_idx = result.rows.len() + state.new_rows.len() - 1;
                    state.cursor = (new_row_idx, 0);
                    state.scroll_to_row = Some(new_row_idx);
                    state.focused = true;
                    actions.message = Some("已添加新行".to_string());
                }

                let has_changes = state.has_changes();
                let save_color = if has_changes { Color32::LIGHT_GRAY } else { Color32::from_gray(60) };
                if ui
                    .add_enabled(
                        has_changes,
                        egui::Button::new(RichText::new("💾").size(13.0).color(save_color))
                            .frame(false)
                            .min_size(Vec2::new(24.0, 24.0)),
                    )
                    .on_hover_text("保存所有修改到数据库 [w / Ctrl+S]")
                    .clicked()
                    && let Some(table) = table_name {
                        actions::generate_save_sql(result, state, table, actions);
                    }

                let discard_color = if has_changes { Color32::LIGHT_GRAY } else { Color32::from_gray(60) };
                if ui
                    .add_enabled(
                        has_changes,
                        egui::Button::new(RichText::new("↩").size(13.0).color(discard_color))
                            .frame(false)
                            .min_size(Vec2::new(24.0, 24.0)),
                    )
                    .on_hover_text("放弃所有未保存的修改 [q]")
                    .clicked()
                {
                    state.clear_edits();
                    actions.message = Some("已放弃所有修改".to_string());
                }

                if has_changes {
                    ui.separator();
                    // 使用图标+文字双重指示，对色盲友好
                    let mut stats = Vec::new();
                    if !state.modified_cells.is_empty() {
                        stats.push(format!("✎ {}处修改", state.modified_cells.len()));
                    }
                    if !state.rows_to_delete.is_empty() {
                        stats.push(format!("− {}行删除", state.rows_to_delete.len()));
                    }
                    if !state.new_rows.is_empty() {
                        stats.push(format!("+ {}行新增", state.new_rows.len()));
                    }
                    ui.label(
                        RichText::new(stats.join(", "))
                            .small()
                            .color(COLOR_CELL_MODIFIED),
                    );
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let help = match state.mode {
                    GridMode::Normal => {
                        "hjkl:移动 i:编辑 v:选择 d:删除 y:复制 p:粘贴 gg:顶部 G:底部"
                    }
                    GridMode::Insert => "Esc:退出 Enter:确认",
                    GridMode::Select => "hjkl:扩展 d:删除 y:复制 Esc:取消",
                };
                ui.label(RichText::new(help).small().color(GRAY));
            });
        });
    }

    /// 计算字符串的显示宽度（考虑中英文差异）
    fn calculate_text_width(text: &str) -> f32 {
        let mut width = 0.0;
        for c in text.chars() {
            // 中日韩字符使用更宽的宽度
            if c > '\u{2E7F}' {
                width += constants::grid::CJK_CHAR_WIDTH;
            } else {
                width += CHAR_WIDTH;
            }
        }
        width
    }

    /// 计算数据的哈希值（用于缓存验证）
    fn calculate_data_hash(
        result: &QueryResult,
        filtered_rows: &[(usize, &Vec<String>)],
        sample_count: usize,
    ) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // 哈希列名
        for col in &result.columns {
            col.hash(&mut hasher);
        }

        // 哈希采样数据的前几个字符（避免大字符串影响性能）
        for (idx, row_data) in filtered_rows.iter().take(sample_count) {
            idx.hash(&mut hasher);
            for cell in row_data.iter() {
                // 只哈希前 50 个字符
                let sample: String = cell.chars().take(50).collect();
                sample.hash(&mut hasher);
            }
        }

        hasher.finish()
    }

    /// 获取列宽（优先使用缓存）
    fn get_column_widths(
        result: &QueryResult,
        filtered_rows: &[(usize, &Vec<String>)],
        cache: &mut state::ColumnWidthCache,
    ) -> Vec<f32> {
        let column_count = result.columns.len();
        let sample_count = filtered_rows.len().min(100);
        let data_hash = Self::calculate_data_hash(result, filtered_rows, sample_count);

        // 检查缓存是否有效
        if cache.is_valid(column_count, sample_count, data_hash) {
            return cache.widths.clone();
        }

        // 计算新的列宽
        let widths = Self::calculate_column_widths_internal(result, filtered_rows, sample_count);

        // 更新缓存
        cache.update(widths.clone(), column_count, sample_count, data_hash);

        widths
    }

    /// 计算每列的最佳宽度（内部实现）
    fn calculate_column_widths_internal(
        result: &QueryResult,
        filtered_rows: &[(usize, &Vec<String>)],
        sample_count: usize,
    ) -> Vec<f32> {
        let mut col_widths = Vec::with_capacity(result.columns.len());

        for (col_idx, col_name) in result.columns.iter().enumerate() {
            // 从列名开始计算最大宽度
            let mut max_width = Self::calculate_text_width(col_name);

            // 采样前 N 行来计算内容最大宽度（避免大数据集性能问题）
            for (_, row_data) in filtered_rows.iter().take(sample_count) {
                if let Some(cell) = row_data.get(col_idx) {
                    let cell_width = Self::calculate_text_width(cell);
                    if cell_width > max_width {
                        max_width = cell_width;
                    }
                }
            }

            // 加上内边距（左右内边距 + 筛选按钮空间）
            let padding = 24.0;
            let width = (max_width + padding).clamp(MIN_COL_WIDTH, MAX_COL_WIDTH);

            col_widths.push(width);
        }

        col_widths
    }

    /// 显示空状态
    fn show_empty(ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(RichText::new("暂无数据").color(GRAY));
        });
    }

    /// 显示跳转对话框 (Ctrl+G)
    fn show_goto_dialog(ctx: &egui::Context, state: &mut DataGridState, max_row: usize) {
        if !state.show_goto_dialog {
            return;
        }

        egui::Window::new("跳转到行")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("行号:");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut state.goto_input)
                            .desired_width(100.0)
                            .hint_text(format!("1-{}", max_row)),
                    );

                    // 自动聚焦
                    if response.gained_focus() || state.goto_input.is_empty() {
                        response.request_focus();
                    }

                    // 回车确认
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        if let Ok(line) = state.goto_input.trim().parse::<usize>()
                            && line >= 1 && line <= max_row {
                                state.cursor.0 = line - 1;
                                state.scroll_to_row = Some(state.cursor.0);
                            }
                        state.show_goto_dialog = false;
                        state.goto_input.clear();
                    }
                });

                ui.horizontal(|ui| {
                    if ui.add(
                        egui::Button::new(RichText::new("↵ 跳转").size(13.0).color(Color32::LIGHT_GRAY))
                            .frame(false)
                            .min_size(Vec2::new(0.0, 24.0)),
                    ).on_hover_text("跳转到指定行 [Enter]").clicked() {
                        if let Ok(line) = state.goto_input.trim().parse::<usize>()
                            && line >= 1 && line <= max_row {
                                state.cursor.0 = line - 1;
                                state.scroll_to_row = Some(state.cursor.0);
                            }
                        state.show_goto_dialog = false;
                        state.goto_input.clear();
                    }
                    if ui.add(
                        egui::Button::new(RichText::new("✕ 取消").size(13.0).color(Color32::LIGHT_GRAY))
                            .frame(false)
                            .min_size(Vec2::new(0.0, 24.0)),
                    ).on_hover_text("取消 [Esc]").clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        state.show_goto_dialog = false;
                        state.goto_input.clear();
                    }
                });
            });
    }

    /// 显示保存确认对话框（危险操作确认）
    fn show_save_confirm_dialog(
        ctx: &egui::Context,
        state: &mut DataGridState,
        actions: &mut DataGridActions,
    ) {
        if !state.show_save_confirm {
            return;
        }

        let delete_count = state.rows_to_delete.len();
        let total_count = state.pending_sql.len();

        egui::Window::new("确认保存")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new("此操作包含危险操作，请确认：").strong());
                    ui.add_space(8.0);

                    // 显示操作统计
                    ui.horizontal(|ui| {
                        ui.label(format!("将删除 {} 行数据", delete_count));
                    });
                    ui.horizontal(|ui| {
                        ui.label(format!("共 {} 条 SQL 语句", total_count));
                    });

                    ui.add_space(8.0);

                    // 显示预览的 SQL（最多显示5条）
                    ui.collapsing("查看 SQL 预览", |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(150.0)
                            .show(ui, |ui| {
                                for (i, sql) in state.pending_sql.iter().enumerate() {
                                    let is_delete = sql.starts_with("DELETE");
                                    let color = if is_delete {
                                        Color32::from_rgb(200, 80, 80)
                                    } else {
                                        GRAY
                                    };
                                    ui.label(
                                        RichText::new(format!("{}. {}", i + 1, sql))
                                            .small()
                                            .color(color),
                                    );
                                }
                            });
                    });

                    ui.add_space(12.0);

                    ui.horizontal(|ui| {
                        // 确认按钮（红色警告文字）
                        if ui
                            .add(
                                egui::Button::new(RichText::new("⚠ 确认执行").size(13.0).color(Color32::from_rgb(255, 100, 100)))
                                    .frame(false)
                                    .min_size(Vec2::new(0.0, 24.0)),
                            )
                            .on_hover_text("确认执行 SQL 操作 [Enter]")
                            .clicked()
                        {
                            actions::confirm_pending_sql(state, actions);
                        }

                        ui.add_space(16.0);

                        if ui.add(
                            egui::Button::new(RichText::new("✕ 取消").size(13.0).color(Color32::LIGHT_GRAY))
                                .frame(false)
                                .min_size(Vec2::new(0.0, 24.0)),
                        ).on_hover_text("取消 [Esc]").clicked()
                            || ui.input(|i| i.key_pressed(egui::Key::Escape))
                        {
                            actions::cancel_pending_sql(state);
                        }
                    });
                });
            });
    }
}
