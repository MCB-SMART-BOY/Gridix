//! Bottom panel tab strip and empty states.

use eframe::egui;

use crate::core::BottomPanelTab;

use super::surface::{SurfaceAction, surface_icon_button};

const CLOSE_BOTTOM_PANEL_ACTION: SurfaceAction = SurfaceAction::new(
    "close_bottom_panel",
    "x",
    "隐藏 BottomPanel",
    "隐藏结果、消息、历史和任务面板",
    Some("workbench.hideBottomPanel"),
    None,
);

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkbenchBottomPanelResponse {
    pub selected_tab: Option<BottomPanelTab>,
    pub close_requested: bool,
}

pub struct WorkbenchBottomPanel;

impl WorkbenchBottomPanel {
    pub fn show_header(
        ui: &mut egui::Ui,
        active_tab: BottomPanelTab,
    ) -> WorkbenchBottomPanelResponse {
        let mut response = WorkbenchBottomPanelResponse::default();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 0.0);

            for tab in bottom_panel_tabs() {
                let is_active = tab == active_tab;
                let button = egui::Button::new(
                    egui::RichText::new(bottom_panel_tab_label(tab))
                        .strong()
                        .color(if is_active {
                            ui.visuals().selection.stroke.color
                        } else {
                            ui.visuals().weak_text_color()
                        }),
                )
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
                .corner_radius(egui::CornerRadius::same(6))
                .min_size(egui::vec2(74.0, 26.0));

                if ui.add(button).clicked() {
                    response.selected_tab = Some(tab);
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if surface_icon_button(ui, CLOSE_BOTTOM_PANEL_ACTION) {
                    response.close_requested = true;
                }
            });
        });

        response
    }

    pub fn show_empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(36.0);
            ui.heading(title);
            ui.add_space(8.0);
            ui.label(egui::RichText::new(detail).color(ui.visuals().weak_text_color()));
        });
    }
}

pub const fn bottom_panel_tabs() -> [BottomPanelTab; 5] {
    [
        BottomPanelTab::Results,
        BottomPanelTab::Messages,
        BottomPanelTab::Explain,
        BottomPanelTab::History,
        BottomPanelTab::Tasks,
    ]
}

pub const fn bottom_panel_tab_label(tab: BottomPanelTab) -> &'static str {
    match tab {
        BottomPanelTab::Results => "结果",
        BottomPanelTab::Messages => "消息",
        BottomPanelTab::Explain => "Explain",
        BottomPanelTab::History => "历史",
        BottomPanelTab::Tasks => "任务",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bottom_panel_tabs_cover_all_config_tabs() {
        assert_eq!(
            bottom_panel_tabs(),
            [
                BottomPanelTab::Results,
                BottomPanelTab::Messages,
                BottomPanelTab::Explain,
                BottomPanelTab::History,
                BottomPanelTab::Tasks,
            ]
        );
    }
}
