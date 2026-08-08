//! 异步消息类型定义（Layer 2）
//!
//! 定义异步任务完成后发送给 UI 线程的消息类型。
//! 部分变体携带 `request_id` 用于丢弃过期回包（6/12 有，6/12 通过其他方式保护）。

use crate::data::{ImportExecutionReport, RoutineInfo, TriggerInfo};
use crate::session::runtime_event::RuntimeEvent;

/// 异步任务完成后发送的消息
pub enum Message {
    /// 统一运行时事件（替代逐步迁移中的旧变体）
    RuntimeEvent(RuntimeEvent),

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
    ImportDone(Result<ImportExecutionReport, String>, u64),
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
}
