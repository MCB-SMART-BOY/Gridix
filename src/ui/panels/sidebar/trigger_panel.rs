//! 触发器面板渲染

use super::SidebarPanelState;
use crate::database::TriggerInfo;
use crate::ui::SidebarSection;
use crate::ui::styles::{GRAY, MARGIN_SM, MUTED, SPACING_LG, SPACING_SM, SUCCESS, theme_text};
use egui::{self, Color32, CornerRadius, RichText, Vec2};

/// 触发器面板
pub struct TriggerPanel;

impl TriggerPanel {
    /// 显示下部面板（触发器）
    pub fn show(
        ui: &mut egui::Ui,
        is_focused: bool,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        height: f32,
    ) {
        // 标题栏
        ui.horizontal(|ui| {
            let trigger_count = panel_state.triggers.len();
            let title = if trigger_count > 0 {
                format!("触发器 ({})", trigger_count)
            } else {
                "触发器".to_string()
            };

            ui.label(RichText::new(title).strong());

            // 显示当前焦点区域提示
            if is_focused && focused_section == SidebarSection::Triggers {
                ui.label(RichText::new("*").small().color(SUCCESS));
            }

            // 加载指示器
            if panel_state.loading_triggers {
                ui.spinner();
            }
        });

        ui.separator();

        // 触发器列表 - 使用固定宽度防止内容扩展面板
        let scroll_width = ui.available_width();
        let highlight_triggers = is_focused && focused_section == SidebarSection::Triggers;
        let selected_idx = panel_state.selection.triggers;

        egui::ScrollArea::vertical()
            .id_salt("trigger_scroll")
            .max_height(height - 30.0)
            .auto_shrink([false, false]) // 不自动收缩，保持固定宽度
            .show(ui, |ui| {
                ui.set_max_width(scroll_width); // 限制内容最大宽度
                if panel_state.triggers.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(SPACING_LG);
                        ui.label(RichText::new("暂无触发器").small().color(MUTED));
                        ui.add_space(SPACING_SM);
                        ui.label(RichText::new("选择数据库后自动加载").small().color(GRAY));
                    });
                } else {
                    for (idx, trigger) in panel_state.triggers.iter().enumerate() {
                        let is_nav_selected = highlight_triggers && idx == selected_idx;

                        let response = Self::show_trigger_item(ui, trigger, is_nav_selected);

                        // 如果是选中项且有焦点，滚动到可见
                        if is_nav_selected && highlight_triggers {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                }
            });
    }

    /// 显示单个触发器项，返回 Response 用于滚动控制
    fn show_trigger_item(
        ui: &mut egui::Ui,
        trigger: &TriggerInfo,
        is_nav_selected: bool,
    ) -> egui::Response {
        let bg_color = if is_nav_selected {
            Color32::from_rgba_unmultiplied(100, 150, 255, 35) // 降低透明度
        } else {
            Color32::TRANSPARENT
        };

        let response = egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(MARGIN_SM, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // 图标
                    let icon = if is_nav_selected { ">" } else { "*" };
                    let text_color = if is_nav_selected {
                        Color32::from_rgb(100, 180, 255)
                    } else {
                        Color32::from_rgb(180, 180, 190)
                    };

                    ui.label(RichText::new(icon).color(text_color));

                    ui.vertical(|ui| {
                        // 触发器名称
                        ui.label(RichText::new(&trigger.name).color(text_color));

                        // 触发器信息：timing event ON table
                        let info = format!(
                            "{} {} ON {}",
                            trigger.timing, trigger.event, trigger.table_name
                        );
                        ui.label(RichText::new(info).small().color(MUTED));
                    });
                });
            })
            .response
            .interact(egui::Sense::click());

        // 右键菜单显示完整定义
        response.context_menu(|ui| {
            ui.label(RichText::new("触发器定义").strong());
            ui.separator();

            // 使用 ScrollArea 显示长定义
            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(&mut trigger.definition.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_width(300.0)
                            .interactive(false),
                    );
                });

            ui.separator();
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("📋 复制")
                            .size(13.0)
                            .color(theme_text(ui.visuals())),
                    )
                    .frame(false)
                    .min_size(Vec2::new(0.0, 24.0)),
                )
                .on_hover_text("复制 SQL")
                .clicked()
            {
                ui.ctx().copy_text(trigger.definition.clone());
                ui.close();
            }
        });

        response
    }
}
