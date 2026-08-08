//! 数据库错误类型

use thiserror::Error;

/// 数据库操作错误
///
/// 包含结构化变体用于程序化匹配（认证失败、数据库不存在等），
/// 同时保留 `Connection(String)` 和 `Query(String)` 作为向后兼容的通用变体。
#[derive(Error, Debug)]
pub enum DbError {
    /// 操作已被取消
    #[error("操作已取消")]
    Cancelled,

    /// 超时
    #[error("操作超时: {operation} ({duration:?})")]
    Timeout {
        operation: &'static str,
        duration: std::time::Duration,
    },

    /// 通用连接错误（向后兼容）
    #[error("连接错误: {0}")]
    Connection(String),

    /// 认证失败
    #[error("认证失败: {message}")]
    Authentication { message: String },

    /// 数据库不存在
    #[error("数据库不存在: {name}")]
    DatabaseNotFound { name: String },

    /// 权限不足
    #[error("权限不足{}", object.as_deref().map(|o| format!(" ({o})")).unwrap_or_default())]
    PermissionDenied { object: Option<String> },

    /// 约束违反
    #[error("约束违反{}: {message}", constraint.as_deref().map(|c| format!(" ({c})")).unwrap_or_default())]
    ConstraintViolation {
        constraint: Option<String>,
        message: String,
    },

    /// 通用 SQL 执行错误（向后兼容）
    #[error("查询错误: {0}")]
    Query(String),

    /// TLS 错误
    #[error("TLS 错误: {0}")]
    Tls(String),

    /// SSH 错误
    #[error("SSH 错误: {0}")]
    Ssh(String),

    /// 不支持的操作
    #[error("不支持的操作: {capability}")]
    Unsupported { capability: &'static str },
    /// 值超出范围
    #[error("值超出范围: {value} ({reason})")]
    ValueOutOfRange { value: String, reason: &'static str },

    /// Keyring 错误
    #[error("Keyring error: {0}")]
    Keyring(String),

    /// I/O 错误
    #[error("I/O 错误: {0}")]
    Io(#[from] std::io::Error),
}

impl DbError {
    /// 创建通用连接错误（向后兼容）
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }

    /// 创建通用查询错误（向后兼容）
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query(message.into())
    }

    /// 创建认证失败错误
    pub fn authentication(message: impl Into<String>) -> Self {
        Self::Authentication {
            message: message.into(),
        }
    }

    /// 创建数据库不存在错误
    pub fn database_not_found(name: impl Into<String>) -> Self {
        Self::DatabaseNotFound { name: name.into() }
    }

    /// 创建权限不足错误
    pub fn permission_denied(object: Option<impl Into<String>>) -> Self {
        Self::PermissionDenied {
            object: object.map(|o| o.into()),
        }
    }

    /// 从原始数据库驱动错误字符串推断错误类别。
    ///
    /// 优先返回结构化变体，无法识别时回退到通用 `Connection`/`Query`。
    pub fn classify_connection(raw: impl Into<String>) -> Self {
        let msg: String = raw.into();
        let lower = msg.to_ascii_lowercase();

        if lower.contains("timeout") || lower.contains("超时") {
            return Self::Connection(msg);
        }
        if lower.contains("password authentication failed")
            || lower.contains("access denied for user")
            || (lower.contains("authentication") && lower.contains("failed"))
        {
            return Self::Authentication { message: msg };
        }
        if lower.contains("database") && lower.contains("does not exist")
            || lower.contains("unknown database")
        {
            // Extract database name if possible
            return Self::DatabaseNotFound {
                name: String::new(),
            };
        }
        if lower.contains("permission denied") {
            return Self::PermissionDenied { object: None };
        }

        Self::Connection(msg)
    }

    /// 判断此错误是否应触发安装/初始化引导。
    ///
    /// 临时性错误（超时、认证失败、连接被拒）→ false；
    /// 需要环境初始化的错误（数据库不存在、文件不可访问）→ true。
    pub fn warrants_onboarding(&self, db_type: crate::types::DatabaseType) -> bool {
        match self {
            Self::Cancelled | Self::Timeout { .. } => false,
            Self::Authentication { .. } => false,
            Self::DatabaseNotFound { .. } => true,
            Self::Connection(msg) => {
                let lower = msg.to_ascii_lowercase();
                // 临时性错误不触发引导
                let transient = lower.contains("timeout")
                    || lower.contains("refused")
                    || lower.contains("can't connect")
                    || lower.contains("could not connect")
                    || lower.contains("access denied")
                    || lower.contains("authentication failed")
                    || lower.contains("password authentication failed");
                if transient {
                    return false;
                }
                match db_type {
                    crate::types::DatabaseType::SQLite => {
                        lower.contains("unable to open database file")
                            || lower.contains("no such file")
                            || lower.contains("permission denied")
                    }
                    _ => lower.contains("unknown database") || lower.contains("does not exist"),
                }
            }
            _ => false,
        }
    }
}
