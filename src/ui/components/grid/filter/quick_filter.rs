//! 快速筛选对话框
//!
//! 提供类似 VS Code 命令面板的快速筛选功能。

use super::condition::ColumnFilter;
use super::logic::FilterLogic;
use super::operators::FilterOperator;
use crate::ui::styles::GRAY;
use egui::{self, Color32, RichText, TextEdit};

/// 快速筛选对话框状态
#[allow(dead_code)] // 公开 API，供外部使用
#[derive(Default)]
pub struct QuickFilterState {
    /// 是否显示对话框
    pub show: bool,
    /// 输入内容
    pub input: String,
}

/// 显示快速筛选对话框
/// 
/// 返回解析成功的筛选条件（如果有）
pub fn show_quick_filter_dialog(
    ctx: &egui::Context,
    show: &mut bool,
    input: &mut String,
    columns: &[String],
) -> Option<ColumnFilter> {
    if !*show {
        return None;
    }

    let mut result: Option<ColumnFilter> = None;
    let mut should_close = false;

    egui::Window::new("快速筛选")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_TOP, [0.0, 100.0])
        .fixed_size([400.0, 0.0])
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                // 输入框
                let response = ui.add(
                    TextEdit::singleline(input)
                        .desired_width(380.0)
                        .hint_text("列名 操作符 值  (例: name ~ john, age > 18)")
                        .font(egui::TextStyle::Monospace),
                );

                // 自动聚焦
                if response.gained_focus() || input.is_empty() {
                    response.request_focus();
                }

                // 解析并预览
                let parsed = parse_quick_filter(input, columns);
                
                ui.add_space(8.0);
                
                match &parsed {
                    Ok(filter) => {
                        show_preview(ui, filter);
                    }
                    Err(hint) => {
                        ui.label(RichText::new(*hint).small().color(GRAY));
                    }
                }

                ui.add_space(8.0);

                // 语法帮助
                show_syntax_help(ui);

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
                show_column_hints(ui, columns, input);
            });
        });

    if should_close {
        *show = false;
        input.clear();
    }

    result
}

/// 显示解析预览
fn show_preview(ui: &mut egui::Ui, filter: &ColumnFilter) {
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

/// 显示语法帮助
fn show_syntax_help(ui: &mut egui::Ui) {
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
}

/// 显示可用列提示
fn show_column_hints(ui: &mut egui::Ui, columns: &[String], input: &mut String) {
    if columns.is_empty() || !input.is_empty() {
        return;
    }

    ui.add_space(8.0);
    ui.label(RichText::new("可用列:").small().color(GRAY));
    ui.horizontal_wrapped(|ui| {
        for col in columns.iter().take(10) {
            if ui.small_button(col).clicked() {
                *input = format!("{} ~ ", col);
            }
        }
        if columns.len() > 10 {
            ui.label(RichText::new(format!("...+{}", columns.len() - 10)).small().color(GRAY));
        }
    });
}

/// 解析快速筛选输入
/// 解析快速筛选输入
pub fn parse_quick_filter(input: &str, columns: &[String]) -> Result<ColumnFilter, &'static str> {
    let input = input.trim();
    
    if input.is_empty() {
        return Err("输入筛选条件...");
    }

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
    let (operator, value_start_idx) = parse_operator(parts[1])?;

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

/// 解析操作符字符串
fn parse_operator(op_str: &str) -> Result<(FilterOperator, usize), &'static str> {
    let op = match op_str.to_uppercase().as_str() {
        "~" => FilterOperator::Contains,
        "!~" => FilterOperator::NotContains,
        "=" | "==" => FilterOperator::Equals,
        "!=" | "<>" => FilterOperator::NotEquals,
        "^" => FilterOperator::StartsWith,
        "$" => FilterOperator::EndsWith,
        ">" => FilterOperator::GreaterThan,
        ">=" => FilterOperator::GreaterOrEqual,
        "<" => FilterOperator::LessThan,
        "<=" => FilterOperator::LessOrEqual,
        "[]" | "BETWEEN" => FilterOperator::Between,
        "![]" | "!BETWEEN" => FilterOperator::NotBetween,
        "IN" => FilterOperator::In,
        "!IN" | "NOTIN" => FilterOperator::NotIn,
        "NULL" | "ISNULL" => FilterOperator::IsNull,
        "!NULL" | "NOTNULL" => FilterOperator::IsNotNull,
        "EMPTY" | "''" => FilterOperator::IsEmpty,
        "!EMPTY" | "!''" => FilterOperator::IsNotEmpty,
        "REGEX" | "/./" => FilterOperator::Regex,
        _ => return Err("未知的操作符"),
    };
    
    Ok((op, 2))
}

