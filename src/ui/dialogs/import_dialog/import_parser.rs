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
    let mut dollar_quote_tag: Option<String> = None;
    let mut delimiter = ";".to_string();
    let mut delimiter_chars: Vec<char> = delimiter.chars().collect();

    for line in content.lines() {
        // MySQL DELIMITER 指令仅在非字符串、非块注释状态下识别
        if !in_block_comment
            && !in_string
            && dollar_quote_tag.is_none()
            && let Some(new_delimiter) = parse_delimiter_command(line)
        {
            if new_delimiter.is_empty() {
                warnings.push("检测到 DELIMITER 语句但未提供分隔符，已保持原配置".to_string());
            } else {
                delimiter = new_delimiter.to_string();
                delimiter_chars = delimiter.chars().collect();
            }
            continue;
        }

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

            // 处理 PostgreSQL 风格 dollar-quoted 字符串 ($$...$$ / $tag$...$tag$)
            if let Some(tag) = &dollar_quote_tag {
                if matches_token(&chars, i, tag) {
                    for c in tag.chars() {
                        processed_line.push(c);
                    }
                    i += tag.chars().count();
                    dollar_quote_tag = None;
                } else {
                    processed_line.push(chars[i]);
                    i += 1;
                }
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

            // 检测 dollar-quoted 字符串开始
            if chars[i] == '$'
                && let Some(tag) = parse_dollar_quote_tag(&chars, i)
            {
                for c in tag.chars() {
                    processed_line.push(c);
                }
                i += tag.chars().count();
                dollar_quote_tag = Some(tag);
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
            if matches_delimiter(&chars, i, &delimiter_chars) {
                let trimmed = processed_line.trim();
                if !trimmed.is_empty() {
                    if !current_statement.is_empty() && !current_statement.ends_with('\n') {
                        current_statement.push('\n');
                    }
                    current_statement.push_str(trimmed);
                }

                let stmt = current_statement.trim().to_string();
                if !stmt.is_empty() {
                    statements.push(stmt);
                }
                current_statement.clear();
                processed_line.clear();
                i += delimiter_chars.len();
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
        warnings.push(format!("最后一条语句没有结束分隔符 '{}'", delimiter));
    }

    // 过滤空语句
    statements.retain(|s| !s.trim().is_empty());

    ImportPreview {
        columns: vec!["SQL 语句".to_string()],
        preview_rows: statements
            .iter()
            .take(10)
            .map(|s| vec![s.clone()])
            .collect(),
        total_rows: statements.len(),
        statement_count: statements.len(),
        warnings,
        sql_statements: statements,
    }
}

fn parse_delimiter_command(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let mut parts = trimmed.split_whitespace();
    let cmd = parts.next()?;
    if !cmd.eq_ignore_ascii_case("delimiter") {
        return None;
    }

    parts.next()
}

fn matches_delimiter(chars: &[char], idx: usize, delimiter_chars: &[char]) -> bool {
    if delimiter_chars.is_empty() || idx + delimiter_chars.len() > chars.len() {
        return false;
    }

    chars[idx..idx + delimiter_chars.len()] == *delimiter_chars
}

fn parse_dollar_quote_tag(chars: &[char], idx: usize) -> Option<String> {
    if chars.get(idx) != Some(&'$') {
        return None;
    }

    let mut j = idx + 1;
    if j >= chars.len() {
        return None;
    }

    if chars[j] == '$' {
        return Some("$$".to_string());
    }

    if !(chars[j].is_ascii_alphabetic() || chars[j] == '_') {
        return None;
    }
    j += 1;
    while j < chars.len() && (chars[j].is_ascii_alphanumeric() || chars[j] == '_') {
        j += 1;
    }

    if j < chars.len() && chars[j] == '$' {
        Some(chars[idx..=j].iter().collect())
    } else {
        None
    }
}

fn matches_token(chars: &[char], idx: usize, token: &str) -> bool {
    let token_len = token.chars().count();
    if idx + token_len > chars.len() {
        return false;
    }

    for (offset, ch) in token.chars().enumerate() {
        if chars[idx + offset] != ch {
            return false;
        }
    }
    true
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
