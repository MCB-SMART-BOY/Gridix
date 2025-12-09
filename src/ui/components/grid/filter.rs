//! 筛选条件和筛选逻辑
//!
//! 提供现代数据库工具风格的筛选功能：
//! - 多条件 AND/OR 组合
//! - 条件启用/禁用
//! - 丰富的操作符
//! - 大小写敏感选项

use super::state::DataGridState;
use crate::database::QueryResult;
use crate::ui::styles::GRAY;
use egui::{self, Color32, RichText, TextEdit};

/// 筛选条件之间的逻辑关系
#[derive(Clone, Default, PartialEq, Copy)]
pub enum FilterLogic {
    #[default]
    And,
    Or,
}

impl FilterLogic {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::And => "AND",
            Self::Or => "OR",
        }
    }

    pub fn toggle(&mut self) {
        *self = match self {
            Self::And => Self::Or,
            Self::Or => Self::And,
        };
    }
}

/// 列筛选条件
#[derive(Clone)]
pub struct ColumnFilter {
    pub column: String,
    pub operator: FilterOperator,
    pub value: String,
    pub value2: String,           // 用于 BETWEEN 操作符的第二个值
    pub enabled: bool,            // 是否启用此筛选条件
    pub case_sensitive: bool,     // 是否大小写敏感
    pub logic: FilterLogic,       // 与下一个条件的逻辑关系
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

impl ColumnFilter {
    pub fn new(column: String) -> Self {
        Self {
            column,
            ..Default::default()
        }
    }

    #[allow(dead_code)]
    pub fn with_operator(mut self, op: FilterOperator) -> Self {
        self.operator = op;
        self
    }

    #[allow(dead_code)]
    pub fn with_value(mut self, value: String) -> Self {
        self.value = value;
        self
    }
}

#[derive(Clone, Default, PartialEq)]
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
    Between,        // 区间
    NotBetween,     // 不在区间
    
    // 集合操作符
    In,             // 在列表中
    NotIn,          // 不在列表中
    
    // 空值操作符
    IsNull,
    IsNotNull,
    IsEmpty,        // 空字符串
    IsNotEmpty,     // 非空字符串
    
    // 正则
    Regex,
}

impl FilterOperator {
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

    /// 获取值输入提示
    pub fn value_hint(&self) -> &'static str {
        match self {
            Self::In | Self::NotIn => "值1, 值2, ...",
            Self::Between | Self::NotBetween => "最小值",
            Self::Regex => "正则表达式",
            _ => "值...",
        }
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

    #[allow(dead_code)]
    pub fn all() -> Vec<FilterOperator> {
        let mut ops = Vec::new();
        ops.extend_from_slice(Self::text_operators());
        ops.extend_from_slice(Self::comparison_operators());
        ops.extend_from_slice(Self::set_operators());
        ops.extend_from_slice(Self::null_operators());
        ops.push(Self::Regex);
        ops
    }
}

/// 显示筛选栏
pub fn show_filter_bar(ui: &mut egui::Ui, result: &QueryResult, state: &mut DataGridState) {
    if state.filters.is_empty() {
        return;
    }

    // 计算启用的筛选条件数量
    let enabled_count = state.filters.iter().filter(|f| f.enabled).count();
    let total_count = state.filters.len();

    ui.vertical(|ui| {
        // 筛选标题栏
        ui.horizontal(|ui| {
            ui.label(RichText::new("筛选条件").small().strong());
            
            ui.label(
                RichText::new(format!("({}/{})", enabled_count, total_count))
                    .small()
                    .color(GRAY),
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 清空全部按钮
                if ui
                    .small_button("清空全部")
                    .on_hover_text("清空所有筛选条件 [Esc]")
                    .clicked()
                {
                    state.filters.clear();
                }

                // 全部启用/禁用
                if enabled_count < total_count {
                    if ui.small_button("全部启用").clicked() {
                        for f in &mut state.filters {
                            f.enabled = true;
                        }
                    }
                } else if total_count > 0 {
                    if ui.small_button("全部禁用").clicked() {
                        for f in &mut state.filters {
                            f.enabled = false;
                        }
                    }
                }

                // 添加筛选按钮
                if ui.small_button("+ 添加").on_hover_text("添加新的筛选条件 [/]").clicked() {
                    state.filters.push(ColumnFilter::new(
                        result.columns.first().cloned().unwrap_or_default(),
                    ));
                }
            });
        });

        ui.add_space(4.0);

        // 筛选条件列表
        let mut filter_to_remove: Option<usize> = None;
        let filters_len = state.filters.len();

        for (idx, filter) in state.filters.iter_mut().enumerate() {
            let is_last = idx == filters_len - 1;
            
            ui.horizontal(|ui| {
                // 启用/禁用复选框
                let checkbox_response = ui.checkbox(&mut filter.enabled, "");
                if checkbox_response.changed() {
                    // 状态已更新
                }

                // 筛选条件框
                let frame_color = if filter.enabled {
                    Color32::from_rgb(45, 50, 60)
                } else {
                    Color32::from_rgb(35, 38, 45)
                };

                egui::Frame::none()
                    .fill(frame_color)
                    .rounding(4.0)
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_enabled(filter.enabled);
                        
                        ui.horizontal(|ui| {
                            // 列选择
                            egui::ComboBox::from_id_source(format!("fc_{}", idx))
                                .selected_text(RichText::new(&filter.column).small())
                                .width(100.0)
                                .show_ui(ui, |ui| {
                                    for col in &result.columns {
                                        ui.selectable_value(&mut filter.column, col.clone(), col);
                                    }
                                });

                            // 操作符选择（分组显示）
                            egui::ComboBox::from_id_source(format!("fo_{}", idx))
                                .selected_text(RichText::new(filter.operator.symbol()).small())
                                .width(70.0)
                                .show_ui(ui, |ui| {
                                    ui.label(RichText::new("文本").small().color(GRAY));
                                    for op in FilterOperator::text_operators() {
                                        ui.selectable_value(
                                            &mut filter.operator,
                                            op.clone(),
                                            format!("{} {}", op.symbol(), op.display_name()),
                                        );
                                    }
                                    ui.separator();
                                    ui.label(RichText::new("比较").small().color(GRAY));
                                    for op in FilterOperator::comparison_operators() {
                                        ui.selectable_value(
                                            &mut filter.operator,
                                            op.clone(),
                                            format!("{} {}", op.symbol(), op.display_name()),
                                        );
                                    }
                                    ui.separator();
                                    ui.label(RichText::new("集合").small().color(GRAY));
                                    for op in FilterOperator::set_operators() {
                                        ui.selectable_value(
                                            &mut filter.operator,
                                            op.clone(),
                                            format!("{} {}", op.symbol(), op.display_name()),
                                        );
                                    }
                                    ui.separator();
                                    ui.label(RichText::new("空值").small().color(GRAY));
                                    for op in FilterOperator::null_operators() {
                                        ui.selectable_value(
                                            &mut filter.operator,
                                            op.clone(),
                                            format!("{} {}", op.symbol(), op.display_name()),
                                        );
                                    }
                                    ui.separator();
                                    ui.selectable_value(
                                        &mut filter.operator,
                                        FilterOperator::Regex,
                                        format!(
                                            "{} {}",
                                            FilterOperator::Regex.symbol(),
                                            FilterOperator::Regex.display_name()
                                        ),
                                    );
                                });

                            // 值输入
                            if filter.operator.needs_value() {
                                ui.add(
                                    TextEdit::singleline(&mut filter.value)
                                        .desired_width(120.0)
                                        .font(egui::TextStyle::Small)
                                        .hint_text(filter.operator.value_hint()),
                                );

                                // 第二个值（BETWEEN）
                                if filter.operator.needs_second_value() {
                                    ui.label(RichText::new("~").small().color(GRAY));
                                    ui.add(
                                        TextEdit::singleline(&mut filter.value2)
                                            .desired_width(80.0)
                                            .font(egui::TextStyle::Small)
                                            .hint_text("最大值"),
                                    );
                                }
                            }

                            // 大小写敏感切换
                            if matches!(
                                filter.operator,
                                FilterOperator::Contains
                                    | FilterOperator::NotContains
                                    | FilterOperator::Equals
                                    | FilterOperator::NotEquals
                                    | FilterOperator::StartsWith
                                    | FilterOperator::EndsWith
                                    | FilterOperator::In
                                    | FilterOperator::NotIn
                            ) {
                                let case_btn = if filter.case_sensitive { "Aa" } else { "aa" };
                                let case_color = if filter.case_sensitive {
                                    Color32::from_rgb(100, 150, 200)
                                } else {
                                    GRAY
                                };
                                if ui
                                    .add(egui::Button::new(RichText::new(case_btn).small().color(case_color)).small())
                                    .on_hover_text(if filter.case_sensitive {
                                        "大小写敏感 (点击切换)"
                                    } else {
                                        "忽略大小写 (点击切换)"
                                    })
                                    .clicked()
                                {
                                    filter.case_sensitive = !filter.case_sensitive;
                                }
                            }

                            // 删除按钮
                            if ui
                                .add(egui::Button::new(RichText::new("x").small().color(Color32::from_rgb(180, 80, 80))).small())
                                .on_hover_text("删除此筛选条件")
                                .clicked()
                            {
                                filter_to_remove = Some(idx);
                            }
                        });
                    });

                // 逻辑关系按钮（非最后一个条件才显示）
                if !is_last {
                    let logic_color = match filter.logic {
                        FilterLogic::And => Color32::from_rgb(100, 180, 100),
                        FilterLogic::Or => Color32::from_rgb(180, 150, 80),
                    };
                    if ui
                        .add(egui::Button::new(
                            RichText::new(filter.logic.display_name())
                                .small()
                                .color(logic_color),
                        ).small())
                        .on_hover_text("点击切换 AND/OR")
                        .clicked()
                    {
                        filter.logic.toggle();
                    }
                }
            });

            ui.add_space(2.0);
        }

        if let Some(idx) = filter_to_remove {
            state.filters.remove(idx);
        }
    });
}

/// 快速筛选菜单（右键单元格时显示）
#[allow(dead_code)]
pub fn show_quick_filter_menu(
    ui: &mut egui::Ui,
    cell_value: &str,
    column_name: &str,
    state: &mut DataGridState,
) {
    ui.label(RichText::new(format!("筛选: {}", column_name)).small().strong());
    ui.separator();

    // 快速筛选选项
    if ui.button(format!("= \"{}\"", truncate_display(cell_value, 20))).clicked() {
        state.filters.push(
            ColumnFilter::new(column_name.to_string())
                .with_operator(FilterOperator::Equals)
                .with_value(cell_value.to_string()),
        );
        ui.close_menu();
    }

    if ui.button(format!("!= \"{}\"", truncate_display(cell_value, 20))).clicked() {
        state.filters.push(
            ColumnFilter::new(column_name.to_string())
                .with_operator(FilterOperator::NotEquals)
                .with_value(cell_value.to_string()),
        );
        ui.close_menu();
    }

    if !cell_value.is_empty() && cell_value != "NULL" {
        if ui.button(format!("包含 \"{}\"", truncate_display(cell_value, 20))).clicked() {
            state.filters.push(
                ColumnFilter::new(column_name.to_string())
                    .with_operator(FilterOperator::Contains)
                    .with_value(cell_value.to_string()),
            );
            ui.close_menu();
        }
    }

    ui.separator();

    if cell_value == "NULL" || cell_value.is_empty() {
        if ui.button("不为 NULL").clicked() {
            state.filters.push(
                ColumnFilter::new(column_name.to_string())
                    .with_operator(FilterOperator::IsNotNull),
            );
            ui.close_menu();
        }
    } else {
        if ui.button("为 NULL").clicked() {
            state.filters.push(
                ColumnFilter::new(column_name.to_string())
                    .with_operator(FilterOperator::IsNull),
            );
            ui.close_menu();
        }
    }

    // 如果是数字，添加比较选项
    if cell_value.parse::<f64>().is_ok() {
        ui.separator();
        if ui.button(format!("> {}", cell_value)).clicked() {
            state.filters.push(
                ColumnFilter::new(column_name.to_string())
                    .with_operator(FilterOperator::GreaterThan)
                    .with_value(cell_value.to_string()),
            );
            ui.close_menu();
        }
        if ui.button(format!("< {}", cell_value)).clicked() {
            state.filters.push(
                ColumnFilter::new(column_name.to_string())
                    .with_operator(FilterOperator::LessThan)
                    .with_value(cell_value.to_string()),
            );
            ui.close_menu();
        }
    }
}

/// 过滤行数据
pub fn filter_rows<'a>(
    result: &'a QueryResult,
    search_text: &str,
    search_column: &Option<String>,
    filters: &[ColumnFilter],
) -> Vec<(usize, &'a Vec<String>)> {
    let search_lower = search_text.to_lowercase();

    // 只使用启用的筛选条件
    let active_filters: Vec<&ColumnFilter> = filters.iter().filter(|f| f.enabled).collect();

    result
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            // 搜索条件
            let search_match = if search_text.is_empty() {
                true
            } else {
                match search_column {
                    Some(col_name) => result
                        .columns
                        .iter()
                        .position(|c| c == col_name)
                        .and_then(|col_idx| row.get(col_idx))
                        .map(|cell| cell.to_lowercase().contains(&search_lower))
                        .unwrap_or(false),
                    None => row
                        .iter()
                        .any(|cell| cell.to_lowercase().contains(&search_lower)),
                }
            };

            if !search_match {
                return false;
            }

            // 筛选条件（支持 AND/OR 逻辑）
            if active_filters.is_empty() {
                return true;
            }

            let mut current_result = true;
            let mut pending_logic = FilterLogic::And;

            for (i, filter) in active_filters.iter().enumerate() {
                let col_idx = result.columns.iter().position(|c| c == &filter.column);
                let filter_match = if let Some(idx) = col_idx {
                    if let Some(cell) = row.get(idx) {
                        check_filter_condition(cell, filter)
                    } else {
                        false
                    }
                } else {
                    false
                };

                if i == 0 {
                    current_result = filter_match;
                } else {
                    match pending_logic {
                        FilterLogic::And => current_result = current_result && filter_match,
                        FilterLogic::Or => current_result = current_result || filter_match,
                    }
                }

                pending_logic = filter.logic;
            }

            current_result
        })
        .collect()
}

/// 检查筛选条件
fn check_filter_condition(cell: &str, filter: &ColumnFilter) -> bool {
    let (cell_cmp, value_cmp, _value2_cmp) = if filter.case_sensitive {
        (
            cell.to_string(),
            filter.value.clone(),
            filter.value2.clone(),
        )
    } else {
        (
            cell.to_lowercase(),
            filter.value.to_lowercase(),
            filter.value2.to_lowercase(),
        )
    };

    match filter.operator {
        FilterOperator::Contains => cell_cmp.contains(&value_cmp),
        FilterOperator::NotContains => !cell_cmp.contains(&value_cmp),
        FilterOperator::Equals => cell_cmp == value_cmp,
        FilterOperator::NotEquals => cell_cmp != value_cmp,
        FilterOperator::StartsWith => cell_cmp.starts_with(&value_cmp),
        FilterOperator::EndsWith => cell_cmp.ends_with(&value_cmp),
        
        FilterOperator::GreaterThan => compare_values(cell, &filter.value, |a, b| a > b),
        FilterOperator::GreaterOrEqual => compare_values(cell, &filter.value, |a, b| a >= b),
        FilterOperator::LessThan => compare_values(cell, &filter.value, |a, b| a < b),
        FilterOperator::LessOrEqual => compare_values(cell, &filter.value, |a, b| a <= b),
        
        FilterOperator::Between => {
            let in_range = compare_values(cell, &filter.value, |a, b| a >= b)
                && compare_values(cell, &filter.value2, |a, b| a <= b);
            in_range
        }
        FilterOperator::NotBetween => {
            let in_range = compare_values(cell, &filter.value, |a, b| a >= b)
                && compare_values(cell, &filter.value2, |a, b| a <= b);
            !in_range
        }

        FilterOperator::In => {
            let values: Vec<&str> = filter.value.split(',').map(|s| s.trim()).collect();
            if filter.case_sensitive {
                values.contains(&cell)
            } else {
                values.iter().any(|v| v.to_lowercase() == cell_cmp)
            }
        }
        FilterOperator::NotIn => {
            let values: Vec<&str> = filter.value.split(',').map(|s| s.trim()).collect();
            if filter.case_sensitive {
                !values.contains(&cell)
            } else {
                !values.iter().any(|v| v.to_lowercase() == cell_cmp)
            }
        }

        FilterOperator::IsNull => cell == "NULL",
        FilterOperator::IsNotNull => cell != "NULL",
        FilterOperator::IsEmpty => cell.is_empty() || cell == "NULL",
        FilterOperator::IsNotEmpty => !cell.is_empty() && cell != "NULL",
        
        FilterOperator::Regex => regex::Regex::new(&filter.value)
            .map(|re| re.is_match(cell))
            .unwrap_or(false),
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

/// 截断显示文本
#[allow(dead_code)]
fn truncate_display(s: &str, max_len: usize) -> String {
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

// ============================================================================
// 快速筛选功能
// ============================================================================

/// 显示快速筛选输入框（类似 VS Code 命令面板）
/// 语法: `列名 操作符 值` 或简写形式
/// 例如: `name ~ john`, `age > 18`, `status = active`
pub fn show_quick_filter_dialog(
    ctx: &egui::Context,
    state: &mut DataGridState,
    columns: &[String],
) -> Option<ColumnFilter> {
    let mut result: Option<ColumnFilter> = None;
    let mut should_close = false;

    if !state.show_quick_filter {
        return None;
    }

    egui::Window::new("快速筛选")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
        .fixed_size([400.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // 输入框
                let response = ui.add(
                    TextEdit::singleline(&mut state.quick_filter_input)
                        .desired_width(380.0)
                        .hint_text("列名 操作符 值  (例: name ~ john, age > 18)")
                        .font(egui::TextStyle::Monospace),
                );

                // 自动聚焦
                if response.gained_focus() || state.quick_filter_input.is_empty() {
                    response.request_focus();
                }

                // 解析并预览
                let parsed = parse_quick_filter(&state.quick_filter_input, columns);
                
                ui.add_space(8.0);
                
                match &parsed {
                    Ok(filter) => {
                        // 显示解析结果预览
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("预览:").small().color(GRAY));
                            ui.label(
                                RichText::new(&filter.column)
                                    .small()
                                    .color(Color32::from_rgb(100, 180, 255)),
                            );
                            ui.label(
                                RichText::new(filter.operator.symbol())
                                    .small()
                                    .color(Color32::from_rgb(255, 180, 100)),
                            );
                            if filter.operator.needs_value() {
                                ui.label(
                                    RichText::new(&filter.value)
                                        .small()
                                        .color(Color32::from_rgb(150, 255, 150)),
                                );
                                if filter.operator.needs_second_value() && !filter.value2.is_empty() {
                                    ui.label(RichText::new("~").small().color(GRAY));
                                    ui.label(
                                        RichText::new(&filter.value2)
                                            .small()
                                            .color(Color32::from_rgb(150, 255, 150)),
                                    );
                                }
                            }
                        });
                    }
                    Err(hint) => {
                        ui.label(RichText::new(*hint).small().color(GRAY));
                    }
                }

                ui.add_space(8.0);

                // 语法帮助
                ui.collapsing("语法帮助", |ui| {
                    ui.label(RichText::new("操作符:").small().strong());
                    let help_items = [
                        ("~  包含", "name ~ john"),
                        ("!~ 不包含", "name !~ test"),
                        ("=  等于", "status = active"),
                        ("!= 不等于", "type != deleted"),
                        ("^  开头是", "name ^ Mr"),
                        ("$  结尾是", "email $ .com"),
                        (">  大于", "age > 18"),
                        (">= 大于等于", "price >= 100"),
                        ("<  小于", "count < 10"),
                        ("<= 小于等于", "score <= 60"),
                        ("[] 介于", "age [] 18 30"),
                        ("IN 在列表中", "status IN active,pending"),
                        ("NULL 为空", "deleted NULL"),
                        ("!NULL 不为空", "name !NULL"),
                    ];
                    for (op, example) in help_items {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(op).small().monospace().color(Color32::from_rgb(255, 180, 100)));
                            ui.label(RichText::new(example).small().monospace().color(GRAY));
                        });
                    }
                });

                ui.add_space(8.0);

                // 按钮
                ui.horizontal(|ui| {
                    let can_add = parsed.is_ok();
                    
                    if ui
                        .add_enabled(can_add, egui::Button::new("添加筛选 [Enter]"))
                        .clicked()
                        || (can_add && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                    {
                        if let Ok(filter) = parsed {
                            result = Some(filter);
                            should_close = true;
                        }
                    }

                    if ui.button("取消 [Esc]").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape))
                    {
                        should_close = true;
                    }
                });

                // 可用列提示
                if !columns.is_empty() && state.quick_filter_input.is_empty() {
                    ui.add_space(8.0);
                    ui.label(RichText::new("可用列:").small().color(GRAY));
                    ui.horizontal_wrapped(|ui| {
                        for col in columns.iter().take(10) {
                            if ui.small_button(col).clicked() {
                                state.quick_filter_input = format!("{} ~ ", col);
                            }
                        }
                        if columns.len() > 10 {
                            ui.label(RichText::new(format!("...+{}", columns.len() - 10)).small().color(GRAY));
                        }
                    });
                }
            });
        });

    if should_close {
        state.show_quick_filter = false;
        state.quick_filter_input.clear();
    }

    result
}

/// 解析快速筛选输入
/// 格式: `列名 操作符 值` 或 `列名 操作符 值1 值2`（用于 BETWEEN）
fn parse_quick_filter(input: &str, columns: &[String]) -> Result<ColumnFilter, &'static str> {
    let input = input.trim();
    
    if input.is_empty() {
        return Err("输入筛选条件...");
    }

    // 尝试解析各种格式
    // 格式1: 列名 操作符 值
    // 格式2: 列名 NULL / 列名 !NULL
    // 格式3: 列名 [] 值1 值2 (BETWEEN)
    // 格式4: 列名 IN 值1,值2,值3

    let parts: Vec<&str> = input.split_whitespace().collect();
    
    if parts.is_empty() {
        return Err("输入筛选条件...");
    }

    // 查找列名（支持部分匹配）
    let col_input = parts[0];
    let matched_col = columns
        .iter()
        .find(|c| c.to_lowercase() == col_input.to_lowercase())
        .or_else(|| {
            columns
                .iter()
                .find(|c| c.to_lowercase().starts_with(&col_input.to_lowercase()))
        });

    let column = match matched_col {
        Some(c) => c.clone(),
        None => {
            if columns.is_empty() {
                return Err("没有可用的列");
            }
            return Err("未找到匹配的列名");
        }
    };

    if parts.len() < 2 {
        return Err("请输入操作符");
    }

    // 解析操作符
    let (operator, value_start_idx) = match parts[1].to_uppercase().as_str() {
        "~" => (FilterOperator::Contains, 2),
        "!~" => (FilterOperator::NotContains, 2),
        "=" | "==" => (FilterOperator::Equals, 2),
        "!=" | "<>" => (FilterOperator::NotEquals, 2),
        "^" => (FilterOperator::StartsWith, 2),
        "$" => (FilterOperator::EndsWith, 2),
        ">" => (FilterOperator::GreaterThan, 2),
        ">=" => (FilterOperator::GreaterOrEqual, 2),
        "<" => (FilterOperator::LessThan, 2),
        "<=" => (FilterOperator::LessOrEqual, 2),
        "[]" | "BETWEEN" => (FilterOperator::Between, 2),
        "![]" | "!BETWEEN" => (FilterOperator::NotBetween, 2),
        "IN" => (FilterOperator::In, 2),
        "!IN" | "NOTIN" => (FilterOperator::NotIn, 2),
        "NULL" | "ISNULL" => (FilterOperator::IsNull, 2),
        "!NULL" | "NOTNULL" => (FilterOperator::IsNotNull, 2),
        "EMPTY" | "''" => (FilterOperator::IsEmpty, 2),
        "!EMPTY" | "!''" => (FilterOperator::IsNotEmpty, 2),
        "REGEX" | "/./" => (FilterOperator::Regex, 2),
        _ => return Err("未知的操作符"),
    };

    // 检查是否需要值
    if !operator.needs_value() {
        return Ok(ColumnFilter {
            column,
            operator,
            value: String::new(),
            value2: String::new(),
            enabled: true,
            case_sensitive: false,
            logic: FilterLogic::And,
        });
    }

    // 获取值
    if parts.len() <= value_start_idx {
        return Err("请输入筛选值");
    }

    let value = if operator == FilterOperator::In || operator == FilterOperator::NotIn {
        // IN 操作符：合并剩余所有部分
        parts[value_start_idx..].join(" ")
    } else {
        parts[value_start_idx].to_string()
    };

    // BETWEEN 需要第二个值
    let value2 = if operator.needs_second_value() {
        if parts.len() > value_start_idx + 1 {
            parts[value_start_idx + 1].to_string()
        } else {
            return Err("BETWEEN 需要两个值");
        }
    } else {
        String::new()
    };

    Ok(ColumnFilter {
        column,
        operator,
        value,
        value2,
        enabled: true,
        case_sensitive: false,
        logic: FilterLogic::And,
    })
}
