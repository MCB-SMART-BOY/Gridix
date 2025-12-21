//! 筛选缓存
//!
//! 提供筛选结果的缓存机制，避免重复计算。
//! 对于大数据集（超过 PARALLEL_FILTER_THRESHOLD 行），使用并行处理。

use super::condition::ColumnFilter;
use super::logic::FilterLogic;
use super::operators::check_filter_match;
use crate::core::constants;
use crate::database::QueryResult;
use rayon::prelude::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 筛选缓存
#[derive(Default)]
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

/// 带缓存的过滤行数据
pub fn filter_rows_cached<'a>(
    result: &'a QueryResult,
    search_text: &str,
    search_column: &Option<String>,
    filters: &[ColumnFilter],
    cache: &mut FilterCache,
) -> Vec<(usize, &'a Vec<String>)> {
    let filter_hash = compute_filter_hash(filters);
    
    // 检查缓存是否有效
    let cache_valid = cache.valid
        && cache.last_search_text == search_text
        && cache.last_search_column == *search_column
        && cache.last_filter_hash == filter_hash
        && cache.last_row_count == result.rows.len();
    
    if cache_valid {
        // 使用缓存的索引构建结果
        return cache
            .filtered_indices
            .iter()
            .filter_map(|&idx| result.rows.get(idx).map(|row| (idx, row)))
            .collect();
    }
    
    // 重新计算筛选结果
    let filtered = filter_rows_internal(result, search_text, search_column, filters);
    
    // 更新缓存
    cache.filtered_indices = filtered.iter().map(|(idx, _)| *idx).collect();
    cache.last_search_text = search_text.to_string();
    cache.last_search_column = search_column.clone();
    cache.last_filter_hash = filter_hash;
    cache.last_row_count = result.rows.len();
    cache.valid = true;
    
    filtered
}



/// 检查单行是否匹配筛选条件
fn row_matches_filter(
    row: &[String],
    search_text: &str,
    search_lower: &str,
    search_col_idx: Option<usize>,
    active_filters: &[&ColumnFilter],
    filter_col_indices: &[Option<usize>],
) -> bool {
    // 搜索条件
    let search_match = if search_text.is_empty() {
        true
    } else {
        match search_col_idx {
            Some(idx) => row
                .get(idx)
                .map(|cell| cell.to_lowercase().contains(search_lower))
                .unwrap_or(false),
            None => row
                .iter()
                .any(|cell| cell.to_lowercase().contains(search_lower)),
        }
    };

    if !search_match {
        return false;
    }

    // 筛选条件（支持 AND/OR 逻辑）
    if active_filters.is_empty() {
        return true;
    }

    let mut current_result = true;
    let mut pending_logic = FilterLogic::And;

    for (i, filter) in active_filters.iter().enumerate() {
        let filter_match = if let Some(Some(idx)) = filter_col_indices.get(i) {
            if let Some(cell) = row.get(*idx) {
                check_filter_match(
                    cell,
                    &filter.operator,
                    &filter.value,
                    &filter.value2,
                    filter.case_sensitive,
                )
            } else {
                false
            }
        } else {
            false
        };

        if i == 0 {
            current_result = filter_match;
        } else {
            match pending_logic {
                FilterLogic::And => current_result = current_result && filter_match,
                FilterLogic::Or => current_result = current_result || filter_match,
            }
        }

        pending_logic = filter.logic;
    }

    current_result
}

/// 过滤行数据（内部实现）
/// 对于大数据集使用并行处理
fn filter_rows_internal<'a>(
    result: &'a QueryResult,
    search_text: &str,
    search_column: &Option<String>,
    filters: &[ColumnFilter],
) -> Vec<(usize, &'a Vec<String>)> {
    let search_lower = search_text.to_lowercase();

    // 只使用启用的筛选条件
    let active_filters: Vec<&ColumnFilter> = filters.iter().filter(|f| f.enabled).collect();

    // 预先查找搜索列索引，避免在循环中重复查找
    let search_col_idx = search_column
        .as_ref()
        .and_then(|col_name| result.columns.iter().position(|c| c == col_name));

    // 预先查找筛选列索引
    let filter_col_indices: Vec<Option<usize>> = active_filters
        .iter()
        .map(|f| result.columns.iter().position(|c| c == &f.column))
        .collect();

    // 对于大数据集使用并行处理
    if result.rows.len() >= constants::database::PARALLEL_FILTER_THRESHOLD {
        result
            .rows
            .par_iter()
            .enumerate()
            .filter(|(_, row)| {
                row_matches_filter(
                    row,
                    search_text,
                    &search_lower,
                    search_col_idx,
                    &active_filters,
                    &filter_col_indices,
                )
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|(idx, row)| (idx, row))
            .collect()
    } else {
        result
            .rows
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                row_matches_filter(
                    row,
                    search_text,
                    &search_lower,
                    search_col_idx,
                    &active_filters,
                    &filter_col_indices,
                )
            })
            .collect()
    }
}

