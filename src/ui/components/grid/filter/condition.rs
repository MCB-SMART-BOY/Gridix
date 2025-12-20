//! 筛选条件定义
//!
//! 定义单个筛选条件的结构和方法。

use super::logic::FilterLogic;
use super::operators::FilterOperator;

/// 列筛选条件
#[derive(Clone)]
pub struct ColumnFilter {
    /// 列名
    pub column: String,
    /// 操作符
    pub operator: FilterOperator,
    /// 筛选值
    pub value: String,
    /// 第二个值（用于 BETWEEN 操作符）
    pub value2: String,
    /// 是否启用此筛选条件
    pub enabled: bool,
    /// 是否大小写敏感
    pub case_sensitive: bool,
    /// 与下一个条件的逻辑关系
    pub logic: FilterLogic,
}

impl Default for ColumnFilter {
    fn default() -> Self {
        Self {
            column: String::new(),
            operator: FilterOperator::default(),
            value: String::new(),
            value2: String::new(),
            enabled: true,
            case_sensitive: false,
            logic: FilterLogic::And,
        }
    }
}

#[allow(dead_code)] // 公开 API，Builder 模式供外部使用
impl ColumnFilter {
    /// 创建新的筛选条件
    pub fn new(column: String) -> Self {
        Self {
            column,
            ..Default::default()
        }
    }

    /// 设置操作符（Builder 模式）
    pub fn with_operator(mut self, op: FilterOperator) -> Self {
        self.operator = op;
        self
    }

    /// 设置值（Builder 模式）
    pub fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }

    /// 设置第二个值（用于 BETWEEN，Builder 模式）
    pub fn with_value2(mut self, value2: String) -> Self {
        self.value2 = value2;
        self
    }

    /// 设置大小写敏感（Builder 模式）
    pub fn with_case_sensitive(mut self, case_sensitive: bool) -> Self {
        self.case_sensitive = case_sensitive;
        self
    }

    /// 设置逻辑关系（Builder 模式）
    pub fn with_logic(mut self, logic: FilterLogic) -> Self {
        self.logic = logic;
        self
    }

    /// 检查条件是否有效（可以应用）
    pub fn is_valid(&self) -> bool {
        if self.column.is_empty() {
            return false;
        }
        
        if self.operator.needs_value() && self.value.is_empty() {
            return false;
        }
        
        if self.operator.needs_second_value() && self.value2.is_empty() {
            return false;
        }
        
        true
    }
}

