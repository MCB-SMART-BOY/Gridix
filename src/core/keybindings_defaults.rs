use std::collections::HashMap;
use super::{Action, KeyBinding, KeyCode, KeyModifiers, KeyBindings};

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // 全局操作
        bindings.insert(Action::NextFocusArea, KeyBinding::key_only(KeyCode::Tab));
        bindings.insert(
            Action::PrevFocusArea,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::SHIFT),
        );
        bindings.insert(Action::NewConnection, KeyBinding::ctrl(KeyCode::N));
        bindings.insert(Action::CommandPalette, KeyBinding::ctrl(KeyCode::P));
        bindings.insert(
            Action::OpenKeybindingsDialog,
            KeyBinding::new(KeyCode::K, KeyModifiers::ALT),
        );
        bindings.insert(
            Action::OpenToolbarActionsMenu,
            KeyBinding::new(KeyCode::A, KeyModifiers::ALT),
        );
        bindings.insert(
            Action::OpenToolbarCreateMenu,
            KeyBinding::new(KeyCode::N, KeyModifiers::ALT),
        );
        bindings.insert(
            Action::OpenThemeSelector,
            KeyBinding::ctrl_shift(KeyCode::T),
        );
        bindings.insert(Action::ToggleSidebar, KeyBinding::ctrl(KeyCode::B));
        bindings.insert(Action::ToggleDarkMode, KeyBinding::ctrl(KeyCode::D));
        bindings.insert(Action::ToggleEditor, KeyBinding::ctrl(KeyCode::J));
        bindings.insert(Action::ToggleErDiagram, KeyBinding::ctrl(KeyCode::R));
        bindings.insert(
            Action::FocusErDiagram,
            KeyBinding::new(KeyCode::R, KeyModifiers::ALT),
        );
        bindings.insert(Action::ShowHelp, KeyBinding::key_only(KeyCode::F1));
        bindings.insert(Action::ShowHistory, KeyBinding::ctrl(KeyCode::H));
        bindings.insert(Action::Export, KeyBinding::ctrl(KeyCode::E));
        bindings.insert(Action::Import, KeyBinding::ctrl(KeyCode::I));
        bindings.insert(Action::Refresh, KeyBinding::key_only(KeyCode::F5));
        bindings.insert(Action::ClearCommandLine, KeyBinding::ctrl(KeyCode::L));
        bindings.insert(Action::ClearSearch, KeyBinding::ctrl(KeyCode::K));

        // 创建操作
        bindings.insert(Action::NewTable, KeyBinding::ctrl_shift(KeyCode::N));
        bindings.insert(Action::NewDatabase, KeyBinding::ctrl_shift(KeyCode::D));
        bindings.insert(Action::NewUser, KeyBinding::ctrl_shift(KeyCode::U));

        // Tab 操作
        bindings.insert(Action::NewTab, KeyBinding::ctrl(KeyCode::T));
        bindings.insert(Action::CloseTab, KeyBinding::ctrl(KeyCode::W));
        bindings.insert(
            Action::NextTab,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::CTRL),
        );
        bindings.insert(
            Action::PrevTab,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::CTRL_SHIFT),
        );

        // 编辑操作
        bindings.insert(Action::Save, KeyBinding::ctrl(KeyCode::S));
        bindings.insert(Action::GotoLine, KeyBinding::ctrl(KeyCode::G));

        // 侧边栏焦点
        bindings.insert(
            Action::FocusSidebarConnections,
            KeyBinding::ctrl(KeyCode::Num1),
        );
        bindings.insert(
            Action::FocusSidebarDatabases,
            KeyBinding::ctrl(KeyCode::Num2),
        );
        bindings.insert(Action::FocusSidebarTables, KeyBinding::ctrl(KeyCode::Num3));
        bindings.insert(Action::FocusSidebarFilters, KeyBinding::ctrl(KeyCode::Num4));
        bindings.insert(
            Action::FocusSidebarTriggers,
            KeyBinding::ctrl(KeyCode::Num5),
        );
        bindings.insert(
            Action::FocusSidebarRoutines,
            KeyBinding::ctrl(KeyCode::Num6),
        );

        // 缩放
        bindings.insert(Action::ZoomIn, KeyBinding::ctrl(KeyCode::Plus));
        bindings.insert(Action::ZoomOut, KeyBinding::ctrl(KeyCode::Minus));
        bindings.insert(Action::ZoomReset, KeyBinding::ctrl(KeyCode::Num0));

        Self {
            bindings,
            local_bindings: HashMap::new(),
            local_sequences: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }
}
