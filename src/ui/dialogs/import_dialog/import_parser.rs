//! 导入文件解析器

use super::import_types::{ImportPreview, SqlImportConfig};

/// 解析 SQL 文件，处理注释并分割语句
pub fn parse_sql_file(content: &str, config: &SqlImportConfig) -> ImportPreview {
    let mut statements = Vec::new();
    let mut warnings = Vec::new();
    let mut current_statement = String::new();
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_char = '"';

    let lines: Vec<&str> = content.lines().collect();

    for line in lines.iter() {
        let mut processed_line = String::new();
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // 处理块注释
            if in_block_comment {
                if i + 1 < chars.len() && chars[i] == '*' && chars[i + 1] == '/' {
                    in_block_comment = false;
                    i += 2;
                    continue;
                }
                i += 1;
                continue;
            }

            // 处理字符串
            if in_string {
                processed_line.push(chars[i]);
                if chars[i] == string_char {
                    // 检查是否是转义
                    if i + 1 < chars.len() && chars[i + 1] == string_char {
                        processed_line.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    in_string = false;
                }
                i += 1;
                continue;
            }

            // 检测块注释开始
            if config.strip_comments
                && i + 1 < chars.len()
                && chars[i] == '/'
                && chars[i + 1] == '*'
            {
                in_block_comment = true;
                i += 2;
                continue;
            }

            // 检测行注释
            if config.strip_comments
                && i + 1 < chars.len()
                && chars[i] == '-'
                && chars[i + 1] == '-'
            {
                // 跳过行的剩余部分
                break;
            }

            // 检测 # 注释（MySQL 风格）
            if config.strip_comments && chars[i] == '#' {
                break;
            }

            // 检测字符串开始
            if chars[i] == '\'' || chars[i] == '"' {
                in_string = true;
                string_char = chars[i];
                processed_line.push(chars[i]);
                i += 1;
                continue;
            }

            // 检测语句结束
            if chars[i] == ';' {
                processed_line.push(';');
                current_statement.push_str(&processed_line);

                let stmt = current_statement.trim().to_string();
                if !stmt.is_empty() && stmt != ";" {
                    statements.push(stmt);
                }
                current_statement.clear();
                processed_line.clear();
                i += 1;
                continue;
            }

            processed_line.push(chars[i]);
            i += 1;
        }

        // 添加剩余的处理过的行
        let trimmed = processed_line.trim();
        if !config.strip_empty_lines || !trimmed.is_empty() {
            if !current_statement.is_empty() && !current_statement.ends_with('\n') {
                current_statement.push('\n');
            }
            current_statement.push_str(trimmed);
        }
    }

    // 处理最后一条语句（可能没有分号）
    let final_stmt = current_statement.trim().to_string();
    if !final_stmt.is_empty() {
        statements.push(final_stmt);
        warnings.push("最后一条语句没有分号".to_string());
    }

    // 过滤空语句
    statements.retain(|s| !s.trim().is_empty());

    ImportPreview {
        columns: vec!["SQL 语句".to_string()],
        preview_rows: statements.iter().take(10).map(|s| vec![s.clone()]).collect(),
        total_rows: statements.len(),
        statement_count: statements.len(),
        warnings,
        sql_statements: statements,
    }
}
