//! 数据导入导出模块
//!
//! 支持 CSV、TSV、SQL、JSON 格式的数据导入导出。

use crate::database::{DatabaseType, QueryResult};
use std::borrow::Cow;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// JSON 导入文件大小上限（防止超大文件导致内存峰值过高）
const MAX_JSON_IMPORT_FILE_BYTES: u64 = 128 * 1024 * 1024;

// ============================================================================
// 导出格式
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Csv,
    Tsv,
    Sql,
    Json,
}

impl ExportFormat {
    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Tsv => "tsv",
            ExportFormat::Sql => "sql",
            ExportFormat::Json => "json",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ExportFormat::Csv => "CSV",
            ExportFormat::Tsv => "TSV",
            ExportFormat::Sql => "SQL",
            ExportFormat::Json => "JSON",
        }
    }
}

// ============================================================================
// 导入配置（简化版，供 core 导入函数使用）
// ============================================================================

/// CSV 导入配置
#[derive(Debug, Clone)]
pub struct CsvImportConfig {
    /// 是否有表头行
    pub has_header: bool,
    /// 分隔符 (默认逗号)
    pub delimiter: char,
    /// 引用字符 (默认双引号)
    pub quote_char: char,
    /// 跳过前 N 行
    pub skip_rows: usize,
    /// 最大导入行数 (0 = 无限制)
    pub max_rows: usize,
    /// 目标表名
    pub table_name: String,
    /// 自定义列名 (如果 has_header = false)
    pub column_names: Vec<String>,
}

impl Default for CsvImportConfig {
    fn default() -> Self {
        Self {
            has_header: true,
            delimiter: ',',
            quote_char: '"',
            skip_rows: 0,
            max_rows: 0,
            table_name: String::new(),
            column_names: Vec::new(),
        }
    }
}

/// JSON 导入配置
#[derive(Debug, Clone, Default)]
pub struct JsonImportConfig {
    /// 目标表名
    pub table_name: String,
    /// 最大导入行数 (0 = 无限制)
    pub max_rows: usize,
    /// JSON 路径 (如 "data.items" 表示从 data.items 开始读取)
    pub json_path: Option<String>,
    /// 是否展平嵌套对象（a.b.c）
    pub flatten_nested: bool,
}

// ============================================================================
// 导入结果
// ============================================================================

/// 导入预览结果
#[derive(Debug, Clone)]
pub struct ImportPreview {
    /// 列名
    pub columns: Vec<String>,
    /// 预览行数据 (最多 100 行)
    pub preview_rows: Vec<Vec<String>>,
    /// 文件总行数 (估计值)
    pub total_rows: usize,
    /// 检测到的问题
    pub warnings: Vec<String>,
}

/// 导入结果
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// 生成的 SQL 语句
    pub sql_statements: Vec<String>,
}

// ============================================================================
// 导出函数（统一导出管线）
// ============================================================================

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SqlDialect {
    #[default]
    Standard,
    MySql,
}

impl SqlDialect {
    pub const fn from_database_type(db_type: DatabaseType) -> Self {
        match db_type {
            DatabaseType::MySQL => Self::MySql,
            DatabaseType::SQLite | DatabaseType::PostgreSQL => Self::Standard,
        }
    }

    fn quote_identifier(self, name: &str) -> String {
        match self {
            Self::MySql => format!("`{}`", name.replace('`', "``")),
            Self::Standard => format!("\"{}\"", name.replace('"', "\"\"")),
        }
    }
}

impl From<DatabaseType> for SqlDialect {
    fn from(value: DatabaseType) -> Self {
        Self::from_database_type(value)
    }
}

#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub format: ExportFormat,
    pub selected_columns: Vec<usize>,
    pub row_limit: usize,
    pub start_row: usize,
    pub csv_delimiter: char,
    pub csv_include_header: bool,
    pub csv_quote_char: char,
    pub sql_use_transaction: bool,
    pub sql_batch_size: usize,
    pub json_pretty: bool,
    pub sql_dialect: SqlDialect,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Csv,
            selected_columns: Vec::new(),
            row_limit: 0,
            start_row: 0,
            csv_delimiter: ',',
            csv_include_header: true,
            csv_quote_char: '"',
            sql_use_transaction: true,
            sql_batch_size: 100,
            json_pretty: true,
            sql_dialect: SqlDialect::Standard,
        }
    }
}

fn selected_column_indices(result: &QueryResult, options: &ExportOptions) -> Vec<usize> {
    let mut seen = HashSet::new();
    let source = if options.selected_columns.is_empty() {
        (0..result.columns.len()).collect::<Vec<_>>()
    } else {
        options.selected_columns.clone()
    };

    source
        .into_iter()
        .filter(|idx| *idx < result.columns.len())
        .filter(|idx| seen.insert(*idx))
        .collect()
}

pub fn filter_result_for_export(result: &QueryResult, options: &ExportOptions) -> QueryResult {
    let selected_indices = selected_column_indices(result, options);

    let columns: Vec<String> = selected_indices
        .iter()
        .filter_map(|&i| result.columns.get(i).cloned())
        .collect();

    let row_range =
        result
            .rows
            .iter()
            .enumerate()
            .skip(options.start_row)
            .take(if options.row_limit > 0 {
                options.row_limit
            } else {
                usize::MAX
            });

    let mut rows = Vec::new();
    let mut null_flags = Vec::new();
    for (row_idx, row) in row_range {
        rows.push(
            selected_indices
                .iter()
                .filter_map(|&i| row.get(i).cloned())
                .collect(),
        );
        null_flags.push(
            selected_indices
                .iter()
                .map(|&i| result.is_null(row_idx, i))
                .collect(),
        );
    }

    QueryResult {
        columns,
        rows,
        null_flags,
        affected_rows: result.affected_rows,
        truncated: result.truncated,
        original_row_count: result.original_row_count,
    }
}

fn export_cell_text(result: &QueryResult, row_idx: usize, col_idx: usize) -> String {
    if result.is_null(row_idx, col_idx) {
        String::new()
    } else {
        result
            .rows
            .get(row_idx)
            .and_then(|row| row.get(col_idx))
            .cloned()
            .unwrap_or_default()
    }
}

fn export_sql_literal(result: &QueryResult, row_idx: usize, col_idx: usize) -> String {
    if result.is_null(row_idx, col_idx) {
        "NULL".to_string()
    } else {
        format!(
            "'{}'",
            result
                .rows
                .get(row_idx)
                .and_then(|row| row.get(col_idx))
                .cloned()
                .unwrap_or_default()
                .replace("'", "''")
        )
    }
}

fn export_json_value(result: &QueryResult, row_idx: usize, col_idx: usize) -> serde_json::Value {
    if result.is_null(row_idx, col_idx) {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(
            result
                .rows
                .get(row_idx)
                .and_then(|row| row.get(col_idx))
                .cloned()
                .unwrap_or_default(),
        )
    }
}

fn escape_delimited_field(field: &str, delimiter: &str, quote: char) -> String {
    if field.contains(delimiter) || field.contains(quote) || field.contains('\n') {
        format!(
            "{}{}{}",
            quote,
            field.replace(quote, &format!("{}{}", quote, quote)),
            quote
        )
    } else {
        field.to_string()
    }
}

fn finish_line_output(lines: Vec<String>) -> String {
    if lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", lines.join("\n"))
    }
}

fn render_delimited(result: &QueryResult, options: &ExportOptions, delimiter: char) -> String {
    let delimiter_text = delimiter.to_string();
    let mut lines = Vec::new();

    if options.csv_include_header {
        lines.push(
            result
                .columns
                .iter()
                .map(|column| {
                    escape_delimited_field(column, &delimiter_text, options.csv_quote_char)
                })
                .collect::<Vec<_>>()
                .join(&delimiter_text),
        );
    }

    for row_idx in 0..result.rows.len() {
        lines.push(
            (0..result.columns.len())
                .map(|col_idx| {
                    escape_delimited_field(
                        &export_cell_text(result, row_idx, col_idx),
                        &delimiter_text,
                        options.csv_quote_char,
                    )
                })
                .collect::<Vec<_>>()
                .join(&delimiter_text),
        );
    }

    finish_line_output(lines)
}

fn render_sql(
    result: &QueryResult,
    table_name: &str,
    options: &ExportOptions,
) -> Result<String, String> {
    if result.columns.is_empty() {
        return Err("未选择任何列".to_string());
    }

    let mut output = String::new();
    output.push_str("-- Exported from Rust DB Manager\n");
    output.push_str(&format!("-- Table: {}\n", table_name));
    output.push_str(&format!("-- Rows: {}\n\n", result.rows.len()));

    if result.rows.is_empty() {
        output.push_str("-- No data to export\n");
        return Ok(output);
    }

    if options.sql_use_transaction {
        output.push_str("BEGIN;\n\n");
    }

    let quoted_table = options.sql_dialect.quote_identifier(table_name);
    let columns_str = result
        .columns
        .iter()
        .map(|column| options.sql_dialect.quote_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");

    if options.sql_batch_size > 0 {
        for start in (0..result.rows.len()).step_by(options.sql_batch_size) {
            let end = (start + options.sql_batch_size).min(result.rows.len());
            let values_list = (start..end)
                .map(|row_idx| {
                    let values = (0..result.columns.len())
                        .map(|col_idx| export_sql_literal(result, row_idx, col_idx))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("({})", values)
                })
                .collect::<Vec<_>>();

            output.push_str(&format!(
                "INSERT INTO {} ({}) VALUES\n  {};\n\n",
                quoted_table,
                columns_str,
                values_list.join(",\n  ")
            ));
        }
    } else {
        for row_idx in 0..result.rows.len() {
            let values = (0..result.columns.len())
                .map(|col_idx| export_sql_literal(result, row_idx, col_idx))
                .collect::<Vec<_>>()
                .join(", ");
            output.push_str(&format!(
                "INSERT INTO {} ({}) VALUES ({});\n",
                quoted_table, columns_str, values
            ));
        }
    }

    if options.sql_use_transaction {
        output.push('\n');
        output.push_str("COMMIT;\n");
    }

    Ok(output)
}

fn render_json(result: &QueryResult, options: &ExportOptions) -> Result<String, String> {
    let json_rows: Vec<serde_json::Map<String, serde_json::Value>> = result
        .rows
        .iter()
        .enumerate()
        .map(|(row_idx, row)| {
            result
                .columns
                .iter()
                .enumerate()
                .zip(row.iter())
                .map(|((col_idx, col), _)| {
                    (col.clone(), export_json_value(result, row_idx, col_idx))
                })
                .collect()
        })
        .collect();

    if options.json_pretty {
        serde_json::to_string_pretty(&json_rows)
    } else {
        serde_json::to_string(&json_rows)
    }
    .map_err(|e| e.to_string())
}

fn render_export_content(
    result: &QueryResult,
    table_name: &str,
    options: &ExportOptions,
) -> Result<String, String> {
    match options.format {
        ExportFormat::Csv => Ok(render_delimited(result, options, options.csv_delimiter)),
        ExportFormat::Tsv => Ok(render_delimited(result, options, '\t')),
        ExportFormat::Sql => render_sql(result, table_name, options),
        ExportFormat::Json => render_json(result, options),
    }
}

pub(crate) fn render_export_content_for_transfer(
    result: &QueryResult,
    table_name: &str,
    options: &ExportOptions,
) -> Result<String, String> {
    render_export_content(result, table_name, options)
}

fn preview_subset(result: &QueryResult, preview_rows: usize) -> QueryResult {
    let row_count = preview_rows.min(result.rows.len());
    QueryResult {
        columns: result.columns.clone(),
        rows: result.rows.iter().take(row_count).cloned().collect(),
        null_flags: result.null_flags.iter().take(row_count).cloned().collect(),
        affected_rows: result.affected_rows,
        truncated: result.truncated || row_count < result.rows.len(),
        original_row_count: Some(result.rows.len()),
    }
}

fn trim_trailing_newlines(mut content: String) -> String {
    while content.ends_with('\n') {
        content.pop();
    }
    content
}

pub fn export_to_path(
    result: &QueryResult,
    table_name: &str,
    path: &Path,
    options: &ExportOptions,
) -> Result<(), String> {
    let filtered = filter_result_for_export(result, options);
    if filtered.columns.is_empty() {
        return Err("未选择任何列".to_string());
    }

    let content = render_export_content(&filtered, table_name, options)?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

pub fn preview_export(
    result: &QueryResult,
    table_name: &str,
    options: &ExportOptions,
    preview_rows: usize,
) -> String {
    let filtered = filter_result_for_export(result, options);
    if filtered.columns.is_empty() {
        return "（未选择任何列）".to_string();
    }

    let preview = preview_subset(&filtered, preview_rows);
    let mut content = render_export_content(&preview, table_name, options)
        .unwrap_or_else(|err| format!("（预览失败: {}）", err));
    content = trim_trailing_newlines(content);

    let remaining_rows = filtered.rows.len().saturating_sub(preview.rows.len());
    if remaining_rows == 0 {
        return content;
    }

    let suffix_label = match options.format {
        ExportFormat::Csv | ExportFormat::Tsv => "行",
        ExportFormat::Sql | ExportFormat::Json => "条",
    };

    if !content.is_empty() {
        content.push('\n');
    }
    content.push_str(&format!("... (+{} {})", remaining_rows, suffix_label));
    content
}

#[allow(dead_code)]
pub fn export_to_csv(result: &QueryResult, path: &Path) -> Result<(), String> {
    let options = ExportOptions {
        format: ExportFormat::Csv,
        ..Default::default()
    };
    export_to_path(result, "result", path, &options)
}

#[allow(dead_code)]
pub fn export_to_sql(result: &QueryResult, table_name: &str, path: &Path) -> Result<(), String> {
    let options = ExportOptions {
        format: ExportFormat::Sql,
        ..Default::default()
    };
    export_to_path(result, table_name, path, &options)
}

#[allow(dead_code)]
pub fn export_to_json(result: &QueryResult, path: &Path) -> Result<(), String> {
    let options = ExportOptions {
        format: ExportFormat::Json,
        ..Default::default()
    };
    export_to_path(result, "result", path, &options)
}

// ============================================================================
// CSV 导入
// ============================================================================

fn csv_config_byte(ch: char, label: &str) -> Result<u8, String> {
    if !ch.is_ascii() {
        return Err(format!("{} 必须是 ASCII 单字节字符", label));
    }
    Ok(ch as u8)
}

fn open_csv_reader(
    path: &Path,
    config: &CsvImportConfig,
) -> Result<csv::Reader<BufReader<File>>, String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let mut reader = BufReader::new(file);

    // 先按物理行跳过文件头部注释/说明
    for _ in 0..config.skip_rows {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| format!("跳过行失败: {}", e))?;
        if read == 0 {
            return Err("文件行数不足".to_string());
        }
    }

    let delimiter = csv_config_byte(config.delimiter, "CSV 分隔符")?;
    let quote = csv_config_byte(config.quote_char, "CSV 引号字符")?;

    let mut builder = csv::ReaderBuilder::new();
    builder
        .has_headers(config.has_header)
        .delimiter(delimiter)
        .quote(quote)
        .flexible(true);

    Ok(builder.from_reader(reader))
}

fn is_blank_csv_record(record: &csv::StringRecord) -> bool {
    record.is_empty() || (record.len() == 1 && record.get(0).is_some_and(|v| v.is_empty()))
}

/// 预览 CSV 文件
pub fn preview_csv(path: &Path, config: &CsvImportConfig) -> Result<ImportPreview, String> {
    let mut reader = open_csv_reader(path, config)?;
    let mut warnings = Vec::new();
    let mut first_data_fields: Option<Vec<String>> = None;

    // 读取列名
    let mut columns = if config.has_header {
        reader
            .headers()
            .map_err(|e| format!("读取表头失败: {}", e))?
            .iter()
            .map(|v| v.to_string())
            .collect()
    } else if !config.column_names.is_empty() {
        config.column_names.clone()
    } else {
        Vec::new()
    };
    let mut records = reader.records();
    if !config.has_header && config.column_names.is_empty() {
        let first_record = records
            .next()
            .ok_or("文件为空")?
            .map_err(|e| format!("读取 CSV 记录失败: {}", e))?;
        let first_fields = first_record
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        columns = (0..first_fields.len())
            .map(|i| format!("column_{}", i + 1))
            .collect();
        first_data_fields = Some(first_fields);
    }

    // 读取预览数据 (最多 100 行)
    let mut preview_rows = Vec::new();
    let mut total_rows = 0;

    if let Some(fields) = first_data_fields {
        total_rows += 1;
        if fields.len() != columns.len() {
            warnings.push(format!(
                "第 {} 行字段数 ({}) 与列数 ({}) 不匹配",
                total_rows,
                fields.len(),
                columns.len()
            ));
        }
        preview_rows.push(fields);
    }

    for record_result in records {
        let record = record_result.map_err(|e| format!("读取 CSV 记录失败: {}", e))?;
        if is_blank_csv_record(&record) {
            continue;
        }

        if columns.is_empty() {
            columns = (0..record.len())
                .map(|i| format!("column_{}", i + 1))
                .collect();
        }

        total_rows += 1;
        let fields = record.iter().map(|v| v.to_string()).collect::<Vec<_>>();

        if preview_rows.len() < 100 {
            // 检查字段数是否匹配
            if fields.len() != columns.len() {
                warnings.push(format!(
                    "第 {} 行字段数 ({}) 与列数 ({}) 不匹配",
                    total_rows,
                    fields.len(),
                    columns.len()
                ));
            }

            preview_rows.push(fields);
        }

        // 限制扫描行数以提高性能
        if total_rows > 10000 {
            break;
        }
    }

    if columns.is_empty() {
        return Err("文件为空".to_string());
    }

    Ok(ImportPreview {
        columns,
        preview_rows,
        total_rows,
        warnings,
    })
}

/// 从 CSV 文件生成 INSERT 语句
pub fn import_csv_to_sql(
    path: &Path,
    config: &CsvImportConfig,
    use_mysql_syntax: bool,
) -> Result<ImportResult, String> {
    let mut reader = open_csv_reader(path, config)?;
    let mut sql_statements = Vec::new();
    let mut rows_imported = 0;
    let mut first_data_fields: Option<Vec<String>> = None;

    // 读取列名
    let mut columns = if config.has_header {
        reader
            .headers()
            .map_err(|e| format!("读取表头失败: {}", e))?
            .iter()
            .map(|v| v.to_string())
            .collect()
    } else if !config.column_names.is_empty() {
        config.column_names.clone()
    } else {
        Vec::new()
    };
    let mut records = reader.records();
    if !config.has_header && config.column_names.is_empty() {
        let first_record = records
            .next()
            .ok_or("文件为空")?
            .map_err(|e| format!("读取 CSV 记录失败: {}", e))?;
        let first_fields = first_record
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>();
        columns = (0..first_fields.len())
            .map(|i| format!("column_{}", i + 1))
            .collect();
        first_data_fields = Some(first_fields);
    }

    if config.table_name.is_empty() {
        return Err("未指定目标表名".to_string());
    }

    // 生成列名部分
    let quote_char = if use_mysql_syntax { '`' } else { '"' };
    let columns_str = columns
        .iter()
        .map(|c| format!("{}{}{}", quote_char, escape_sql_identifier(c), quote_char))
        .collect::<Vec<_>>()
        .join(", ");

    let table_name = format!(
        "{}{}{}",
        quote_char,
        escape_sql_identifier(&config.table_name),
        quote_char
    );

    let append_insert_statement = |fields: Vec<String>,
                                   data_row_idx: usize,
                                   rows_imported: &mut usize,
                                   sql_statements: &mut Vec<String>|
     -> Result<(), String> {
        if fields.len() != columns.len() {
            return Err(format!(
                "第 {} 行字段数 ({}) 与列数 ({}) 不匹配",
                data_row_idx,
                fields.len(),
                columns.len()
            ));
        }

        let values = fields
            .iter()
            .map(|field| sql_value_from_string(field))
            .collect::<Vec<_>>()
            .join(", ");

        sql_statements.push(format!(
            "INSERT INTO {} ({}) VALUES ({});",
            table_name, columns_str, values
        ));
        *rows_imported += 1;
        Ok(())
    };

    let mut data_row_idx = 0usize;

    if let Some(fields) = first_data_fields
        && (config.max_rows == 0 || rows_imported < config.max_rows)
    {
        data_row_idx += 1;
        append_insert_statement(
            fields,
            data_row_idx,
            &mut rows_imported,
            &mut sql_statements,
        )?;
    }

    // 处理数据记录
    for record_result in records {
        if config.max_rows > 0 && rows_imported >= config.max_rows {
            break;
        }

        let record = record_result.map_err(|e| format!("读取 CSV 记录失败: {}", e))?;
        if is_blank_csv_record(&record) {
            continue;
        }

        data_row_idx += 1;
        let fields = record.iter().map(|v| v.to_string()).collect::<Vec<_>>();
        append_insert_statement(
            fields,
            data_row_idx,
            &mut rows_imported,
            &mut sql_statements,
        )?;
    }

    Ok(ImportResult { sql_statements })
}

/// 解析 CSV 行
/// 解析 CSV 行，处理引号转义
#[allow(dead_code)] // 公开 API，供测试和外部调用
pub fn parse_csv_line(line: &str, delimiter: char, quote_char: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current_field = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        if in_quotes {
            if c == quote_char {
                // 检查是否是转义的引号
                if chars.peek() == Some(&quote_char) {
                    current_field.push(quote_char);
                    chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current_field.push(c);
            }
        } else if c == quote_char {
            in_quotes = true;
        } else if c == delimiter {
            fields.push(current_field);
            current_field = String::new();
        } else {
            current_field.push(c);
        }
    }

    // 添加最后一个字段
    fields.push(current_field);

    fields
}

// ============================================================================
// JSON 导入
// ============================================================================

fn read_json_content(path: &Path) -> Result<String, String> {
    let metadata = std::fs::metadata(path).map_err(|e| format!("无法读取文件信息: {}", e))?;
    if metadata.len() > MAX_JSON_IMPORT_FILE_BYTES {
        return Err(format!(
            "JSON 文件过大 ({} MB)，当前上限 {} MB。请先拆分文件后再导入",
            metadata.len() / 1024 / 1024,
            MAX_JSON_IMPORT_FILE_BYTES / 1024 / 1024
        ));
    }

    std::fs::read_to_string(path).map_err(|e| format!("无法读取文件: {}", e))
}

fn collect_json_columns(
    array: &[serde_json::Value],
    flatten_nested: bool,
    max_scan_rows: usize,
) -> (Vec<String>, bool, bool) {
    let mut columns = Vec::new();
    let mut seen = HashSet::new();
    let mut has_object = false;
    let mut has_non_object = false;

    let scan_rows = array.len().min(max_scan_rows);
    for item in array.iter().take(scan_rows) {
        let normalized = normalize_json_item(item, flatten_nested);
        match normalized.as_ref() {
            serde_json::Value::Object(obj) => {
                has_object = true;
                for key in obj.keys() {
                    if seen.insert(key.clone()) {
                        columns.push(key.clone());
                    }
                }
            }
            _ => {
                has_non_object = true;
            }
        }
    }

    if !has_object {
        return (
            vec!["value".to_string()],
            scan_rows < array.len(),
            has_non_object,
        );
    }

    if columns.is_empty() {
        return (vec!["value".to_string()], scan_rows < array.len(), true);
    }

    (columns, scan_rows < array.len(), has_non_object)
}

/// 预览 JSON 文件
pub fn preview_json(path: &Path, config: &JsonImportConfig) -> Result<ImportPreview, String> {
    let content = read_json_content(path)?;

    let json_value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;

    // 提取数组数据
    let array = extract_json_array(&json_value, config.json_path.as_deref())?;

    if array.is_empty() {
        return Err("JSON 数组为空".to_string());
    }

    let mut warnings = Vec::new();

    let (columns, scan_truncated, has_non_object) =
        collect_json_columns(array, config.flatten_nested, 10_000);
    if columns.len() == 1 && columns[0] == "value" {
        warnings.push("未检测到可用对象字段，使用 value 列导入".to_string());
    } else if has_non_object {
        warnings.push("检测到非对象元素或空对象，将其写入第一个列，其余列填充 NULL".to_string());
    }
    if scan_truncated {
        warnings.push("列推断仅扫描前 10000 行，后续新字段可能未显示".to_string());
    }

    // 读取预览数据
    let mut preview_rows = Vec::new();
    let total_rows = array.len();

    for (idx, item) in array.iter().enumerate() {
        if idx >= 100 {
            break;
        }

        let normalized_item = normalize_json_item(item, config.flatten_nested);
        let row = match normalized_item.as_ref() {
            serde_json::Value::Object(obj) => columns
                .iter()
                .map(|col| {
                    obj.get(col)
                        .map(json_value_to_string)
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect(),
            other => {
                let mut row = vec!["NULL".to_string(); columns.len()];
                if let Some(first) = row.first_mut() {
                    *first = json_value_to_string(other);
                }
                row
            }
        };

        preview_rows.push(row);
    }

    Ok(ImportPreview {
        columns,
        preview_rows,
        total_rows,
        warnings,
    })
}

/// 从 JSON 文件生成 INSERT 语句
pub fn import_json_to_sql(
    path: &Path,
    config: &JsonImportConfig,
    use_mysql_syntax: bool,
) -> Result<ImportResult, String> {
    let content = read_json_content(path)?;

    let json_value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 解析失败: {}", e))?;

    // 提取数组数据
    let array = extract_json_array(&json_value, config.json_path.as_deref())?;

    if array.is_empty() {
        return Ok(ImportResult {
            sql_statements: Vec::new(),
        });
    }

    if config.table_name.is_empty() {
        return Err("未指定目标表名".to_string());
    }

    let mut sql_statements = Vec::new();
    let mut rows_imported = 0;

    let scan_rows = if config.max_rows > 0 {
        config.max_rows
    } else {
        array.len()
    };
    let (columns, _scan_truncated, _has_non_object) =
        collect_json_columns(array, config.flatten_nested, scan_rows);

    let quote_char = if use_mysql_syntax { '`' } else { '"' };
    let columns_str = columns
        .iter()
        .map(|c| format!("{}{}{}", quote_char, escape_sql_identifier(c), quote_char))
        .collect::<Vec<_>>()
        .join(", ");

    let table_name = format!(
        "{}{}{}",
        quote_char,
        escape_sql_identifier(&config.table_name),
        quote_char
    );

    for item in array {
        if config.max_rows > 0 && rows_imported >= config.max_rows {
            break;
        }

        let normalized_item = normalize_json_item(item, config.flatten_nested);
        let values = match normalized_item.as_ref() {
            serde_json::Value::Object(obj) => columns
                .iter()
                .map(|col| {
                    obj.get(col)
                        .map(json_value_to_sql)
                        .unwrap_or_else(|| "NULL".to_string())
                })
                .collect::<Vec<_>>()
                .join(", "),
            other => {
                if columns.len() == 1 {
                    json_value_to_sql(other)
                } else {
                    let mut values = Vec::with_capacity(columns.len());
                    values.push(json_value_to_sql(other));
                    values.extend((1..columns.len()).map(|_| "NULL".to_string()));
                    values.join(", ")
                }
            }
        };

        if values.is_empty() {
            continue;
        }

        sql_statements.push(format!(
            "INSERT INTO {} ({}) VALUES ({});",
            table_name, columns_str, values
        ));

        rows_imported += 1;
    }

    Ok(ImportResult { sql_statements })
}

/// 在需要时展平 JSON 对象；未启用时返回借用
fn normalize_json_item<'a>(
    item: &'a serde_json::Value,
    flatten_nested: bool,
) -> Cow<'a, serde_json::Value> {
    if !flatten_nested {
        return Cow::Borrowed(item);
    }

    match item {
        serde_json::Value::Object(obj) => {
            let mut flattened = serde_json::Map::new();
            flatten_json_object(obj, None, &mut flattened);
            Cow::Owned(serde_json::Value::Object(flattened))
        }
        _ => Cow::Borrowed(item),
    }
}

/// 将嵌套对象展平为点路径键（a.b.c）
fn flatten_json_object(
    obj: &serde_json::Map<String, serde_json::Value>,
    prefix: Option<&str>,
    output: &mut serde_json::Map<String, serde_json::Value>,
) {
    for (key, value) in obj {
        let flat_key = if let Some(parent) = prefix {
            format!("{}.{}", parent, key)
        } else {
            key.clone()
        };

        match value {
            serde_json::Value::Object(child) => {
                flatten_json_object(child, Some(&flat_key), output);
            }
            _ => {
                output.insert(flat_key, value.clone());
            }
        }
    }
}

/// 从 JSON 值中提取数组
fn extract_json_array<'a>(
    value: &'a serde_json::Value,
    json_path: Option<&str>,
) -> Result<&'a Vec<serde_json::Value>, String> {
    let target = if let Some(path) = json_path {
        let mut current = value;
        for key in path.split('.') {
            current = current
                .get(key)
                .ok_or_else(|| format!("JSON 路径 '{}' 不存在", key))?;
        }
        current
    } else {
        value
    };

    match target {
        serde_json::Value::Array(arr) => Ok(arr),
        _ => Err("目标不是 JSON 数组".to_string()),
    }
}

/// 将 JSON 值转换为显示字符串
fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string())
        }
        serde_json::Value::Object(obj) => {
            serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string())
        }
    }
}

/// 将 JSON 值转换为 SQL 值
/// 将 JSON 值转换为 SQL 值
pub fn json_value_to_sql(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        serde_json::Value::Array(arr) => {
            let json_str = serde_json::to_string(arr).unwrap_or_else(|_| "[]".to_string());
            format!("'{}'", json_str.replace('\'', "''"))
        }
        serde_json::Value::Object(obj) => {
            let json_str = serde_json::to_string(obj).unwrap_or_else(|_| "{}".to_string());
            format!("'{}'", json_str.replace('\'', "''"))
        }
    }
}

/// 将字符串转换为 SQL 值
pub fn sql_value_from_string(s: &str) -> String {
    let trimmed = s.trim();

    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("null") {
        "NULL".to_string()
    } else if trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok() {
        trimmed.to_string()
    } else if trimmed.eq_ignore_ascii_case("true") {
        "1".to_string()
    } else if trimmed.eq_ignore_ascii_case("false") {
        "0".to_string()
    } else {
        format!("'{}'", trimmed.replace('\'', "''"))
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 转义 CSV 字段中的特殊字符
#[allow(dead_code)] // 被 export_to_csv 使用
fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// 转义 SQL 标识符（表名、列名）中的特殊字符
fn escape_sql_identifier(name: &str) -> String {
    name.replace('`', "``").replace('"', "\"\"")
}

/// 读取 SQL 文件内容
#[allow(dead_code)] // 公开 API，供外部使用
pub fn import_sql_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| e.to_string())
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::QueryResult;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn filter_result_for_export_deduplicates_and_preserves_null_flags() {
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string(), "note".to_string()],
            rows: vec![vec!["1".to_string(), "alice".to_string(), String::new()]],
            null_flags: vec![vec![false, false, true]],
            affected_rows: 0,
            truncated: false,
            original_row_count: None,
        };
        let options = ExportOptions {
            selected_columns: vec![2, 0, 2, 9],
            ..Default::default()
        };

        let filtered = filter_result_for_export(&result, &options);

        assert_eq!(filtered.columns, vec!["note".to_string(), "id".to_string()]);
        assert_eq!(filtered.rows, vec![vec![String::new(), "1".to_string()]]);
        assert_eq!(filtered.null_flags, vec![vec![true, false]]);
    }

    #[test]
    fn preview_export_matches_csv_file_content() {
        let result = QueryResult {
            columns: vec!["name".to_string(), "note".to_string()],
            rows: vec![vec!["a,b".to_string(), "he\"llo".to_string()]],
            null_flags: vec![vec![false, false]],
            affected_rows: 0,
            truncated: false,
            original_row_count: None,
        };
        let options = ExportOptions {
            format: ExportFormat::Csv,
            ..Default::default()
        };
        let file = NamedTempFile::new().expect("create temp file");

        export_to_path(&result, "preview_test", file.path(), &options).expect("export csv");
        let exported = fs::read_to_string(file.path()).expect("read csv");
        let preview = preview_export(&result, "preview_test", &options, 1);

        assert_eq!(preview, exported.trim_end_matches('\n'));
        assert!(preview.contains('"'));
    }

    #[test]
    fn preview_export_matches_sql_file_content_and_preserves_null_semantics() {
        let result = QueryResult {
            columns: vec!["user name".to_string(), "value".to_string()],
            rows: vec![vec!["NULL".to_string(), String::new()]],
            null_flags: vec![vec![false, true]],
            affected_rows: 0,
            truncated: false,
            original_row_count: None,
        };
        let options = ExportOptions {
            format: ExportFormat::Sql,
            sql_use_transaction: false,
            sql_batch_size: 0,
            sql_dialect: SqlDialect::from_database_type(DatabaseType::PostgreSQL),
            ..Default::default()
        };
        let file = NamedTempFile::new().expect("create temp file");

        export_to_path(&result, "public.users", file.path(), &options).expect("export sql");
        let exported = fs::read_to_string(file.path()).expect("read sql");
        let preview = preview_export(&result, "public.users", &options, 1);

        assert_eq!(preview, exported.trim_end_matches('\n'));
        assert!(preview.contains(
            "INSERT INTO \"public.users\" (\"user name\", \"value\") VALUES ('NULL', NULL);"
        ));
    }

    #[test]
    fn export_to_path_json_preserves_literal_null_string() {
        let result = QueryResult {
            columns: vec!["value".to_string()],
            rows: vec![vec!["NULL".to_string()], vec![String::new()]],
            null_flags: vec![vec![false], vec![true]],
            affected_rows: 0,
            truncated: false,
            original_row_count: None,
        };
        let options = ExportOptions {
            format: ExportFormat::Json,
            json_pretty: false,
            ..Default::default()
        };
        let file = NamedTempFile::new().expect("create temp file");

        export_to_path(&result, "json_test", file.path(), &options).expect("export json");

        let content = fs::read_to_string(file.path()).expect("read json");
        assert_eq!(content, r#"[{"value":"NULL"},{"value":null}]"#);
    }
}
