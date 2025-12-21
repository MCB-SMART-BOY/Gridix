//! 筛选逻辑关系
//!
//! 定义筛选条件之间的 AND/OR 逻辑关系。

/// 筛选条件之间的逻辑关系
#[derive(Clone, Debug, Default, PartialEq, Copy)]
pub enum FilterLogic {
    #[default]
    And,
    Or,
}

impl FilterLogic {
    /// 切换逻辑关系
    pub fn toggle(&mut self) {
        *self = match self {
            Self::And => Self::Or,
            Self::Or => Self::And,
        };
    }
}

