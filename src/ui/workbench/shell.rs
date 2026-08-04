//! Minimal workbench shell wrapper.
//!
//! This shell intentionally preserves the existing central layout. It only
//! reserves a bottom status-bar strip when status-bar content is provided.

use eframe::egui;

use super::status_bar::{WorkbenchStatusBarContent, show_status_bar};

#[derive(Debug, Clone, Default)]
pub struct WorkbenchShell {
    status_bar: Option<WorkbenchStatusBarContent>,
}

impl WorkbenchShell {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn status_bar(mut self, status_bar: Option<WorkbenchStatusBarContent>) -> Self {
        self.status_bar = status_bar;
        self
    }

    pub fn show_inside(
        self,
        root_ui: &mut egui::Ui,
        frame: egui::Frame,
        add_content: impl FnOnce(&mut egui::Ui),
    ) {
        egui::CentralPanel::default()
            .frame(frame)
            .show_inside(root_ui, |ui| {
                if let Some(status_bar) = self.status_bar {
                    let status_height = 24.0;
                    let content_height = (ui.available_height() - status_height).max(0.0);

                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), content_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        add_content,
                    );
                    show_status_bar(ui, &status_bar);
                } else {
                    add_content(ui);
                }
            });
    }
}
