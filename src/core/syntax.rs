//! SQL 语法高亮模块
//!
//! 使用 syntect 库提供专业级的 SQL 语法高亮，支持多种主题。

use super::theme::ThemeColors;
use egui::{text::LayoutJob, Color32, FontFamily, FontId, TextFormat};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;

// ============================================================================
// 全局语法高亮资源（延迟初始化）
// ============================================================================

/// 全局语法集（包含 SQL 语法定义）
static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(|| {
    SyntaxSet::load_defaults_newlines()
});

/// 全局主题集
static THEME_SET: Lazy<ThemeSet> = Lazy::new(|| {
    ThemeSet::load_defaults()
});

/// 高亮缓存（避免重复计算）
static HIGHLIGHT_CACHE: Lazy<RwLock<HighlightCache>> = Lazy::new(|| {
    RwLock::new(HighlightCache::new(1000))
});

// ============================================================================
// 高亮缓存
// ============================================================================

struct HighlightCache {
    cache: HashMap<(String, String), LayoutJob>,
    max_size: usize,
    access_order: Vec<(String, String)>,
}

impl HighlightCache {
    fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            access_order: Vec::new(),
        }
    }

    fn get(&mut self, key: &(String, String)) -> Option<LayoutJob> {
        if let Some(job) = self.cache.get(key) {
            // 更新访问顺序
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                self.access_order.remove(pos);
            }
            self.access_order.push(key.clone());
            Some(job.clone())
        } else {
            None
        }
    }

    fn insert(&mut self, key: (String, String), job: LayoutJob) {
        // 如果缓存已满，删除最久未访问的条目
        while self.cache.len() >= self.max_size && !self.access_order.is_empty() {
            let oldest = self.access_order.remove(0);
            self.cache.remove(&oldest);
        }
        
        self.cache.insert(key.clone(), job);
        self.access_order.push(key);
    }

    fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }
}

// ============================================================================
// 高亮颜色配置
// ============================================================================

/// 语法高亮颜色配置
#[derive(Clone)]
pub struct HighlightColors {
    pub keyword: Color32,     // SQL 关键字
    pub function: Color32,    // 函数
    pub string: Color32,      // 字符串
    pub number: Color32,      // 数字
    pub operator: Color32,    // 操作符
    pub comment: Color32,     // 注释
    pub identifier: Color32,  // 标识符/列名
    pub punctuation: Color32, // 标点符号
    pub default: Color32,     // 默认文本
    pub theme_name: String,   // 当前主题名称（用于缓存键）
}

impl HighlightColors {
    /// 从主题颜色创建高亮配置
    pub fn from_theme(theme: &ThemeColors) -> Self {
        Self {
            keyword: theme.accent,
            function: theme.info,
            string: theme.success,
            number: theme.warning,
            operator: theme.fg_secondary,
            comment: theme.fg_muted,
            identifier: theme.fg_primary,
            punctuation: theme.fg_secondary,
            default: theme.fg_primary,
            theme_name: format!("{:?}", theme.accent), // 使用 accent 颜色作为主题标识
        }
    }
}

impl Default for HighlightColors {
    fn default() -> Self {
        // Tokyo Night 风格默认颜色
        Self {
            keyword: Color32::from_rgb(122, 162, 247),     // 蓝色
            function: Color32::from_rgb(125, 207, 255),    // 青色
            string: Color32::from_rgb(158, 206, 106),      // 绿色
            number: Color32::from_rgb(255, 158, 100),      // 橙色
            operator: Color32::from_rgb(169, 177, 214),    // 灰蓝
            comment: Color32::from_rgb(86, 95, 137),       // 暗灰
            identifier: Color32::from_rgb(192, 202, 245),  // 浅蓝
            punctuation: Color32::from_rgb(169, 177, 214), // 灰蓝
            default: Color32::from_rgb(192, 202, 245),     // 浅蓝
            theme_name: "tokyo-night".to_string(),
        }
    }
}

// ============================================================================
// SQL 关键字和函数列表（用于自定义高亮回退）
// ============================================================================

/// SQL 关键字列表
const SQL_KEYWORDS: &[&str] = &[
    // DML
    "SELECT", "FROM", "WHERE", "AND", "OR", "NOT", "IN", "LIKE", "BETWEEN",
    "IS", "NULL", "AS", "ON", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER",
    "FULL", "CROSS", "NATURAL", "USING", "ORDER", "BY", "ASC", "DESC",
    "LIMIT", "OFFSET", "GROUP", "HAVING", "DISTINCT", "ALL", "UNION",
    "INTERSECT", "EXCEPT", "INSERT", "INTO", "VALUES", "UPDATE", "SET",
    "DELETE", "TRUNCATE", "MERGE",
    // DDL
    "CREATE", "ALTER", "DROP", "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA",
    "CONSTRAINT", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "UNIQUE", "CHECK",
    "DEFAULT", "AUTO_INCREMENT", "AUTOINCREMENT", "IF", "EXISTS", "CASCADE",
    "RESTRICT", "TEMPORARY", "TEMP",
    // 数据类型
    "INT", "INTEGER", "BIGINT", "SMALLINT", "TINYINT", "FLOAT", "DOUBLE",
    "DECIMAL", "NUMERIC", "REAL", "BOOLEAN", "BOOL", "CHAR", "VARCHAR",
    "TEXT", "BLOB", "DATE", "TIME", "DATETIME", "TIMESTAMP", "YEAR", "JSON",
    "UUID", "SERIAL", "BYTEA",
    // 其他
    "CASE", "WHEN", "THEN", "ELSE", "END", "CAST", "CONVERT", "COALESCE",
    "NULLIF", "WITH", "RECURSIVE", "RETURNING", "EXPLAIN", "ANALYZE",
    "BEGIN", "COMMIT", "ROLLBACK", "TRANSACTION", "SAVEPOINT", "GRANT",
    "REVOKE", "DENY", "TO", "ROLE", "USER", "TRUE", "FALSE",
];

/// SQL 内置函数
const SQL_FUNCTIONS: &[&str] = &[
    // 聚合函数
    "COUNT", "SUM", "AVG", "MIN", "MAX", "TOTAL", "GROUP_CONCAT",
    // 字符串函数
    "CONCAT", "SUBSTRING", "SUBSTR", "LENGTH", "UPPER", "LOWER", "TRIM",
    "LTRIM", "RTRIM", "REPLACE", "REVERSE", "LPAD", "RPAD", "INSTR",
    "LOCATE", "POSITION", "CHAR_LENGTH", "CHARACTER_LENGTH", "OCTET_LENGTH",
    "BIT_LENGTH", "ASCII", "FORMAT", "QUOTE", "SOUNDEX", "SPACE", "REPEAT",
    // 数学函数
    "ABS", "CEIL", "CEILING", "FLOOR", "ROUND", "TRUNCATE", "MOD", "POWER",
    "POW", "SQRT", "EXP", "LOG", "LOG10", "LOG2", "LN", "SIGN", "RAND",
    "RANDOM", "PI", "SIN", "COS", "TAN", "ASIN", "ACOS", "ATAN", "ATAN2",
    "COT", "DEGREES", "RADIANS",
    // 日期时间函数
    "NOW", "CURRENT_DATE", "CURRENT_TIME", "CURRENT_TIMESTAMP", "YEAR",
    "MONTH", "DAY", "HOUR", "MINUTE", "SECOND", "DAYOFWEEK", "DAYOFMONTH",
    "DAYOFYEAR", "WEEK", "WEEKDAY", "QUARTER", "EXTRACT", "DATE_ADD",
    "DATE_SUB", "DATEDIFF", "TIMEDIFF", "TIMESTAMPDIFF", "DATE_FORMAT",
    "TIME_FORMAT", "STR_TO_DATE", "MAKEDATE", "MAKETIME", "LAST_DAY",
    "ADDDATE", "SUBDATE", "ADDTIME", "SUBTIME", "PERIOD_ADD", "PERIOD_DIFF",
    // 条件函数
    "IF", "IFNULL", "COALESCE", "GREATEST", "LEAST", "IIF", "NVL", "NVL2",
    "DECODE",
    // 类型转换
    "TYPEOF",
    // 窗口函数
    "ROW_NUMBER", "RANK", "DENSE_RANK", "NTILE", "LAG", "LEAD",
    "FIRST_VALUE", "LAST_VALUE", "NTH_VALUE", "OVER", "PARTITION",
    "ROWID", "OID", "CTID",
];

// ============================================================================
// Token 类型
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
enum TokenType {
    Keyword,
    Function,
    String,
    Number,
    Operator,
    Comment,
    Identifier,
    Punctuation,
    Whitespace,
}

struct Token {
    text: String,
    token_type: TokenType,
}

// ============================================================================
// SQL 高亮器
// ============================================================================

/// SQL 语法高亮器
pub struct SqlHighlighter {
    pub colors: HighlightColors,
    use_syntect: bool,
}

impl SqlHighlighter {
    pub fn new(colors: HighlightColors) -> Self {
        // 检查 syntect 是否可用
        let use_syntect = SYNTAX_SET.find_syntax_by_extension("sql").is_some();
        
        Self { colors, use_syntect }
    }

    /// 创建带语法高亮的 LayoutJob
    pub fn highlight(&self, text: &str) -> LayoutJob {
        // 尝试使用缓存
        let cache_key = (text.to_string(), self.colors.theme_name.clone());
        
        {
            let mut cache = HIGHLIGHT_CACHE.write();
            if let Some(job) = cache.get(&cache_key) {
                return job;
            }
        }

        // 计算高亮
        let job = if self.use_syntect {
            self.highlight_with_syntect(text)
        } else {
            self.highlight_fallback(text)
        };

        // 存入缓存
        {
            let mut cache = HIGHLIGHT_CACHE.write();
            cache.insert(cache_key, job.clone());
        }

        job
    }

    /// 使用 syntect 进行高亮
    fn highlight_with_syntect(&self, text: &str) -> LayoutJob {
        use syntect::easy::HighlightLines;
        
        let syntax = SYNTAX_SET
            .find_syntax_by_extension("sql")
            .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
        
        // 选择合适的主题
        let theme_name = if self.is_dark_theme() {
            "base16-ocean.dark"
        } else {
            "base16-ocean.light"
        };
        
        let theme = THEME_SET
            .themes
            .get(theme_name)
            .or_else(|| THEME_SET.themes.values().next())
            .expect("No themes available");
        
        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut job = LayoutJob::default();
        let font_id = FontId::new(14.0, FontFamily::Monospace);

        for line in text.lines() {
            match highlighter.highlight_line(line, &SYNTAX_SET) {
                Ok(ranges) => {
                    for (style, text_part) in ranges {
                        let color = self.syntect_style_to_color(&style);
                        job.append(
                            text_part,
                            0.0,
                            TextFormat {
                                font_id: font_id.clone(),
                                color,
                                ..Default::default()
                            },
                        );
                    }
                }
                Err(_) => {
                    // 高亮失败，使用默认颜色
                    job.append(
                        line,
                        0.0,
                        TextFormat {
                            font_id: font_id.clone(),
                            color: self.colors.default,
                            ..Default::default()
                        },
                    );
                }
            }
            // 添加换行符
            job.append(
                "\n",
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color: self.colors.default,
                    ..Default::default()
                },
            );
        }

        // 移除最后多余的换行符
        if text.ends_with('\n') || job.text.ends_with("\n\n") {
            // 保持原样
        } else if job.text.ends_with('\n') && !text.ends_with('\n') {
            job.text.pop();
            if let Some(section) = job.sections.last_mut() {
                if section.byte_range.end > job.text.len() {
                    section.byte_range.end = job.text.len();
                }
            }
        }

        job
    }

    /// 将 syntect 样式转换为 egui 颜色
    fn syntect_style_to_color(&self, style: &Style) -> Color32 {
        Color32::from_rgba_unmultiplied(
            style.foreground.r,
            style.foreground.g,
            style.foreground.b,
            style.foreground.a,
        )
    }

    /// 判断是否是深色主题
    fn is_dark_theme(&self) -> bool {
        // 通过背景色亮度判断
        let r = self.colors.default.r() as u32;
        let g = self.colors.default.g() as u32;
        let b = self.colors.default.b() as u32;
        let luminance = (r * 299 + g * 587 + b * 114) / 1000;
        luminance > 128
    }

    /// 使用自定义 tokenizer 的回退高亮方式
    fn highlight_fallback(&self, text: &str) -> LayoutJob {
        let mut job = LayoutJob::default();
        let tokens = self.tokenize(text);
        let font_id = FontId::new(14.0, FontFamily::Monospace);

        for token in tokens {
            let color = self.get_token_color(&token.token_type);
            job.append(
                &token.text,
                0.0,
                TextFormat {
                    font_id: font_id.clone(),
                    color,
                    ..Default::default()
                },
            );
        }

        job
    }

    fn get_token_color(&self, token_type: &TokenType) -> Color32 {
        match token_type {
            TokenType::Keyword => self.colors.keyword,
            TokenType::Function => self.colors.function,
            TokenType::String => self.colors.string,
            TokenType::Number => self.colors.number,
            TokenType::Operator => self.colors.operator,
            TokenType::Comment => self.colors.comment,
            TokenType::Identifier => self.colors.identifier,
            TokenType::Punctuation => self.colors.punctuation,
            TokenType::Whitespace => self.colors.default,
        }
    }

    fn tokenize(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // 空白字符
            if c.is_whitespace() {
                let start = i;
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Whitespace,
                });
                continue;
            }

            // 单行注释 --
            if c == '-' && i + 1 < chars.len() && chars[i + 1] == '-' {
                let start = i;
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Comment,
                });
                continue;
            }

            // 多行注释 /* */
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                let start = i;
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                if i + 1 < chars.len() {
                    i += 2;
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Comment,
                });
                continue;
            }

            // 字符串 (单引号)
            if c == '\'' {
                let start = i;
                i += 1;
                while i < chars.len() {
                    if chars[i] == '\'' {
                        if i + 1 < chars.len() && chars[i + 1] == '\'' {
                            // 转义的单引号
                            i += 2;
                        } else {
                            i += 1;
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::String,
                });
                continue;
            }

            // 字符串 (双引号 - 用于标识符)
            if c == '"' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Identifier,
                });
                continue;
            }

            // 反引号标识符 (MySQL)
            if c == '`' {
                let start = i;
                i += 1;
                while i < chars.len() && chars[i] != '`' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1;
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Identifier,
                });
                continue;
            }

            // 数字
            if c.is_ascii_digit()
                || (c == '.' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
            {
                let start = i;
                let mut has_dot = c == '.';
                i += 1;
                while i < chars.len() {
                    let ch = chars[i];
                    if ch.is_ascii_digit() {
                        i += 1;
                    } else if ch == '.' && !has_dot {
                        has_dot = true;
                        i += 1;
                    } else if ch == 'e' || ch == 'E' {
                        i += 1;
                        if i < chars.len() && (chars[i] == '+' || chars[i] == '-') {
                            i += 1;
                        }
                    } else {
                        break;
                    }
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Number,
                });
                continue;
            }

            // 标识符或关键字
            if c.is_alphabetic() || c == '_' {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                let upper = word.to_uppercase();

                let token_type = if SQL_KEYWORDS.contains(&upper.as_str()) {
                    TokenType::Keyword
                } else if SQL_FUNCTIONS.contains(&upper.as_str()) {
                    TokenType::Function
                } else {
                    TokenType::Identifier
                };

                tokens.push(Token {
                    text: word,
                    token_type,
                });
                continue;
            }

            // 操作符
            if "+-*/%=<>!&|^~".contains(c) {
                let start = i;
                i += 1;
                // 处理多字符操作符
                if i < chars.len() {
                    let next = chars[i];
                    if (c == '<' && (next == '=' || next == '>' || next == '<'))
                        || (c == '>' && (next == '=' || next == '>'))
                        || (c == '!' && next == '=')
                        || (c == '|' && next == '|')
                        || (c == '&' && next == '&')
                        || (c == ':' && next == ':')
                    {
                        i += 1;
                    }
                }
                tokens.push(Token {
                    text: chars[start..i].iter().collect(),
                    token_type: TokenType::Operator,
                });
                continue;
            }

            // 标点符号
            if "(),;.:[]{}".contains(c) {
                tokens.push(Token {
                    text: c.to_string(),
                    token_type: TokenType::Punctuation,
                });
                i += 1;
                continue;
            }

            // 其他字符
            tokens.push(Token {
                text: c.to_string(),
                token_type: TokenType::Identifier,
            });
            i += 1;
        }

        tokens
    }
}

/// 用于 egui TextEdit 的语法高亮功能
pub fn highlight_sql(text: &str, colors: &HighlightColors) -> LayoutJob {
    let highlighter = SqlHighlighter::new(colors.clone());
    highlighter.highlight(text)
}

/// 清除高亮缓存（在主题切换时调用）
pub fn clear_highlight_cache() {
    let mut cache = HIGHLIGHT_CACHE.write();
    cache.clear();
}

// ============================================================================
// 测试
// ============================================================================

