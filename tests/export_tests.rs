//! 导入导出模块测试

use gridix::core::{
    CsvImportConfig, ExportFormat, ImportFormat, JsonImportConfig,
};
use gridix::database::QueryResult;
use std::io::Write;
use tempfile::NamedTempFile;

#[cfg(test)]
mod export_format_tests {
    use super::*;

    #[test]
    fn test_csv_format() {
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Csv.display_name(), "CSV");
    }

    #[test]
    fn test_sql_format() {
        assert_eq!(ExportFormat::Sql.extension(), "sql");
        assert_eq!(ExportFormat::Sql.display_name(), "SQL");
    }

    #[test]
    fn test_json_format() {
        assert_eq!(ExportFormat::Json.extension(), "json");
        assert_eq!(ExportFormat::Json.display_name(), "JSON");
    }
}

#[cfg(test)]
mod import_format_tests {
    use super::*;

    #[test]
    fn test_import_format_from_extension() {
        assert_eq!(ImportFormat::from_extension("csv"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::from_extension("CSV"), Some(ImportFormat::Csv));
        assert_eq!(ImportFormat::from_extension("json"), Some(ImportFormat::Json));
        assert_eq!(ImportFormat::from_extension("txt"), None);
    }

    #[test]
    fn test_import_format_properties() {
        assert_eq!(ImportFormat::Csv.extension(), "csv");
        assert_eq!(ImportFormat::Json.extension(), "json");
    }
}

#[cfg(test)]
mod csv_import_tests {
    use super::*;
    use gridix::core::preview_csv;

    #[test]
    fn test_csv_import_config_default() {
        let config = CsvImportConfig::default();
        assert!(config.has_header);
        assert_eq!(config.delimiter, ',');
        assert_eq!(config.quote_char, '"');
        assert_eq!(config.skip_rows, 0);
        assert_eq!(config.max_rows, 0);
    }

    #[test]
    fn test_preview_simple_csv() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "id,name,age").unwrap();
        writeln!(file, "1,Alice,30").unwrap();
        writeln!(file, "2,Bob,25").unwrap();
        file.flush().unwrap();

        let config = CsvImportConfig {
            has_header: true,
            table_name: "users".to_string(),
            ..Default::default()
        };

        let preview = preview_csv(file.path(), &config).unwrap();
        
        assert_eq!(preview.columns, vec!["id", "name", "age"]);
        assert_eq!(preview.preview_rows.len(), 2);
        assert_eq!(preview.preview_rows[0], vec!["1", "Alice", "30"]);
    }

    #[test]
    fn test_preview_csv_with_quotes() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "name,description").unwrap();
        writeln!(file, r#""John Doe","Hello, World""#).unwrap();
        file.flush().unwrap();

        let config = CsvImportConfig::default();
        let preview = preview_csv(file.path(), &config).unwrap();
        
        assert_eq!(preview.preview_rows[0][0], "John Doe");
        assert_eq!(preview.preview_rows[0][1], "Hello, World");
    }

    #[test]
    fn test_preview_csv_skip_rows() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "# This is a comment").unwrap();
        writeln!(file, "id,name").unwrap();
        writeln!(file, "1,Alice").unwrap();
        file.flush().unwrap();

        let config = CsvImportConfig {
            has_header: true,
            skip_rows: 1,
            ..Default::default()
        };

        let preview = preview_csv(file.path(), &config).unwrap();
        assert_eq!(preview.columns, vec!["id", "name"]);
    }
}

#[cfg(test)]
mod json_import_tests {
    use super::*;
    use gridix::core::preview_json;

    #[test]
    fn test_json_import_config_default() {
        let config = JsonImportConfig::default();
        assert!(config.table_name.is_empty());
        assert_eq!(config.max_rows, 0);
        assert!(config.json_path.is_none());
    }

    #[test]
    fn test_preview_simple_json() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"[
            {{"id": 1, "name": "Alice"}},
            {{"id": 2, "name": "Bob"}}
        ]"#).unwrap();
        file.flush().unwrap();

        let config = JsonImportConfig {
            table_name: "users".to_string(),
            ..Default::default()
        };

        let preview = preview_json(file.path(), &config).unwrap();
        
        assert!(preview.columns.contains(&"id".to_string()));
        assert!(preview.columns.contains(&"name".to_string()));
        assert_eq!(preview.total_rows, 2);
    }

    #[test]
    fn test_preview_nested_json() {
        let mut file = NamedTempFile::new().unwrap();
        write!(file, r#"{{
            "data": {{
                "users": [
                    {{"id": 1, "name": "Alice"}}
                ]
            }}
        }}"#).unwrap();
        file.flush().unwrap();

        let config = JsonImportConfig {
            table_name: "users".to_string(),
            json_path: Some("data.users".to_string()),
            ..Default::default()
        };

        let preview = preview_json(file.path(), &config).unwrap();
        assert_eq!(preview.total_rows, 1);
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use gridix::core::{export_to_csv, export_to_json, export_to_sql};
    use std::fs;

    fn sample_result() -> QueryResult {
        QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
            affected_rows: 0,
        }
    }

    #[test]
    fn test_export_csv() {
        let result = sample_result();
        let file = NamedTempFile::new().unwrap();
        
        export_to_csv(&result, file.path()).unwrap();
        
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("id,name"));
        assert!(content.contains("1,Alice"));
        assert!(content.contains("2,Bob"));
    }

    #[test]
    fn test_export_json() {
        let result = sample_result();
        let file = NamedTempFile::new().unwrap();
        
        export_to_json(&result, file.path()).unwrap();
        
        let content = fs::read_to_string(file.path()).unwrap();
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap();
        
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0]["name"], "Alice");
    }

    #[test]
    fn test_export_sql() {
        let result = sample_result();
        let file = NamedTempFile::new().unwrap();
        
        export_to_sql(&result, "users", file.path()).unwrap();
        
        let content = fs::read_to_string(file.path()).unwrap();
        assert!(content.contains("INSERT INTO"));
        assert!(content.contains("users"));
        assert!(content.contains("Alice"));
    }

    #[test]
    fn test_export_csv_with_special_chars() {
        let result = QueryResult {
            columns: vec!["text".to_string()],
            rows: vec![
                vec!["hello, world".to_string()],
                vec!["quote\"test".to_string()],
            ],
            affected_rows: 0,
        };
        
        let file = NamedTempFile::new().unwrap();
        export_to_csv(&result, file.path()).unwrap();
        
        let content = fs::read_to_string(file.path()).unwrap();
        // 包含逗号的字段应该被引号包裹
        assert!(content.contains("\"hello, world\""));
    }
}
