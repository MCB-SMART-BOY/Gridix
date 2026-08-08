//! 执行结果类型 — 支持多语句查询
//!
//! 一条 SQL 可能包含多条语句（`SELECT 1; SELECT 2`），
//! `ExecutionOutcome` 表达每个语句的独立结果。

use super::result::ResultSet;

/// 一次 SQL 执行的完整结果
#[derive(Debug, Clone)]
pub struct ExecutionOutcome {
    /// 各语句的执行结果，按顺序排列
    pub statements: Vec<StatementOutcome>,
}

/// 单个语句的执行结果
#[derive(Debug, Clone)]
pub enum StatementOutcome {
    /// 查询结果集（SELECT 等）
    ResultSet(ResultSet),

    /// 影响行数（INSERT, UPDATE, DELETE 等）
    AffectedRows { rows: u64 },

    /// 命令完成（CREATE, DROP, ALTER 等），可选的受影响行数
    Command {
        tag: String,
        affected_rows: Option<u64>,
    },

    /// 数据库通知/警告
    Notice(String),
}

impl ExecutionOutcome {
    /// 构建仅包含一个 ResultSet 的执行结果
    pub fn single_result(result: ResultSet) -> Self {
        Self {
            statements: vec![StatementOutcome::ResultSet(result)],
        }
    }

    /// 构建仅包含影响行数的执行结果
    pub fn affected_rows(rows: u64) -> Self {
        Self {
            statements: vec![StatementOutcome::AffectedRows { rows }],
        }
    }

    /// 构建空结果（无语句执行）
    pub fn empty() -> Self {
        Self {
            statements: Vec::new(),
        }
    }
}
