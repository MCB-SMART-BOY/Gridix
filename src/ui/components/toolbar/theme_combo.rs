use crate::core::ThemePreset;
use crate::ui::styles::MUTED;
use egui::{Color32, CornerRadius, Id, Key, RichText, Vec2};

use super::actions::ThemeComboState;
use super::utils::render_combo_item;

// 暗色主题列表
pub const DARK_THEMES: &[ThemePreset] = &[
    ThemePreset::TokyoNight,
    ThemePreset::TokyoNightStorm,
    ThemePreset::CatppuccinMocha,
    ThemePreset::CatppuccinMacchiato,
    ThemePreset::CatppuccinFrappe,
    ThemePreset::OneDark,
    ThemePreset::OneDarkVivid,
    ThemePreset::GruvboxDark,
    ThemePreset::Dracula,
    ThemePreset::Nord,
    ThemePreset::SolarizedDark,
    ThemePreset::MonokaiPro,
    ThemePreset::GithubDark,
];

// 亮色主题列表
pub const LIGHT_THEMES: &[ThemePreset] = &[
    ThemePreset::TokyoNightLight,
    ThemePreset::CatppuccinLatte,
    ThemePreset::OneLight,
    ThemePreset::GruvboxLight,
    ThemePreset::SolarizedLight,
    ThemePreset::GithubLight,
];

/// 简化版主题选择下拉框（只显示指定的主题列表）
pub fn helix_theme_combo_simple(
    ui: &mut egui::Ui,
    id_source: &str,
    current_theme: ThemePreset,
    selected_index: usize,
    themes: &[ThemePreset],
    width: f32,
    force_open: bool,
) -> Option<usize> {
    let id = Id::new(id_source);
    let popup_id = id.with("popup");
    let mut result = None;

    // 获取状态
    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<ThemeComboState>(id).unwrap_or_default());

    // 快捷键触发打开
    if force_open {
        state.is_open = true;
    }

    // 同步选中索引
    if !state.is_open {
        state.selected_index = selected_index;
    }

    // 按钮 - 无边框图标样式
    let display_text = current_theme.display_name();
    let response = ui.add(
        egui::Button::new(RichText::new(display_text).size(13.0).color(Color32::LIGHT_GRAY))
            .frame(false)
            .min_size(Vec2::new(0.0, 24.0))
    ).on_hover_text("选择主题 (Ctrl+Shift+T)");

    if response.clicked() {
        state.is_open = !state.is_open;
    }

    // 弹出菜单
    if state.is_open {
        egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(
                response.rect.left_bottom()
                    - Vec2::new(width - response.rect.width(), -4.0),
            )
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(CornerRadius::same(8))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(width.min(200.0));
                        ui.set_max_width(width);

                        let themes_len = themes.len();

                        // 键盘处理
                        let input_result = ui.input(|i| {
                            let mut close = false;
                            let mut confirm = false;
                            let mut new_idx: Option<usize> = None;

                            if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                                let next = state.selected_index.saturating_add(1);
                                if next < themes_len {
                                    new_idx = Some(next);
                                }
                            }

                            if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp))
                                && state.selected_index > 0
                            {
                                new_idx = Some(state.selected_index - 1);
                            }

                            if i.key_pressed(Key::Enter) || i.key_pressed(Key::L) {
                                confirm = true;
                                close = true;
                            }

                            if i.key_pressed(Key::Escape) || i.key_pressed(Key::H) {
                                close = true;
                            }

                            if i.key_pressed(Key::G) && !i.modifiers.shift {
                                new_idx = Some(0);
                            }

                            if i.key_pressed(Key::G) && i.modifiers.shift {
                                new_idx = Some(themes_len.saturating_sub(1));
                            }

                            (close, confirm, new_idx)
                        });

                        if let Some(new_idx) = input_result.2 {
                            state.selected_index = new_idx;
                        }

                        if input_result.0 {
                            if input_result.1 {
                                result = Some(state.selected_index);
                            }
                            state.is_open = false;
                        }

                        // 主题列表
                        egui::ScrollArea::vertical()
                            .max_height(300.0)
                            .show(ui, |ui| {
                                ui.add_space(4.0);
                                for (idx, theme) in themes.iter().enumerate() {
                                    let is_hover = idx == state.selected_index;
                                    let item_response =
                                        render_combo_item(ui, theme.display_name(), is_hover, false);

                                    // 键盘选中时自动滚动到该项
                                    if is_hover {
                                        item_response.scroll_to_me(Some(egui::Align::Center));
                                    }

                                    if item_response.clicked() {
                                        result = Some(idx);
                                        state.is_open = false;
                                    }

                                    if item_response.hovered() {
                                        state.selected_index = idx;
                                    }
                                }
                                ui.add_space(4.0);
                            });

                        // 提示
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("j/k 选择  Enter 确认  Esc 取消")
                                    .small()
                                    .color(MUTED),
                            );
                        });
                        ui.add_space(4.0);
                    });
            });

        // 点击外部关闭
        let click_outside = ui.input(|i| {
            i.pointer.any_click()
                && !response
                    .rect
                    .contains(i.pointer.interact_pos().unwrap_or_default())
        });
        if click_outside {
            state.is_open = false;
        }
    }

    // 保存状态
    ui.ctx().data_mut(|d| {
        d.insert_temp(id, state);
    });

    result
}
