//! SQL 语法高亮测试

use gridix::core::{HighlightColors, SqlHighlighter};

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
fn test_cache_works() {
    let colors = HighlightColors::default();
    let highlighter = SqlHighlighter::new(colors);

    let sql = "SELECT 1";

    // 第一次调用
    let job1 = highlighter.highlight(sql);
    // 第二次调用应该使用缓存
    let job2 = highlighter.highlight(sql);

    assert_eq!(job1.text, job2.text);
}
