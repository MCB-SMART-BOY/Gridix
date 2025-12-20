//! UI 组件测试

use gridix::ui::components::{
    QueryTabManager, QueryTab,
    FilterOperator, FilterLogic, ColumnFilter,
    parse_quick_filter, check_filter_match,
    FilterCache, filter_rows_cached,
    escape_identifier, quote_identifier, escape_value,
};
use gridix::database::QueryResult;

// ============================================================================
// Query Tabs 测试
// ============================================================================

#[test]
fn test_new_tab() {
    let mut manager = QueryTabManager::new();
    assert_eq!(manager.tabs.len(), 1);
    
    manager.new_tab();
    assert_eq!(manager.tabs.len(), 2);
    assert_eq!(manager.active_index, 1);
}

#[test]
fn test_close_tab() {
    let mut manager = QueryTabManager::new();
    manager.new_tab();
    manager.new_tab();
    assert_eq!(manager.tabs.len(), 3);
    
    manager.close_tab(1);
    assert_eq!(manager.tabs.len(), 2);
}

#[test]
fn test_cannot_close_last_tab() {
    let mut manager = QueryTabManager::new();
    assert_eq!(manager.tabs.len(), 1);
    
    manager.close_tab(0);
    assert_eq!(manager.tabs.len(), 1);
}

#[test]
fn test_extract_title() {
    let sql = "SELECT * FROM users WHERE id = 1";
    let tab = QueryTab::from_sql(sql);
    assert_eq!(tab.title, "查询 users");
}

#[test]
fn test_tab_for_table() {
    let mut manager = QueryTabManager::new();
    
    let idx1 = manager.new_tab_for_table("users", "SELECT * FROM users");
    let idx2 = manager.new_tab_for_table("users", "SELECT * FROM users WHERE id = 1");
    
    assert_eq!(idx1, idx2);
}

// ============================================================================
// Filter Operators 测试
// ============================================================================

#[test]
fn test_contains_operator() {
    assert!(check_filter_match("hello world", &FilterOperator::Contains, "world", "", false));
    assert!(!check_filter_match("hello world", &FilterOperator::Contains, "foo", "", false));
}

#[test]
fn test_case_sensitivity() {
    assert!(check_filter_match("Hello", &FilterOperator::Equals, "hello", "", false));
    assert!(!check_filter_match("Hello", &FilterOperator::Equals, "hello", "", true));
}

#[test]
fn test_comparison_operators() {
    assert!(check_filter_match("10", &FilterOperator::GreaterThan, "5", "", false));
    assert!(!check_filter_match("5", &FilterOperator::GreaterThan, "10", "", false));
}

#[test]
fn test_between() {
    assert!(check_filter_match("5", &FilterOperator::Between, "1", "10", false));
    assert!(!check_filter_match("15", &FilterOperator::Between, "1", "10", false));
}

#[test]
fn test_in_operator() {
    assert!(check_filter_match("apple", &FilterOperator::In, "apple, banana, orange", "", false));
    assert!(!check_filter_match("grape", &FilterOperator::In, "apple, banana, orange", "", false));
}

// ============================================================================
// Filter Logic 测试
// ============================================================================

#[test]
fn test_default_is_and() {
    let logic = FilterLogic::default();
    assert_eq!(logic, FilterLogic::And);
}

#[test]
fn test_toggle() {
    let mut logic = FilterLogic::And;
    logic.toggle();
    assert_eq!(logic, FilterLogic::Or);
    logic.toggle();
    assert_eq!(logic, FilterLogic::And);
}

#[test]
fn test_display_name() {
    assert_eq!(FilterLogic::And.display_name(), "AND");
    assert_eq!(FilterLogic::Or.display_name(), "OR");
}

// ============================================================================
// Filter Condition 测试
// ============================================================================

#[test]
fn test_default_filter() {
    let filter = ColumnFilter::default();
    assert!(filter.column.is_empty());
    assert!(filter.enabled);
    assert!(!filter.case_sensitive);
    assert_eq!(filter.logic, FilterLogic::And);
}

#[test]
fn test_new_filter() {
    let filter = ColumnFilter::new("name".to_string());
    assert_eq!(filter.column, "name");
    assert!(filter.enabled);
}

#[test]
fn test_builder_pattern() {
    let filter = ColumnFilter::new("age".to_string())
        .with_operator(FilterOperator::GreaterThan)
        .with_value("18".to_string())
        .with_case_sensitive(true);
    
    assert_eq!(filter.column, "age");
    assert_eq!(filter.operator, FilterOperator::GreaterThan);
    assert_eq!(filter.value, "18");
    assert!(filter.case_sensitive);
}

#[test]
fn test_filter_is_valid() {
    let filter = ColumnFilter::default();
    assert!(!filter.is_valid());

    let filter = ColumnFilter::new("name".to_string())
        .with_operator(FilterOperator::Contains);
    assert!(!filter.is_valid());

    let filter = ColumnFilter::new("name".to_string())
        .with_operator(FilterOperator::Contains)
        .with_value("test".to_string());
    assert!(filter.is_valid());

    let filter = ColumnFilter::new("deleted".to_string())
        .with_operator(FilterOperator::IsNull);
    assert!(filter.is_valid());
}

// ============================================================================
// Quick Filter 测试
// ============================================================================

#[test]
fn test_parse_simple_filter() {
    let columns = vec!["name".to_string(), "age".to_string()];
    
    let result = parse_quick_filter("name ~ john", &columns);
    assert!(result.is_ok());
    
    let filter = result.unwrap();
    assert_eq!(filter.column, "name");
    assert_eq!(filter.operator, FilterOperator::Contains);
    assert_eq!(filter.value, "john");
}

#[test]
fn test_parse_comparison_filter() {
    let columns = vec!["age".to_string()];
    
    let result = parse_quick_filter("age > 18", &columns);
    assert!(result.is_ok());
    
    let filter = result.unwrap();
    assert_eq!(filter.operator, FilterOperator::GreaterThan);
    assert_eq!(filter.value, "18");
}

#[test]
fn test_parse_null_filter() {
    let columns = vec!["deleted".to_string()];
    
    let result = parse_quick_filter("deleted NULL", &columns);
    assert!(result.is_ok());
    
    let filter = result.unwrap();
    assert_eq!(filter.operator, FilterOperator::IsNull);
    assert!(filter.value.is_empty());
}

#[test]
fn test_parse_between_filter() {
    let columns = vec!["age".to_string()];
    
    let result = parse_quick_filter("age [] 18 30", &columns);
    assert!(result.is_ok());
    
    let filter = result.unwrap();
    assert_eq!(filter.operator, FilterOperator::Between);
    assert_eq!(filter.value, "18");
    assert_eq!(filter.value2, "30");
}

#[test]
fn test_partial_column_match() {
    let columns = vec!["username".to_string()];
    
    let result = parse_quick_filter("user ~ test", &columns);
    assert!(result.is_ok());
    
    let filter = result.unwrap();
    assert_eq!(filter.column, "username");
}

#[test]
fn test_invalid_column() {
    let columns = vec!["name".to_string()];
    
    let result = parse_quick_filter("invalid ~ test", &columns);
    assert!(result.is_err());
}

#[test]
fn test_missing_value() {
    let columns = vec!["name".to_string()];
    
    let result = parse_quick_filter("name ~", &columns);
    assert!(result.is_err());
}

// ============================================================================
// Filter Cache 测试
// ============================================================================

fn sample_result() -> QueryResult {
    QueryResult {
        columns: vec!["id".to_string(), "name".to_string(), "age".to_string()],
        rows: vec![
            vec!["1".to_string(), "Alice".to_string(), "30".to_string()],
            vec!["2".to_string(), "Bob".to_string(), "25".to_string()],
            vec!["3".to_string(), "Charlie".to_string(), "35".to_string()],
        ],
        affected_rows: 0,
        truncated: false,
        original_row_count: None,
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
    assert_eq!(filtered.len(), 2);
}

#[test]
fn test_cache_validity() {
    let result = sample_result();
    let mut cache = FilterCache::new();
    
    let _ = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
    assert!(cache.valid);
    
    let filtered = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
    assert_eq!(filtered.len(), 1);
    
    let filtered = filter_rows_cached(&result, "Bob", &None, &[], &mut cache);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].1[1], "Bob");
}

// ============================================================================
// Grid Actions 测试
// ============================================================================

#[test]
fn test_escape_identifier_valid() {
    assert_eq!(escape_identifier("users").unwrap(), "users");
    assert_eq!(escape_identifier("user_name").unwrap(), "user_name");
    assert_eq!(escape_identifier("_private").unwrap(), "_private");
    assert_eq!(escape_identifier("Table123").unwrap(), "Table123");
    assert_eq!(escape_identifier("用户表").unwrap(), "用户表");
}

#[test]
fn test_escape_identifier_invalid() {
    assert!(escape_identifier("").is_err());
    assert!(escape_identifier("user;drop").is_err());
    assert!(escape_identifier("table'name").is_err());
    assert!(escape_identifier("table\"name").is_err());
    assert!(escape_identifier("table`name").is_err());
    assert!(escape_identifier("table-name").is_err());
    let long_name = "a".repeat(64);
    assert!(escape_identifier(&long_name).is_err());
}

#[test]
fn test_escape_identifier_sql_keywords() {
    assert!(escape_identifier("DROP").is_err());
    assert!(escape_identifier("drop").is_err());
    assert!(escape_identifier("DELETE").is_err());
    assert!(escape_identifier("UNION").is_err());
    assert!(escape_identifier("SELECT").is_err());
    assert!(escape_identifier("user_select").is_ok());
    assert!(escape_identifier("dropdown").is_ok());
}

#[test]
fn test_quote_identifier() {
    assert_eq!(quote_identifier("users", true).unwrap(), "`users`");
    assert_eq!(quote_identifier("user_name", true).unwrap(), "`user_name`");
    
    assert_eq!(quote_identifier("users", false).unwrap(), "\"users\"");
    assert_eq!(quote_identifier("user_name", false).unwrap(), "\"user_name\"");
}

#[test]
fn test_escape_value() {
    assert_eq!(escape_value("hello"), "'hello'");
    assert_eq!(escape_value("it's"), "'it''s'");
    assert_eq!(escape_value("NULL"), "NULL");
    assert_eq!(escape_value("O'Brien"), "'O''Brien'");
}
