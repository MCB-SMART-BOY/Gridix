//! Dialog host state derived from the app.
//!
//! Gridix historically stored each dialog as an independent boolean.  The host
//! keeps those fields compatible while making the modal owner explicit for
//! input routing and rendering.

use super::DbManagerApp;

/// Stable dialog identity used by app-level routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::app) enum DialogId {
    Connection,
    Export,
    Import,
    DeleteConfirm,
    Help,
    About,
    WelcomeSetup,
    History,
    Ddl,
    CreateDatabase,
    CreateUser,
    Keybindings,
    CommandPalette,
}

impl DialogId {
    pub(in crate::app) const fn scope_path(self) -> &'static str {
        match self {
            Self::Connection => "dialog.connection",
            Self::Export => "dialog.export",
            Self::Import => "dialog.import",
            Self::DeleteConfirm => "dialog.confirm",
            Self::Help => "dialog.help",
            Self::About => "dialog.about",
            Self::WelcomeSetup => "dialog.welcome_setup",
            Self::History => "dialog.history",
            Self::Ddl => "dialog.ddl",
            Self::CreateDatabase => "dialog.create_database",
            Self::CreateUser => "dialog.create_user",
            Self::Keybindings => "dialog.keybindings",
            Self::CommandPalette => "dialog.command_palette",
        }
    }
}

/// Boolean dialog state sampled from `DbManagerApp` for one frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::app) struct DialogHostSnapshot {
    pub connection: bool,
    pub export: bool,
    pub import: bool,
    pub delete_confirm: bool,
    pub help: bool,
    pub about: bool,
    pub welcome_setup: bool,
    pub history: bool,
    pub ddl: bool,
    pub create_database: bool,
    pub create_user: bool,
    pub keybindings: bool,
    pub command_palette: bool,
}

impl DialogHostSnapshot {
    /// Returns the only dialog that should receive input for this frame.
    ///
    /// The order is intentionally stable.  It preserves the previous routing
    /// priority while making the rule reusable outside the input router.
    pub(in crate::app) fn active_dialog(self) -> Option<DialogId> {
        [
            (self.connection, DialogId::Connection),
            (self.export, DialogId::Export),
            (self.import, DialogId::Import),
            (self.delete_confirm, DialogId::DeleteConfirm),
            (self.help, DialogId::Help),
            (self.about, DialogId::About),
            (self.welcome_setup, DialogId::WelcomeSetup),
            (self.history, DialogId::History),
            (self.ddl, DialogId::Ddl),
            (self.create_database, DialogId::CreateDatabase),
            (self.create_user, DialogId::CreateUser),
            (self.keybindings, DialogId::Keybindings),
            (self.command_palette, DialogId::CommandPalette),
        ]
        .into_iter()
        .find_map(|(open, id)| open.then_some(id))
    }

    pub(in crate::app) fn has_active_dialog(self) -> bool {
        self.active_dialog().is_some()
    }
}

impl DbManagerApp {
    pub(in crate::app) fn dialog_host_snapshot(&self) -> DialogHostSnapshot {
        DialogHostSnapshot {
            connection: self.show_connection_dialog,
            export: self.show_export_dialog,
            import: self.show_import_dialog,
            delete_confirm: self.show_delete_confirm,
            help: self.show_help,
            about: self.show_about,
            welcome_setup: self.show_welcome_setup_dialog,
            history: self.show_history_panel,
            ddl: self.ddl_dialog_state.show,
            create_database: self.create_db_dialog_state.show,
            create_user: self.create_user_dialog_state.show,
            keybindings: self.keybindings_dialog_state.show,
            command_palette: self.command_palette_state.open,
        }
    }

    pub(in crate::app) fn active_dialog_id(&self) -> Option<DialogId> {
        self.dialog_host_snapshot().active_dialog()
    }
}

#[cfg(test)]
mod tests {
    use super::{DialogHostSnapshot, DialogId};

    #[test]
    fn active_dialog_uses_stable_modal_priority() {
        let snapshot = DialogHostSnapshot {
            connection: true,
            help: true,
            command_palette: true,
            ..Default::default()
        };

        assert_eq!(snapshot.active_dialog(), Some(DialogId::Connection));
    }

    #[test]
    fn command_palette_is_dialog_owner_when_alone() {
        let snapshot = DialogHostSnapshot {
            command_palette: true,
            ..Default::default()
        };

        assert_eq!(snapshot.active_dialog(), Some(DialogId::CommandPalette));
    }
}
