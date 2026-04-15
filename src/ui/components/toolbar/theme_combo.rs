use crate::core::ThemePreset;
use crate::ui::styles::theme_text;
use eframe::egui::{self, RichText, Vec2};

/// 顶部工具栏主题选择 trigger。
///
/// 真正的主题选择由显式 overlay dialog 负责；这里仅保留当前主题显示与鼠标入口。
pub fn show_theme_selector_trigger(
    ui: &mut egui::Ui,
    current_theme: ThemePreset,
    tooltip: &str,
) -> bool {
    ui.add(
        egui::Button::new(
            RichText::new(current_theme.display_name())
                .size(13.0)
                .color(theme_text(ui.visuals())),
        )
        .frame(true)
        .frame_when_inactive(false)
        .min_size(Vec2::new(0.0, 24.0)),
    )
    .on_hover_text(tooltip)
    .clicked()
}
