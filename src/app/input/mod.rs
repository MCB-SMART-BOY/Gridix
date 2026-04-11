//! 应用输入层。
//!
//! 负责 focus-aware 输入路由和保留的键盘兼容路径。

pub(in crate::app) mod input_router;
pub(in crate::app) mod keyboard;
pub(in crate::app) mod owner;

pub(in crate::app) use super::DbManagerApp;
pub(in crate::app) use super::action::action_system;
