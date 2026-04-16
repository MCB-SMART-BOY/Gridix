//! Command palette UI shell.
//!
//! The palette owns search text and selection only; command execution stays in the app action layer.

use crate::app::dialogs::host::DialogId;
use crate::ui::{LocalShortcut, consume_local_shortcut, local_shortcut_text, local_shortcuts_text};
use eframe::egui;

use super::DbManagerApp;
use super::action_system::{AppAction, search_commands};

const MAX_VISIBLE_COMMANDS: usize = 12;
const COMMAND_PALETTE_VIEWPORT_MARGIN: f32 = 32.0;
const COMMAND_PALETTE_MIN_WIDTH: f32 = 360.0;
const COMMAND_PALETTE_DEFAULT_WIDTH: f32 = 640.0;
const COMMAND_PALETTE_MAX_WIDTH: f32 = 720.0;
const COMMAND_PALETTE_MIN_LIST_HEIGHT: f32 = 180.0;
const COMMAND_PALETTE_MAX_LIST_HEIGHT: f32 = 420.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandPaletteKeyAction {
    Prev,
    Next,
    Confirm,
    Dismiss,
}

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
        let content_rect = ctx.input(|input| input.content_rect());
        let (min_width, default_width, max_width) = command_palette_widths(content_rect.width());
        let list_height = command_palette_list_height(content_rect.height());

        egui::Window::new("命令面板")
            .id(egui::Id::new("gridix_command_palette"))
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 72.0))
            .collapsible(false)
            .resizable(false)
            .default_width(default_width)
            .min_width(min_width)
            .max_width(max_width)
            .constrain_to(content_rect)
            .show(ctx, |ui| {
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

                if let Some(action) = consume_palette_key_action(ui) {
                    match action {
                        CommandPaletteKeyAction::Dismiss => close_palette = true,
                        CommandPaletteKeyAction::Next => {
                            move_selection(&mut selected_index, 1, matches.len());
                        }
                        CommandPaletteKeyAction::Prev => {
                            move_selection(&mut selected_index, -1, matches.len());
                        }
                        CommandPaletteKeyAction::Confirm => {
                            if let Some(entry) = matches.get(selected_index) {
                                if entry.availability.enabled {
                                    action_to_execute = Some(entry.descriptor.action);
                                } else {
                                    disabled_reason = entry.availability.reason;
                                }
                            }
                        }
                    }
                }

                ui.separator();

                if matches.is_empty() {
                    ui.label(
                        egui::RichText::new("没有匹配命令").color(ui.visuals().weak_text_color()),
                    );
                } else {
                    egui::ScrollArea::vertical()
                        .max_height(list_height)
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
                    ui.label(
                        egui::RichText::new(format!(
                            "{} 执行",
                            local_shortcut_text(LocalShortcut::CommandPaletteConfirm)
                        ))
                        .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} 选择",
                            local_shortcuts_text(&[
                                LocalShortcut::CommandPalettePrev,
                                LocalShortcut::CommandPaletteNext,
                            ])
                        ))
                        .small(),
                    );
                    ui.label(
                        egui::RichText::new(format!(
                            "{} 关闭",
                            local_shortcut_text(LocalShortcut::CommandPaletteDismiss)
                        ))
                        .small(),
                    );
                });
            });

        self.command_palette_state.query = query;
        self.command_palette_state.selected_index = selected_index;
        self.command_palette_state.request_focus = false;

        if let Some(reason) = disabled_reason {
            self.notifications.warning(reason);
        }

        if let Some(action) = action_to_execute {
            self.close_dialog(DialogId::CommandPalette);
            self.dispatch_app_action(ctx, action);
            return;
        }

        if close_palette {
            self.close_dialog(DialogId::CommandPalette);
        }
    }
}

fn command_palette_widths(viewport_width: f32) -> (f32, f32, f32) {
    let usable = (viewport_width - COMMAND_PALETTE_VIEWPORT_MARGIN).max(280.0);
    let max_width = usable.min(COMMAND_PALETTE_MAX_WIDTH);
    let min_width = COMMAND_PALETTE_MIN_WIDTH.min(max_width);
    let default_width = COMMAND_PALETTE_DEFAULT_WIDTH.clamp(min_width, max_width);
    (min_width, default_width, max_width)
}

fn command_palette_list_height(viewport_height: f32) -> f32 {
    let usable = (viewport_height - 220.0).max(COMMAND_PALETTE_MIN_LIST_HEIGHT);
    usable.min(COMMAND_PALETTE_MAX_LIST_HEIGHT)
}

fn consume_palette_key_action(ui: &mut egui::Ui) -> Option<CommandPaletteKeyAction> {
    ui.input_mut(|input| {
        if consume_local_shortcut(input, LocalShortcut::CommandPaletteDismiss) {
            Some(CommandPaletteKeyAction::Dismiss)
        } else if consume_local_shortcut(input, LocalShortcut::CommandPaletteNext) {
            Some(CommandPaletteKeyAction::Next)
        } else if consume_local_shortcut(input, LocalShortcut::CommandPalettePrev) {
            Some(CommandPaletteKeyAction::Prev)
        } else if consume_local_shortcut(input, LocalShortcut::CommandPaletteConfirm) {
            Some(CommandPaletteKeyAction::Confirm)
        } else {
            None
        }
    })
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

#[cfg(test)]
mod tests {
    use super::{
        AppAction, CommandPaletteKeyAction, DbManagerApp, command_palette_list_height,
        command_palette_widths, consume_palette_key_action, search_commands,
    };
    use crate::app::dialogs::host::DialogId;
    use crate::database::{Connection, ConnectionConfig, DatabaseType, QueryResult};
    use crate::ui::FocusArea;
    use eframe::egui::{Area, Context, Event, Id, Key, Modifiers, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn run_palette_key(key: Key) -> Option<CommandPaletteKeyAction> {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            events: vec![key_event(key)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
        let mut action = None;
        Area::new(Id::new("command_palette_key_test")).show(&ctx, |ui| {
            action = consume_palette_key_action(ui);
        });
        let _ = ctx.end_pass();
        action
    }

    fn prime_active_connection_with_tables(app: &mut DbManagerApp, tables: &[&str]) {
        let mut connection = Connection::new(ConnectionConfig::new("demo", DatabaseType::SQLite));
        connection.connected = true;
        connection.selected_database = Some("main".to_string());
        connection.tables = tables.iter().map(|name| (*name).to_string()).collect();
        app.manager
            .connections
            .insert("demo".to_string(), connection);
        app.manager.active = Some("demo".to_string());
    }

    #[test]
    fn command_palette_navigation_uses_local_shortcuts() {
        assert_eq!(
            run_palette_key(Key::ArrowDown),
            Some(CommandPaletteKeyAction::Next)
        );
        assert_eq!(
            run_palette_key(Key::ArrowUp),
            Some(CommandPaletteKeyAction::Prev)
        );
        assert_eq!(
            run_palette_key(Key::Escape),
            Some(CommandPaletteKeyAction::Dismiss)
        );
        assert_eq!(
            run_palette_key(Key::Enter),
            Some(CommandPaletteKeyAction::Confirm)
        );
    }

    #[test]
    fn command_palette_widths_clamp_to_small_viewport() {
        let (min_width, default_width, max_width) = command_palette_widths(480.0);

        assert!(max_width <= 448.0 + f32::EPSILON);
        assert!(min_width <= max_width);
        assert!(default_width <= max_width);
        assert_eq!(default_width, max_width);
    }

    #[test]
    fn command_palette_widths_preserve_default_when_room_allows() {
        let (min_width, default_width, max_width) = command_palette_widths(1280.0);

        assert_eq!(min_width, 360.0);
        assert_eq!(default_width, 640.0);
        assert_eq!(max_width, 720.0);
    }

    #[test]
    fn command_palette_list_height_stays_within_bounds() {
        assert_eq!(command_palette_list_height(360.0), 180.0);
        assert_eq!(command_palette_list_height(1200.0), 420.0);
    }

    #[test]
    fn command_palette_confirm_executes_toggle_er_diagram_from_connected_workspace() {
        let ctx = Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        app.selected_table = Some("customers".to_string());
        app.set_focus_area(FocusArea::DataGrid);
        app.open_dialog(DialogId::CommandPalette);
        app.command_palette_state.query = "toggle_er_diagram".to_string();
        app.command_palette_state.selected_index = 0;
        app.command_palette_state.request_focus = false;

        let matches = search_commands(&app.action_context(), &app.command_palette_state.query);
        assert_eq!(
            matches.first().map(|entry| entry.descriptor.action),
            Some(AppAction::ToggleErDiagram)
        );

        ctx.begin_pass(RawInput {
            events: vec![key_event(Key::Enter)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
        app.render_command_palette(&ctx);
        let _ = ctx.end_pass();

        assert!(!app.command_palette_state.open);
        assert!(app.show_er_diagram);
        assert_eq!(app.focus_area, FocusArea::ErDiagram);
        assert!(!app.grid_state.focused);
        assert_eq!(
            app.er_diagram_state.selected_table_name(),
            Some("customers")
        );
    }
}
