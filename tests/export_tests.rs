//! 导出模块测试

use gridix::core::{
    CsvImportConfig, JsonImportConfig, import_csv_to_sql, import_json_to_sql, json_value_to_sql,
    parse_csv_line, preview_csv, preview_json, sql_value_from_string,
};
use std::io::Write;
use tempfile::NamedTempFile;

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
fn test_parse_csv_line_preserves_whitespace() {
    let line = "  a  ,\" b \",c  ";
    let fields = parse_csv_line(line, ',', '"');
    assert_eq!(fields, vec!["  a  ", " b ", "c  "]);
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

#[test]
fn test_preview_csv_without_header_keeps_first_row() {
    let mut file = NamedTempFile::new().expect("create temp file");
    writeln!(file, "1,alice").expect("write first row");
    writeln!(file, "2,bob").expect("write second row");

    let config = CsvImportConfig {
        has_header: false,
        table_name: "users".to_string(),
        ..Default::default()
    };

    let preview = preview_csv(file.path(), &config).expect("preview csv");
    assert_eq!(preview.columns, vec!["column_1", "column_2"]);
    assert_eq!(preview.total_rows, 2);
    assert_eq!(preview.preview_rows.len(), 2);
    assert_eq!(preview.preview_rows[0], vec!["1", "alice"]);
}

#[test]
fn test_import_csv_without_header_auto_generates_columns() {
    let mut file = NamedTempFile::new().expect("create temp file");
    writeln!(file, "1,alice").expect("write first row");
    writeln!(file, "2,bob").expect("write second row");

    let config = CsvImportConfig {
        has_header: false,
        table_name: "users".to_string(),
        ..Default::default()
    };

    let result = import_csv_to_sql(file.path(), &config, false).expect("import csv");
    assert_eq!(result.sql_statements.len(), 2);
    assert!(
        result.sql_statements[0]
            .contains("INSERT INTO \"users\" (\"column_1\", \"column_2\") VALUES (1, 'alice');")
    );
    assert!(
        result.sql_statements[1]
            .contains("INSERT INTO \"users\" (\"column_1\", \"column_2\") VALUES (2, 'bob');")
    );
}

#[test]
fn test_import_csv_reports_mismatched_field_count() {
    let mut file = NamedTempFile::new().expect("create temp file");
    writeln!(file, "id,name").expect("write header");
    writeln!(file, "1,alice").expect("write first row");
    writeln!(file, "2").expect("write mismatched row");

    let config = CsvImportConfig {
        has_header: true,
        table_name: "users".to_string(),
        ..Default::default()
    };

    let err = import_csv_to_sql(file.path(), &config, false).expect_err("expect mismatch error");
    assert!(err.contains("字段数"));
    assert!(err.contains("不匹配"));
}

#[test]
fn test_csv_multiline_quoted_field_preview_and_import() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, "id,notes\n1,\"hello\nworld\"\n2,\"single line\"\n").expect("write csv");

    let config = CsvImportConfig {
        has_header: true,
        table_name: "logs".to_string(),
        ..Default::default()
    };

    let preview = preview_csv(file.path(), &config).expect("preview csv");
    assert_eq!(preview.total_rows, 2);
    assert_eq!(preview.preview_rows[0][1], "hello\nworld");

    let result = import_csv_to_sql(file.path(), &config, false).expect("import csv");
    assert_eq!(result.sql_statements.len(), 2);
    assert!(result.sql_statements[0].contains("'hello\nworld'"));
}

#[test]
fn test_json_flatten_nested_preview_and_import() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(
        file,
        r#"[{{"id":1,"profile":{{"name":"alice","meta":{{"age":30}}}}}}]"#
    )
    .expect("write json");

    let preview_config = JsonImportConfig {
        flatten_nested: true,
        ..Default::default()
    };
    let preview = preview_json(file.path(), &preview_config).expect("preview json");
    assert_eq!(preview.total_rows, 1);
    assert!(preview.columns.iter().any(|c| c == "id"));
    assert!(preview.columns.iter().any(|c| c == "profile.name"));
    assert!(preview.columns.iter().any(|c| c == "profile.meta.age"));

    let import_config = JsonImportConfig {
        table_name: "users".to_string(),
        flatten_nested: true,
        ..Default::default()
    };
    let result = import_json_to_sql(file.path(), &import_config, false).expect("import json");
    assert_eq!(result.sql_statements.len(), 1);
    let sql = &result.sql_statements[0];
    assert!(sql.contains("\"profile.name\""));
    assert!(sql.contains("\"profile.meta.age\""));
    assert!(sql.contains("'alice'"));
    assert!(sql.contains("30"));
}

#[test]
fn test_json_union_columns_across_rows() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, r#"[{{"id":1}},{{"id":2,"name":"bob"}}]"#).expect("write json");

    let preview = preview_json(file.path(), &JsonImportConfig::default()).expect("preview json");
    assert!(preview.columns.iter().any(|c| c == "id"));
    assert!(preview.columns.iter().any(|c| c == "name"));

    let config = JsonImportConfig {
        table_name: "users".to_string(),
        ..Default::default()
    };
    let result = import_json_to_sql(file.path(), &config, false).expect("import json");
    assert_eq!(result.sql_statements.len(), 2);
    assert!(result.sql_statements[0].contains("NULL"));
    assert!(result.sql_statements[1].contains("'bob'"));
}

#[test]
fn test_json_empty_objects_fallback_to_value_column() {
    let mut file = NamedTempFile::new().expect("create temp file");
    write!(file, r#"[{{}},{{}}]"#).expect("write json");

    let preview = preview_json(file.path(), &JsonImportConfig::default()).expect("preview json");
    assert_eq!(preview.columns, vec!["value"]);
    assert_eq!(preview.preview_rows.len(), 2);
    assert_eq!(preview.preview_rows[0], vec!["NULL"]);

    let config = JsonImportConfig {
        table_name: "payloads".to_string(),
        ..Default::default()
    };
    let result = import_json_to_sql(file.path(), &config, false).expect("import json");
    assert_eq!(result.sql_statements.len(), 2);
    assert!(result.sql_statements[0].contains("\"value\""));
    assert!(result.sql_statements[0].contains("VALUES (NULL);"));
}
