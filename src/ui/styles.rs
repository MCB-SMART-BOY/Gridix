//! 全局样式常量与主题语义色 helper

#![allow(dead_code)] // 公开 API

use egui::{Color32, CornerRadius, Visuals};

// 颜色常量
pub const SUCCESS: Color32 = Color32::from_rgb(82, 196, 106);
pub const DANGER: Color32 = Color32::from_rgb(235, 87, 87);
pub const GRAY: Color32 = Color32::from_rgb(140, 140, 150);
pub const MUTED: Color32 = Color32::from_rgb(100, 100, 110);
/// 全局强调蓝色 (用于 toolbar avatar、spinner、进度条等)
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(100, 149, 237);

// 间距常量 (f32 用于 add_space 等)
pub const SPACING_SM: f32 = 4.0;
pub const SPACING_MD: f32 = 8.0;
pub const SPACING_LG: f32 = 12.0;

// 间距常量 (i8 用于 Margin)
pub const MARGIN_SM: i8 = 4;
pub const MARGIN_MD: i8 = 8;
pub const MARGIN_LG: i8 = 12;

// 圆角 Token (CornerRadius)
pub const RADIUS_SM: CornerRadius = CornerRadius::same(4);
pub const RADIUS_MD: CornerRadius = CornerRadius::same(8);
pub const RADIUS_LG: CornerRadius = CornerRadius::same(12);
pub const RADIUS_XL: CornerRadius = CornerRadius::same(16);

// 字体大小 Token
pub const FONT_XS: f32 = 10.0;
pub const FONT_SM: f32 = 12.0;
pub const FONT_MD: f32 = 14.0;
pub const FONT_LG: f32 = 18.0;
pub const FONT_XL: f32 = 24.0;

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

/// 主题错误色（红）。`error_fg_color` 已由主题系统从 `ThemeColors.error` 映射。
pub fn theme_error(visuals: &Visuals) -> Color32 {
    visuals.error_fg_color
}

/// 主题成功色（绿）。egui Visuals 无专用槽，按明暗模式给出对比安全的绿色。
pub fn theme_success(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(126, 211, 138)
    } else {
        Color32::from_rgb(39, 145, 75)
    }
}

/// 在给定饱和填充色上保证可读的前景色（按相对亮度选黑或白）。
/// 用于"白字压在危险红底"这类需要随主题填充色变化的场景。
pub fn contrasting_text(fill: Color32) -> Color32 {
    // Rec. 601 亮度近似，阈值 0.55 偏向深底用白字。
    let luma = 0.299 * fill.r() as f32 + 0.587 * fill.g() as f32 + 0.114 * fill.b() as f32;
    if luma > 140.0 {
        Color32::from_rgb(20, 20, 24)
    } else {
        Color32::from_rgb(245, 245, 248)
    }
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

    #[test]
    fn theme_error_follows_visuals_and_success_varies_by_mode() {
        let light = Visuals::light();
        let dark = Visuals::dark();
        assert_eq!(theme_error(&dark), dark.error_fg_color);
        // 成功色随明暗模式不同，保证两种背景下都可读。
        assert_ne!(theme_success(&light), theme_success(&dark));
    }

    #[test]
    fn contrasting_text_picks_readable_foreground() {
        // 浅底 → 深字；深底 → 浅字。
        let on_light = contrasting_text(Color32::from_rgb(230, 230, 230));
        let on_dark = contrasting_text(Color32::from_rgb(150, 50, 50));
        assert!(on_light.r() < 60, "light fill must get dark text");
        assert!(on_dark.r() > 200, "dark/saturated fill must get light text");
    }
}
