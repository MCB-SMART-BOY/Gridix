//! 应用运行时层。
//!
//! 负责数据库请求、异步消息、元数据加载和请求生命周期。

pub(in crate::app) mod database;
pub(in crate::app) mod er_diagram;
pub(in crate::app) mod handler;
pub(in crate::app) mod message;
pub(in crate::app) mod metadata;
pub(in crate::app) mod request_lifecycle;

pub(in crate::app) use super::DbManagerApp;
pub(in crate::app) use super::GridSaveContext;
pub(in crate::app) use message::Message;
