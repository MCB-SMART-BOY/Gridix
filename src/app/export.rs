//! 数据导出功能
//!
//! 提供 CSV、SQL、JSON 格式的数据导出功能。

use crate::core::ExportFormat;
use crate::database::QueryResult;
use crate::ui::ExportConfig;
use std::path::Path;

/// 根据导出配置过滤查询结果
pub fn filter_result_for_export(result: &QueryResult, config: &ExportConfig) -> QueryResult {
    let selected_indices = config.get_selected_column_indices();

    // 过滤列
    let columns: Vec<String> = selected_indices
        .iter()
        .filter_map(|&i| result.columns.get(i).cloned())
        .collect();

    // 过滤行（根据起始行和限制）
    let rows: Vec<Vec<String>> = result
        .rows
        .iter()
        .skip(config.start_row)
        .take(if config.row_limit > 0 {
            config.row_limit
        } else {
            usize::MAX
        })
        .map(|row| {
            selected_indices
                .iter()
                .filter_map(|&i| row.get(i).cloned())
                .collect()
        })
        .collect();

    QueryResult {
        columns,
        rows,
        affected_rows: result.affected_rows,
    }
}

/// 导出为 CSV 格式
pub fn export_csv(
    result: &QueryResult,
    path: &Path,
    config: &ExportConfig,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path).map_err(|e| e.to_string())?;
    let delimiter = config.csv_delimiter.to_string();
    let quote = config.csv_quote_char;

    // 转义 CSV 字段
    let escape_field = |field: &str| -> String {
        if field.contains(config.csv_delimiter)
            || field.contains(quote)
            || field.contains('\n')
        {
            format!(
                "{}{}{}",
                quote,
                field.replace(quote, &format!("{}{}", quote, quote)),
                quote
            )
        } else {
            field.to_string()
        }
    };

    // 写入表头
    if config.csv_include_header {
        let header = result
            .columns
            .iter()
            .map(|c| escape_field(c))
            .collect::<Vec<_>>()
            .join(&delimiter);
        writeln!(file, "{}", header).map_err(|e| e.to_string())?;
    }

    // 写入数据行
    for row in &result.rows {
        let line = row
            .iter()
            .map(|cell| escape_field(cell))
            .collect::<Vec<_>>()
            .join(&delimiter);
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 导出为 SQL 格式
pub fn export_sql(
    result: &QueryResult,
    table_name: &str,
    path: &Path,
    config: &ExportConfig,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path).map_err(|e| e.to_string())?;

    writeln!(file, "-- Exported from Rust DB Manager").map_err(|e| e.to_string())?;
    writeln!(file, "-- Table: {}", table_name).map_err(|e| e.to_string())?;
    writeln!(file, "-- Rows: {}\n", result.rows.len()).map_err(|e| e.to_string())?;

    if result.columns.is_empty() || result.rows.is_empty() {
        writeln!(file, "-- No data to export").map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 开始事务
    if config.sql_use_transaction {
        writeln!(file, "BEGIN;").map_err(|e| e.to_string())?;
        writeln!(file).map_err(|e| e.to_string())?;
    }

    let escaped_table = table_name.replace('`', "``");
    let columns_str = result
        .columns
        .iter()
        .map(|c| format!("`{}`", c.replace('`', "``")))
        .collect::<Vec<_>>()
        .join(", ");

    if config.sql_batch_size > 0 {
        // 批量插入
        for chunk in result.rows.chunks(config.sql_batch_size) {
            let values_list: Vec<String> = chunk
                .iter()
                .map(|row| {
                    let values = row
                        .iter()
                        .map(|cell| {
                            if cell == "NULL" {
                                "NULL".to_string()
                            } else {
                                format!("'{}'", cell.replace('\'', "''"))
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("({})", values)
                })
                .collect();

            writeln!(
                file,
                "INSERT INTO `{}` ({}) VALUES\n  {};",
                escaped_table,
                columns_str,
                values_list.join(",\n  ")
            )
            .map_err(|e| e.to_string())?;
            writeln!(file).map_err(|e| e.to_string())?;
        }
    } else {
        // 单行插入
        for row in &result.rows {
            let values = row
                .iter()
                .map(|cell| {
                    if cell == "NULL" {
                        "NULL".to_string()
                    } else {
                        format!("'{}'", cell.replace('\'', "''"))
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");

            writeln!(
                file,
                "INSERT INTO `{}` ({}) VALUES ({});",
                escaped_table, columns_str, values
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // 提交事务
    if config.sql_use_transaction {
        writeln!(file).map_err(|e| e.to_string())?;
        writeln!(file, "COMMIT;").map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 导出为 JSON 格式
pub fn export_json(
    result: &QueryResult,
    path: &Path,
    config: &ExportConfig,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::Write;

    let mut file = File::create(path).map_err(|e| e.to_string())?;

    let json_rows: Vec<serde_json::Map<String, serde_json::Value>> = result
        .rows
        .iter()
        .map(|row| {
            result
                .columns
                .iter()
                .zip(row.iter())
                .map(|(col, cell)| {
                    let value = if cell == "NULL" {
                        serde_json::Value::Null
                    } else if let Ok(num) = cell.parse::<i64>() {
                        serde_json::Value::Number(num.into())
                    } else if let Ok(num) = cell.parse::<f64>() {
                        serde_json::json!(num)
                    } else {
                        serde_json::Value::String(cell.clone())
                    };
                    (col.clone(), value)
                })
                .collect()
        })
        .collect();

    let json = if config.json_pretty {
        serde_json::to_string_pretty(&json_rows)
    } else {
        serde_json::to_string(&json_rows)
    }
    .map_err(|e| e.to_string())?;

    write!(file, "{}", json).map_err(|e| e.to_string())?;

    Ok(())
}

/// 执行导出操作
///
/// 根据配置选择相应的导出格式并执行
pub fn execute_export(
    result: &QueryResult,
    table_name: &str,
    path: &Path,
    config: &ExportConfig,
) -> Result<String, String> {
    // 根据配置过滤数据
    let filtered_result = filter_result_for_export(result, config);

    let export_result = match config.format {
        ExportFormat::Csv => export_csv(&filtered_result, path, config),
        ExportFormat::Sql => export_sql(&filtered_result, table_name, path, config),
        ExportFormat::Json => export_json(&filtered_result, path, config),
    };

    export_result.map(|()| {
        format!(
            "已导出 {} 行到 {}",
            filtered_result.rows.len(),
            path.display()
        )
    })
}
