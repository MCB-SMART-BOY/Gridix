//! 表格编辑状态

use super::filter::{ColumnFilter, FilterCache};
use super::mode::GridMode;
use std::collections::HashMap;

/// 表格编辑状态
#[derive(Default)]
pub struct DataGridState {
    /// 当前模式
    pub mode: GridMode,
    /// 当前光标位置 (row, col)
    pub cursor: (usize, usize),
    /// 选择起始位置（Select 模式）
    pub select_anchor: Option<(usize, usize)>,
    /// 当前编辑的单元格 (row, col)
    pub editing_cell: Option<(usize, usize)>,
    /// 编辑中的文本
    pub edit_text: String,
    /// 原始值（用于比较是否修改）
    pub original_value: String,
    /// 已修改的单元格 (row, col) -> 新值
    pub modified_cells: HashMap<(usize, usize), String>,
    /// 待删除的行索引列表
    pub rows_to_delete: Vec<usize>,
    /// 新增的行数据
    pub new_rows: Vec<Vec<String>>,
    /// 筛选条件列表
    pub filters: Vec<ColumnFilter>,
    /// 剪贴板内容
    pub clipboard: Option<String>,
    /// 命令输入缓冲（用于组合键如 gg）
    pub command_buffer: String,
    /// 是否聚焦表格
    pub focused: bool,
    /// 计数前缀（如 5j 向下移动5行）
    pub count: Option<usize>,
    /// 需要滚动到的行
    pub scroll_to_row: Option<usize>,
    /// 需要滚动到的列
    pub scroll_to_col: Option<usize>,
    /// 当前水平滚动偏移量
    pub h_scroll_offset: f32,
    /// 上一次光标所在列
    pub last_cursor_col: usize,
    /// 显示跳转对话框
    pub show_goto_dialog: bool,
    /// 跳转输入
    pub goto_input: String,
    /// 待保存标记 (Ctrl+S 触发)
    pub pending_save: bool,
    /// 显示保存确认对话框
    pub show_save_confirm: bool,
    /// 待确认的 SQL 语句
    pub pending_sql: Vec<String>,
    /// 显示快速筛选输入框
    pub show_quick_filter: bool,
    /// 快速筛选输入内容
    pub quick_filter_input: String,
    /// 筛选结果缓存
    pub filter_cache: FilterCache,
    /// 主键列索引（None 表示未知，编辑功能将被禁用）
    pub primary_key_column: Option<usize>,
    /// 正则表达式错误信息（用于显示给用户）
    #[allow(dead_code)] // 预留字段供将来使用
    pub regex_error: Option<String>,
    /// 待处理的新增行编辑 (虚拟行索引, 列索引, 新值)
    pub pending_new_row_edit: Option<(usize, usize, String)>,
}

impl DataGridState {
    pub fn new() -> Self {
        Self {
            focused: true,
            ..Default::default()
        }
    }

    pub fn clear_edits(&mut self) {
        self.editing_cell = None;
        self.edit_text.clear();
        self.original_value.clear();
        self.modified_cells.clear();
        self.rows_to_delete.clear();
        self.new_rows.clear();
    }

    pub fn has_changes(&self) -> bool {
        !self.modified_cells.is_empty()
            || !self.rows_to_delete.is_empty()
            || !self.new_rows.is_empty()
    }

    /// 获取选择范围
    pub fn get_selection(&self) -> Option<((usize, usize), (usize, usize))> {
        self.select_anchor.map(|anchor| {
            let min_row = anchor.0.min(self.cursor.0);
            let max_row = anchor.0.max(self.cursor.0);
            let min_col = anchor.1.min(self.cursor.1);
            let max_col = anchor.1.max(self.cursor.1);
            ((min_row, min_col), (max_row, max_col))
        })
    }

    /// 检查单元格是否在选择范围内
    pub fn is_in_selection(&self, row: usize, col: usize) -> bool {
        if let Some(((min_r, min_c), (max_r, max_c))) = self.get_selection() {
            row >= min_r && row <= max_r && col >= min_c && col <= max_c
        } else {
            false
        }
    }

    /// 移动光标
    pub fn move_cursor(
        &mut self,
        delta_row: isize,
        delta_col: isize,
        max_row: usize,
        max_col: usize,
    ) {
        let count = self.count.unwrap_or(1) as isize;
        let new_row = (self.cursor.0 as isize + delta_row * count)
            .max(0)
            .min(max_row as isize - 1) as usize;
        let new_col = (self.cursor.1 as isize + delta_col * count)
            .max(0)
            .min(max_col as isize - 1) as usize;
        self.cursor = (new_row, new_col);
        self.count = None;
        self.scroll_to_row = Some(new_row);
        self.scroll_to_col = Some(new_col);
    }

    /// 跳转到行首
    pub fn goto_line_start(&mut self) {
        self.cursor.1 = 0;
        self.count = None;
    }

    /// 跳转到行尾
    pub fn goto_line_end(&mut self, max_col: usize) {
        self.cursor.1 = max_col.saturating_sub(1);
        self.count = None;
    }

    /// 跳转到文件首
    pub fn goto_file_start(&mut self) {
        self.cursor = (0, 0);
        self.count = None;
        self.command_buffer.clear();
        self.scroll_to_row = Some(0);
    }

    /// 跳转到文件尾
    pub fn goto_file_end(&mut self, max_row: usize) {
        self.cursor.0 = max_row.saturating_sub(1);
        self.count = None;
        self.scroll_to_row = Some(self.cursor.0);
    }
}
