//! 结果集类型 — 取代 `QueryResult` 的 `Vec<Vec<String>>` 模式
//!
//! 采用 flat row-major 存储：`cells[row * column_count + col]`，
//! 减少每行一次 Vec allocation，改善 cache locality。

use super::value::{DbTypeInfo, DbValue};
use std::sync::Arc;

/// 结果集中的列描述
#[derive(Debug, Clone)]
pub struct ResultColumn {
    pub name: String,
    pub type_info: DbTypeInfo,
}

/// 结果集的完整度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultCompleteness {
    /// 全部数据已包含
    Complete,
    /// 结果被截断（超出 MAX_RESULT_SET_ROWS）
    Truncated { displayed: usize },
}

/// 类型化查询结果集
///
/// 使用 flat row-major 布局：`cells[row * self.column_count() + col]`
#[derive(Debug, Clone)]
pub struct ResultSet {
    /// 列描述（共享引用，避免克隆开销）
    pub columns: Arc<[ResultColumn]>,

    /// flat row-major 存储
    pub cells: Vec<DbValue>,

    /// 行数
    pub row_count: usize,

    /// 完整度
    pub completeness: ResultCompleteness,
}

impl ResultSet {
    /// 列数
    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// 获取指定单元格的引用
    ///
    /// # Panics
    /// 当 row 或 col 越界时 panic（与 slice 索引行为一致）
    pub fn cell(&self, row: usize, col: usize) -> &DbValue {
        &self.cells[row * self.column_count() + col]
    }

    /// 获取指定行的切片
    pub fn row(&self, row: usize) -> &[DbValue] {
        let n = self.column_count();
        let start = row * n;
        &self.cells[start..start + n]
    }

    /// 检查指定单元格是否为 NULL
    pub fn is_null(&self, row: usize, col: usize) -> bool {
        matches!(self.cell(row, col), DbValue::Null)
    }

    /// 列名列表（便捷方法，用于向后兼容）
    pub fn column_names(&self) -> Vec<String> {
        self.columns.iter().map(|c| c.name.clone()).collect()
    }

    /// 是否为空结果集
    pub fn is_empty(&self) -> bool {
        self.row_count == 0
    }

    /// 构建一个空的 ResultSet
    pub fn empty() -> Self {
        Self {
            columns: Arc::new([]),
            cells: Vec::new(),
            row_count: 0,
            completeness: ResultCompleteness::Complete,
        }
    }
}
