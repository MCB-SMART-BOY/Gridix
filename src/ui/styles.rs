//! 全局样式常量与主题语义色 helper

#![allow(dead_code)] // 公开 API

use egui::{Color32, Visuals};

// 颜色常量
pub const SUCCESS: Color32 = Color32::from_rgb(82, 196, 106);
pub const DANGER: Color32 = Color32::from_rgb(235, 87, 87);
pub const GRAY: Color32 = Color32::from_rgb(140, 140, 150);
pub const MUTED: Color32 = Color32::from_rgb(100, 100, 110);

// 间距常量 (f32 用于 add_space 等)
pub const SPACING_SM: f32 = 4.0;
pub const SPACING_MD: f32 = 8.0;
pub const SPACING_LG: f32 = 12.0;

// 间距常量 (i8 用于 Margin)
pub const MARGIN_SM: i8 = 4;
pub const MARGIN_MD: i8 = 8;
pub const MARGIN_LG: i8 = 12;

pub fn theme_text(visuals: &Visuals) -> Color32 {
    visuals.text_color()
}

pub fn theme_muted_text(visuals: &Visuals) -> Color32 {
    visuals.weak_text_color()
}

pub fn theme_disabled_text(visuals: &Visuals) -> Color32 {
    visuals
        .weak_text_color()
        .gamma_multiply(if visuals.dark_mode { 0.65 } else { 0.85 })
}

pub fn theme_accent(visuals: &Visuals) -> Color32 {
    visuals.hyperlink_color
}

pub fn theme_warn(visuals: &Visuals) -> Color32 {
    visuals.warn_fg_color
}

pub fn theme_selection_fill(visuals: &Visuals, alpha: u8) -> Color32 {
    let color = visuals.selection.bg_fill;
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
}

pub fn theme_subtle_stroke(visuals: &Visuals) -> Color32 {
    visuals.widgets.noninteractive.bg_stroke.color
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_color_helpers_follow_visuals() {
        let light = Visuals::light();
        let dark = Visuals::dark();

        assert_eq!(theme_text(&light), light.text_color());
        assert_eq!(theme_muted_text(&light), light.weak_text_color());
        assert_eq!(theme_accent(&dark), dark.hyperlink_color);
        assert_eq!(theme_warn(&dark), dark.warn_fg_color);
        assert_eq!(
            theme_selection_fill(&light, 42),
            Color32::from_rgba_unmultiplied(
                light.selection.bg_fill.r(),
                light.selection.bg_fill.g(),
                light.selection.bg_fill.b(),
                42,
            )
        );
    }
}
