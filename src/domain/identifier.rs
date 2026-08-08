//! 数据库标识符引用
//!
//! 按 dialect 正确转义标识符（表名、列名），防止注入。
//! 不再拒绝合法标识符（如 `order-items`、`select`）——引用机制本身就保证了安全性。

/// 数据库标识符（已按 dialect 转义）
#[derive(Debug, Clone)]
pub struct Identifier(pub String);

impl Identifier {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// 标识符引用方言
#[derive(Debug, Clone, Copy)]
pub enum IdentifierDialect {
    /// PostgreSQL / SQLite：双引号 `"…"`，内部 `"` 转义为 `""`
    PostgreSql,
    /// MySQL：反引号 `` `…` ``，内部 `` ` `` 转义为 ``` `` ```
    MySql,
    /// SQLite（与 PG 相同，双引号）
    SQLite,
}

impl IdentifierDialect {
    /// 安全引用一个标识符
    pub fn quote(&self, raw: &str) -> String {
        match self {
            Self::PostgreSql | Self::SQLite => {
                format!("\"{}\"", raw.replace('"', "\"\""))
            }
            Self::MySql => {
                format!("`{}`", raw.replace('`', "``"))
            }
        }
    }

    /// 从 DatabaseType 推断方言
    pub fn from_db_type(db_type: crate::types::DatabaseType) -> Self {
        match db_type {
            crate::types::DatabaseType::SQLite => Self::SQLite,
            crate::types::DatabaseType::PostgreSQL => Self::PostgreSql,
            crate::types::DatabaseType::MySQL => Self::MySql,
        }
    }
}
