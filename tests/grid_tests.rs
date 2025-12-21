//! 数据表格相关测试
//!
//! 测试 SQL 标识符转义、值转义、列宽缓存等功能

use gridix::ui::{escape_identifier, escape_value, quote_identifier, DataGridState};

// ============================================================================
// 标识符转义测试
// ============================================================================

mod identifier_escape {
    use super::*;

    #[test]
    fn test_escape_identifier_valid() {
        // escape_identifier 只验证并返回原始标识符
        assert_eq!(escape_identifier("users").unwrap(), "users");
        assert_eq!(escape_identifier("user_name").unwrap(), "user_name");
        assert_eq!(escape_identifier("_private").unwrap(), "_private");
        assert_eq!(escape_identifier("Table123").unwrap(), "Table123");
        // 支持中文表名
        assert_eq!(escape_identifier("用户表").unwrap(), "用户表");
    }

    #[test]
    fn test_escape_identifier_invalid() {
        assert!(escape_identifier("").is_err());
        // 危险字符被禁止
        assert!(escape_identifier("user;drop").is_err());
        assert!(escape_identifier("table'name").is_err());
        assert!(escape_identifier("table\"name").is_err());
        assert!(escape_identifier("table`name").is_err());
        assert!(escape_identifier("table-name").is_err()); // 连字符也禁止
                                                           // 超长标识符
        let long_name = "a".repeat(64);
        assert!(escape_identifier(&long_name).is_err());
    }

    #[test]
    fn test_escape_identifier_sql_keywords() {
        // SQL 危险保留字被禁止
        assert!(escape_identifier("DROP").is_err());
        assert!(escape_identifier("drop").is_err());
        assert!(escape_identifier("DELETE").is_err());
        assert!(escape_identifier("UNION").is_err());
        assert!(escape_identifier("SELECT").is_err());
        // 包含保留字但不完全匹配的标识符应该通过
        assert!(escape_identifier("user_select").is_ok());
        assert!(escape_identifier("dropdown").is_ok());
    }

    #[test]
    fn test_quote_identifier() {
        // MySQL 使用反引号
        assert_eq!(quote_identifier("users", true).unwrap(), "`users`");
        assert_eq!(quote_identifier("user_name", true).unwrap(), "`user_name`");

        // PostgreSQL/SQLite 使用双引号
        assert_eq!(quote_identifier("users", false).unwrap(), "\"users\"");
        assert_eq!(
            quote_identifier("user_name", false).unwrap(),
            "\"user_name\""
        );
    }

    #[test]
    fn test_escape_value() {
        assert_eq!(escape_value("hello"), "'hello'");
        assert_eq!(escape_value("it's"), "'it''s'");
        assert_eq!(escape_value("NULL"), "NULL");
        assert_eq!(escape_value("O'Brien"), "'O''Brien'");
    }
}

// ============================================================================
// DataGridState 测试
// ============================================================================

mod grid_state {
    use super::*;

    #[test]
    fn test_new_state() {
        let state = DataGridState::new();
        assert!(state.focused);
        assert_eq!(state.cursor, (0, 0));
        assert!(state.modified_cells.is_empty());
    }

    #[test]
    fn test_clear_edits() {
        let mut state = DataGridState::new();
        state.modified_cells.insert((0, 0), "new_value".to_string());
        state.rows_to_delete.push(1);
        
        assert!(state.has_changes());
        
        state.clear_edits();
        
        assert!(!state.has_changes());
        assert!(state.modified_cells.is_empty());
        assert!(state.rows_to_delete.is_empty());
    }

    #[test]
    fn test_has_changes() {
        let mut state = DataGridState::new();
        assert!(!state.has_changes());
        
        state.modified_cells.insert((0, 0), "value".to_string());
        assert!(state.has_changes());
        
        state.modified_cells.clear();
        state.rows_to_delete.push(0);
        assert!(state.has_changes());
        
        state.rows_to_delete.clear();
        state.new_rows.push(vec!["a".to_string()]);
        assert!(state.has_changes());
    }

    #[test]
    fn test_selection() {
        let mut state = DataGridState::new();
        state.cursor = (5, 3);
        state.select_anchor = Some((2, 1));
        
        let selection = state.get_selection();
        assert!(selection.is_some());
        
        let ((min_r, min_c), (max_r, max_c)) = selection.unwrap();
        assert_eq!(min_r, 2);
        assert_eq!(max_r, 5);
        assert_eq!(min_c, 1);
        assert_eq!(max_c, 3);
    }

    #[test]
    fn test_is_in_selection() {
        let mut state = DataGridState::new();
        state.cursor = (3, 3);
        state.select_anchor = Some((1, 1));
        
        assert!(state.is_in_selection(2, 2));
        assert!(state.is_in_selection(1, 1));
        assert!(state.is_in_selection(3, 3));
        assert!(!state.is_in_selection(0, 0));
        assert!(!state.is_in_selection(4, 4));
    }

    #[test]
    fn test_move_cursor() {
        let mut state = DataGridState::new();
        state.cursor = (5, 5);
        
        state.move_cursor(1, 0, 10, 10);
        assert_eq!(state.cursor, (6, 5));
        
        state.move_cursor(-2, 0, 10, 10);
        assert_eq!(state.cursor, (4, 5));
        
        state.move_cursor(0, 1, 10, 10);
        assert_eq!(state.cursor, (4, 6));
    }

    #[test]
    fn test_move_cursor_bounds() {
        let mut state = DataGridState::new();
        state.cursor = (0, 0);
        
        // Should not go below 0
        state.move_cursor(-1, 0, 10, 10);
        assert_eq!(state.cursor, (0, 0));
        
        // Should not exceed max
        state.cursor = (9, 9);
        state.move_cursor(1, 1, 10, 10);
        assert_eq!(state.cursor, (9, 9));
    }

    #[test]
    fn test_goto_line_start_end() {
        let mut state = DataGridState::new();
        state.cursor = (5, 5);
        
        state.goto_line_start();
        assert_eq!(state.cursor.1, 0);
        
        state.goto_line_end(10);
        assert_eq!(state.cursor.1, 9);
    }

    #[test]
    fn test_goto_file_start_end() {
        let mut state = DataGridState::new();
        state.cursor = (50, 5);
        
        state.goto_file_start();
        assert_eq!(state.cursor, (0, 0));
        
        state.goto_file_end(100);
        assert_eq!(state.cursor.0, 99);
    }

    #[test]
    fn test_count_prefix() {
        let mut state = DataGridState::new();
        state.cursor = (0, 0);
        state.count = Some(5);
        
        state.move_cursor(1, 0, 100, 10);
        assert_eq!(state.cursor, (5, 0));
        assert!(state.count.is_none()); // Count should be cleared after use
    }
}
