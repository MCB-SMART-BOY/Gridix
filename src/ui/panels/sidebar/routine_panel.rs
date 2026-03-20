//! 存储过程/函数面板渲染

use super::SidebarPanelState;
use crate::database::{RoutineInfo, RoutineType};
use crate::ui::SidebarSection;
use crate::ui::styles::{GRAY, MARGIN_SM, MUTED, SPACING_LG, SPACING_SM, SUCCESS};
use egui::{self, Color32, CornerRadius, RichText, Vec2};

/// 存储过程/函数面板
pub struct RoutinePanel;

impl RoutinePanel {
    /// 显示存储过程/函数面板
    pub fn show(
        ui: &mut egui::Ui,
        is_focused: bool,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        height: f32,
    ) {
        // 标题栏
        ui.horizontal(|ui| {
            let routine_count = panel_state.routines.len();
            let proc_count = panel_state
                .routines
                .iter()
                .filter(|r| r.routine_type == RoutineType::Procedure)
                .count();
            let func_count = routine_count - proc_count;

            let title = if routine_count > 0 {
                format!("过程/函数 ({}P/{}F)", proc_count, func_count)
            } else {
                "过程/函数".to_string()
            };

            ui.label(RichText::new(title).strong());

            // 显示当前焦点区域提示
            if is_focused && focused_section == SidebarSection::Routines {
                ui.label(RichText::new("*").small().color(SUCCESS));
            }

            // 加载指示器
            if panel_state.loading_routines {
                ui.spinner();
            }
        });

        ui.separator();

        // 存储过程/函数列表
        let scroll_width = ui.available_width();
        let highlight_routines = is_focused && focused_section == SidebarSection::Routines;
        let selected_idx = panel_state.selection.routines;

        egui::ScrollArea::vertical()
            .id_salt("routine_scroll")
            .max_height(height - 30.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(scroll_width);
                if panel_state.routines.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(SPACING_LG);
                        ui.label(RichText::new("暂无存储过程/函数").small().color(MUTED));
                        ui.add_space(SPACING_SM);
                        ui.label(RichText::new("SQLite 不支持存储过程").small().color(GRAY));
                    });
                } else {
                    for (idx, routine) in panel_state.routines.iter().enumerate() {
                        let is_nav_selected = highlight_routines && idx == selected_idx;

                        let response = Self::show_routine_item(ui, routine, is_nav_selected);

                        // 如果是选中项且有焦点，滚动到可见
                        if is_nav_selected && highlight_routines {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                }
            });
    }

    /// 显示单个存储过程/函数项，返回 Response 用于滚动控制
    fn show_routine_item(
        ui: &mut egui::Ui,
        routine: &RoutineInfo,
        is_nav_selected: bool,
    ) -> egui::Response {
        let bg_color = if is_nav_selected {
            Color32::from_rgba_unmultiplied(100, 150, 255, 35)
        } else {
            Color32::TRANSPARENT
        };

        // 根据类型选择图标和颜色
        let (icon, type_color) = match routine.routine_type {
            RoutineType::Procedure => ("P", Color32::from_rgb(100, 180, 255)),
            RoutineType::Function => ("F", Color32::from_rgb(180, 255, 100)),
        };

        let response = egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(MARGIN_SM, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // 类型图标
                    let nav_icon = if is_nav_selected { ">" } else { " " };
                    let text_color = if is_nav_selected {
                        Color32::from_rgb(100, 180, 255)
                    } else {
                        Color32::from_rgb(180, 180, 190)
                    };

                    ui.label(RichText::new(nav_icon).color(text_color));
                    ui.label(RichText::new(icon).color(type_color).monospace());

                    ui.vertical(|ui| {
                        // 名称
                        ui.label(RichText::new(&routine.name).color(text_color));

                        // 参数和返回类型
                        let mut info_parts = Vec::new();
                        if !routine.parameters.is_empty() {
                            // 截断过长的参数列表
                            let params = if routine.parameters.len() > 40 {
                                format!("{}...", &routine.parameters[..37])
                            } else {
                                routine.parameters.clone()
                            };
                            info_parts.push(format!("({})", params));
                        } else {
                            info_parts.push("()".to_string());
                        }

                        if let Some(ret) = &routine.return_type {
                            info_parts.push(format!("-> {}", ret));
                        }

                        let info = info_parts.join(" ");
                        ui.label(RichText::new(info).small().color(MUTED));
                    });
                });
            })
            .response
            .interact(egui::Sense::click());

        // 右键菜单显示完整定义
        response.context_menu(|ui| {
            ui.label(RichText::new(format!("{} 定义", routine.routine_type)).strong());
            ui.separator();

            // 显示参数信息
            if !routine.parameters.is_empty() {
                ui.label(RichText::new("参数:").small());
                ui.label(RichText::new(&routine.parameters).small().color(MUTED));
                ui.separator();
            }

            // 使用 ScrollArea 显示完整定义
            egui::ScrollArea::vertical()
                .max_height(250.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut routine.definition.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(350.0)
                            .interactive(false),
                    );
                });

            ui.separator();
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("📋 复制")
                            .size(13.0)
                            .color(Color32::LIGHT_GRAY),
                    )
                    .frame(false)
                    .min_size(Vec2::new(0.0, 24.0)),
                )
                .on_hover_text("复制 SQL")
                .clicked()
            {
                ui.ctx().copy_text(routine.definition.clone());
                ui.close();
            }
        });

        // 悬停显示完整参数
        if !routine.parameters.is_empty() && routine.parameters.len() > 40 {
            response.clone().on_hover_text(&routine.parameters);
        }

        response
    }
}
