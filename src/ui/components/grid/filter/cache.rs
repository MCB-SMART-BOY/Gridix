//! 筛选缓存
//!
//! 提供筛选结果的缓存机制，避免重复计算。

use super::condition::ColumnFilter;
use super::logic::FilterLogic;
use super::operators::check_filter_match;
use crate::database::QueryResult;
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

impl FilterCache {
    /// 创建新的缓存
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// 使缓存失效
    pub fn invalidate(&mut self) {
        self.valid = false;
    }

    /// 获取缓存的过滤后行数（如果缓存有效）
    #[allow(dead_code)]
    pub fn get_filtered_count(&self) -> Option<usize> {
        if self.valid {
            Some(self.filtered_indices.len())
        } else {
            None
        }
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

/// 快速搜索计数（仅用于状态栏显示，不使用筛选条件）
/// 
/// 返回 (匹配行数, 总行数)
pub fn count_search_matches(
    result: &QueryResult,
    search_text: &str,
    search_column: &Option<String>,
) -> (usize, usize) {
    let total = result.rows.len();
    
    if search_text.is_empty() {
        return (total, total);
    }
    
    let search_lower = search_text.to_lowercase();
    
    // 预先查找搜索列索引
    let search_col_idx = search_column
        .as_ref()
        .and_then(|col_name| result.columns.iter().position(|c| c == col_name));
    
    let matched = result.rows.iter().filter(|row| {
        match search_col_idx {
            Some(idx) => row
                .get(idx)
                .map(|cell| cell.to_lowercase().contains(&search_lower))
                .unwrap_or(false),
            None => row
                .iter()
                .any(|cell| cell.to_lowercase().contains(&search_lower)),
        }
    }).count();
    
    (matched, total)
}

/// 过滤行数据（内部实现）
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

    result
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            // 搜索条件
            let search_match = if search_text.is_empty() {
                true
            } else {
                match search_col_idx {
                    Some(idx) => row
                        .get(idx)
                        .map(|cell| cell.to_lowercase().contains(&search_lower))
                        .unwrap_or(false),
                    None => row
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&search_lower)),
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
                let col_idx = result.columns.iter().position(|c| c == &filter.column);
                let filter_match = if let Some(idx) = col_idx {
                    if let Some(cell) = row.get(idx) {
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
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::operators::FilterOperator;

    fn sample_result() -> QueryResult {
        QueryResult {
            columns: vec!["id".to_string(), "name".to_string(), "age".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string(), "30".to_string()],
                vec!["2".to_string(), "Bob".to_string(), "25".to_string()],
                vec!["3".to_string(), "Charlie".to_string(), "35".to_string()],
            ],
            affected_rows: 0,
        }
    }

    #[test]
    fn test_no_filters() {
        let result = sample_result();
        let mut cache = FilterCache::new();
        
        let filtered = filter_rows_cached(&result, "", &None, &[], &mut cache);
        assert_eq!(filtered.len(), 3);
    }

    #[test]
    fn test_search_filter() {
        let result = sample_result();
        let mut cache = FilterCache::new();
        
        let filtered = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[1], "Alice");
    }

    #[test]
    fn test_column_filter() {
        let result = sample_result();
        let mut cache = FilterCache::new();
        
        let filter = ColumnFilter::new("age".to_string())
            .with_operator(FilterOperator::GreaterThan)
            .with_value("28".to_string());
        
        let filtered = filter_rows_cached(&result, "", &None, &[filter], &mut cache);
        assert_eq!(filtered.len(), 2); // Alice (30) and Charlie (35)
    }

    #[test]
    fn test_cache_validity() {
        let result = sample_result();
        let mut cache = FilterCache::new();
        
        // 第一次调用
        let _ = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
        assert!(cache.valid);
        
        // 第二次调用（应该使用缓存）
        let filtered = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
        assert_eq!(filtered.len(), 1);
        
        // 改变搜索条件（缓存失效）
        let filtered = filter_rows_cached(&result, "Bob", &None, &[], &mut cache);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[1], "Bob");
    }
}
