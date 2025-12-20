//! 数据导入导出模块
//!
//! 支持 CSV、SQL、JSON 格式的数据导入导出。

use crate::database::QueryResult;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

// ============================================================================
// 导出格式
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Csv,
    Sql,
    Json,
}

impl ExportFormat {
    pub fn extension(&self) -> &str {
        match self {
            ExportFormat::Csv => "csv",
            ExportFormat::Sql => "sql",
            ExportFormat::Json => "json",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ExportFormat::Csv => "CSV",
            ExportFormat::Sql => "SQL",
            ExportFormat::Json => "JSON",
        }
    }
}

// ============================================================================
// 导入格式
// ============================================================================

/// 导入格式（供导入对话框使用）
#[allow(dead_code)] // 公开 API，供外部使用
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ImportFormat {
    Csv,
    Json,
}

#[allow(dead_code)] // 公开 API，供外部使用
impl ImportFormat {
    pub fn extension(&self) -> &str {
        match self {
            ImportFormat::Csv => "csv",
            ImportFormat::Json => "json",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ImportFormat::Csv => "CSV",
            ImportFormat::Json => "JSON",
        }
    }

    /// 从文件扩展名推断格式
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "csv" => Some(ImportFormat::Csv),
            "json" => Some(ImportFormat::Json),
            _ => None,
        }
    }
}

// ============================================================================
// 导入配置
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
}

// ============================================================================
// 导入结果
// ============================================================================

/// 导入预览结果（供导入对话框使用）
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

/// 导入结果（供导入对话框使用）
#[allow(dead_code)] // 公开 API，供外部使用
#[derive(Debug, Clone)]
pub struct ImportResult {
    /// 生成的 SQL 语句
    pub sql_statements: Vec<String>,
    /// 成功导入的行数
    pub rows_imported: usize,
    /// 跳过的行数
    pub rows_skipped: usize,
    /// 警告信息
    pub warnings: Vec<String>,
}

// ============================================================================
// 导出函数（简化版本，供测试和基本导出使用）
// ============================================================================

/// 导出查询结果到 CSV 文件
#[allow(dead_code)] // 公开 API，供外部使用
pub fn export_to_csv(result: &QueryResult, path: &Path) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| e.to_string())?;

    // 写入列头
    let header = result
        .columns
        .iter()
        .map(|c| escape_csv_field(c))
        .collect::<Vec<_>>()
        .join(",");
    writeln!(file, "{}", header).map_err(|e| e.to_string())?;

    // 写入数据行
    for row in &result.rows {
        let line = row
            .iter()
            .map(|cell| escape_csv_field(cell))
            .collect::<Vec<_>>()
            .join(",");
        writeln!(file, "{}", line).map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 导出查询结果到 SQL INSERT 语句文件
#[allow(dead_code)] // 公开 API，供外部使用
pub fn export_to_sql(result: &QueryResult, table_name: &str, path: &Path) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| e.to_string())?;

    let escaped_table_name = escape_sql_identifier(table_name);
    
    writeln!(file, "-- Exported from Rust DB Manager").map_err(|e| e.to_string())?;
    writeln!(file, "-- Table: {}", table_name).map_err(|e| e.to_string())?;
    writeln!(file, "-- Rows: {}\n", result.rows.len()).map_err(|e| e.to_string())?;

    if result.columns.is_empty() || result.rows.is_empty() {
        writeln!(file, "-- No data to export").map_err(|e| e.to_string())?;
        return Ok(());
    }

    let columns_str = result
        .columns
        .iter()
        .map(|c| format!("`{}`", escape_sql_identifier(c)))
        .collect::<Vec<_>>()
        .join(", ");

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
            escaped_table_name, columns_str, values
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// 导出查询结果到 JSON 文件
#[allow(dead_code)] // 公开 API，供外部使用
pub fn export_to_json(result: &QueryResult, path: &Path) -> Result<(), String> {
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

    let json = serde_json::to_string_pretty(&json_rows).map_err(|e| e.to_string())?;
    write!(file, "{}", json).map_err(|e| e.to_string())?;

    Ok(())
}

// ============================================================================
// CSV 导入
// ============================================================================

/// 预览 CSV 文件
pub fn preview_csv(path: &Path, config: &CsvImportConfig) -> Result<ImportPreview, String> {
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let reader = BufReader::new(file);
    
    let mut lines = reader.lines();
    let mut warnings = Vec::new();
    
    // 跳过指定行数
    for _ in 0..config.skip_rows {
        if lines.next().is_none() {
            return Err("文件行数不足".to_string());
        }
    }
    
    // 读取列名
    let columns = if config.has_header {
        let header_line = lines
            .next()
            .ok_or("文件为空")?
            .map_err(|e| format!("读取表头失败: {}", e))?;
        parse_csv_line(&header_line, config.delimiter, config.quote_char)
    } else if !config.column_names.is_empty() {
        config.column_names.clone()
    } else {
        // 自动生成列名
        let first_line = lines
            .next()
            .ok_or("文件为空")?
            .map_err(|e| format!("读取失败: {}", e))?;
        let field_count = parse_csv_line(&first_line, config.delimiter, config.quote_char).len();
        (0..field_count).map(|i| format!("column_{}", i + 1)).collect()
    };
    
    // 读取预览数据 (最多 100 行)
    let mut preview_rows = Vec::new();
    let mut total_rows = 0;
    
    for line_result in lines {
        let line = line_result.map_err(|e| format!("读取行失败: {}", e))?;
        if line.trim().is_empty() {
            continue;
        }
        
        total_rows += 1;
        
        if preview_rows.len() < 100 {
            let fields = parse_csv_line(&line, config.delimiter, config.quote_char);
            
            // 检查字段数是否匹配
            if fields.len() != columns.len() {
                warnings.push(format!(
                    "第 {} 行字段数 ({}) 与列数 ({}) 不匹配",
                    total_rows, fields.len(), columns.len()
                ));
            }
            
            preview_rows.push(fields);
        }
        
        // 限制扫描行数以提高性能
        if total_rows > 10000 {
            break;
        }
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
    let file = File::open(path).map_err(|e| format!("无法打开文件: {}", e))?;
    let reader = BufReader::new(file);
    
    let mut lines = reader.lines();
    let mut warnings = Vec::new();
    let mut sql_statements = Vec::new();
    let mut rows_imported = 0;
    let mut rows_skipped = 0;
    
    // 跳过指定行数
    for _ in 0..config.skip_rows {
        if lines.next().is_none() {
            return Err("文件行数不足".to_string());
        }
    }
    
    // 读取列名
    let columns = if config.has_header {
        let header_line = lines
            .next()
            .ok_or("文件为空")?
            .map_err(|e| format!("读取表头失败: {}", e))?;
        parse_csv_line(&header_line, config.delimiter, config.quote_char)
    } else if !config.column_names.is_empty() {
        config.column_names.clone()
    } else {
        return Err("未指定列名且文件无表头".to_string());
    };
    
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
    
    // 处理数据行
    for (idx, line_result) in lines.enumerate() {
        if config.max_rows > 0 && rows_imported >= config.max_rows {
            break;
        }
        
        let line = match line_result {
            Ok(l) => l,
            Err(e) => {
                warnings.push(format!("第 {} 行读取失败: {}", idx + 1, e));
                rows_skipped += 1;
                continue;
            }
        };
        
        if line.trim().is_empty() {
            continue;
        }
        
        let fields = parse_csv_line(&line, config.delimiter, config.quote_char);
        
        // 检查字段数
        if fields.len() != columns.len() {
            warnings.push(format!(
                "第 {} 行字段数不匹配，跳过",
                idx + 1
            ));
            rows_skipped += 1;
            continue;
        }
        
        // 生成值部分
        let values = fields
            .iter()
            .map(|field| sql_value_from_string(field))
            .collect::<Vec<_>>()
            .join(", ");
        
        sql_statements.push(format!(
            "INSERT INTO {} ({}) VALUES ({});",
            table_name, columns_str, values
        ));
        
        rows_imported += 1;
    }
    
    Ok(ImportResult {
        sql_statements,
        rows_imported,
        rows_skipped,
        warnings,
    })
}

/// 解析 CSV 行
/// 解析 CSV 行，处理引号转义
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
            fields.push(current_field.trim().to_string());
            current_field = String::new();
        } else {
            current_field.push(c);
        }
    }
    
    // 添加最后一个字段
    fields.push(current_field.trim().to_string());
    
    fields
}

// ============================================================================
// JSON 导入
// ============================================================================

/// 预览 JSON 文件
pub fn preview_json(path: &Path, config: &JsonImportConfig) -> Result<ImportPreview, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取文件: {}", e))?;
    
    let json_value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;
    
    // 提取数组数据
    let array = extract_json_array(&json_value, config.json_path.as_deref())?;
    
    if array.is_empty() {
        return Err("JSON 数组为空".to_string());
    }
    
    let mut warnings = Vec::new();
    
    // 从第一个对象提取列名
    let columns: Vec<String> = match &array[0] {
        serde_json::Value::Object(obj) => obj.keys().cloned().collect(),
        _ => {
            warnings.push("JSON 数组元素不是对象，使用索引作为列名".to_string());
            vec!["value".to_string()]
        }
    };
    
    // 读取预览数据
    let mut preview_rows = Vec::new();
    let total_rows = array.len();
    
    for (idx, item) in array.iter().enumerate() {
        if idx >= 100 {
            break;
        }
        
        let row = match item {
            serde_json::Value::Object(obj) => {
                columns.iter().map(|col| {
                    obj.get(col)
                        .map(json_value_to_string)
                        .unwrap_or_else(|| "NULL".to_string())
                }).collect()
            }
            other => vec![json_value_to_string(other)],
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
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("无法读取文件: {}", e))?;
    
    let json_value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 解析失败: {}", e))?;
    
    // 提取数组数据
    let array = extract_json_array(&json_value, config.json_path.as_deref())?;
    
    if array.is_empty() {
        return Ok(ImportResult {
            sql_statements: Vec::new(),
            rows_imported: 0,
            rows_skipped: 0,
            warnings: vec!["JSON 数组为空".to_string()],
        });
    }
    
    if config.table_name.is_empty() {
        return Err("未指定目标表名".to_string());
    }
    
    let mut warnings = Vec::new();
    let mut sql_statements = Vec::new();
    let mut rows_imported = 0;
    let mut rows_skipped = 0;
    
    // 从第一个对象提取列名
    let columns: Vec<String> = match &array[0] {
        serde_json::Value::Object(obj) => obj.keys().cloned().collect(),
        _ => vec!["value".to_string()],
    };
    
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
    
    for (idx, item) in array.iter().enumerate() {
        if config.max_rows > 0 && rows_imported >= config.max_rows {
            break;
        }
        
        let values = match item {
            serde_json::Value::Object(obj) => {
                columns.iter().map(|col| {
                    obj.get(col)
                        .map(json_value_to_sql)
                        .unwrap_or_else(|| "NULL".to_string())
                }).collect::<Vec<_>>().join(", ")
            }
            other => json_value_to_sql(other),
        };
        
        if values.is_empty() {
            warnings.push(format!("第 {} 项数据为空，跳过", idx + 1));
            rows_skipped += 1;
            continue;
        }
        
        sql_statements.push(format!(
            "INSERT INTO {} ({}) VALUES ({});",
            table_name, columns_str, values
        ));
        
        rows_imported += 1;
    }
    
    Ok(ImportResult {
        sql_statements,
        rows_imported,
        rows_skipped,
        warnings,
    })
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

