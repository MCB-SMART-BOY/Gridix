//! UI 组件测试
//!
//! 测试数据表格、筛选、Tab 管理等 UI 组件

use gridix::database::QueryResult;
use gridix::ui::{
    ColumnFilter, FilterCache, FilterLogic, FilterOperator, QueryTab, QueryTabManager,
};

// ============================================================================
// 筛选操作符测试
// ============================================================================

mod filter_operators {
    use super::*;
    use gridix::ui::components::check_filter_match;

    #[test]
    fn test_contains_operator() {
        assert!(check_filter_match(
            "hello world",
            &FilterOperator::Contains,
            "world",
            "",
            false
        ));
        assert!(!check_filter_match(
            "hello world",
            &FilterOperator::Contains,
            "foo",
            "",
            false
        ));
    }

    #[test]
    fn test_case_sensitivity() {
        assert!(check_filter_match(
            "Hello",
            &FilterOperator::Equals,
            "hello",
            "",
            false
        ));
        assert!(!check_filter_match(
            "Hello",
            &FilterOperator::Equals,
            "hello",
            "",
            true
        ));
    }

    #[test]
    fn test_comparison_operators() {
        assert!(check_filter_match(
            "10",
            &FilterOperator::GreaterThan,
            "5",
            "",
            false
        ));
        assert!(!check_filter_match(
            "5",
            &FilterOperator::GreaterThan,
            "10",
            "",
            false
        ));
    }

    #[test]
    fn test_between() {
        assert!(check_filter_match(
            "5",
            &FilterOperator::Between,
            "1",
            "10",
            false
        ));
        assert!(!check_filter_match(
            "15",
            &FilterOperator::Between,
            "1",
            "10",
            false
        ));
    }

    #[test]
    fn test_in_operator() {
        assert!(check_filter_match(
            "apple",
            &FilterOperator::In,
            "apple, banana, orange",
            "",
            false
        ));
        assert!(!check_filter_match(
            "grape",
            &FilterOperator::In,
            "apple, banana, orange",
            "",
            false
        ));
    }
}

// ============================================================================
// 筛选逻辑测试
// ============================================================================

mod filter_logic {
    use super::*;

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
}

// ============================================================================
// 筛选条件测试
// ============================================================================

mod filter_condition {
    use super::*;

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
    fn test_is_valid() {
        // 空列名无效
        let filter = ColumnFilter::default();
        assert!(!filter.is_valid());

        // 有列名但需要值时无值也无效
        let filter =
            ColumnFilter::new("name".to_string()).with_operator(FilterOperator::Contains);
        assert!(!filter.is_valid());

        // 完整的筛选条件有效
        let filter = ColumnFilter::new("name".to_string())
            .with_operator(FilterOperator::Contains)
            .with_value("test".to_string());
        assert!(filter.is_valid());

        // 不需要值的操作符
        let filter =
            ColumnFilter::new("deleted".to_string()).with_operator(FilterOperator::IsNull);
        assert!(filter.is_valid());
    }
}

// ============================================================================
// 筛选缓存测试
// ============================================================================

mod filter_cache {
    use super::*;
    use gridix::ui::components::filter_rows_cached;

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
        assert_eq!(filtered.len(), 2); // Alice (30) and Charlie (35)
    }

    #[test]
    fn test_cache_validity() {
        let result = sample_result();
        let mut cache = FilterCache::new();

        // 第一次调用
        let _ = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
        assert!(cache.is_valid());

        // 第二次调用（应该使用缓存）
        let filtered = filter_rows_cached(&result, "Alice", &None, &[], &mut cache);
        assert_eq!(filtered.len(), 1);

        // 改变搜索条件（缓存失效）
        let filtered = filter_rows_cached(&result, "Bob", &None, &[], &mut cache);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].1[1], "Bob");
    }
}

// ============================================================================
// 查询 Tab 管理测试
// ============================================================================

mod query_tabs {
    use super::*;

    #[test]
    fn test_new_tab() {
        let mut manager = QueryTabManager::new();
        assert_eq!(manager.len(), 1);

        manager.new_tab();
        assert_eq!(manager.len(), 2);
        assert_eq!(manager.active_index(), 1);
    }

    #[test]
    fn test_close_tab() {
        let mut manager = QueryTabManager::new();
        manager.new_tab();
        manager.new_tab();
        assert_eq!(manager.len(), 3);

        manager.close_tab(1);
        assert_eq!(manager.len(), 2);
    }

    #[test]
    fn test_cannot_close_last_tab() {
        let mut manager = QueryTabManager::new();
        assert_eq!(manager.len(), 1);

        manager.close_tab(0);
        assert_eq!(manager.len(), 1); // 仍然保留一个 Tab
    }

    #[test]
    fn test_extract_title() {
        let sql = "SELECT * FROM users WHERE id = 1";
        let tab = QueryTab::from_sql(sql);
        assert_eq!(tab.title(), "查询 users");
    }

    #[test]
    fn test_tab_for_table() {
        let mut manager = QueryTabManager::new();

        // 第一次为表创建 Tab
        let idx1 = manager.new_tab_for_table("users", "SELECT * FROM users");

        // 第二次应该复用同一个 Tab
        let idx2 = manager.new_tab_for_table("users", "SELECT * FROM users WHERE id = 1");

        assert_eq!(idx1, idx2);
    }
}
