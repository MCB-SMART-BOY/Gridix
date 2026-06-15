//! 会话层（Layer 2）
//!
//! 管理数据库连接生命周期、查询执行、异步消息分发和 Tab 状态。
//! 依赖 data/ layer，被 state/ 和 ui/ 层使用。

pub mod tab;
