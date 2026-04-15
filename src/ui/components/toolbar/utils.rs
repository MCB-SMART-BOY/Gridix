use crate::ui::styles::{
    theme_accent, theme_disabled_text, theme_selection_fill, theme_subtle_stroke, theme_text,
};
use egui::{RichText, Vec2};

#[derive(Debug, Clone, Copy)]
struct ToolbarButtonChrome {
    text_color: egui::Color32,
    fill: Option<egui::Color32>,
    stroke: Option<egui::Stroke>,
    frame_when_inactive: bool,
    selected: bool,
}

fn toolbar_button_chrome(
    visuals: &egui::Visuals,
    enabled: bool,
    is_selected: bool,
) -> ToolbarButtonChrome {
    let text_color = if is_selected {
        theme_accent(visuals)
    } else if enabled {
        theme_text(visuals)
    } else {
        theme_disabled_text(visuals)
    };

    let fill = is_selected.then(|| theme_selection_fill(visuals, 56));
    let stroke = is_selected.then(|| egui::Stroke::new(1.0, theme_accent(visuals)));

    ToolbarButtonChrome {
        text_color,
        fill,
        stroke,
        frame_when_inactive: is_selected,
        selected: is_selected,
    }
}

/// 工具栏图标按钮 - 默认透明，仅在 hover/focus/selected 时显示 chrome
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
    let chrome = toolbar_button_chrome(visuals, enabled, is_selected);

    let mut button = egui::Button::new(RichText::new(icon).size(15.0).color(chrome.text_color))
        .min_size(Vec2::new(24.0, 24.0))
        .frame(true)
        .frame_when_inactive(chrome.frame_when_inactive)
        .selected(chrome.selected);
    if let Some(fill) = chrome.fill {
        button = button.fill(fill);
    }
    if let Some(stroke) = chrome.stroke {
        button = button.stroke(stroke);
    }

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

#[cfg(test)]
mod tests {
    use super::toolbar_button_chrome;
    use crate::ui::styles::{theme_accent, theme_selection_fill};

    #[test]
    fn toolbar_button_chrome_hides_inactive_frame_for_unselected_buttons() {
        let visuals = egui::Visuals::dark();
        let chrome = toolbar_button_chrome(&visuals, true, false);

        assert!(!chrome.frame_when_inactive);
        assert!(!chrome.selected);
        assert!(chrome.fill.is_none());
        assert!(chrome.stroke.is_none());
    }

    #[test]
    fn toolbar_button_chrome_keeps_selected_state_visible_when_inactive() {
        let visuals = egui::Visuals::dark();
        let chrome = toolbar_button_chrome(&visuals, true, true);

        assert!(chrome.frame_when_inactive);
        assert!(chrome.selected);
        assert_eq!(chrome.fill, Some(theme_selection_fill(&visuals, 56)));
        let stroke = chrome.stroke.expect("selected toolbar button stroke");
        assert_eq!(stroke.width, 1.0);
        assert_eq!(stroke.color, theme_accent(&visuals));
    }
}
