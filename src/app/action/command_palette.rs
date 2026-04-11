//! Command palette UI shell.
//!
//! The palette owns search text and selection only; command execution stays in the app action layer.

use eframe::egui;

use super::DbManagerApp;
use super::action_system::{AppAction, search_commands};

const MAX_VISIBLE_COMMANDS: usize = 12;

#[derive(Debug, Clone, Default)]
pub(in crate::app) struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub selected_index: usize,
    pub request_focus: bool,
}

impl CommandPaletteState {
    pub(in crate::app) fn open(&mut self) {
        self.open = true;
        self.request_focus = true;
        self.selected_index = 0;
    }

    pub(in crate::app) fn close(&mut self) {
        self.open = false;
        self.query.clear();
        self.selected_index = 0;
        self.request_focus = false;
    }
}

impl DbManagerApp {
    pub(in crate::app) fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.command_palette_state.open {
            return;
        }

        let command_context = self.action_context();
        let mut query = self.command_palette_state.query.clone();
        let mut selected_index = self.command_palette_state.selected_index;
        let request_focus = self.command_palette_state.request_focus;
        let mut close_palette = false;
        let mut action_to_execute: Option<AppAction> = None;
        let mut disabled_reason: Option<&'static str> = None;

        egui::Window::new("命令面板")
            .id(egui::Id::new("gridix_command_palette"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 72.0))
            .collapsible(false)
            .resizable(false)
            .default_width(640.0)
            .show(ctx, |ui| {
                ui.set_min_width(600.0);
                ui.spacing_mut().item_spacing.y = 8.0;

                ui.label(
                    egui::RichText::new(command_context.status_line())
                        .small()
                        .color(ui.visuals().weak_text_color()),
                );

                let input_id = egui::Id::new("gridix_command_palette_input");
                let previous_query = query.clone();
                let input_response = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .id(input_id)
                        .hint_text("输入命令、别名或工作区，例如 sql / filter / export")
                        .desired_width(f32::INFINITY),
                );
                if request_focus {
                    input_response.request_focus();
                }
                if input_response.changed() && query != previous_query {
                    selected_index = 0;
                }

                let matches = search_commands(&command_context, &query);
                clamp_selection(&mut selected_index, matches.len());

                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    close_palette = true;
                }
                if ui.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                    move_selection(&mut selected_index, 1, matches.len());
                }
                if ui.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                    move_selection(&mut selected_index, -1, matches.len());
                }
                let enter_pressed = ui.input(|input| input.key_pressed(egui::Key::Enter));

                if enter_pressed && let Some(entry) = matches.get(selected_index) {
                    if entry.availability.enabled {
                        action_to_execute = Some(entry.descriptor.action);
                    } else {
                        disabled_reason = entry.availability.reason;
                    }
                }

                ui.separator();

                if matches.is_empty() {
                    ui.label(
                        egui::RichText::new("没有匹配命令").color(ui.visuals().weak_text_color()),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(420.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (index, entry) in
                                matches.iter().take(MAX_VISIBLE_COMMANDS).enumerate()
                            {
                                let selected = index == selected_index;
                                let shortcut = self
                                    .shortcut_label_for_action(entry.descriptor.action)
                                    .filter(|label| !label.is_empty())
                                    .unwrap_or_default();
                                let suffix = if shortcut.is_empty() {
                                    String::new()
                                } else {
                                    format!("    {}", shortcut)
                                };
                                let disabled = entry
                                    .availability
                                    .reason
                                    .map(|reason| format!("    {}", reason))
                                    .unwrap_or_default();
                                let label = format!(
                                    "{}{}\n{} · {}{}",
                                    entry.descriptor.title,
                                    suffix,
                                    entry.descriptor.scope.label(),
                                    entry.descriptor.subtitle,
                                    disabled
                                );
                                let text_color = if entry.availability.enabled {
                                    ui.visuals().text_color()
                                } else {
                                    ui.visuals().weak_text_color()
                                };
                                let fill = if selected {
                                    ui.visuals().selection.bg_fill
                                } else {
                                    ui.visuals().widgets.inactive.bg_fill
                                };
                                let response = ui.add_sized(
                                    [ui.available_width(), 48.0],
                                    egui::Button::new(
                                        egui::RichText::new(label)
                                            .color(text_color)
                                            .text_style(egui::TextStyle::Body),
                                    )
                                    .fill(fill),
                                );

                                if response.hovered() {
                                    selected_index = index;
                                }
                                if response.clicked() {
                                    if entry.availability.enabled {
                                        action_to_execute = Some(entry.descriptor.action);
                                    } else {
                                        disabled_reason = entry.availability.reason;
                                    }
                                }
                            }
                        });
                }

                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Enter 执行").small());
                    ui.label(egui::RichText::new("↑/↓ 选择").small());
                    ui.label(egui::RichText::new("Esc 关闭").small());
                });
            });

        self.command_palette_state.query = query;
        self.command_palette_state.selected_index = selected_index;
        self.command_palette_state.request_focus = false;

        if let Some(reason) = disabled_reason {
            self.notifications.warning(reason);
        }

        if let Some(action) = action_to_execute {
            self.command_palette_state.close();
            self.dispatch_app_action(ctx, action);
            return;
        }

        if close_palette {
            self.command_palette_state.close();
        }
    }
}

fn clamp_selection(selected_index: &mut usize, len: usize) {
    if len == 0 {
        *selected_index = 0;
    } else if *selected_index >= len {
        *selected_index = len - 1;
    }
}

fn move_selection(selected_index: &mut usize, delta: isize, len: usize) {
    if len == 0 {
        *selected_index = 0;
        return;
    }

    let len = len as isize;
    let next = (*selected_index as isize + delta).rem_euclid(len);
    *selected_index = next as usize;
}
