//! 异步消息类型定义（Layer 2）
//!
//! 定义异步任务完成后发送给 UI 线程的统一运行时事件消息。
//! `RuntimeEvent` 携带任务身份，由 `TaskRegistry` 丢弃过期回包。

use crate::session::runtime_event::RuntimeEvent;

/// 异步任务完成后发送的消息。
///
/// 所有异步完成路径都必须携带任务身份，通过 `TaskRegistry` 做 stale guard。
pub enum Message {
    RuntimeEvent(RuntimeEvent),
}
