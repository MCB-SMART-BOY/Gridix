use crate::ui::styles::{
    MUTED, theme_accent, theme_disabled_text, theme_muted_text, theme_selection_fill,
    theme_subtle_stroke, theme_text, theme_warn,
};
use egui::{Color32, RichText, Vec2};

/// 无边框图标按钮 - 统一样式
/// 鼠标悬停显示 tooltip（内容 + 快捷键）
pub fn icon_button(ui: &mut egui::Ui, icon: &str, tooltip: &str, enabled: bool) -> bool {
    icon_button_with_focus(ui, icon, tooltip, enabled, false)
}

/// 带焦点状态的图标按钮
/// is_selected: 当工具栏有焦点且此按钮被选中时为 true
pub fn icon_button_with_focus(
    ui: &mut egui::Ui,
    icon: &str,
    tooltip: &str,
    enabled: bool,
    is_selected: bool,
) -> bool {
    let visuals = ui.visuals();
    let color = if is_selected {
        theme_accent(visuals)
    } else if enabled {
        theme_text(visuals)
    } else {
        theme_disabled_text(visuals)
    };

    let button = egui::Button::new(RichText::new(icon).size(15.0).color(color))
        .min_size(Vec2::new(24.0, 24.0));

    // 选中时显示边框
    let button = if is_selected {
        button.stroke(egui::Stroke::new(1.0, theme_accent(visuals)))
    } else {
        button.frame(false)
    };

    ui.add_enabled(enabled, button)
        .on_hover_text(tooltip)
        .clicked()
}

/// 纯文字按钮（无边框）
pub fn text_button(ui: &mut egui::Ui, text: &str, tooltip: &str, enabled: bool) -> bool {
    let visuals = ui.visuals();
    let color = if enabled {
        theme_text(visuals)
    } else {
        theme_disabled_text(visuals)
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(text).size(13.0).color(color))
            .frame(false)
            .min_size(Vec2::new(0.0, 24.0)),
    )
    .on_hover_text(tooltip)
    .clicked()
}

/// 分隔符
pub fn separator(ui: &mut egui::Ui) {
    ui.add_space(2.0);
    let rect = ui.available_rect_before_wrap();
    let height = 20.0;
    let y_center = rect.center().y;
    ui.painter().vline(
        rect.left(),
        (y_center - height / 2.0)..=(y_center + height / 2.0),
        egui::Stroke::new(1.0, theme_subtle_stroke(ui.visuals())),
    );
    ui.add_space(2.0);
}

/// 渲染菜单项
pub fn render_menu_item(
    ui: &mut egui::Ui,
    label: &str,
    shortcut: &str,
    is_selected: bool,
    enabled: bool,
) -> egui::Response {
    let visuals = ui.visuals();
    let is_dark = visuals.dark_mode;
    let accent_color = theme_accent(visuals);
    let enabled_text_color = theme_text(visuals);
    let muted_text_color = theme_muted_text(visuals);
    let disabled_text_color = theme_disabled_text(visuals);
    let bg_color = if is_selected {
        theme_selection_fill(visuals, if is_dark { 40 } else { 28 })
    } else {
        Color32::TRANSPARENT
    };

    let text_color = if !enabled {
        disabled_text_color
    } else if is_selected {
        enabled_text_color
    } else {
        muted_text_color
    };

    let frame_response = egui::Frame::NONE
        .fill(bg_color)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                // 选中指示器
                let indicator = if is_selected { ">" } else { " " };
                ui.label(RichText::new(indicator).color(accent_color).monospace());

                // 标签
                ui.label(RichText::new(label).color(text_color));

                // 快捷键（右对齐）
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(shortcut).small().color(MUTED));
                });
            });
        });

    frame_response.response.interact(egui::Sense::click())
}

/// 渲染下拉选项
pub fn render_combo_item(
    ui: &mut egui::Ui,
    text: &str,
    is_hover: bool,
    is_light_theme: bool,
) -> egui::Response {
    let visuals = ui.visuals();
    let is_dark = visuals.dark_mode;
    let accent_color = theme_accent(visuals);
    let hover_text_color = theme_text(visuals);
    let muted_text_color = theme_muted_text(visuals);
    let warn_color = theme_warn(visuals);
    let bg_color = if is_hover {
        theme_selection_fill(visuals, if is_dark { 40 } else { 28 })
    } else {
        Color32::TRANSPARENT
    };

    let frame_response = egui::Frame::NONE
        .fill(bg_color)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                // 选中指示器
                let indicator = if is_hover { ">" } else { " " };
                ui.label(RichText::new(indicator).color(accent_color).monospace());

                // 主题名称
                let text_color = if is_hover {
                    hover_text_color
                } else {
                    muted_text_color
                };
                ui.label(RichText::new(text).color(text_color));

                // 浅色主题标识
                if is_light_theme {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("日").small().color(warn_color));
                    });
                }
            });
        });

    frame_response.response.interact(egui::Sense::click())
}
