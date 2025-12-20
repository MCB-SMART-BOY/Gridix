//! 筛选 UI 组件
//!
//! 提供筛选栏的 UI 渲染功能。

use super::condition::ColumnFilter;
use super::logic::FilterLogic;
use super::operators::{validate_regex, FilterOperator};
use crate::database::QueryResult;
use crate::ui::styles::GRAY;
use egui::{self, Color32, RichText, TextEdit};

/// 筛选栏状态
#[allow(dead_code)] // 公开 API，供外部使用
pub struct FilterBarState {
    pub filters: Vec<ColumnFilter>,
}

/// 显示筛选栏
/// 
/// 返回是否有修改（用于使缓存失效）
pub fn show_filter_bar(
    ui: &mut egui::Ui,
    result: &QueryResult,
    filters: &mut Vec<ColumnFilter>,
) -> bool {
    if filters.is_empty() {
        return false;
    }
    
    let initial_filter_count = filters.len();
    let enabled_count = filters.iter().filter(|f| f.enabled).count();
    let total_count = filters.len();

    ui.vertical(|ui| {
        // 筛选标题栏
        show_filter_header(ui, filters, enabled_count, total_count, result);
        
        ui.add_space(4.0);

        // 筛选条件列表
        show_filter_list(ui, result, filters);
    });
    
    filters.len() != initial_filter_count
}

/// 显示筛选标题栏
fn show_filter_header(
    ui: &mut egui::Ui,
    filters: &mut Vec<ColumnFilter>,
    enabled_count: usize,
    total_count: usize,
    result: &QueryResult,
) {
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
                filters.clear();
            }

            // 全部启用/禁用
            if enabled_count < total_count {
                if ui.small_button("全部启用").clicked() {
                    for f in filters.iter_mut() {
                        f.enabled = true;
                    }
                }
            } else if total_count > 0
                && ui.small_button("全部禁用").clicked() {
                    for f in filters.iter_mut() {
                        f.enabled = false;
                    }
                }

            // 添加筛选按钮
            if ui.small_button("+ 添加").on_hover_text("添加新的筛选条件 [/]").clicked() {
                filters.push(ColumnFilter::new(
                    result.columns.first().cloned().unwrap_or_default(),
                ));
            }
        });
    });
}

/// 显示筛选条件列表
fn show_filter_list(
    ui: &mut egui::Ui,
    result: &QueryResult,
    filters: &mut Vec<ColumnFilter>,
) {
    let mut filter_to_remove: Option<usize> = None;
    let filters_len = filters.len();

    for (idx, filter) in filters.iter_mut().enumerate() {
        let is_last = idx == filters_len - 1;
        
        ui.horizontal(|ui| {
            // 启用/禁用复选框
            ui.checkbox(&mut filter.enabled, "");

            // 筛选条件框
            let frame_color = if filter.enabled {
                Color32::from_rgb(45, 50, 60)
            } else {
                Color32::from_rgb(35, 38, 45)
            };

            egui::Frame::NONE
                .fill(frame_color)
                .corner_radius(4.0)
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    if !filter.enabled {
                        ui.disable();
                    }
                    
                    ui.horizontal(|ui| {
                        // 列选择
                        show_column_selector(ui, idx, filter, result);

                        // 操作符选择
                        show_operator_selector(ui, idx, filter);

                        // 值输入
                        show_value_inputs(ui, filter);

                        // 大小写敏感切换
                        show_case_sensitivity_button(ui, filter);

                        // 删除按钮
                        if ui
                            .add(egui::Button::new(
                                RichText::new("x").small().color(Color32::from_rgb(180, 80, 80))
                            ).small())
                            .on_hover_text("删除此筛选条件")
                            .clicked()
                        {
                            filter_to_remove = Some(idx);
                        }
                    });
                });

            // 逻辑关系按钮
            if !is_last {
                show_logic_button(ui, filter);
            }
        });

        ui.add_space(2.0);
    }

    if let Some(idx) = filter_to_remove {
        filters.remove(idx);
    }
}

/// 显示列选择器
fn show_column_selector(
    ui: &mut egui::Ui,
    idx: usize,
    filter: &mut ColumnFilter,
    result: &QueryResult,
) {
    egui::ComboBox::new(format!("fc_{}", idx), "")
        .selected_text(RichText::new(&filter.column).small())
        .width(100.0)
        .show_ui(ui, |ui| {
            for col in &result.columns {
                ui.selectable_value(&mut filter.column, col.clone(), col);
            }
        });
}

/// 显示操作符选择器
fn show_operator_selector(ui: &mut egui::Ui, idx: usize, filter: &mut ColumnFilter) {
    egui::ComboBox::new(format!("fo_{}", idx), "")
        .selected_text(RichText::new(filter.operator.symbol()).small())
        .width(70.0)
        .show_ui(ui, |ui| {
            // 文本操作符
            ui.label(RichText::new("文本").small().color(GRAY));
            for op in FilterOperator::text_operators() {
                ui.selectable_value(
                    &mut filter.operator,
                    op.clone(),
                    format!("{} {}", op.symbol(), op.display_name()),
                );
            }
            
            ui.separator();
            
            // 比较操作符
            ui.label(RichText::new("比较").small().color(GRAY));
            for op in FilterOperator::comparison_operators() {
                ui.selectable_value(
                    &mut filter.operator,
                    op.clone(),
                    format!("{} {}", op.symbol(), op.display_name()),
                );
            }
            
            ui.separator();
            
            // 集合操作符
            ui.label(RichText::new("集合").small().color(GRAY));
            for op in FilterOperator::set_operators() {
                ui.selectable_value(
                    &mut filter.operator,
                    op.clone(),
                    format!("{} {}", op.symbol(), op.display_name()),
                );
            }
            
            ui.separator();
            
            // 空值操作符
            ui.label(RichText::new("空值").small().color(GRAY));
            for op in FilterOperator::null_operators() {
                ui.selectable_value(
                    &mut filter.operator,
                    op.clone(),
                    format!("{} {}", op.symbol(), op.display_name()),
                );
            }
            
            ui.separator();
            
            // 正则
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
}

/// 显示值输入框
fn show_value_inputs(ui: &mut egui::Ui, filter: &mut ColumnFilter) {
    if !filter.operator.needs_value() {
        return;
    }

    // 检查正则表达式错误
    let regex_error = if filter.operator == FilterOperator::Regex {
        validate_regex(&filter.value)
    } else {
        None
    };
    
    let text_color = if regex_error.is_some() {
        Color32::from_rgb(255, 100, 100)
    } else {
        Color32::WHITE
    };
    
    let response = ui.add(
        TextEdit::singleline(&mut filter.value)
            .desired_width(120.0)
            .font(egui::TextStyle::Small)
            .text_color(text_color)
            .hint_text(filter.operator.value_hint()),
    );
    
    // 显示正则错误提示
    if let Some(err) = regex_error {
        response.on_hover_text(RichText::new(err).color(Color32::from_rgb(255, 100, 100)));
    }

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

/// 显示大小写敏感按钮
fn show_case_sensitivity_button(ui: &mut egui::Ui, filter: &mut ColumnFilter) {
    if !filter.operator.supports_case_sensitivity() {
        return;
    }

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

/// 显示逻辑关系按钮
fn show_logic_button(ui: &mut egui::Ui, filter: &mut ColumnFilter) {
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
