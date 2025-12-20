//! 导出模块测试

use gridix::core::{parse_csv_line, sql_value_from_string, json_value_to_sql};

#[test]
fn test_parse_csv_line_simple() {
    let line = "a,b,c";
    let fields = parse_csv_line(line, ',', '"');
    assert_eq!(fields, vec!["a", "b", "c"]);
}

#[test]
fn test_parse_csv_line_quoted() {
    let line = r#""hello, world","test""value","normal""#;
    let fields = parse_csv_line(line, ',', '"');
    assert_eq!(fields, vec!["hello, world", "test\"value", "normal"]);
}

#[test]
fn test_sql_value_from_string() {
    assert_eq!(sql_value_from_string("123"), "123");
    assert_eq!(sql_value_from_string("3.14"), "3.14");
    assert_eq!(sql_value_from_string("null"), "NULL");
    assert_eq!(sql_value_from_string("hello"), "'hello'");
    assert_eq!(sql_value_from_string("it's"), "'it''s'");
}

#[test]
fn test_json_value_to_sql() {
    assert_eq!(json_value_to_sql(&serde_json::Value::Null), "NULL");
    assert_eq!(json_value_to_sql(&serde_json::json!(42)), "42");
    assert_eq!(json_value_to_sql(&serde_json::json!("test")), "'test'");
    assert_eq!(json_value_to_sql(&serde_json::json!(true)), "1");
}
