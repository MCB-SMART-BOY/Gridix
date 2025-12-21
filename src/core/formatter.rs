//! SQL 格式化模块

/// SQL 格式化 - 美化 SQL 语句
pub fn format_sql(sql: &str) -> String {
    let mut result = String::new();
    let mut indent_level: usize = 0;
    let mut in_string = false;
    let mut string_char = ' ';
    let mut last_was_keyword = false;

    // 主要关键字 - 需要新行
    let major_keywords = [
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

    // 规范化空白字符
    let normalized: String = sql.split_whitespace().collect::<Vec<_>>().join(" ");

    let upper = normalized.to_uppercase();
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

        // 处理字符串
        if !in_string && (c == '\'' || c == '"') {
            in_string = true;
            string_char = c;
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

        // 检查主要关键字
        let remaining = &upper[i..];
        let mut found_keyword = false;

        for keyword in &major_keywords {
            if remaining.starts_with(keyword) {
                // 确保是完整的关键字（后面是空格或结束）
                let kw_len = keyword.len();
                if i + kw_len <= chars.len() {
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

                        // 添加关键字（保持原始大小写但转为大写）
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

