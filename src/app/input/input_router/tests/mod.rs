pub(super) use super::{InputContextSnapshot, ResolvedInputAction};

mod dialogs;
mod editor;
mod er_diagram;
mod er_focus;
mod grid;
mod keymap;
mod misc;
mod workspace;

pub(super) use crate::app::DbManagerApp;
pub(super) use crate::core::{Action, KeyBinding, KeyBindings, KeyCode};
pub(super) use crate::ui::{ERTable, EditorMode, FocusArea, GridMode, SidebarSection};
pub(super) use egui::{Event, Key, Modifiers, Pos2, Vec2};

fn snapshot() -> InputContextSnapshot {
    InputContextSnapshot {
        has_modal_dialog: false,
        active_dialog: None,
        text_focus: false,
        egui_captures_keyboard: false,
        show_autocomplete: false,
        show_sql_editor: true,
        focus_sql_editor: false,
        focus_area: FocusArea::DataGrid,
        editor_mode: EditorMode::Insert,
        sidebar_section: SidebarSection::Connections,
        filter_input_has_focus: false,
        grid_mode: GridMode::Normal,
        grid_editing_cell: false,
        show_connection_dialog: false,
        show_export_dialog: false,
        show_import_dialog: false,
        show_delete_confirm: false,
        show_help: false,
        show_about: false,
        show_welcome_setup_dialog: false,
        show_history_panel: false,
        show_ddl_dialog: false,
        show_create_db_dialog: false,
        show_create_user_dialog: false,
        show_keybindings_dialog: false,
        show_command_palette: false,
        show_er_diagram: false,
        er_diagram_viewport_mode: false,
        keybindings_recording: false,
    }
}

fn key_event(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn key_event_with_modifiers(key: Key, modifiers: Modifiers) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

fn test_app() -> DbManagerApp {
    DbManagerApp::new_for_test()
}

fn er_table(name: &str, x: f32, y: f32) -> ERTable {
    let mut table = ERTable::new(name.to_string());
    table.position = Pos2::new(x, y);
    table.size = Vec2::new(160.0, 96.0);
    table
}

fn resolve_event(
    context: InputContextSnapshot,
    event: Event,
    triggered_action: Option<Action>,
) -> ResolvedInputAction {
    let modifiers = match &event {
        Event::Key { modifiers, .. } => *modifiers,
        _ => Modifiers::NONE,
    };
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        events: vec![event],
        modifiers,
        ..Default::default()
    };

    ctx.begin_pass(raw_input);
    let resolved = ctx.input(|input| {
        super::resolve_input_action_with(
            context,
            input,
            |action| triggered_action == Some(action),
            |action| triggered_action == Some(action),
            |_shortcut| false,
            || None,
        )
    });
    let _ = ctx.end_pass();
    resolved
}

fn resolve_event_with_keybindings(
    context: InputContextSnapshot,
    event: Event,
    keybindings: &KeyBindings,
) -> ResolvedInputAction {
    let modifiers = match &event {
        Event::Key { modifiers, .. } => *modifiers,
        _ => Modifiers::NONE,
    };
    let ctx = egui::Context::default();
    let raw_input = egui::RawInput {
        events: vec![event],
        modifiers,
        ..Default::default()
    };

    ctx.begin_pass(raw_input);
    let resolved = ctx.input(|input| {
        super::resolve_input_action_with(
            context,
            input,
            |action| {
                super::scoped_keybinding_triggered_in_input(keybindings, context, action, input)
            },
            |action| super::global_keybinding_triggered_in_input(keybindings, action, input),
            |shortcut| super::local_shortcut_triggered_in_input(keybindings, shortcut, input),
            || super::focus_area_switch_triggered_in_input(keybindings, input),
        )
    });
    let _ = ctx.end_pass();
    resolved
}
