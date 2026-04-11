//! 应用动作层。
//!
//! 负责把快捷键、命令面板和按钮入口收束到统一动作语义。

pub(in crate::app) mod action_system;
pub(in crate::app) mod command_palette;

pub(in crate::app) use super::DbManagerApp;
