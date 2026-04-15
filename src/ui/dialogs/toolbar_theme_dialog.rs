use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStyle, DialogWindow,
    WorkspaceDialogShell,
};
use crate::core::ThemePreset;
use crate::ui::{LocalShortcut, local_shortcut_text, local_shortcuts_text};
use eframe::egui::{self, RichText, ScrollArea};
use std::cell::Cell;

const TOOLBAR_THEME_WIDTH: f32 = 420.0;
const TOOLBAR_THEME_HEIGHT: f32 = 480.0;

const DARK_THEMES: &[ThemePreset] = &[
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

const LIGHT_THEMES: &[ThemePreset] = &[
    ThemePreset::TokyoNightLight,
    ThemePreset::CatppuccinLatte,
    ThemePreset::OneLight,
    ThemePreset::GruvboxLight,
    ThemePreset::SolarizedLight,
    ThemePreset::GithubLight,
];

#[derive(Debug, Clone, Default)]
pub struct ToolbarThemeDialogState {
    pub show: bool,
    pub selected_preset: Option<ThemePreset>,
    reset_selection: bool,
}

impl ToolbarThemeDialogState {
    pub fn open(&mut self) {
        self.show = true;
        self.reset_selection = true;
    }

    pub fn close(&mut self) {
        self.show = false;
        self.reset_selection = false;
    }

    fn ensure_selection(&mut self, current_theme: ThemePreset, themes: &[ThemePreset]) {
        if themes.is_empty() {
            self.selected_preset = None;
            self.reset_selection = false;
            return;
        }

        let needs_reset = self.reset_selection
            || self
                .selected_preset
                .is_none_or(|preset| !themes.contains(&preset));
        if needs_reset {
            self.selected_preset = Some(if themes.contains(&current_theme) {
                current_theme
            } else {
                themes[0]
            });
            self.reset_selection = false;
        }
    }

    fn selected_index(&self, themes: &[ThemePreset]) -> usize {
        self.selected_preset
            .and_then(|preset| themes.iter().position(|candidate| *candidate == preset))
            .unwrap_or(0)
    }

    fn move_selection(&mut self, delta: isize, themes: &[ThemePreset]) {
        if themes.is_empty() {
            self.selected_preset = None;
            return;
        }

        let current = self.selected_index(themes) as isize;
        let max_index = (themes.len() - 1) as isize;
        let next = (current + delta).clamp(0, max_index) as usize;
        self.selected_preset = themes.get(next).copied();
    }

    fn move_to_start(&mut self, themes: &[ThemePreset]) {
        self.selected_preset = themes.first().copied();
    }

    fn move_to_end(&mut self, themes: &[ThemePreset]) {
        self.selected_preset = themes.last().copied();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolbarThemeFrameAction {
    Prev,
    Next,
    Confirm,
    Dismiss,
    Start,
    End,
}

pub struct ToolbarThemeDialog;

impl ToolbarThemeDialog {
    pub fn show(
        ctx: &egui::Context,
        state: &mut ToolbarThemeDialogState,
        current_theme: ThemePreset,
        is_dark_mode: bool,
    ) -> Option<ThemePreset> {
        if !state.show {
            return None;
        }

        let themes = if is_dark_mode {
            DARK_THEMES
        } else {
            LIGHT_THEMES
        };
        state.ensure_selection(current_theme, themes);
        let selected_preset = Cell::new(state.selected_preset);
        let close_requested = Cell::new(false);

        let activated = Cell::new(None);
        if let Some(frame_action) = Self::consume_frame_action(ctx) {
            match frame_action {
                ToolbarThemeFrameAction::Prev => state.move_selection(-1, themes),
                ToolbarThemeFrameAction::Next => state.move_selection(1, themes),
                ToolbarThemeFrameAction::Confirm => {
                    activated.set(state.selected_preset);
                }
                ToolbarThemeFrameAction::Dismiss => close_requested.set(true),
                ToolbarThemeFrameAction::Start => state.move_to_start(themes),
                ToolbarThemeFrameAction::End => state.move_to_end(themes),
            }
        }
        selected_preset.set(state.selected_preset);

        let style = DialogStyle::MEDIUM;
        let dialog_title = "主题选择器";
        DialogWindow::fixed_style(
            ctx,
            dialog_title,
            &style,
            TOOLBAR_THEME_WIDTH,
            TOOLBAR_THEME_HEIGHT,
        )
        .open(&mut state.show)
        .show(ctx, |ui| {
            WorkspaceDialogShell::show(
                ui,
                "toolbar_theme_dialog",
                |ui| {
                    DialogContent::toolbar(ui, |ui| {
                        ui.label(RichText::new(dialog_title).strong());
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(if is_dark_mode {
                                "当前为深色主题列表。"
                            } else {
                                "当前为浅色主题列表。"
                            })
                            .small()
                            .weak(),
                        );
                    });
                    ui.add_space(8.0);
                },
                |ui| {
                    DialogContent::toolbar(ui, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{} 选择",
                                    local_shortcuts_text(&[
                                        LocalShortcut::ToolbarThemePrev,
                                        LocalShortcut::ToolbarThemeNext,
                                    ])
                                ))
                                .small(),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} 确认",
                                    local_shortcut_text(LocalShortcut::ToolbarThemeConfirm)
                                ))
                                .small(),
                            );
                            ui.label(
                                RichText::new(format!(
                                    "{} 关闭",
                                    local_shortcut_text(LocalShortcut::ToolbarThemeDismiss)
                                ))
                                .small(),
                            );
                        });
                    });
                    ui.add_space(8.0);
                },
                |ui| {
                    DialogContent::workspace_pane(
                        ui,
                        "主题列表",
                        "保留 toolbar 文字按钮作为 trigger，真正的选择与确认在这里完成。",
                        |ui| {
                            ScrollArea::vertical()
                                .id_salt(ui.id().with("toolbar_theme_entries"))
                                .show(ui, |ui| {
                                    for theme in themes {
                                        let selected = selected_preset.get() == Some(*theme);
                                        let response = Self::show_theme_entry(
                                            ui,
                                            *theme,
                                            selected,
                                            *theme == current_theme,
                                        );

                                        if response.hovered() || response.clicked() {
                                            selected_preset.set(Some(*theme));
                                        }

                                        if selected {
                                            response.scroll_to_me(Some(egui::Align::Center));
                                        }

                                        if response.double_clicked() {
                                            activated.set(Some(*theme));
                                        }
                                    }
                                });
                        },
                    );
                },
                |ui| {
                    let footer = DialogFooter::show(ui, "应用主题", "关闭", true, &style);
                    if footer.cancelled {
                        close_requested.set(true);
                    }
                    if footer.confirmed {
                        activated.set(selected_preset.get());
                    }
                },
            );
        });

        state.selected_preset = selected_preset.get();

        if close_requested.get() {
            state.show = false;
        }

        if activated.get().is_some() || !state.show {
            state.close();
        }

        activated.get()
    }

    fn consume_frame_action(ctx: &egui::Context) -> Option<ToolbarThemeFrameAction> {
        DialogShortcutContext::new(ctx).resolve(&[
            (
                LocalShortcut::ToolbarThemePrev,
                ToolbarThemeFrameAction::Prev,
            ),
            (
                LocalShortcut::ToolbarThemeNext,
                ToolbarThemeFrameAction::Next,
            ),
            (
                LocalShortcut::ToolbarThemeConfirm,
                ToolbarThemeFrameAction::Confirm,
            ),
            (
                LocalShortcut::ToolbarThemeDismiss,
                ToolbarThemeFrameAction::Dismiss,
            ),
            (
                LocalShortcut::ToolbarThemeStart,
                ToolbarThemeFrameAction::Start,
            ),
            (LocalShortcut::ToolbarThemeEnd, ToolbarThemeFrameAction::End),
        ])
    }

    fn show_theme_entry(
        ui: &mut egui::Ui,
        theme: ThemePreset,
        selected: bool,
        current: bool,
    ) -> egui::Response {
        let selection_bg = ui.visuals().selection.bg_fill;
        let selection_stroke = ui.visuals().selection.stroke.color;
        let fill = if selected {
            selection_bg.gamma_multiply(0.22)
        } else {
            egui::Color32::TRANSPARENT
        };
        let stroke = if selected {
            egui::Stroke::new(1.0, selection_stroke.gamma_multiply(0.7))
        } else {
            egui::Stroke::NONE
        };
        let meta = if current {
            "当前主题"
        } else if theme.is_dark() {
            "深色"
        } else {
            "浅色"
        };

        egui::Frame::NONE
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    let indicator = if selected { ">" } else { " " };
                    ui.label(RichText::new(indicator).monospace().color(selection_stroke));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(theme.display_name()).strong());
                        ui.label(RichText::new(meta).small().weak());
                    });
                });
            })
            .response
            .interact(egui::Sense::click())
    }
}

#[cfg(test)]
mod tests {
    use super::{DARK_THEMES, ToolbarThemeDialogState};
    use crate::core::ThemePreset;

    #[test]
    fn open_reseeds_selection_from_current_theme() {
        let mut state = ToolbarThemeDialogState {
            selected_preset: Some(ThemePreset::GithubDark),
            ..Default::default()
        };
        state.open();
        state.ensure_selection(ThemePreset::Dracula, DARK_THEMES);

        assert_eq!(state.selected_preset, Some(ThemePreset::Dracula));
    }

    #[test]
    fn move_selection_clamps_within_theme_list() {
        let mut state = ToolbarThemeDialogState {
            show: true,
            selected_preset: Some(DARK_THEMES[0]),
            reset_selection: false,
        };

        state.move_selection(-1, DARK_THEMES);
        assert_eq!(state.selected_preset, Some(DARK_THEMES[0]));

        state.move_to_end(DARK_THEMES);
        state.move_selection(1, DARK_THEMES);
        assert_eq!(state.selected_preset, DARK_THEMES.last().copied());
    }

    #[test]
    fn invalid_selection_falls_back_to_current_theme() {
        let mut state = ToolbarThemeDialogState {
            show: true,
            selected_preset: Some(ThemePreset::TokyoNightLight),
            reset_selection: false,
        };

        state.ensure_selection(ThemePreset::Nord, DARK_THEMES);

        assert_eq!(state.selected_preset, Some(ThemePreset::Nord));
    }
}
