//! Core 模块测试

use gridix::core::{
    NotificationManager,
    ProgressManager,
    KeyBinding, KeyBindings, KeyCode, Action,
    SessionState, TabState,
    AutoComplete,
    format_sql,
    SqlHighlighter, HighlightColors,
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
// Session 测试
// ============================================================================

#[test]
fn test_tab_state() {
    let tab = TabState::new("Query 1", "SELECT * FROM users");
    assert_eq!(tab.title, "Query 1");
    assert_eq!(tab.sql, "SELECT * FROM users");
    assert!(tab.associated_table.is_none());

    let tab = TabState::with_table("Users", "SELECT * FROM users", "users");
    assert!(tab.associated_table.is_some());
    assert_eq!(tab.associated_table.unwrap(), "users");
}

#[test]
fn test_session_state_tabs() {
    let mut session = SessionState::new();
    assert_eq!(session.tab_count(), 0);

    session.add_tab(TabState::new("Tab 1", ""));
    assert_eq!(session.tab_count(), 1);
    assert_eq!(session.active_tab_index, 0);

    session.add_tab(TabState::new("Tab 2", ""));
    assert_eq!(session.tab_count(), 2);
    assert_eq!(session.active_tab_index, 1);

    session.remove_tab(0);
    assert_eq!(session.tab_count(), 1);
    assert_eq!(session.active_tab_index, 0);
}

#[test]
fn test_session_state_update() {
    let mut session = SessionState::new();
    session.add_tab(TabState::new("Tab 1", ""));
    
    session.update_tab(0, "SELECT 1".to_string());
    assert_eq!(session.tabs[0].sql, "SELECT 1");
}

#[test]
fn test_session_state_location() {
    let mut session = SessionState::new();
    session.record_last_location(
        Some("my_conn".to_string()),
        Some("my_db".to_string()),
        Some("my_table".to_string()),
    );
    
    assert_eq!(session.last_connection, Some("my_conn".to_string()));
    assert_eq!(session.last_database, Some("my_db".to_string()));
    assert_eq!(session.last_table, Some("my_table".to_string()));
}

#[test]
fn test_has_valid_session() {
    let mut session = SessionState::new();
    assert!(!session.has_valid_session());

    session.add_tab(TabState::new("Tab", ""));
    assert!(session.has_valid_session());

    session = SessionState::new();
    session.last_connection = Some("conn".to_string());
    assert!(session.has_valid_session());
}

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
