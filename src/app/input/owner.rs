//! Keyboard ownership model.
//!
//! This module names the single receiver for keyboard input in a frame.  It is
//! deliberately small: component-specific command state can stay local, but the
//! decision about who owns the keyboard must be app-level and testable.

use super::input_router::{DialogScope, FocusScope, InputMode};

/// The resolved input owner for the current frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum InputOwner {
    /// A keybinding capture prompt is active.
    Recording(DialogScope),
    /// A modal dialog/panel is the top input surface.
    Modal(DialogScope),
    /// A text widget owns the keyboard and command keys must not fire.
    TextEntry(FocusScope),
    /// A selectable scope owns input, such as DataGrid select mode.
    Select(FocusScope),
    /// A normal command scope owns input.
    Command(FocusScope),
    /// The resolved scope is currently unavailable.
    Disabled(FocusScope),
}

impl InputOwner {
    pub(in crate::app) fn from_scope_and_mode(scope: FocusScope, mode: InputMode) -> Self {
        match mode {
            InputMode::Recording => match scope {
                FocusScope::Dialog(dialog) => Self::Recording(dialog),
                _ => Self::Disabled(scope),
            },
            InputMode::TextEntry => Self::TextEntry(scope),
            InputMode::Select => Self::Select(scope),
            InputMode::Disabled => Self::Disabled(scope),
            InputMode::Command => match scope {
                FocusScope::Dialog(dialog) => Self::Modal(dialog),
                _ => Self::Command(scope),
            },
        }
    }

    pub(in crate::app) const fn scope(self) -> FocusScope {
        match self {
            Self::Recording(dialog) | Self::Modal(dialog) => FocusScope::Dialog(dialog),
            Self::TextEntry(scope)
            | Self::Select(scope)
            | Self::Command(scope)
            | Self::Disabled(scope) => scope,
        }
    }

    pub(in crate::app) const fn mode(self) -> InputMode {
        match self {
            Self::Recording(_) => InputMode::Recording,
            Self::Modal(_) | Self::Command(_) => InputMode::Command,
            Self::TextEntry(_) => InputMode::TextEntry,
            Self::Select(_) => InputMode::Select,
            Self::Disabled(_) => InputMode::Disabled,
        }
    }

    pub(in crate::app) const fn is_modal(self) -> bool {
        matches!(self, Self::Modal(_) | Self::Recording(_))
    }

    pub(in crate::app) const fn is_text_entry(self) -> bool {
        matches!(self, Self::TextEntry(_))
    }
}

#[cfg(test)]
mod tests {
    use super::InputOwner;
    use crate::app::input::input_router::{DialogScope, FocusScope, GridFocusScope, InputMode};

    #[test]
    fn modal_command_scope_resolves_to_modal_owner() {
        let owner = InputOwner::from_scope_and_mode(
            FocusScope::Dialog(DialogScope::Export),
            InputMode::Command,
        );

        assert_eq!(owner, InputOwner::Modal(DialogScope::Export));
        assert!(owner.is_modal());
    }

    #[test]
    fn text_entry_scope_resolves_to_text_owner() {
        let scope = FocusScope::Grid(GridFocusScope::Insert);
        let owner = InputOwner::from_scope_and_mode(scope, InputMode::TextEntry);

        assert_eq!(owner, InputOwner::TextEntry(scope));
        assert!(owner.is_text_entry());
    }
}
