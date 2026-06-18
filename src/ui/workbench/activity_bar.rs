//! ActivityBar for switching primary sidebar activities.

use eframe::egui;

use crate::core::WorkbenchActivity;

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkbenchActivityBarResponse {
    pub selected_activity: Option<WorkbenchActivity>,
    pub toggle_sidebar: bool,
}

pub struct WorkbenchActivityBar;

impl WorkbenchActivityBar {
    pub fn show(
        ui: &mut egui::Ui,
        active_activity: WorkbenchActivity,
        sidebar_visible: bool,
    ) -> WorkbenchActivityBarResponse {
        let mut response = WorkbenchActivityBarResponse::default();

        egui::Frame::NONE
            .fill(ui.visuals().extreme_bg_color)
            .inner_margin(egui::Margin::symmetric(4, 8))
            .show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(0.0, 6.0);

                    for (activity, label, tooltip) in activity_items() {
                        let is_active = activity == active_activity;
                        let text =
                            egui::RichText::new(label)
                                .size(12.0)
                                .strong()
                                .color(if is_active {
                                    ui.visuals().selection.stroke.color
                                } else {
                                    ui.visuals().weak_text_color()
                                });
                        let button = egui::Button::new(text)
                            .fill(if is_active {
                                ui.visuals().selection.bg_fill
                            } else {
                                egui::Color32::TRANSPARENT
                            })
                            .stroke(if is_active {
                                ui.visuals().selection.stroke
                            } else {
                                egui::Stroke::NONE
                            })
                            .corner_radius(egui::CornerRadius::same(8))
                            .min_size(egui::vec2(38.0, 34.0));

                        if ui.add(button).on_hover_text(tooltip).clicked() {
                            if is_active && sidebar_visible {
                                response.toggle_sidebar = true;
                            } else {
                                response.selected_activity = Some(activity);
                            }
                        }
                    }
                });
            });

        response
    }
}

fn activity_items() -> [(WorkbenchActivity, &'static str, &'static str); 6] {
    [
        (WorkbenchActivity::Explorer, "探索", "连接、数据库和表"),
        (WorkbenchActivity::Filters, "筛选", "当前结果筛选条件"),
        (WorkbenchActivity::Objects, "对象", "触发器和存储过程"),
        (WorkbenchActivity::History, "历史", "查询历史入口"),
        (WorkbenchActivity::Help, "帮助", "学习与帮助入口"),
        (WorkbenchActivity::Settings, "设置", "设置入口"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_items_cover_all_workbench_activities() {
        let items = activity_items();

        assert_eq!(items.len(), 6);
        assert!(
            items
                .iter()
                .any(|(activity, _, _)| *activity == WorkbenchActivity::Explorer)
        );
        assert!(
            items
                .iter()
                .any(|(activity, _, _)| *activity == WorkbenchActivity::Filters)
        );
        assert!(
            items
                .iter()
                .any(|(activity, _, _)| *activity == WorkbenchActivity::Objects)
        );
        assert!(
            items
                .iter()
                .any(|(activity, _, _)| *activity == WorkbenchActivity::History)
        );
        assert!(
            items
                .iter()
                .any(|(activity, _, _)| *activity == WorkbenchActivity::Help)
        );
        assert!(
            items
                .iter()
                .any(|(activity, _, _)| *activity == WorkbenchActivity::Settings)
        );
    }
}
