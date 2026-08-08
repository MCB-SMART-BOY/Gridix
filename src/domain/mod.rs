//! 领域层 — 纯数据模型，不依赖 egui、数据库驱动、keyring、russh。
//!
//! 定义 Gridix 的核心类型：标识符、数据库值、结果集、变异操作、错误类型。

pub mod execution;
pub mod identifier;
pub mod ids;
pub mod metadata;
pub mod mutation;
pub mod result;
pub mod value;
