//! 类型化变异操作 IR
//!
//! 取代 `generate_save_sql()` 生成的 `Vec<String>` SQL 字符串，
//! 使用结构化 `MutationBatch` + 参数化值绑定。
//!
//! 关键设计决策：
//! - `InputValue::Unspecified` 与 `NULL` 分离（INSERT 时省略列 vs 显式 NULL）
//! - `RowIdentity` 支持单列和复合主键
//! - `ExpectedRows` 强制 UPDATE/DELETE 的 affected rows 不变式

use super::value::DbValue;

/// 用户对单元格的输入值
#[derive(Debug, Clone)]
pub enum InputValue {
    /// INSERT 时不提交该字段（让 DB 使用 DEFAULT 或自增）
    Unspecified,
    /// 显式要求数据库 DEFAULT
    Default,
    /// SQL NULL
    Null,
    /// 具体值
    Value(DbValue),
}

/// 列引用（未引用名称，由后端按 dialect 转义）
#[derive(Debug, Clone)]
pub struct ColumnRef {
    pub name: String,
}

/// 行的唯一标识（主键或唯一键）
#[derive(Debug, Clone)]
pub enum RowIdentity {
    /// 主键列
    PrimaryKey(Vec<(ColumnRef, DbValue)>),
    /// 命名唯一约束
    UniqueKey {
        constraint: String,
        columns: Vec<(ColumnRef, DbValue)>,
    },
}

/// UPDATE/DELETE 期望影响的行数
#[derive(Debug, Clone, Copy)]
pub enum ExpectedRows {
    Exactly(u64),
    AtLeast(u64),
    Any,
}

/// 单个变异操作
#[derive(Debug, Clone)]
pub enum Mutation {
    Insert {
        table: ColumnRef,
        columns: Vec<ColumnRef>,
        values: Vec<InputValue>,
    },
    Update {
        table: ColumnRef,
        identity: RowIdentity,
        changes: Vec<(ColumnRef, InputValue)>,
        expected_rows: ExpectedRows,
    },
    Delete {
        table: ColumnRef,
        identity: RowIdentity,
        expected_rows: ExpectedRows,
    },
}

/// 批量变异（可选事务包裹）
#[derive(Debug, Clone)]
pub struct MutationBatch {
    pub mutations: Vec<Mutation>,
    /// 是否在事务中原子执行
    pub atomic: bool,
}

/// 批量变异执行结果
#[derive(Debug, Clone)]
pub struct MutationBatchResult {
    /// 每条变异的受影响行数
    pub affected: Vec<u64>,
    /// 是否有任何变异失败
    pub all_success: bool,
}

impl MutationBatch {
    pub fn new() -> Self {
        Self {
            mutations: Vec::new(),
            atomic: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.mutations.is_empty()
    }

    pub fn len(&self) -> usize {
        self.mutations.len()
    }
}

impl Default for MutationBatch {
    fn default() -> Self {
        Self::new()
    }
}

impl InputValue {
    /// 是否为 `Unspecified`（INSERT 时跳过该列）
    pub fn is_unspecified(&self) -> bool {
        matches!(self, Self::Unspecified)
    }

    /// 转换为 SQL 绑定的值（用于 prepared statement）
    pub fn as_bind_value(&self) -> Option<&DbValue> {
        match self {
            Self::Value(v) => Some(v),
            Self::Null => None, // NULL 通过 bind_null 处理
            _ => None,
        }
    }
}
