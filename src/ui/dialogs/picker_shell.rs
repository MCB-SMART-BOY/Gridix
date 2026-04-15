use super::common::{DialogContent, DialogShortcutContext};
use crate::ui::LocalShortcut;
use crate::ui::styles::theme_muted_text;
use eframe::egui::{self, Color32, RichText, Stroke, Vec2};
use std::hash::Hash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerPaneMode {
    Full,
    Compact,
    Hidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayeredPickerLayout {
    pub navigator: PickerPaneMode,
    pub items: PickerPaneMode,
    pub detail: PickerPaneMode,
}

impl LayeredPickerLayout {
    pub const fn new(
        navigator: PickerPaneMode,
        items: PickerPaneMode,
        detail: PickerPaneMode,
    ) -> Self {
        Self {
            navigator,
            items,
            detail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayeredPickerWidths {
    pub navigator_full: f32,
    pub navigator_compact: f32,
    pub items_full: f32,
    pub items_compact: f32,
}

impl LayeredPickerWidths {
    pub const fn new(
        navigator_full: f32,
        navigator_compact: f32,
        items_full: f32,
        items_compact: f32,
    ) -> Self {
        Self {
            navigator_full,
            navigator_compact,
            items_full,
            items_compact,
        }
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerHeaderBlocksLayout {
    Inline,
    Stacked,
}

pub struct PickerDialogShell;

impl PickerDialogShell {
    const HEADER_BLOCKS_INLINE_MIN_WIDTH: f32 = 860.0;

    #[cfg(test)]
    fn split_widths(available_width: f32, left_width: f32, middle_width: f32) -> (f32, f32, f32) {
        let spacing = 12.0;
        let content_width = (available_width.max(0.0) - spacing * 2.0).max(0.0);
        if content_width <= 0.0 {
            return (0.0, 0.0, 0.0);
        }

        let total = (left_width + middle_width + 360.0).max(1.0);
        let left_ratio = (left_width / total).clamp(0.18, 0.3);
        let middle_ratio = (middle_width / total).clamp(0.24, 0.38);

        let left_width = (content_width * left_ratio).clamp(0.0, content_width);
        let middle_width =
            (content_width * middle_ratio).clamp(0.0, (content_width - left_width).max(0.0));
        let right_width = (content_width - left_width - middle_width).max(0.0);

        (left_width, middle_width, right_width)
    }

    pub fn consume_nav_action(ctx: &egui::Context) -> Option<PickerNavAction> {
        DialogShortcutContext::new(ctx).resolve(&[
            (LocalShortcut::PickerFocusPrev, PickerNavAction::FocusPrev),
            (LocalShortcut::PickerFocusNext, PickerNavAction::FocusNext),
            (LocalShortcut::PickerMovePrev, PickerNavAction::MovePrev),
            (LocalShortcut::PickerMoveNext, PickerNavAction::MoveNext),
            (LocalShortcut::PickerOpen, PickerNavAction::Open),
            (LocalShortcut::PickerBack, PickerNavAction::Back),
        ])
    }

    pub fn consume_detail_nav_action(ctx: &egui::Context) -> Option<PickerNavAction> {
        DialogShortcutContext::new(ctx).resolve(&[
            (LocalShortcut::PickerFocusPrev, PickerNavAction::FocusPrev),
            (LocalShortcut::PickerFocusNext, PickerNavAction::FocusNext),
            (LocalShortcut::PickerBack, PickerNavAction::Back),
        ])
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

    pub fn header_blocks_layout(available_width: f32) -> PickerHeaderBlocksLayout {
        if available_width >= Self::HEADER_BLOCKS_INLINE_MIN_WIDTH {
            PickerHeaderBlocksLayout::Inline
        } else {
            PickerHeaderBlocksLayout::Stacked
        }
    }

    pub fn header_blocks(
        ui: &mut egui::Ui,
        layout: PickerHeaderBlocksLayout,
        left: impl FnOnce(&mut egui::Ui),
        right: impl FnOnce(&mut egui::Ui),
    ) {
        const SPACING: f32 = 12.0;

        match layout {
            PickerHeaderBlocksLayout::Inline => {
                let available_width = ui.available_width().max(0.0);
                let right_width = (available_width * 0.38).clamp(280.0, 420.0);
                let left_width = (available_width - SPACING - right_width).max(0.0);
                let mut left = Some(left);
                let mut right = Some(right);

                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(left_width);
                            DialogContent::toolbar(ui, |ui| {
                                left.take().expect("left header block should run once")(ui);
                            });
                        },
                    );

                    ui.add_space(SPACING);

                    ui.allocate_ui_with_layout(
                        Vec2::new(right_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(right_width);
                            DialogContent::toolbar(ui, |ui| {
                                right.take().expect("right header block should run once")(ui);
                            });
                        },
                    );
                });
            }
            PickerHeaderBlocksLayout::Stacked => {
                DialogContent::toolbar(ui, left);
                ui.add_space(8.0);
                DialogContent::toolbar(ui, right);
            }
        }
    }

    fn width_for_mode(mode: PickerPaneMode, full: f32, compact: f32) -> f32 {
        match mode {
            PickerPaneMode::Full => full,
            PickerPaneMode::Compact => compact,
            PickerPaneMode::Hidden => 0.0,
        }
    }

    fn layered_widths(
        available_width: f32,
        layout: LayeredPickerLayout,
        left_full: f32,
        left_compact: f32,
        middle_full: f32,
        middle_compact: f32,
    ) -> (f32, f32, f32) {
        let spacing = 12.0;
        let visible_count = [
            layout.navigator != PickerPaneMode::Hidden,
            layout.items != PickerPaneMode::Hidden,
            layout.detail != PickerPaneMode::Hidden,
        ]
        .into_iter()
        .filter(|visible| *visible)
        .count();

        if visible_count == 0 {
            return (0.0, 0.0, 0.0);
        }

        let content_width = (available_width.max(0.0)
            - spacing * (visible_count.saturating_sub(1) as f32))
            .max(0.0);
        let mut remaining = content_width;

        let mut left =
            Self::width_for_mode(layout.navigator, left_full, left_compact).min(remaining);
        remaining -= left;

        let mut middle =
            Self::width_for_mode(layout.items, middle_full, middle_compact).min(remaining);
        remaining -= middle;

        let mut right = if layout.detail == PickerPaneMode::Hidden {
            0.0
        } else {
            remaining
        };

        if layout.detail == PickerPaneMode::Hidden {
            if layout.items != PickerPaneMode::Hidden {
                middle += remaining;
            } else if layout.navigator != PickerPaneMode::Hidden {
                left += remaining;
            }
        } else if layout.items == PickerPaneMode::Hidden {
            right += remaining;
        }

        (left, middle, right)
    }

    pub fn split_layered(
        ui: &mut egui::Ui,
        layout: LayeredPickerLayout,
        widths: LayeredPickerWidths,
        left: impl FnOnce(&mut egui::Ui),
        middle: impl FnOnce(&mut egui::Ui),
        right: impl FnOnce(&mut egui::Ui),
    ) {
        let spacing = 12.0;
        let (left_width, middle_width, right_width) = Self::layered_widths(
            ui.available_width(),
            layout,
            widths.navigator_full,
            widths.navigator_compact,
            widths.items_full,
            widths.items_compact,
        );

        let mut left = Some(left);
        let mut middle = Some(middle);
        let mut right = Some(right);
        let mut rendered_any = false;

        ui.horizontal_top(|ui| {
            if layout.navigator != PickerPaneMode::Hidden {
                ui.allocate_ui_with_layout(
                    Vec2::new(left_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(left_width);
                        left.take().expect("left pane closure should only run once")(ui);
                    },
                );
                rendered_any = true;
            }

            if layout.items != PickerPaneMode::Hidden {
                if rendered_any {
                    ui.add_space(spacing);
                }
                ui.allocate_ui_with_layout(
                    Vec2::new(middle_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(middle_width);
                        middle
                            .take()
                            .expect("middle pane closure should only run once")(
                            ui
                        );
                    },
                );
                rendered_any = true;
            }

            if layout.detail != PickerPaneMode::Hidden {
                if rendered_any {
                    ui.add_space(spacing);
                }
                ui.allocate_ui_with_layout(
                    Vec2::new(right_width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(right_width);
                        right
                            .take()
                            .expect("right pane closure should only run once")(
                            ui
                        );
                    },
                );
            }
        });
    }

    pub fn pane(
        ui: &mut egui::Ui,
        title: &str,
        description: &str,
        focused: bool,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        Self::pane_with_mode(
            ui,
            title,
            description,
            focused,
            PickerPaneMode::Full,
            content,
        );
    }

    pub fn pane_with_mode(
        ui: &mut egui::Ui,
        title: &str,
        description: &str,
        focused: bool,
        mode: PickerPaneMode,
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
            if mode == PickerPaneMode::Full && !description.is_empty() {
                ui.add_space(2.0);
                ui.label(
                    RichText::new(description)
                        .small()
                        .color(theme_muted_text(ui.visuals())),
                );
            }
            ui.add_space(if mode == PickerPaneMode::Compact {
                6.0
            } else {
                8.0
            });
            if mode == PickerPaneMode::Full {
                ui.separator();
                ui.add_space(8.0);
            }
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
                    ui.set_width(ui.available_width());
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

    pub fn reveal_selected(response: &egui::Response, selected: bool) {
        if selected {
            response.scroll_to_me(Some(egui::Align::Center));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LayeredPickerLayout, PickerDialogShell, PickerHeaderBlocksLayout, PickerPaneMode};

    #[test]
    fn split_widths_never_exceed_available_width_when_narrow() {
        let available_width = 320.0;
        let (left, middle, right) = PickerDialogShell::split_widths(available_width, 220.0, 280.0);

        assert!(left >= 0.0);
        assert!(middle >= 0.0);
        assert!(right >= 0.0);
        assert!(left + middle + right + 24.0 <= available_width + f32::EPSILON);
    }

    #[test]
    fn split_widths_preserve_three_panes_without_forcing_growth() {
        let available_width = 960.0;
        let (left, middle, right) = PickerDialogShell::split_widths(available_width, 250.0, 330.0);

        assert!(left > 0.0);
        assert!(middle > 0.0);
        assert!(right > 0.0);
        assert!(left + middle + right + 24.0 <= available_width + f32::EPSILON);
    }

    #[test]
    fn layered_widths_hide_navigation_without_overflow() {
        let available_width = 820.0;
        let layout = LayeredPickerLayout::new(
            PickerPaneMode::Hidden,
            PickerPaneMode::Compact,
            PickerPaneMode::Full,
        );

        let (left, middle, right) =
            PickerDialogShell::layered_widths(available_width, layout, 250.0, 108.0, 330.0, 220.0);

        assert_eq!(left, 0.0);
        assert!(middle > 0.0);
        assert!(right > 0.0);
        assert!(middle + right + 12.0 <= available_width + f32::EPSILON);
    }

    #[test]
    fn layered_widths_compact_previous_levels_before_detail() {
        let available_width = 1200.0;
        let layout = LayeredPickerLayout::new(
            PickerPaneMode::Compact,
            PickerPaneMode::Compact,
            PickerPaneMode::Full,
        );

        let (left, middle, right) =
            PickerDialogShell::layered_widths(available_width, layout, 250.0, 108.0, 330.0, 220.0);

        assert_eq!(left, 108.0);
        assert_eq!(middle, 220.0);
        assert!(right > middle);
        assert!(left + middle + right + 24.0 <= available_width + f32::EPSILON);
    }

    #[test]
    fn header_blocks_layout_prefers_inline_when_width_allows() {
        assert_eq!(
            PickerDialogShell::header_blocks_layout(900.0),
            PickerHeaderBlocksLayout::Inline
        );
    }

    #[test]
    fn header_blocks_layout_stacks_when_width_is_narrow() {
        assert_eq!(
            PickerDialogShell::header_blocks_layout(820.0),
            PickerHeaderBlocksLayout::Stacked
        );
    }
}
