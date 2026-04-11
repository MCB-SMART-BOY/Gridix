//! 应用表面层。
//!
//! 负责主渲染循环、对话框编排和偏好设置落地。

pub(in crate::app) mod dialogs;
pub(in crate::app) mod preferences;
pub(in crate::app) mod render;

pub(in crate::app) use super::DbManagerApp;
pub(in crate::app) use super::action::action_system;
