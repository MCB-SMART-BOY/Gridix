use super::SidebarPanelState;
use crate::ui::{
    LocalShortcut, consume_local_shortcut_with_text_priority, text_entry_has_priority,
};
use egui::{self, Key};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SidebarKeyAction {
    ItemPrev,
    ItemNext,
    ItemStart,
    ItemEnd,
    MoveLeft,
    MoveRight,
    Toggle,
    Delete,
    Activate,
    Edit,
    Rename,
    Refresh,
    InspectSchema,
    AddFilterBelow,
    AppendFilter,
    DeleteFilterAlternative,
    ClearFilters,
    FilterColumnNext,
    FilterColumnPrev,
    FilterOperatorNext,
    FilterOperatorPrev,
    FilterLogicToggle,
    FilterFocusInput,
    FilterCaseToggle,
}

pub(super) fn detect_key_action(
    ctx: &egui::Context,
    panel_state: &mut SidebarPanelState,
) -> Option<SidebarKeyAction> {
    let text_entry_active = text_entry_has_priority(ctx);
    ctx.input_mut(|i| {
        if text_entry_active {
            panel_state.command_buffer.clear();
            return None;
        }

        if i.key_pressed(Key::G) && i.modifiers.is_none() {
            if panel_state.command_buffer == "g" {
                panel_state.command_buffer.clear();
                return Some(SidebarKeyAction::ItemStart);
            }

            panel_state.command_buffer.clear();
            panel_state.command_buffer.push('g');
            return None;
        }

        if i.key_pressed(Key::S) && i.modifiers.is_none() && panel_state.command_buffer == "g" {
            panel_state.command_buffer.clear();
            return Some(SidebarKeyAction::InspectSchema);
        }

        let action = if i.key_pressed(Key::A) && i.modifiers.shift_only() {
            Some(SidebarKeyAction::AppendFilter)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarItemNext,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::ItemNext)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarItemPrev,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::ItemPrev)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarItemStart,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::ItemStart)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarItemEnd,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::ItemEnd)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarMoveLeft,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::MoveLeft)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarMoveRight,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::MoveRight)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarToggle,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::Toggle)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarDelete,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::Delete)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarActivate,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::Activate)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarEdit,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::Edit)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarRename,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::Rename)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::SidebarRefresh,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::Refresh)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterAdd,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::AddFilterBelow)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterDelete,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::DeleteFilterAlternative)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterClearAll,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::ClearFilters)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterColumnNext,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterColumnNext)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterColumnPrev,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterColumnPrev)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterOperatorNext,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterOperatorNext)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterOperatorPrev,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterOperatorPrev)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterLogicToggle,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterLogicToggle)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterFocusInput,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterFocusInput)
        } else if consume_local_shortcut_with_text_priority(
            i,
            LocalShortcut::FilterCaseToggle,
            text_entry_active,
        ) {
            Some(SidebarKeyAction::FilterCaseToggle)
        } else {
            None
        };

        if action.is_some() {
            panel_state.command_buffer.clear();
        }

        action
    })
}

#[cfg(test)]
mod tests {
    use super::{SidebarKeyAction, detect_key_action};
    use crate::ui::SidebarPanelState;
    use egui::{Context, Event, Key, Modifiers, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn run_detector(
        ctx: &Context,
        event: Event,
        panel_state: &mut SidebarPanelState,
    ) -> Option<SidebarKeyAction> {
        let modifiers = match &event {
            Event::Key { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        };
        ctx.begin_pass(RawInput {
            events: vec![event],
            modifiers,
            ..Default::default()
        });
        let action = detect_key_action(ctx, panel_state);
        let _ = ctx.end_pass();
        action
    }

    #[test]
    fn detector_recognizes_g_prefix_commands() {
        let ctx = Context::default();
        let mut panel_state = SidebarPanelState::default();

        assert_eq!(
            run_detector(&ctx, key_event(Key::G), &mut panel_state),
            None
        );
        assert_eq!(panel_state.command_buffer, "g");
        assert_eq!(
            run_detector(&ctx, key_event(Key::G), &mut panel_state),
            Some(SidebarKeyAction::ItemStart)
        );
        assert!(panel_state.command_buffer.is_empty());

        assert_eq!(
            run_detector(&ctx, key_event(Key::G), &mut panel_state),
            None
        );
        assert_eq!(
            run_detector(&ctx, key_event(Key::S), &mut panel_state),
            Some(SidebarKeyAction::InspectSchema)
        );
        assert!(panel_state.command_buffer.is_empty());
    }

    #[test]
    fn detector_consumes_navigation_shortcut_and_clears_command_buffer() {
        let ctx = Context::default();
        let mut panel_state = SidebarPanelState::default();
        panel_state.command_buffer.push('g');

        assert_eq!(
            run_detector(&ctx, key_event(Key::J), &mut panel_state),
            Some(SidebarKeyAction::ItemNext)
        );
        assert!(panel_state.command_buffer.is_empty());
    }
}
