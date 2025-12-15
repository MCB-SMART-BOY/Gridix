//! 异步消息类型定义
//!
//! 定义应用程序中异步任务完成后发送的消息类型。

use crate::database::QueryResult;

/// 异步任务完成后发送的消息
pub enum Message {
    /// 数据库连接完成 - SQLite 模式 (连接名, 表列表结果)
    ConnectedWithTables(String, Result<Vec<String>, String>),
    /// 数据库连接完成 - MySQL/PostgreSQL 模式 (连接名, 数据库列表结果)
    ConnectedWithDatabases(String, Result<Vec<String>, String>),
    /// 数据库选择完成 (连接名, 数据库名, 表列表结果)
    DatabaseSelected(String, String, Result<Vec<String>, String>),
    /// 查询执行完成 (SQL语句, 查询结果, 耗时毫秒)
    QueryDone(String, Result<QueryResult, String>, u64),
}
