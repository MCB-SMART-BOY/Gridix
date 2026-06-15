//! 数据库错误类型

use thiserror::Error;

/// 数据库操作错误
#[derive(Error, Debug)]
pub enum DbError {
    #[error("连接错误: {0}")]
    Connection(String),
    #[error("查询错误: {0}")]
    Query(String),
}

impl DbError {
    /// 创建连接错误
    pub fn connection(message: impl Into<String>) -> Self {
        Self::Connection(message.into())
    }

    /// 创建查询错误
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query(message.into())
    }
}
