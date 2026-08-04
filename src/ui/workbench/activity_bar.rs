//! Dormant ActivityBar widget for a future optional SurfaceRail.

use eframe::egui;

use crate::core::WorkbenchActivity;
use crate::state::WorkbenchSurfaceKind;

use super::surface::surface_icon_glyph;

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

                    for activity in activity_items() {
                        let surface = WorkbenchSurfaceKind::from_activity(activity);
                        let descriptor = surface.descriptor();
                        let is_active = activity == active_activity;
                        let text = egui::RichText::new(surface_icon_glyph(descriptor.icon))
                            .size(12.0)
                            .strong()
                            .monospace()
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

                        if ui.add(button).on_hover_text(descriptor.tooltip()).clicked() {
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

fn activity_items() -> [WorkbenchActivity; 6] {
    [
        WorkbenchActivity::Explorer,
        WorkbenchActivity::Filters,
        WorkbenchActivity::Objects,
        WorkbenchActivity::History,
        WorkbenchActivity::Help,
        WorkbenchActivity::Settings,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activity_items_cover_all_workbench_activities() {
        let items = activity_items();

        assert_eq!(items.len(), 6);
        assert!(items.contains(&WorkbenchActivity::Explorer));
        assert!(items.contains(&WorkbenchActivity::Filters));
        assert!(items.contains(&WorkbenchActivity::Objects));
        assert!(items.contains(&WorkbenchActivity::History));
        assert!(items.contains(&WorkbenchActivity::Help));
        assert!(items.contains(&WorkbenchActivity::Settings));
    }

    #[test]
    fn activity_items_reuse_surface_descriptor_tooltips() {
        for activity in activity_items() {
            let surface = WorkbenchSurfaceKind::from_activity(activity);
            let descriptor = surface.descriptor();
            let tooltip = descriptor.tooltip();

            assert!(tooltip.contains(&descriptor.title));
            assert!(tooltip.contains(descriptor.description));
            assert!(descriptor.command_id.is_some());
            assert_ne!(surface_icon_glyph(descriptor.icon), "•");
        }
    }
}
