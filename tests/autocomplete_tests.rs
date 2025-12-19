//! 自动补全测试

use gridix::core::AutoComplete;

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
