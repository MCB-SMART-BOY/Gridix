//! 筛选条件面板
//!
//! 显示在左侧栏的筛选条件管理面板

use crate::ui::{ColumnFilter, FilterLogic, FilterOperator, SidebarSection};
use crate::ui::styles::{GRAY, MUTED, SUCCESS};
use egui::{self, Color32, RichText, Vec2, TextEdit, CornerRadius};

/// 筛选面板
pub struct FilterPanel;

impl FilterPanel {
    /// 显示筛选面板
    ///
    /// 返回是否有修改（用于使缓存失效）
    pub fn show(
        ui: &mut egui::Ui,
        is_focused: bool,
        focused_section: SidebarSection,
        filters: &mut Vec<ColumnFilter>,
        columns: &[String],
        height: f32,
    ) -> bool {
        let mut changed = false;
        let mut filter_to_remove: Option<usize> = None;

        // 标题栏
        ui.horizontal(|ui| {
            let filter_count = filters.iter().filter(|f| f.enabled).count();
            let title = if filter_count > 0 {
                format!("筛选 ({})", filter_count)
            } else {
                "筛选".to_string()
            };

            ui.label(RichText::new(title).size(12.0).strong());

            if is_focused && focused_section == SidebarSection::Filters {
                ui.label(RichText::new("*").small().color(SUCCESS));
            }

            // 工具按钮
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                
                // 添加按钮
                if !columns.is_empty() {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("+").size(13.0).color(Color32::from_rgb(100, 180, 100)))
                                .frame(false)
                                .min_size(Vec2::new(18.0, 18.0)),
                        )
                        .on_hover_text("添加筛选条件")
                        .clicked()
                    {
                        filters.push(ColumnFilter::new(columns.first().cloned().unwrap_or_default()));
                        changed = true;
                    }
                }
                
                // 清空按钮
                if !filters.is_empty() {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("×").size(13.0).color(Color32::from_rgb(160, 100, 100)))
                                .frame(false)
                                .min_size(Vec2::new(18.0, 18.0)),
                        )
                        .on_hover_text("清空全部")
                        .clicked()
                    {
                        filters.clear();
                        changed = true;
                    }
                }
            });
        });

        ui.add_space(2.0);
        
        // 分隔线
        let rect = ui.available_rect_before_wrap();
        ui.painter().line_segment(
            [egui::pos2(rect.left() + 4.0, rect.top()), egui::pos2(rect.right() - 4.0, rect.top())],
            egui::Stroke::new(1.0, Color32::from_gray(60)),
        );
        ui.add_space(4.0);

        // 筛选条件列表
        let scroll_height = (height - 36.0).max(60.0);
        egui::ScrollArea::vertical()
            .id_salt("filter_panel_scroll")
            .max_height(scroll_height)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if filters.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        if columns.is_empty() {
                            ui.label(RichText::new("请先查询数据").size(11.0).color(MUTED));
                        } else {
                            ui.label(RichText::new("点击 + 添加筛选").size(11.0).color(MUTED));
                        }
                    });
                } else {
                    let filters_len = filters.len();
                    for (idx, filter) in filters.iter_mut().enumerate() {
                        let is_last = idx == filters_len - 1;

                        ui.push_id(format!("filter_{}", idx), |ui| {
                            // 筛选条件行
                            let bg_color = if filter.enabled {
                                Color32::from_rgba_unmultiplied(50, 70, 90, 50)
                            } else {
                                Color32::from_rgba_unmultiplied(40, 40, 40, 30)
                            };
                            
                            egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(CornerRadius::same(3))
                                .inner_margin(egui::Margin::symmetric(6, 4))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    
                                    // 第一行：启用 + 列选择 + 操作符 + 删除
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 3.0;

                                        // 启用复选框
                                        let checkbox_response = ui.add(
                                            egui::Checkbox::without_text(&mut filter.enabled)
                                        );
                                        if checkbox_response.changed() {
                                            changed = true;
                                        }

                                        ui.add_enabled_ui(filter.enabled, |ui| {
                                            // 列选择
                                            egui::ComboBox::new(format!("col_{}", idx), "")
                                                .selected_text(RichText::new(truncate_str(&filter.column, 6)).size(10.0))
                                                .width(55.0)
                                                .show_ui(ui, |ui| {
                                                    for c in columns {
                                                        if ui.selectable_value(&mut filter.column, c.clone(), c).changed() {
                                                            changed = true;
                                                        }
                                                    }
                                                });

                                            // 操作符
                                            egui::ComboBox::new(format!("op_{}", idx), "")
                                                .selected_text(RichText::new(filter.operator.symbol()).size(10.0))
                                                .width(40.0)
                                                .show_ui(ui, |ui| {
                                                    ui.label(RichText::new("文本").size(10.0).color(GRAY));
                                                    for op in FilterOperator::text_operators() {
                                                        if ui.selectable_value(&mut filter.operator, op.clone(), op.display_name()).changed() {
                                                            changed = true;
                                                        }
                                                    }
                                                    ui.separator();
                                                    ui.label(RichText::new("比较").size(10.0).color(GRAY));
                                                    for op in FilterOperator::comparison_operators() {
                                                        if ui.selectable_value(&mut filter.operator, op.clone(), format!("{} {}", op.symbol(), op.display_name())).changed() {
                                                            changed = true;
                                                        }
                                                    }
                                                    ui.separator();
                                                    ui.label(RichText::new("空值").size(10.0).color(GRAY));
                                                    for op in FilterOperator::null_operators() {
                                                        if ui.selectable_value(&mut filter.operator, op.clone(), op.display_name()).changed() {
                                                            changed = true;
                                                        }
                                                    }
                                                });
                                        });

                                        // 删除按钮
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui
                                                .add(
                                                    egui::Button::new(RichText::new("×").size(12.0).color(Color32::from_gray(120)))
                                                        .frame(false)
                                                        .min_size(Vec2::new(16.0, 16.0)),
                                                )
                                                .on_hover_text("删除")
                                                .clicked()
                                            {
                                                filter_to_remove = Some(idx);
                                            }
                                        });
                                    });

                                    // 第二行：值输入（如果需要）
                                    if filter.operator.needs_value() {
                                        ui.add_space(3.0);
                                        ui.add_enabled_ui(filter.enabled, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.add_space(20.0);
                                                
                                                if ui
                                                    .add(
                                                        TextEdit::singleline(&mut filter.value)
                                                            .desired_width(ui.available_width() - 24.0)
                                                            .font(egui::TextStyle::Small)
                                                            .hint_text("值..."),
                                                    )
                                                    .changed()
                                                {
                                                    changed = true;
                                                }

                                                // 大小写切换
                                                if filter.operator.supports_case_sensitivity() {
                                                    let case_color = if filter.case_sensitive {
                                                        Color32::from_rgb(100, 160, 220)
                                                    } else {
                                                        Color32::from_gray(80)
                                                    };
                                                    if ui
                                                        .add(
                                                            egui::Button::new(RichText::new("Aa").size(9.0).color(case_color))
                                                                .frame(false)
                                                                .min_size(Vec2::new(16.0, 16.0)),
                                                        )
                                                        .on_hover_text(if filter.case_sensitive { "区分大小写" } else { "忽略大小写" })
                                                        .clicked()
                                                    {
                                                        filter.case_sensitive = !filter.case_sensitive;
                                                        changed = true;
                                                    }
                                                }
                                            });
                                        });
                                    }
                                });

                            // AND/OR 逻辑（非最后一条）
                            if !is_last {
                                ui.horizontal(|ui| {
                                    ui.add_space(12.0);
                                    let (logic_text, logic_color) = match filter.logic {
                                        FilterLogic::And => ("AND", Color32::from_rgb(80, 140, 80)),
                                        FilterLogic::Or => ("OR", Color32::from_rgb(180, 140, 60)),
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(RichText::new(logic_text).size(9.0).color(logic_color))
                                                .frame(false),
                                        )
                                        .on_hover_text("点击切换")
                                        .clicked()
                                    {
                                        filter.logic.toggle();
                                        changed = true;
                                    }
                                });
                            }
                            
                            ui.add_space(2.0);
                        });
                    }
                }
            });

        if let Some(idx) = filter_to_remove {
            filters.remove(idx);
            changed = true;
        }

        changed
    }
}

/// 截断字符串
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        format!("{}…", s.chars().take(max_len).collect::<String>())
    } else {
        s.to_string()
    }
}
