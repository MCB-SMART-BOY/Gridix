//! 数据库类型定义

use serde::{Deserialize, Serialize};

// ============================================================================
// 数据库类型
// ============================================================================

/// 支持的数据库类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub enum DatabaseType {
    #[default]
    SQLite,
    PostgreSQL,
    MySQL,
}

impl DatabaseType {
    /// 获取显示名称
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::SQLite => "SQLite",
            Self::PostgreSQL => "PostgreSQL",
            Self::MySQL => "MySQL",
        }
    }

    /// 获取所有数据库类型
    pub const fn all() -> &'static [DatabaseType] {
        &[Self::SQLite, Self::PostgreSQL, Self::MySQL]
    }

    /// 获取默认端口
    pub const fn default_port(&self) -> u16 {
        match self {
            Self::SQLite => 0,
            Self::PostgreSQL => 5432,
            Self::MySQL => 3306,
        }
    }

    /// 是否需要网络连接
    #[allow(dead_code)] // 公开 API，供外部使用
    pub const fn requires_network(&self) -> bool {
        !matches!(self, Self::SQLite)
    }
}

// ============================================================================
// PostgreSQL SSL 模式
// ============================================================================

/// PostgreSQL SSL 模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum PostgresSslMode {
    /// 禁用 SSL（默认）
    #[default]
    Disable,
    /// 优先使用 SSL，但允许不安全连接
    Prefer,
    /// 必须使用 SSL
    Require,
    /// 验证 CA 证书
    VerifyCa,
    /// 验证 CA 证书和主机名
    VerifyFull,
}

impl PostgresSslMode {
    /// 获取显示名称
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Disable => "禁用",
            Self::Prefer => "优先",
            Self::Require => "必需",
            Self::VerifyCa => "验证 CA",
            Self::VerifyFull => "完全验证",
        }
    }

    /// 获取描述
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Disable => "不使用 SSL 加密",
            Self::Prefer => "优先 SSL，允许不安全连接",
            Self::Require => "必须使用 SSL 加密",
            Self::VerifyCa => "验证服务器 CA 证书",
            Self::VerifyFull => "验证证书和主机名",
        }
    }

    /// 获取所有选项
    pub const fn all() -> &'static [PostgresSslMode] {
        &[
            Self::Disable,
            Self::Prefer,
            Self::Require,
            Self::VerifyCa,
            Self::VerifyFull,
        ]
    }

    /// 是否需要 TLS
    #[allow(dead_code)] // 公开 API，供外部使用
    pub const fn requires_tls(&self) -> bool {
        !matches!(self, Self::Disable)
    }
}

// ============================================================================
// MySQL SSL 模式
// ============================================================================

/// MySQL SSL 模式
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum MySqlSslMode {
    /// 禁用 SSL（默认）
    #[default]
    Disabled,
    /// 优先使用 SSL，但允许不安全连接
    Preferred,
    /// 必须使用 SSL
    Required,
    /// 验证 CA 证书
    VerifyCa,
    /// 验证 CA 证书和主机名
    VerifyIdentity,
}

impl MySqlSslMode {
    /// 获取显示名称
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Disabled => "禁用",
            Self::Preferred => "优先",
            Self::Required => "必需",
            Self::VerifyCa => "验证 CA",
            Self::VerifyIdentity => "完全验证",
        }
    }

    /// 获取描述
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Disabled => "不使用 SSL 加密",
            Self::Preferred => "优先 SSL，允许不安全连接",
            Self::Required => "必须使用 SSL 加密",
            Self::VerifyCa => "验证服务器 CA 证书",
            Self::VerifyIdentity => "验证证书和主机名",
        }
    }

    /// 获取所有选项
    pub const fn all() -> &'static [MySqlSslMode] {
        &[
            Self::Disabled,
            Self::Preferred,
            Self::Required,
            Self::VerifyCa,
            Self::VerifyIdentity,
        ]
    }
}

// ============================================================================
// 查询结果
// ============================================================================

/// 查询结果
#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub affected_rows: u64,
    /// 是否被截断（原始结果超过限制）
    pub truncated: bool,
    /// 原始总行数（如果被截断）
    pub original_row_count: Option<usize>,
}
