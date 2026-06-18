//! Workbench status bar.

use eframe::egui;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchStatusBarContent {
    pub status_line: String,
    pub query_time_ms: Option<u64>,
    pub row_count: Option<usize>,
}

impl WorkbenchStatusBarContent {
    pub fn new(status_line: impl Into<String>) -> Self {
        Self {
            status_line: status_line.into(),
            query_time_ms: None,
            row_count: None,
        }
    }
}

pub(crate) fn show_status_bar(ui: &mut egui::Ui, content: &WorkbenchStatusBarContent) {
    let visuals = ui.visuals().clone();
    let height = 24.0;
    let available_width = ui.available_width();

    egui::Frame::NONE
        .fill(visuals.extreme_bg_color)
        .inner_margin(egui::Margin::symmetric(10, 3))
        .show(ui, |ui| {
            ui.set_min_size(egui::vec2(available_width, height));
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(&content.status_line).small());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(row_count) = content.row_count {
                        ui.label(egui::RichText::new(format!("{} rows", row_count)).small());
                    }
                    if let Some(query_time_ms) = content.query_time_ms {
                        ui.label(egui::RichText::new(format!("{} ms", query_time_ms)).small());
                    }
                });
            });
        });
}
