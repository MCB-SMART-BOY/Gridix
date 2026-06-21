//! 异步消息类型定义（Layer 2）
//!
//! 定义异步任务完成后发送给 UI 线程的消息类型。
//! 部分变体携带 `request_id` 用于丢弃过期回包（6/12 有，6/12 通过其他方式保护）。

use crate::data::{
    ColumnInfo, ForeignKeyInfo, ImportExecutionReport, QueryResult, RoutineInfo, TriggerInfo,
};

/// 异步任务完成后发送的消息
pub enum Message {
    /// 数据库连接完成 - SQLite 模式 (连接名, 请求ID, 表列表结果)
    ConnectedWithTables(String, u64, Result<Vec<String>, String>),
    /// 数据库连接完成 - MySQL/PostgreSQL 模式 (连接名, 请求ID, 数据库列表结果)
    ConnectedWithDatabases(String, u64, Result<Vec<String>, String>),
    /// 数据库选择完成 (连接名, 数据库名, 请求ID, 表列表结果)
    DatabaseSelected(String, String, u64, Result<Vec<String>, String>),
    /// 数据库删除完成 (连接名, 数据库名, 删除结果)
    DatabaseDropped(String, String, Result<(), String>),
    /// 表删除完成 (连接名, 表名, 删除结果)
    TableDropped(String, String, Result<(), String>),
    /// schema 变更后的静默表列表重载完成 (连接名, 请求ID, 表列表结果)
    ///
    /// 与连接/选库不同：不发"已连接/已选库"提示，只静默刷新表列表与 autocomplete。
    ActiveTablesReloaded(String, u64, Result<Vec<String>, String>),
    /// 查询执行完成 (SQL语句, 连接名, 目标Tab ID, 请求ID, 查询结果, 耗时毫秒)
    QueryDone(
        String,
        String,
        String,
        u64,
        Result<QueryResult, String>,
        u64,
    ),
    /// 导入执行完成 (执行报告, 耗时毫秒)
    ImportDone(Result<ImportExecutionReport, String>, u64),
    /// 表格保存批次执行完成 (执行报告, 表名, 请求ID, 耗时毫秒)
    ///
    /// 网格编辑保存走事务化批量通道，成功后由 handler 清除编辑状态并刷新该表。
    GridSaveDone {
        result: Result<ImportExecutionReport, String>,
        table: String,
        request_id: u64,
        elapsed_ms: u64,
    },
    /// 主键列获取完成 (表名, 主键列名)
    PrimaryKeyFetched(String, Option<String>),
    /// 触发器列表获取完成 (连接名, 数据库名, 请求ID, 触发器列表结果)
    TriggersFetched(
        String,
        Option<String>,
        u64,
        Result<Vec<TriggerInfo>, String>,
    ),
    /// 存储过程/函数列表获取完成 (连接名, 数据库名, 请求ID, 存储过程列表结果)
    RoutinesFetched(
        String,
        Option<String>,
        u64,
        Result<Vec<RoutineInfo>, String>,
    ),
    /// 外键关系获取完成 (加载代号, 外键列表结果)
    ForeignKeysFetched(u64, Result<Vec<ForeignKeyInfo>, String>),
    /// ER图表结构获取完成 (加载代号, 表名, 列信息列表)
    ERTableColumnsFetched(u64, String, Result<Vec<ColumnInfo>, String>),
}
