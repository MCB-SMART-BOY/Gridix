//! Right inspector tab strip and empty states.

use eframe::egui;

use crate::core::RightInspectorTab;

use super::surface::{SurfaceAction, surface_icon_button};

const CLOSE_RIGHT_INSPECTOR_ACTION: SurfaceAction = SurfaceAction::new(
    "close_right_inspector",
    "x",
    "隐藏 Inspector",
    "隐藏当前上下文的属性、结构、行、单元格和连接详情",
    Some("workbench.hideRightInspector"),
    None,
);

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkbenchRightInspectorResponse {
    pub selected_tab: Option<RightInspectorTab>,
    pub close_requested: bool,
}

pub struct WorkbenchRightInspector;

impl WorkbenchRightInspector {
    pub fn show_header(
        ui: &mut egui::Ui,
        active_tab: RightInspectorTab,
    ) -> WorkbenchRightInspectorResponse {
        let mut response = WorkbenchRightInspectorResponse::default();

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);

            for tab in right_inspector_tabs() {
                let is_active = tab == active_tab;
                let button = egui::Button::new(
                    egui::RichText::new(right_inspector_tab_label(tab))
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
                .min_size(egui::vec2(52.0, 24.0));

                if ui.add(button).clicked() {
                    response.selected_tab = Some(tab);
                }
            }

            if surface_icon_button(ui, CLOSE_RIGHT_INSPECTOR_ACTION) {
                response.close_requested = true;
            }
        });

        response
    }

    pub fn show_empty_state(ui: &mut egui::Ui, title: &str, detail: &str) {
        ui.vertical_centered(|ui| {
            ui.add_space(32.0);
            ui.heading(title);
            ui.add_space(8.0);
            ui.label(egui::RichText::new(detail).color(ui.visuals().weak_text_color()));
        });
    }
}

pub const fn right_inspector_tabs() -> [RightInspectorTab; 6] {
    [
        RightInspectorTab::Properties,
        RightInspectorTab::Schema,
        RightInspectorTab::Row,
        RightInspectorTab::Cell,
        RightInspectorTab::ErSelection,
        RightInspectorTab::Connection,
    ]
}

pub const fn right_inspector_tab_label(tab: RightInspectorTab) -> &'static str {
    match tab {
        RightInspectorTab::Properties => "属性",
        RightInspectorTab::Schema => "结构",
        RightInspectorTab::Row => "行",
        RightInspectorTab::Cell => "单元",
        RightInspectorTab::ErSelection => "ER",
        RightInspectorTab::Connection => "连接",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_inspector_tabs_cover_all_config_tabs() {
        assert_eq!(
            right_inspector_tabs(),
            [
                RightInspectorTab::Properties,
                RightInspectorTab::Schema,
                RightInspectorTab::Row,
                RightInspectorTab::Cell,
                RightInspectorTab::ErSelection,
                RightInspectorTab::Connection,
            ]
        );
    }
}
