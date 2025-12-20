//! SQL 自动补全模块

#![allow(dead_code)] // 预留 API

use super::constants::autocomplete as consts;

/// SQL 关键字列表
const SQL_KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "AND",
    "OR",
    "NOT",
    "IN",
    "IS",
    "NULL",
    "LIKE",
    "BETWEEN",
    "EXISTS",
    "HAVING",
    "GROUP BY",
    "ORDER BY",
    "ASC",
    "DESC",
    "LIMIT",
    "OFFSET",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "CREATE",
    "TABLE",
    "INDEX",
    "VIEW",
    "DROP",
    "ALTER",
    "ADD",
    "COLUMN",
    "PRIMARY",
    "KEY",
    "FOREIGN",
    "REFERENCES",
    "UNIQUE",
    "CHECK",
    "DEFAULT",
    "CONSTRAINT",
    "JOIN",
    "INNER",
    "LEFT",
    "RIGHT",
    "OUTER",
    "CROSS",
    "ON",
    "UNION",
    "ALL",
    "DISTINCT",
    "AS",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "COUNT",
    "SUM",
    "AVG",
    "MIN",
    "MAX",
    "COALESCE",
    "NULLIF",
    "CAST",
    "CONVERT",
    "SUBSTRING",
    "TRIM",
    "UPPER",
    "LOWER",
    "LENGTH",
    "CONCAT",
    "REPLACE",
    "ROUND",
    "FLOOR",
    "CEIL",
    "NOW",
    "DATE",
    "TIME",
    "DATETIME",
    "TIMESTAMP",
    "TRUE",
    "FALSE",
    "BOOLEAN",
    "INTEGER",
    "INT",
    "BIGINT",
    "SMALLINT",
    "FLOAT",
    "DOUBLE",
    "DECIMAL",
    "NUMERIC",
    "VARCHAR",
    "CHAR",
    "TEXT",
    "BLOB",
    "BINARY",
    "VARBINARY",
    "IF",
    "IFNULL",
    "IIF",
    "NULLIF",
    "BEGIN",
    "COMMIT",
    "ROLLBACK",
    "TRANSACTION",
    "SAVEPOINT",
    "GRANT",
    "REVOKE",
    "PRIVILEGES",
    "EXPLAIN",
    "ANALYZE",
    "VACUUM",
    "PRAGMA",
];

/// SQL 函数列表
const SQL_FUNCTIONS: &[&str] = &[
    "ABS",
    "AVG",
    "CEIL",
    "CEILING",
    "COUNT",
    "FLOOR",
    "MAX",
    "MIN",
    "ROUND",
    "SUM",
    "CONCAT",
    "LENGTH",
    "LOWER",
    "UPPER",
    "TRIM",
    "LTRIM",
    "RTRIM",
    "REPLACE",
    "SUBSTRING",
    "SUBSTR",
    "LEFT",
    "RIGHT",
    "COALESCE",
    "IFNULL",
    "NULLIF",
    "CAST",
    "CONVERT",
    "NOW",
    "CURDATE",
    "CURTIME",
    "DATE",
    "TIME",
    "DATETIME",
    "YEAR",
    "MONTH",
    "DAY",
    "HOUR",
    "MINUTE",
    "SECOND",
    "DATE_ADD",
    "DATE_SUB",
    "DATEDIFF",
    "TIMESTAMPDIFF",
    "GROUP_CONCAT",
    "STRING_AGG",
    "ROW_NUMBER",
    "RANK",
    "DENSE_RANK",
    "NTILE",
    "FIRST_VALUE",
    "LAST_VALUE",
    "LAG",
    "LEAD",
    "JSON_EXTRACT",
    "JSON_OBJECT",
    "JSON_ARRAY",
];

/// 自动补全建议
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// 显示文本
    pub label: String,
    /// 插入文本
    pub insert_text: String,
    /// 类型（keyword, function, table, column）
    pub kind: CompletionKind,
    /// 详细说明
    pub detail: Option<String>,
}

/// 补全类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
    Table,
    Column,
}

impl CompletionKind {
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::Keyword => "K",
            CompletionKind::Function => "F",
            CompletionKind::Table => "T",
            CompletionKind::Column => "C",
        }
    }
}

/// 自动补全引擎
pub struct AutoComplete {
    /// 当前数据库的表列表
    tables: Vec<String>,
    /// 表的列信息 (table_name -> columns)
    columns: std::collections::HashMap<String, Vec<String>>,
}

impl Default for AutoComplete {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoComplete {
    pub fn new() -> Self {
        Self {
            tables: Vec::new(),
            columns: std::collections::HashMap::new(),
        }
    }

    /// 更新表列表（限制最大数量）
    pub fn set_tables(&mut self, tables: Vec<String>) {
        self.tables = if tables.len() > consts::MAX_CACHED_TABLES {
            tables.into_iter().take(consts::MAX_CACHED_TABLES).collect()
        } else {
            tables
        };
    }

    /// 添加表的列信息（限制最大数量）
    pub fn set_columns(&mut self, table: String, columns: Vec<String>) {
        let limited_columns = if columns.len() > consts::MAX_CACHED_COLUMNS_PER_TABLE {
            columns.into_iter().take(consts::MAX_CACHED_COLUMNS_PER_TABLE).collect()
        } else {
            columns
        };
        self.columns.insert(table, limited_columns);
    }

    /// 清空所有信息
    pub fn clear(&mut self) {
        self.tables.clear();
        self.columns.clear();
    }

    /// 获取补全建议
    pub fn get_completions(&self, text: &str, cursor_pos: usize) -> Vec<CompletionItem> {
        // 获取光标前的文本
        let text_before_cursor = if cursor_pos <= text.len() {
            &text[..cursor_pos]
        } else {
            text
        };

        // 找到当前正在输入的单词
        let current_word = self.get_current_word(text_before_cursor);

        if current_word.is_empty() {
            return Vec::new();
        }

        let prefix = current_word.to_uppercase();
        let mut completions = Vec::new();

        // 关键字补全
        for keyword in SQL_KEYWORDS {
            if keyword.starts_with(&prefix) {
                completions.push(CompletionItem {
                    label: keyword.to_string(),
                    insert_text: keyword.to_string(),
                    kind: CompletionKind::Keyword,
                    detail: Some("SQL 关键字".to_string()),
                });
            }
        }

        // 函数补全
        for func in SQL_FUNCTIONS {
            if func.starts_with(&prefix) {
                completions.push(CompletionItem {
                    label: format!("{}()", func),
                    insert_text: format!("{}(", func),
                    kind: CompletionKind::Function,
                    detail: Some("SQL 函数".to_string()),
                });
            }
        }

        // 表名补全
        for table in &self.tables {
            if table.to_uppercase().starts_with(&prefix) {
                completions.push(CompletionItem {
                    label: table.clone(),
                    insert_text: table.clone(),
                    kind: CompletionKind::Table,
                    detail: Some("数据表".to_string()),
                });
            }
        }

        // 列名补全 - 检查是否在 FROM 子句后
        let upper_text = text_before_cursor.to_uppercase();
        if upper_text.contains("FROM")
            || upper_text.contains("JOIN")
            || upper_text.contains("WHERE")
            || upper_text.contains("SELECT")
        {
            for (table, cols) in &self.columns {
                for col in cols {
                    if col.to_uppercase().starts_with(&prefix) {
                        completions.push(CompletionItem {
                            label: col.clone(),
                            insert_text: col.clone(),
                            kind: CompletionKind::Column,
                            detail: Some(format!("列 ({})", table)),
                        });
                    }
                }
            }
        }

        // 按类型和名称排序
        completions.sort_by(|a, b| {
            let kind_order = |k: &CompletionKind| match k {
                CompletionKind::Column => 0,
                CompletionKind::Table => 1,
                CompletionKind::Function => 2,
                CompletionKind::Keyword => 3,
            };
            kind_order(&a.kind)
                .cmp(&kind_order(&b.kind))
                .then_with(|| a.label.cmp(&b.label))
        });

        // 限制返回数量
        completions.truncate(consts::MAX_COMPLETIONS);
        completions
    }

    /// 获取当前正在输入的单词
    fn get_current_word(&self, text: &str) -> String {
        let mut word = String::new();
        for c in text.chars().rev() {
            if c.is_alphanumeric() || c == '_' {
                word.insert(0, c);
            } else {
                break;
            }
        }
        word
    }
}

