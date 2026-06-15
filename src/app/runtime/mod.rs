//! 应用运行时层。
//!
//! 负责数据库请求、异步消息、元数据加载和请求生命周期。

pub(crate) mod database;
pub(crate) mod er_diagram;
pub(crate) mod handler;
pub(crate) mod message;
pub(crate) mod metadata;
pub(crate) mod request_lifecycle;

pub(crate) use super::DbManagerApp;
pub(crate) use message::Message;
