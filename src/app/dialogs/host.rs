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
    ToolbarActionsMenu,
    ToolbarCreateMenu,
    ToolbarThemeMenu,
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
            Self::ToolbarActionsMenu => "dialog.toolbar_actions",
            Self::ToolbarCreateMenu => "dialog.toolbar_create",
            Self::ToolbarThemeMenu => "dialog.toolbar_theme",
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
    pub toolbar_actions_menu: bool,
    pub toolbar_create_menu: bool,
    pub toolbar_theme_menu: bool,
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
            (self.toolbar_actions_menu, DialogId::ToolbarActionsMenu),
            (self.toolbar_create_menu, DialogId::ToolbarCreateMenu),
            (self.toolbar_theme_menu, DialogId::ToolbarThemeMenu),
            (self.command_palette, DialogId::CommandPalette),
        ]
        .into_iter()
        .find_map(|(open, id)| open.then_some(id))
    }
}

impl DbManagerApp {
    pub(in crate::app) fn is_dialog_visible(&self, id: DialogId) -> bool {
        match id {
            DialogId::Connection => self.show_connection_dialog,
            DialogId::Export => self.show_export_dialog,
            DialogId::Import => self.show_import_dialog,
            DialogId::DeleteConfirm => self.show_delete_confirm,
            DialogId::Help => self.show_help,
            DialogId::About => self.show_about,
            DialogId::WelcomeSetup => self.show_welcome_setup_dialog,
            DialogId::History => self.show_history_panel,
            DialogId::Ddl => self.ddl_dialog_state.show,
            DialogId::CreateDatabase => self.create_db_dialog_state.show,
            DialogId::CreateUser => self.create_user_dialog_state.show,
            DialogId::Keybindings => self.keybindings_dialog_state.show,
            DialogId::ToolbarActionsMenu => self.toolbar_actions_menu_state.show,
            DialogId::ToolbarCreateMenu => self.toolbar_create_menu_state.show,
            DialogId::ToolbarThemeMenu => self.toolbar_theme_dialog_state.show,
            DialogId::CommandPalette => self.command_palette_state.open,
        }
    }

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
            toolbar_actions_menu: self.toolbar_actions_menu_state.show,
            toolbar_create_menu: self.toolbar_create_menu_state.show,
            toolbar_theme_menu: self.toolbar_theme_dialog_state.show,
            command_palette: self.command_palette_state.open,
        }
    }

    pub(in crate::app) fn mark_dialog_owner(&mut self, id: DialogId) {
        self.active_dialog_owner = Some(id);
    }

    pub(in crate::app) fn reconcile_active_dialog_owner(&mut self) {
        if self
            .active_dialog_owner
            .is_some_and(|id| self.is_dialog_visible(id))
        {
            return;
        }

        self.active_dialog_owner = self.dialog_host_snapshot().active_dialog();
    }

    pub(in crate::app) fn open_dialog(&mut self, id: DialogId) {
        match id {
            DialogId::Connection => self.show_connection_dialog = true,
            DialogId::Export => self.show_export_dialog = true,
            DialogId::Import => self.show_import_dialog = true,
            DialogId::DeleteConfirm => self.show_delete_confirm = true,
            DialogId::Help => self.show_help = true,
            DialogId::About => self.show_about = true,
            DialogId::WelcomeSetup => self.show_welcome_setup_dialog = true,
            DialogId::History => self.show_history_panel = true,
            DialogId::Ddl => self.ddl_dialog_state.show = true,
            DialogId::CreateDatabase => self.create_db_dialog_state.show = true,
            DialogId::CreateUser => self.create_user_dialog_state.show = true,
            DialogId::Keybindings => self.keybindings_dialog_state.show = true,
            DialogId::ToolbarActionsMenu => self.toolbar_actions_menu_state.open(),
            DialogId::ToolbarCreateMenu => self.toolbar_create_menu_state.open(),
            DialogId::ToolbarThemeMenu => self.toolbar_theme_dialog_state.open(),
            DialogId::CommandPalette => self.command_palette_state.open(),
        }

        self.active_dialog_owner = Some(id);
    }

    pub(in crate::app) fn close_dialog(&mut self, id: DialogId) {
        match id {
            DialogId::Connection => self.show_connection_dialog = false,
            DialogId::Export => self.show_export_dialog = false,
            DialogId::Import => self.show_import_dialog = false,
            DialogId::DeleteConfirm => self.show_delete_confirm = false,
            DialogId::Help => self.show_help = false,
            DialogId::About => self.show_about = false,
            DialogId::WelcomeSetup => self.show_welcome_setup_dialog = false,
            DialogId::History => self.show_history_panel = false,
            DialogId::Ddl => self.ddl_dialog_state.close(),
            DialogId::CreateDatabase => self.create_db_dialog_state.close(),
            DialogId::CreateUser => self.create_user_dialog_state.close(),
            DialogId::Keybindings => self.keybindings_dialog_state.close(),
            DialogId::ToolbarActionsMenu => self.toolbar_actions_menu_state.close(),
            DialogId::ToolbarCreateMenu => self.toolbar_create_menu_state.close(),
            DialogId::ToolbarThemeMenu => self.toolbar_theme_dialog_state.close(),
            DialogId::CommandPalette => self.command_palette_state.close(),
        }

        if self.active_dialog_owner == Some(id) {
            self.active_dialog_owner = None;
        }
        self.reconcile_active_dialog_owner();
    }

    pub(in crate::app) fn toggle_dialog(&mut self, id: DialogId) {
        if self.is_dialog_visible(id) {
            self.close_dialog(id);
        } else {
            self.open_dialog(id);
        }
    }

    pub(in crate::app) fn active_dialog_id(&self) -> Option<DialogId> {
        self.active_dialog_owner
            .filter(|id| self.is_dialog_visible(*id))
            .or_else(|| self.dialog_host_snapshot().active_dialog())
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
            toolbar_actions_menu: false,
            toolbar_create_menu: false,
            command_palette: true,
            ..Default::default()
        };

        assert_eq!(snapshot.active_dialog(), Some(DialogId::Connection));
    }

    #[test]
    fn command_palette_is_dialog_owner_when_alone() {
        let snapshot = DialogHostSnapshot {
            command_palette: true,
            toolbar_actions_menu: false,
            toolbar_create_menu: false,
            toolbar_theme_menu: false,
            ..Default::default()
        };

        assert_eq!(snapshot.active_dialog(), Some(DialogId::CommandPalette));
    }

    #[test]
    fn toolbar_actions_menu_is_dialog_owner_when_alone() {
        let snapshot = DialogHostSnapshot {
            toolbar_actions_menu: true,
            ..Default::default()
        };

        assert_eq!(snapshot.active_dialog(), Some(DialogId::ToolbarActionsMenu));
    }

    #[test]
    fn toolbar_theme_menu_is_dialog_owner_when_alone() {
        let snapshot = DialogHostSnapshot {
            toolbar_theme_menu: true,
            ..Default::default()
        };

        assert_eq!(snapshot.active_dialog(), Some(DialogId::ToolbarThemeMenu));
    }
}
