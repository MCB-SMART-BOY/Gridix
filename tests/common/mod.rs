//! Shared test utilities for external integration tests.
//!
//! Usage from `tests/*.rs`:
//! ```ignore
//! mod common;
//! use common::{begin_key_pass, focus_text_input};
//! ```

use egui::{Event, Key, Modifiers, RawInput};

/// Inject a keypress event into an egui context via `begin_pass`.
pub fn begin_key_pass(ctx: &egui::Context, key: Key) {
    ctx.begin_pass(RawInput {
        events: vec![Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
        modifiers: Modifiers::NONE,
        ..Default::default()
    });
}

/// Focus a transient text input so that keyboard conflict tests can detect
/// text-entry priority.
pub fn focus_text_input(ctx: &egui::Context) {
    let mut text = String::new();
    ctx.begin_pass(RawInput::default());
    egui::Window::new("shared test text input").show(ctx, |ui| {
        let response = ui.add(
            egui::TextEdit::singleline(&mut text).id_salt("shared_test_text_input"),
        );
        response.request_focus();
    });
    let _ = ctx.end_pass();
}
