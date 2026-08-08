//! SQL 格式化模块

/// 规范化 SQL 中的空白字符，保护字符串字面量内容不变。
///
/// 将引号外的连续空白压缩为单个空格，引号内的内容（包括空白）原样保留。
/// 跟踪单引号字符串 `'…'`、双引号标识符 `"…"` 和反引号标识符 `` `…` ``。
fn normalize_whitespace_preserving_quotes(sql: &str) -> String {
    let mut result = String::with_capacity(sql.len());
    let mut in_single = false;
    let mut in_double = false;
    let mut in_backtick = false;
    let mut prev_was_space = false;
    let chars: Vec<char> = sql.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];

        // 进入引号状态
        if !in_single && !in_double && !in_backtick {
            match c {
                '\'' => {
                    in_single = true;
                    result.push(c);
                    prev_was_space = false;
                    i += 1;
                    continue;
                }
                '"' => {
                    in_double = true;
                    result.push(c);
                    prev_was_space = false;
                    i += 1;
                    continue;
                }
                '`' => {
                    in_backtick = true;
                    result.push(c);
                    prev_was_space = false;
                    i += 1;
                    continue;
                }
                _ => {}
            }
        }

        // 单引号字符串内：原样保留
        if in_single {
            result.push(c);
            if c == '\'' {
                // 检查转义的单引号 ''
                if i + 1 < chars.len() && chars[i + 1] == '\'' {
                    result.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                in_single = false;
            }
            i += 1;
            continue;
        }

        // 双引号标识符内：原样保留
        if in_double {
            result.push(c);
            if c == '"' {
                in_double = false;
            }
            i += 1;
            continue;
        }

        // 反引号标识符内：原样保留
        if in_backtick {
            result.push(c);
            if c == '`' {
                in_backtick = false;
            }
            i += 1;
            continue;
        }

        // 引号外：压缩空白
        if c.is_whitespace() {
            if !prev_was_space {
                result.push(' ');
                prev_was_space = true;
            }
        } else {
            result.push(c);
            prev_was_space = false;
        }
        i += 1;
    }

    result
}
/// SQL 格式化 - 美化 SQL 语句
pub fn format_sql(sql: &str) -> String {
    let mut result = String::new();
    let mut indent_level: usize = 0;
    let mut in_string = false;
    let mut in_backtick = false;
    let mut string_char = ' ';
    let mut last_was_keyword = false;

    // 主要关键字 - 需要新行
    let major_keywords = [
        "WITH",
        "RECURSIVE",
        "SELECT",
        "FROM",
        "WHERE",
        "AND",
        "OR",
        "ORDER BY",
        "GROUP BY",
        "HAVING",
        "LIMIT",
        "OFFSET",
        "JOIN",
        "LEFT JOIN",
        "RIGHT JOIN",
        "INNER JOIN",
        "OUTER JOIN",
        "CROSS JOIN",
        "ON",
        "SET",
        "VALUES",
        "INSERT INTO",
        "UPDATE",
        "DELETE FROM",
        "CREATE TABLE",
        "ALTER TABLE",
        "DROP TABLE",
        "CREATE INDEX",
        "DROP INDEX",
        "UNION",
        "UNION ALL",
        "EXCEPT",
        "INTERSECT",
        "CASE",
        "WHEN",
        "THEN",
        "ELSE",
        "END",
    ];

    // 规范化空白字符（保护字符串字面量内部的空白）
    let normalized: String = normalize_whitespace_preserving_quotes(sql);

    let chars: Vec<char> = normalized.chars().collect();
    let mut i = 0;

    // 安全计数器，防止无限循环（最多处理字符数的2倍迭代）
    let max_iterations = chars.len() * 2 + 1;
    let mut iterations = 0;

    while i < chars.len() {
        iterations += 1;
        if iterations > max_iterations {
            tracing::warn!("SQL 格式化器达到最大迭代次数，返回原始 SQL");
            return sql.to_string();
        }
        let c = chars[i];

        // 处理字符串和反引号标识符
        if !in_string && !in_backtick && (c == '\'' || c == '"' || c == '`') {
            if c == '`' {
                in_backtick = true;
            } else {
                in_string = true;
                string_char = c;
            }
            result.push(c);
            i += 1;
            continue;
        }

        if in_string {
            result.push(c);
            if c == string_char {
                // 检查转义
                if i + 1 < chars.len() && chars[i + 1] == string_char {
                    result.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                in_string = false;
            }
            i += 1;
            continue;
        }

        if in_backtick {
            result.push(c);
            if c == '`' {
                in_backtick = false;
            }
            i += 1;
            continue;
        }

        // 检查括号
        if c == '(' {
            result.push(c);
            indent_level += 1;
            i += 1;
            continue;
        }

        if c == ')' {
            indent_level = indent_level.saturating_sub(1);
            result.push(c);
            i += 1;
            continue;
        }

        // 检查逗号 - 在 SELECT 子句中换行
        if c == ',' {
            result.push(c);
            if last_was_keyword {
                result.push('\n');
                result.push_str(&"    ".repeat(indent_level + 1));
            }
            i += 1;
            continue;
        }

        // 检查主要关键字 — 使用字符数组避免字节索引问题
        let mut found_keyword = false;

        for keyword in &major_keywords {
            let kw_chars: Vec<char> = keyword.chars().collect();
            let kw_len = kw_chars.len();

            // Compare case-insensitively using chars (avoid byte-index panic)
            if i + kw_len <= chars.len() {
                let slice = &chars[i..i + kw_len];
                let matches_kw = slice
                    .iter()
                    .zip(kw_chars.iter())
                    .all(|(a, b)| a.eq_ignore_ascii_case(b));

                if matches_kw {
                    // 确保是完整的关键字（后面是空格或结束，不是字母数字）
                    let next_char = if i + kw_len < chars.len() {
                        Some(chars[i + kw_len])
                    } else {
                        None
                    };

                    if !next_char.is_some_and(|c| c.is_alphanumeric()) {
                        // 添加换行和缩进
                        if !result.is_empty() && !result.ends_with('\n') {
                            result.push('\n');
                        }
                        result.push_str(&"    ".repeat(indent_level));

                        // 添加关键字（大写形式）
                        result.push_str(keyword);

                        // 特定关键字后设置标记
                        last_was_keyword = *keyword == "SELECT";

                        i += kw_len;
                        found_keyword = true;
                        break;
                    }
                }
            }
        }

        if found_keyword {
            continue;
        }

        // 普通字符
        result.push(c);
        i += 1;
    }

    // 清理多余的空行
    let lines: Vec<&str> = result.lines().collect();
    let cleaned: Vec<String> = lines
        .iter()
        .map(|line| line.trim_end().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    cleaned.join("\n")
}
