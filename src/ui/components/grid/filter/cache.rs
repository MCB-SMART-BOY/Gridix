//! 筛选缓存
//!
//! 提供筛选结果的缓存机制，避免重复计算。
//! 对于大数据集（超过 PARALLEL_FILTER_THRESHOLD 行），使用并行处理。

use super::condition::ColumnFilter;
use super::logic::FilterLogic;
use super::operators::check_filter_match_typed;
use crate::domain::result::ResultSet;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 筛选缓存
#[derive(Default, Clone)]
pub struct FilterCache {
    /// 缓存是否有效
    pub valid: bool,
    /// 上次搜索文本
    pub last_search_text: String,
    /// 上次搜索列
    pub last_search_column: Option<String>,
    /// 上次筛选条件的哈希值
    pub last_filter_hash: u64,
    /// 上次行数
    pub last_row_count: usize,
    /// 缓存的筛选后行索引
    pub filtered_indices: Vec<usize>,
}

#[allow(dead_code)] // 公开 API，供外部使用
impl FilterCache {
    /// 创建新的缓存
    pub fn new() -> Self {
        Self::default()
    }

    /// 使缓存失效
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// 获取缓存的过滤后行数（如果缓存有效）
    pub fn get_filtered_count(&self) -> Option<usize> {
        if self.valid {
            Some(self.filtered_indices.len())
        } else {
            None
        }
    }

    /// 检查缓存是否有效
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

/// 计算筛选条件的哈希值
fn compute_filter_hash(filters: &[ColumnFilter]) -> u64 {
    let mut hasher = DefaultHasher::new();
    for f in filters {
        f.column.hash(&mut hasher);
        f.value.hash(&mut hasher);
        f.value2.hash(&mut hasher);
        f.enabled.hash(&mut hasher);
        f.case_sensitive.hash(&mut hasher);
        std::mem::discriminant(&f.operator).hash(&mut hasher);
        std::mem::discriminant(&f.logic).hash(&mut hasher);
    }
    hasher.finish()
}

/// 从类型化结果集中筛选行并缓存行索引。
/// 从类型化结果集中筛选行并缓存行索引。
pub(crate) fn filter_result_set_cached(
    result: &ResultSet,
    search_text: &str,
    search_column: &Option<String>,
    filters: &[ColumnFilter],
    cache: &mut FilterCache,
) -> Vec<usize> {
    let filter_hash = compute_filter_hash(filters);
    let is_cache_valid = cache.valid
        && cache.last_search_text == search_text
        && cache.last_search_column == *search_column
        && cache.last_filter_hash == filter_hash
        && cache.last_row_count == result.row_count;

    if is_cache_valid {
        return cache.filtered_indices.clone();
    }

    let filtered_indices = filter_result_set_rows(result, search_text, search_column, filters);
    cache.filtered_indices.clone_from(&filtered_indices);
    cache.last_search_text = search_text.to_string();
    cache.last_search_column = search_column.clone();
    cache.last_filter_hash = filter_hash;
    cache.last_row_count = result.row_count;
    cache.valid = true;
    filtered_indices
}

fn filter_result_set_rows(
    result: &ResultSet,
    search_text: &str,
    search_column: &Option<String>,
    filters: &[ColumnFilter],
) -> Vec<usize> {
    let search_lower = search_text.to_lowercase();
    let active_filters: Vec<&ColumnFilter> =
        filters.iter().filter(|filter| filter.enabled).collect();
    let search_col_idx = search_column.as_ref().and_then(|name| {
        result
            .columns
            .iter()
            .position(|column| column.name == *name)
    });
    let filter_col_indices: Vec<Option<usize>> = active_filters
        .iter()
        .map(|filter| {
            result
                .columns
                .iter()
                .position(|column| column.name == filter.column)
        })
        .collect();

    (0..result.row_count)
        .filter(|row_idx| {
            row_matches_result_set(
                result,
                *row_idx,
                search_text,
                &search_lower,
                search_col_idx,
                &active_filters,
                &filter_col_indices,
            )
        })
        .collect()
}

fn row_matches_result_set(
    result: &ResultSet,
    row_idx: usize,
    search_text: &str,
    search_lower: &str,
    search_col_idx: Option<usize>,
    active_filters: &[&ColumnFilter],
    filter_col_indices: &[Option<usize>],
) -> bool {
    if !search_text.is_empty() {
        let is_search_match = match search_col_idx {
            Some(column_idx) => result
                .cell(row_idx, column_idx)
                .display()
                .to_lowercase()
                .contains(search_lower),
            None => result
                .row(row_idx)
                .iter()
                .any(|cell| cell.display().to_lowercase().contains(search_lower)),
        };
        if !is_search_match {
            return false;
        }
    }

    let mut is_match = true;
    let mut logic = FilterLogic::And;
    for (index, filter) in active_filters.iter().enumerate() {
        let is_filter_match = filter_col_indices
            .get(index)
            .and_then(|column_idx| *column_idx)
            .is_some_and(|column_idx| {
                check_filter_match_typed(
                    result.cell(row_idx, column_idx),
                    &result.columns[column_idx].type_info,
                    &filter.operator,
                    &filter.value,
                    &filter.value2,
                    filter.case_sensitive,
                )
            });
        if index == 0 {
            is_match = is_filter_match;
        } else if logic == FilterLogic::And {
            is_match = is_match && is_filter_match;
        } else {
            is_match = is_match || is_filter_match;
        }
        logic = filter.logic;
    }
    is_match
}
