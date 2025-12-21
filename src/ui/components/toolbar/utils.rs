use crate::ui::styles::MUTED;
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
    let color = if is_selected {
        Color32::from_rgb(130, 180, 255) // 选中时高亮蓝色
    } else if enabled {
        Color32::LIGHT_GRAY
    } else {
        Color32::from_gray(60)
    };

    let button = egui::Button::new(RichText::new(icon).size(15.0).color(color))
        .min_size(Vec2::new(24.0, 24.0));

    // 选中时显示边框
    let button = if is_selected {
        button.stroke(egui::Stroke::new(1.0, Color32::from_rgb(100, 149, 237)))
    } else {
        button.frame(false)
    };

    ui.add_enabled(enabled, button)
        .on_hover_text(tooltip)
        .clicked()
}

/// 纯文字按钮（无边框）
pub fn text_button(ui: &mut egui::Ui, text: &str, tooltip: &str, enabled: bool) -> bool {
    let color = if enabled { Color32::LIGHT_GRAY } else { Color32::from_gray(60) };
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
        egui::Stroke::new(1.0, Color32::from_white_alpha(30)),
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
    let bg_color = if is_selected {
        Color32::from_rgba_unmultiplied(100, 140, 200, 40)
    } else {
        Color32::TRANSPARENT
    };
    
    let text_color = if !enabled {
        Color32::from_gray(100)
    } else if is_selected {
        Color32::from_rgb(200, 220, 255)
    } else {
        Color32::from_rgb(180, 180, 190)
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
                ui.label(RichText::new(indicator).color(Color32::from_rgb(130, 180, 255)).monospace());
                
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
    let bg_color = if is_hover {
        Color32::from_rgba_unmultiplied(100, 140, 200, 40)
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
                ui.label(
                    RichText::new(indicator)
                        .color(Color32::from_rgb(130, 180, 255))
                        .monospace(),
                );

                // 主题名称
                let text_color = if is_hover {
                    Color32::from_rgb(200, 220, 255)
                } else {
                    Color32::from_rgb(180, 180, 190)
                };
                ui.label(RichText::new(text).color(text_color));

                // 浅色主题标识
                if is_light_theme {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new("日").small().color(Color32::from_rgb(255, 200, 100)));
                    });
                }
            });
        });

    frame_response.response.interact(egui::Sense::click())
}
