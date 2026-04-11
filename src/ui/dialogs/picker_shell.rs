use super::common::DialogContent;
use crate::ui::styles::theme_muted_text;
use crate::ui::text_entry_has_priority;
use eframe::egui::{self, Color32, Key, Modifiers, RichText, Stroke, Vec2};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PickerPaneFocus {
    #[default]
    Navigator,
    Items,
    Detail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerNavAction {
    MovePrev,
    MoveNext,
    Open,
    Back,
    FocusNext,
    FocusPrev,
}

pub struct PickerDialogShell;

impl PickerDialogShell {
    pub fn consume_nav_action(ctx: &egui::Context) -> Option<PickerNavAction> {
        if text_entry_has_priority(ctx) {
            return None;
        }

        ctx.input_mut(|input| {
            if input.consume_key(Modifiers::SHIFT, Key::Tab) {
                Some(PickerNavAction::FocusPrev)
            } else if input.consume_key(Modifiers::NONE, Key::Tab) {
                Some(PickerNavAction::FocusNext)
            } else if input.consume_key(Modifiers::NONE, Key::ArrowUp)
                || input.consume_key(Modifiers::NONE, Key::K)
            {
                Some(PickerNavAction::MovePrev)
            } else if input.consume_key(Modifiers::NONE, Key::ArrowDown)
                || input.consume_key(Modifiers::NONE, Key::J)
            {
                Some(PickerNavAction::MoveNext)
            } else if input.consume_key(Modifiers::NONE, Key::ArrowRight)
                || input.consume_key(Modifiers::NONE, Key::L)
                || input.consume_key(Modifiers::NONE, Key::Enter)
            {
                Some(PickerNavAction::Open)
            } else if input.consume_key(Modifiers::NONE, Key::ArrowLeft)
                || input.consume_key(Modifiers::NONE, Key::H)
            {
                Some(PickerNavAction::Back)
            } else {
                None
            }
        })
    }

    pub fn next_focus(current: PickerPaneFocus) -> PickerPaneFocus {
        match current {
            PickerPaneFocus::Navigator => PickerPaneFocus::Items,
            PickerPaneFocus::Items => PickerPaneFocus::Detail,
            PickerPaneFocus::Detail => PickerPaneFocus::Navigator,
        }
    }

    pub fn prev_focus(current: PickerPaneFocus) -> PickerPaneFocus {
        match current {
            PickerPaneFocus::Navigator => PickerPaneFocus::Detail,
            PickerPaneFocus::Items => PickerPaneFocus::Navigator,
            PickerPaneFocus::Detail => PickerPaneFocus::Items,
        }
    }

    pub fn breadcrumb(ui: &mut egui::Ui, segments: &[String]) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
            for (index, segment) in segments.iter().enumerate() {
                if index > 0 {
                    ui.label(RichText::new("/").small().weak());
                }
                ui.label(RichText::new(segment).small().monospace().weak());
            }
        });
    }

    pub fn split(
        ui: &mut egui::Ui,
        left_width: f32,
        middle_width: f32,
        left: impl FnOnce(&mut egui::Ui),
        middle: impl FnOnce(&mut egui::Ui),
        right: impl FnOnce(&mut egui::Ui),
    ) {
        let available_width = ui.available_width().max(420.0);
        let spacing = 12.0;
        let content_width = (available_width - spacing * 2.0).max(300.0);

        let left_ratio = (left_width / (left_width + middle_width + 360.0)).clamp(0.18, 0.3);
        let middle_ratio = (middle_width / (left_width + middle_width + 360.0)).clamp(0.24, 0.38);

        let left_width = content_width * left_ratio;
        let middle_width = content_width * middle_ratio;
        let right_width = (content_width - left_width - middle_width).max(0.0);

        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_width(left_width);
                    ui.set_max_width(left_width);
                    left(ui);
                },
            );

            ui.add_space(spacing);

            ui.allocate_ui_with_layout(
                Vec2::new(middle_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_width(middle_width);
                    ui.set_max_width(middle_width);
                    middle(ui);
                },
            );

            ui.add_space(spacing);

            ui.allocate_ui_with_layout(
                Vec2::new(right_width, ui.available_height()),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_min_width(right_width);
                    ui.set_max_width(right_width);
                    right(ui);
                },
            );
        });
    }

    pub fn pane(
        ui: &mut egui::Ui,
        title: &str,
        description: &str,
        focused: bool,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        let tint = focused.then(|| {
            ui.visuals()
                .selection
                .bg_fill
                .gamma_multiply(if ui.visuals().dark_mode { 0.14 } else { 0.09 })
        });

        DialogContent::card(ui, tint, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(title).strong());
                if focused {
                    ui.label(
                        RichText::new("当前焦点")
                            .small()
                            .color(ui.visuals().selection.stroke.color),
                    );
                }
            });
            if !description.is_empty() {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(description)
                        .small()
                        .color(theme_muted_text(ui.visuals())),
                );
            }
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            content(ui);
        });
    }

    pub fn section_label(ui: &mut egui::Ui, label: &str) {
        ui.label(RichText::new(label).small().strong().weak());
        ui.add_space(4.0);
    }

    pub fn entry(
        ui: &mut egui::Ui,
        id_source: impl Hash,
        opened: bool,
        selected: bool,
        title: &str,
        meta: Option<&str>,
        detail: Option<&str>,
    ) -> egui::Response {
        ui.push_id(id_source, |ui| {
            let fill = if opened {
                ui.visuals()
                    .selection
                    .bg_fill
                    .gamma_multiply(if ui.visuals().dark_mode { 0.24 } else { 0.16 })
            } else if selected {
                ui.visuals()
                    .selection
                    .bg_fill
                    .gamma_multiply(if ui.visuals().dark_mode { 0.14 } else { 0.1 })
            } else {
                Color32::TRANSPARENT
            };

            egui::Frame::NONE
                .fill(fill)
                .stroke(Stroke::new(
                    1.0,
                    if opened || selected {
                        ui.visuals().selection.stroke.color.gamma_multiply(0.45)
                    } else {
                        Color32::TRANSPARENT
                    },
                ))
                .corner_radius(egui::CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(10, 9))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let indicator = if opened {
                            "▸"
                        } else if selected {
                            "•"
                        } else {
                            " "
                        };
                        ui.label(
                            RichText::new(indicator)
                                .small()
                                .color(ui.visuals().selection.stroke.color),
                        );
                        ui.vertical(|ui| {
                            ui.label(RichText::new(title).strong().color(if opened {
                                ui.visuals().selection.stroke.color
                            } else {
                                ui.visuals().text_color()
                            }));
                            if let Some(meta) = meta
                                && !meta.is_empty()
                            {
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(meta)
                                        .small()
                                        .color(theme_muted_text(ui.visuals())),
                                );
                            }
                            if let Some(detail) = detail
                                && !detail.is_empty()
                            {
                                ui.add_space(2.0);
                                ui.label(
                                    RichText::new(detail)
                                        .small()
                                        .monospace()
                                        .color(theme_muted_text(ui.visuals())),
                                );
                            }
                        });
                    });
                })
                .response
                .interact(egui::Sense::click())
        })
        .inner
    }
}
