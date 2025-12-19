//! 数据表格相关测试
//!
//! 测试 SQL 标识符转义、值转义等安全相关功能

use gridix::ui::{escape_identifier, escape_value, quote_identifier};

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
