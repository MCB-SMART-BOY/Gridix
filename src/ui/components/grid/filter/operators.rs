//! 筛选操作符定义
//!
//! 定义所有支持的筛选操作符及其属性。

/// 筛选操作符
#[derive(Clone, Debug, Default, PartialEq)]
pub enum FilterOperator {
    // 文本操作符
    #[default]
    Contains,
    NotContains,
    Equals,
    NotEquals,
    StartsWith,
    EndsWith,
    
    // 比较操作符
    GreaterThan,
    GreaterOrEqual,
    LessThan,
    LessOrEqual,
    Between,
    NotBetween,
    
    // 集合操作符
    In,
    NotIn,
    
    // 空值操作符
    IsNull,
    IsNotNull,
    IsEmpty,
    IsNotEmpty,
    
    // 正则
    Regex,
}

impl FilterOperator {
    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Contains => "包含",
            Self::NotContains => "不包含",
            Self::Equals => "等于",
            Self::NotEquals => "不等于",
            Self::StartsWith => "开头是",
            Self::EndsWith => "结尾是",
            Self::GreaterThan => "大于",
            Self::GreaterOrEqual => "大于等于",
            Self::LessThan => "小于",
            Self::LessOrEqual => "小于等于",
            Self::Between => "介于",
            Self::NotBetween => "不介于",
            Self::In => "在列表中",
            Self::NotIn => "不在列表中",
            Self::IsNull => "为 NULL",
            Self::IsNotNull => "不为 NULL",
            Self::IsEmpty => "为空字符串",
            Self::IsNotEmpty => "非空字符串",
            Self::Regex => "正则匹配",
        }
    }

    /// 操作符符号（用于紧凑显示）
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Contains => "~",
            Self::NotContains => "!~",
            Self::Equals => "=",
            Self::NotEquals => "!=",
            Self::StartsWith => "^...",
            Self::EndsWith => "...$",
            Self::GreaterThan => ">",
            Self::GreaterOrEqual => ">=",
            Self::LessThan => "<",
            Self::LessOrEqual => "<=",
            Self::Between => "[a,b]",
            Self::NotBetween => "![a,b]",
            Self::In => "IN",
            Self::NotIn => "NOT IN",
            Self::IsNull => "NULL",
            Self::IsNotNull => "!NULL",
            Self::IsEmpty => "''",
            Self::IsNotEmpty => "!''",
            Self::Regex => "/.*/",
        }
    }

    /// 是否需要输入值
    pub fn needs_value(&self) -> bool {
        !matches!(
            self,
            Self::IsNull | Self::IsNotNull | Self::IsEmpty | Self::IsNotEmpty
        )
    }

    /// 是否需要第二个值（BETWEEN）
    pub fn needs_second_value(&self) -> bool {
        matches!(self, Self::Between | Self::NotBetween)
    }

    /// 文本类操作符
    pub fn text_operators() -> &'static [FilterOperator] {
        &[
            Self::Contains,
            Self::NotContains,
            Self::Equals,
            Self::NotEquals,
            Self::StartsWith,
            Self::EndsWith,
        ]
    }

    /// 比较类操作符
    pub fn comparison_operators() -> &'static [FilterOperator] {
        &[
            Self::GreaterThan,
            Self::GreaterOrEqual,
            Self::LessThan,
            Self::LessOrEqual,
            Self::Between,
            Self::NotBetween,
        ]
    }

    /// 集合类操作符
    pub fn set_operators() -> &'static [FilterOperator] {
        &[Self::In, Self::NotIn]
    }

    /// 空值类操作符
    pub fn null_operators() -> &'static [FilterOperator] {
        &[Self::IsNull, Self::IsNotNull, Self::IsEmpty, Self::IsNotEmpty]
    }

    /// 获取所有操作符
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn all() -> Vec<FilterOperator> {
        let mut ops = Vec::new();
        ops.extend_from_slice(Self::text_operators());
        ops.extend_from_slice(Self::comparison_operators());
        ops.extend_from_slice(Self::set_operators());
        ops.extend_from_slice(Self::null_operators());
        ops.push(Self::Regex);
        ops
    }

    /// 是否支持大小写敏感选项
    pub fn supports_case_sensitivity(&self) -> bool {
        matches!(
            self,
            Self::Contains
                | Self::NotContains
                | Self::Equals
                | Self::NotEquals
                | Self::StartsWith
                | Self::EndsWith
                | Self::In
                | Self::NotIn
        )
    }
}



/// 检查筛选条件是否匹配
pub fn check_filter_match(
    cell: &str,
    operator: &FilterOperator,
    value: &str,
    value2: &str,
    case_sensitive: bool,
) -> bool {
    let (cell_cmp, value_cmp, value2_cmp) = if case_sensitive {
        (cell.to_string(), value.to_string(), value2.to_string())
    } else {
        (cell.to_lowercase(), value.to_lowercase(), value2.to_lowercase())
    };

    match operator {
        FilterOperator::Contains => cell_cmp.contains(&value_cmp),
        FilterOperator::NotContains => !cell_cmp.contains(&value_cmp),
        FilterOperator::Equals => cell_cmp == value_cmp,
        FilterOperator::NotEquals => cell_cmp != value_cmp,
        FilterOperator::StartsWith => cell_cmp.starts_with(&value_cmp),
        FilterOperator::EndsWith => cell_cmp.ends_with(&value_cmp),
        
        FilterOperator::GreaterThan => compare_values(cell, value, |a, b| a > b),
        FilterOperator::GreaterOrEqual => compare_values(cell, value, |a, b| a >= b),
        FilterOperator::LessThan => compare_values(cell, value, |a, b| a < b),
        FilterOperator::LessOrEqual => compare_values(cell, value, |a, b| a <= b),
        
        FilterOperator::Between => {
            compare_values(cell, value, |a, b| a >= b)
                && compare_values(cell, &value2_cmp, |a, b| a <= b)
        }
        FilterOperator::NotBetween => {
            !(compare_values(cell, value, |a, b| a >= b)
                && compare_values(cell, &value2_cmp, |a, b| a <= b))
        }

        FilterOperator::In => {
            let values: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
            if case_sensitive {
                values.contains(&cell)
            } else {
                values.iter().any(|v| v.to_lowercase() == cell_cmp)
            }
        }
        FilterOperator::NotIn => {
            let values: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
            if case_sensitive {
                !values.contains(&cell)
            } else {
                !values.iter().any(|v| v.to_lowercase() == cell_cmp)
            }
        }

        FilterOperator::IsNull => cell == "NULL",
        FilterOperator::IsNotNull => cell != "NULL",
        FilterOperator::IsEmpty => cell.is_empty() || cell == "NULL",
        FilterOperator::IsNotEmpty => !cell.is_empty() && cell != "NULL",
        
        FilterOperator::Regex => {
            if value.len() > 100 {
                false
            } else {
                match regex::RegexBuilder::new(value)
                    .size_limit(1024 * 10)
                    .build()
                {
                    Ok(re) => re.is_match(cell),
                    Err(_) => false,
                }
            }
        }
    }
}

/// 比较值（支持数字和字符串）
fn compare_values<F>(cell: &str, value: &str, cmp: F) -> bool
where
    F: Fn(f64, f64) -> bool,
{
    cell.parse::<f64>()
        .ok()
        .zip(value.parse::<f64>().ok())
        .map(|(a, b)| cmp(a, b))
        .unwrap_or_else(|| cell > value)
}

