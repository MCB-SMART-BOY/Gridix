//! Core 模块测试

use gridix::core::{
    Action, AutoComplete, HighlightColors, KeyBinding, KeyBindings, KeyCode, NotificationManager,
    ProgressManager, SqlHighlighter, format_sql,
};
use std::sync::atomic::Ordering;

// ============================================================================
// Notification 测试
// ============================================================================

#[test]
fn test_notification_manager() {
    let mut manager = NotificationManager::new();

    let id1 = manager.info("Info message");
    let id2 = manager.warning("Warning message");
    let id3 = manager.error("Error message");

    assert!(id1 < id2);
    assert!(id2 < id3);

    let count = manager.iter().count();
    assert_eq!(count, 3);

    manager.dismiss(id2);
    let count = manager.iter().count();
    assert_eq!(count, 2);
}

#[test]
fn test_max_notifications() {
    let mut manager = NotificationManager::new().with_max_notifications(3);

    manager.info("1");
    manager.info("2");
    manager.info("3");
    manager.info("4");

    let count = manager.iter().count();
    assert_eq!(count, 3);
}

// ============================================================================
// Progress 测试
// ============================================================================

#[test]
fn test_progress_manager() {
    let mut manager = ProgressManager::new();

    let id1 = manager.start("连接数据库", true);
    let id2 = manager.start("执行查询", false);

    assert_eq!(manager.active_count(), 2);

    manager.update(id1, 0.5);
    assert_eq!(manager.get(id1).unwrap().progress, Some(0.5));

    manager.finish(id1);
    assert_eq!(manager.active_count(), 1);

    manager.cancel(id2);
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn test_cancel_token() {
    let mut manager = ProgressManager::new();
    let id = manager.start("长时间操作", true);

    let token = manager.get(id).unwrap().cancel_token();
    assert!(!token.load(Ordering::Relaxed));

    manager.cancel(id);
    assert!(token.load(Ordering::Relaxed));
}

// ============================================================================
// Keybindings 测试
// ============================================================================

#[test]
fn test_key_binding_parse() {
    let binding = KeyBinding::parse("Ctrl+N").unwrap();
    assert_eq!(binding.key, KeyCode::N);
    assert!(binding.modifiers.ctrl);
    assert!(!binding.modifiers.shift);

    let binding = KeyBinding::parse("Ctrl+Shift+N").unwrap();
    assert_eq!(binding.key, KeyCode::N);
    assert!(binding.modifiers.ctrl);
    assert!(binding.modifiers.shift);

    let binding = KeyBinding::parse("F5").unwrap();
    assert_eq!(binding.key, KeyCode::F5);
    assert!(!binding.modifiers.ctrl);
}

#[test]
fn test_key_binding_display() {
    let binding = KeyBinding::ctrl(KeyCode::N);
    assert_eq!(binding.display(), "Ctrl+N");

    let binding = KeyBinding::ctrl_shift(KeyCode::N);
    assert_eq!(binding.display(), "Ctrl+Shift+N");

    let binding = KeyBinding::key_only(KeyCode::F5);
    assert_eq!(binding.display(), "F5");
}

#[test]
fn test_default_bindings() {
    let bindings = KeyBindings::default();

    assert!(bindings.get(Action::NewConnection).is_some());
    assert_eq!(
        bindings.get(Action::NewConnection).unwrap().display(),
        "Ctrl+N"
    );
}

#[test]
fn test_find_conflicts() {
    let mut bindings = KeyBindings::default();

    bindings.set(Action::NewTab, KeyBinding::ctrl(KeyCode::N));

    let conflicts = bindings.find_conflicts();
    assert!(!conflicts.is_empty());
}

// ============================================================================
// ============================================================================
// Autocomplete 测试
// ============================================================================

#[test]
fn test_keyword_completion() {
    let ac = AutoComplete::new();
    let completions = ac.get_completions("SEL", 3);
    assert!(completions.iter().any(|c| c.label == "SELECT"));
}

#[test]
fn test_table_completion() {
    let mut ac = AutoComplete::new();
    ac.set_tables(vec!["users".to_string(), "orders".to_string()]);
    let completions = ac.get_completions("SELECT * FROM us", 16);
    assert!(completions.iter().any(|c| c.label == "users"));
}

// ============================================================================
// Formatter 测试
// ============================================================================

#[test]
fn test_simple_select() {
    let sql = "select * from users where id = 1";
    let formatted = format_sql(sql);
    assert!(formatted.contains("SELECT"));
    assert!(formatted.contains("FROM"));
    assert!(formatted.contains("WHERE"));
}

#[test]
fn test_multicolumn_select() {
    let sql = "select id, name, email from users";
    let formatted = format_sql(sql);
    assert!(formatted.contains("SELECT"));
}

#[test]
fn test_cte_formatting() {
    let sql = "with cte as (select id from users) select * from cte";
    let formatted = format_sql(sql);
    assert!(formatted.contains("WITH"));
    assert!(formatted.contains("SELECT"));
}

#[test]
fn test_recursive_cte_formatting() {
    let sql = "with recursive cte as (select 1 union all select n+1 from cte where n<10) select * from cte";
    let formatted = format_sql(sql);
    assert!(formatted.contains("WITH"));
    assert!(formatted.contains("RECURSIVE"));
    assert!(formatted.contains("SELECT"));
}

// ============================================================================
// Syntax Highlighter 测试
// ============================================================================

#[test]
fn test_highlight_basic_sql() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "SELECT * FROM users WHERE id = 1";
    let job = highlighter.highlight(sql);

    assert!(!job.text.is_empty());
    assert_eq!(job.text.trim(), sql);
}

#[test]
fn test_highlight_with_comments() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "-- This is a comment\nSELECT * FROM users";
    let job = highlighter.highlight(sql);

    assert!(job.text.contains("comment"));
}

#[test]
fn test_highlight_with_strings() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "SELECT * FROM users WHERE name = 'John''s'";
    let job = highlighter.highlight(sql);

    assert!(job.text.contains("John"));
}

#[test]
fn test_highlight_cache_works() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "SELECT 1";

    let job1 = highlighter.highlight(sql);
    let job2 = highlighter.highlight(sql);

    assert_eq!(job1.text, job2.text);
}

#[test]
fn test_highlight_cache_different_text() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql1 = "SELECT * FROM users";
    let sql2 = "SELECT * FROM orders";

    let job1 = highlighter.highlight(sql1);
    let job2 = highlighter.highlight(sql2);

    // Different text should produce different results
    assert_ne!(job1.text, job2.text);
}

#[test]
fn test_highlight_multiline_sql() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "SELECT id, name\nFROM users\nWHERE active = true";
    let job = highlighter.highlight(sql);

    assert!(job.text.contains("SELECT"));
    assert!(job.text.contains("FROM"));
    assert!(job.text.contains("WHERE"));
}

#[test]
fn test_highlight_with_numbers() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "SELECT * FROM users WHERE id = 123 AND price = 45.67";
    let job = highlighter.highlight(sql);

    assert!(job.text.contains("123"));
    assert!(job.text.contains("45.67"));
}

// ============================================================================
// DbError 测试
// ============================================================================

#[test]
fn test_db_error_connection_display() {
    let err = gridix::data::DbError::Connection("连接超时".to_string());
    assert!(err.to_string().contains("连接超时"));
    assert!(err.to_string().contains("连接错误"));
}

#[test]
fn test_db_error_query_display() {
    let err = gridix::data::DbError::Query("syntax error near FROM".to_string());
    assert!(err.to_string().contains("syntax error"));
    assert!(err.to_string().contains("查询错误"));
}

// ============================================================================
// Formatter 边界测试
// ============================================================================

#[test]
fn test_formatter_handles_empty_input() {
    let formatted = format_sql("");
    assert!(formatted.is_empty());
}

#[test]
fn test_formatter_preserves_keyword_case() {
    let sql = "select * from users where id = 1";
    let formatted = format_sql(sql);
    assert!(formatted.contains("SELECT"));
    assert!(formatted.contains("FROM"));
    assert!(formatted.contains("WHERE"));
}

#[test]
fn test_formatter_handles_multiple_joins() {
    let sql = "select * from users join orders on users.id = orders.user_id left join products on orders.product_id = products.id";
    let formatted = format_sql(sql);
    assert!(formatted.contains("JOIN"));
    assert!(formatted.contains("ON"));
}
