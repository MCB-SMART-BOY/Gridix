//! 导入文件解析器

use crate::core::{
    TransferDirection, TransferFormat, TransferFormatOptions, TransferRowWindow, TransferSchema,
    TransferSession, TransferSqlOptions, preview_sql_transfer_content,
};

use super::import_types::{ImportPreview, SqlImportConfig};

/// 解析 SQL 文件，处理注释并分割语句。
///
/// 真实解析逻辑位于 core transfer 层，这里只保留 UI 兼容包装。
pub fn parse_sql_file(content: &str, config: &SqlImportConfig) -> ImportPreview {
    let session = TransferSession {
        direction: TransferDirection::Import,
        format: TransferFormat::Sql,
        schema: TransferSchema::default(),
        mapping: Default::default(),
        row_window: TransferRowWindow {
            preview_rows: 10,
            ..Default::default()
        },
        options: TransferFormatOptions::Sql(TransferSqlOptions {
            use_transaction: config.use_transaction,
            batch_size: 0,
            strip_comments: config.strip_comments,
            strip_empty_lines: config.strip_empty_lines,
            stop_on_error: config.stop_on_error,
            ..Default::default()
        }),
    };

    preview_sql_transfer_content(content, &session)
        .map(ImportPreview::from_transfer_preview)
        .unwrap_or_else(|error| ImportPreview {
            warnings: vec![error],
            ..Default::default()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sql_file_with_custom_delimiter() {
        let content = r#"
DELIMITER //
CREATE PROCEDURE test_proc()
BEGIN
  INSERT INTO t VALUES (1);
  INSERT INTO t VALUES (2);
END//
DELIMITER ;
SELECT 1;
"#;

        let config = SqlImportConfig::default();
        let preview = parse_sql_file(content, &config);
        assert_eq!(preview.statement_count, 2);
        assert!(preview.sql_statements[0].starts_with("CREATE PROCEDURE test_proc()"));
        assert!(preview.sql_statements[0].contains("INSERT INTO t VALUES (1);"));
        assert!(preview.sql_statements[1].starts_with("SELECT 1"));
        assert!(preview.warnings.is_empty());
    }

    #[test]
    fn test_parse_sql_file_without_final_delimiter_warns() {
        let content = "SELECT 1";
        let config = SqlImportConfig::default();
        let preview = parse_sql_file(content, &config);

        assert_eq!(preview.statement_count, 1);
        assert_eq!(preview.warnings.len(), 1);
        assert!(preview.warnings[0].contains("最后一条语句没有结束分隔符"));
    }

    #[test]
    fn test_parse_sql_file_with_postgres_dollar_quoted_body() {
        let content = r#"
CREATE FUNCTION f_test() RETURNS void AS $$
BEGIN
  PERFORM 1;
  PERFORM 2;
END;
$$ LANGUAGE plpgsql;
SELECT 1;
"#;

        let config = SqlImportConfig::default();
        let preview = parse_sql_file(content, &config);

        assert_eq!(preview.statement_count, 2);
        assert!(preview.sql_statements[0].starts_with("CREATE FUNCTION f_test()"));
        assert!(preview.sql_statements[0].contains("PERFORM 2;"));
        assert!(preview.sql_statements[1].starts_with("SELECT 1"));
    }
}
