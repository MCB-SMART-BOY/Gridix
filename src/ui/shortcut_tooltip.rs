//! 快捷键提示文本工具
//!
//! 统一生成“功能 + 快捷键”的悬停提示，避免快捷键改配置后 UI 文案失真。

use crate::core::{Action, KeyBinding, KeyBindings, KeyCode, KeyModifiers};
use egui::InputState;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalBinding {
    key: KeyCode,
    modifiers: KeyModifiers,
}

impl LocalBinding {
    pub const fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    pub fn from_key_binding(binding: &KeyBinding) -> Self {
        Self::new(binding.key, binding.modifiers)
    }

    fn key_binding(self) -> KeyBinding {
        KeyBinding::new(self.key, self.modifiers)
    }

    pub fn display(self) -> String {
        self.key_binding().display()
    }

    pub fn is_pressed(self, ctx: &egui::Context) -> bool {
        self.key_binding().is_pressed(ctx)
    }

    pub fn consume(self, input: &mut InputState) -> bool {
        input.consume_key(self.modifiers.to_egui(), self.key.to_egui_key())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalShortcut {
    Confirm,
    Cancel,
    Dismiss,
    DangerConfirm,
    DangerCancel,
    HelpScrollUp,
    HelpScrollDown,
    HelpPageUp,
    HelpPageDown,
    SidebarItemPrev,
    SidebarItemNext,
    SidebarItemStart,
    SidebarItemEnd,
    SidebarMoveLeft,
    SidebarMoveRight,
    SidebarToggle,
    SidebarDelete,
    SidebarEdit,
    SidebarRename,
    SidebarRefresh,
    SidebarActivate,
    FilterAdd,
    FilterDelete,
    FilterClearAll,
    FilterColumnNext,
    FilterColumnPrev,
    FilterOperatorNext,
    FilterOperatorPrev,
    FilterLogicToggle,
    FilterFocusInput,
    FilterCaseToggle,
    ExportFormatCsv,
    ExportFormatTsv,
    ExportFormatSql,
    ExportFormatJson,
    ExportCyclePrev,
    ExportCycleNext,
    ExportColumnPrev,
    ExportColumnNext,
    ExportColumnStart,
    ExportColumnEnd,
    ExportColumnToggle,
    ExportColumnsToggleAll,
    SqlExecute,
    SqlExplain,
    SqlClear,
    SqlAutocompleteTrigger,
    SqlAutocompleteConfirm,
    SqlHistoryPrev,
    SqlHistoryNext,
    SqlHistoryBrowse,
    ImportRefresh,
    ImportFormatSql,
    ImportFormatCsv,
    ImportFormatTsv,
    ImportFormatJson,
    ImportCyclePrev,
    ImportCycleNext,
    ConnectionTypeSqlite,
    ConnectionTypePostgres,
    ConnectionTypeMySql,
    ConnectionTypePrev,
    ConnectionTypeNext,
    DdlColumnPrev,
    DdlColumnNext,
    DdlColumnStart,
    DdlColumnEnd,
    DdlColumnDelete,
    DdlColumnAddBelow,
    DdlColumnAddAbove,
    DdlColumnTogglePrimaryKey,
    SqliteBrowseFile,
    FormatSelectionCycle,
    HistoryClear,
    HistoryPrev,
    HistoryNext,
    HistoryStart,
    HistoryEnd,
    HistoryPageUp,
    HistoryPageDown,
    HistoryUse,
}

impl LocalShortcut {
    pub fn all() -> &'static [Self] {
        &[
            Self::Confirm,
            Self::Cancel,
            Self::Dismiss,
            Self::DangerConfirm,
            Self::DangerCancel,
            Self::HelpScrollUp,
            Self::HelpScrollDown,
            Self::HelpPageUp,
            Self::HelpPageDown,
            Self::SidebarItemPrev,
            Self::SidebarItemNext,
            Self::SidebarItemStart,
            Self::SidebarItemEnd,
            Self::SidebarMoveLeft,
            Self::SidebarMoveRight,
            Self::SidebarToggle,
            Self::SidebarDelete,
            Self::SidebarEdit,
            Self::SidebarRename,
            Self::SidebarRefresh,
            Self::SidebarActivate,
            Self::FilterAdd,
            Self::FilterDelete,
            Self::FilterClearAll,
            Self::FilterColumnNext,
            Self::FilterColumnPrev,
            Self::FilterOperatorNext,
            Self::FilterOperatorPrev,
            Self::FilterLogicToggle,
            Self::FilterFocusInput,
            Self::FilterCaseToggle,
            Self::ExportFormatCsv,
            Self::ExportFormatTsv,
            Self::ExportFormatSql,
            Self::ExportFormatJson,
            Self::ExportCyclePrev,
            Self::ExportCycleNext,
            Self::ExportColumnPrev,
            Self::ExportColumnNext,
            Self::ExportColumnStart,
            Self::ExportColumnEnd,
            Self::ExportColumnToggle,
            Self::ExportColumnsToggleAll,
            Self::SqlExecute,
            Self::SqlExplain,
            Self::SqlClear,
            Self::SqlAutocompleteTrigger,
            Self::SqlAutocompleteConfirm,
            Self::SqlHistoryPrev,
            Self::SqlHistoryNext,
            Self::SqlHistoryBrowse,
            Self::ImportRefresh,
            Self::ImportFormatSql,
            Self::ImportFormatCsv,
            Self::ImportFormatTsv,
            Self::ImportFormatJson,
            Self::ImportCyclePrev,
            Self::ImportCycleNext,
            Self::ConnectionTypeSqlite,
            Self::ConnectionTypePostgres,
            Self::ConnectionTypeMySql,
            Self::ConnectionTypePrev,
            Self::ConnectionTypeNext,
            Self::DdlColumnPrev,
            Self::DdlColumnNext,
            Self::DdlColumnStart,
            Self::DdlColumnEnd,
            Self::DdlColumnDelete,
            Self::DdlColumnAddBelow,
            Self::DdlColumnAddAbove,
            Self::DdlColumnTogglePrimaryKey,
            Self::SqliteBrowseFile,
            Self::FormatSelectionCycle,
            Self::HistoryClear,
            Self::HistoryPrev,
            Self::HistoryNext,
            Self::HistoryStart,
            Self::HistoryEnd,
            Self::HistoryPageUp,
            Self::HistoryPageDown,
            Self::HistoryUse,
        ]
    }

    pub fn config_key(self) -> &'static str {
        match self {
            LocalShortcut::Confirm => "dialog.common.confirm",
            LocalShortcut::Cancel => "dialog.common.cancel",
            LocalShortcut::Dismiss => "dialog.common.dismiss",
            LocalShortcut::DangerConfirm => "dialog.confirm.confirm",
            LocalShortcut::DangerCancel => "dialog.confirm.cancel",
            LocalShortcut::HelpScrollUp => "dialog.help.scroll_up",
            LocalShortcut::HelpScrollDown => "dialog.help.scroll_down",
            LocalShortcut::HelpPageUp => "dialog.help.page_up",
            LocalShortcut::HelpPageDown => "dialog.help.page_down",
            LocalShortcut::SidebarItemPrev => "sidebar.list.prev",
            LocalShortcut::SidebarItemNext => "sidebar.list.next",
            LocalShortcut::SidebarItemStart => "sidebar.list.start",
            LocalShortcut::SidebarItemEnd => "sidebar.list.end",
            LocalShortcut::SidebarMoveLeft => "sidebar.list.move_left",
            LocalShortcut::SidebarMoveRight => "sidebar.list.move_right",
            LocalShortcut::SidebarToggle => "sidebar.list.toggle",
            LocalShortcut::SidebarDelete => "sidebar.list.delete",
            LocalShortcut::SidebarEdit => "sidebar.list.edit",
            LocalShortcut::SidebarRename => "sidebar.list.rename",
            LocalShortcut::SidebarRefresh => "sidebar.list.refresh",
            LocalShortcut::SidebarActivate => "sidebar.list.activate",
            LocalShortcut::FilterAdd => "sidebar.filters.add",
            LocalShortcut::FilterDelete => "sidebar.filters.delete",
            LocalShortcut::FilterClearAll => "sidebar.filters.clear_all",
            LocalShortcut::FilterColumnNext => "sidebar.filters.column_next",
            LocalShortcut::FilterColumnPrev => "sidebar.filters.column_prev",
            LocalShortcut::FilterOperatorNext => "sidebar.filters.operator_next",
            LocalShortcut::FilterOperatorPrev => "sidebar.filters.operator_prev",
            LocalShortcut::FilterLogicToggle => "sidebar.filters.logic_toggle",
            LocalShortcut::FilterFocusInput => "sidebar.filters.focus_input",
            LocalShortcut::FilterCaseToggle => "sidebar.filters.case_toggle",
            LocalShortcut::ExportFormatCsv => "dialog.export.format_csv",
            LocalShortcut::ExportFormatTsv => "dialog.export.format_tsv",
            LocalShortcut::ExportFormatSql => "dialog.export.format_sql",
            LocalShortcut::ExportFormatJson => "dialog.export.format_json",
            LocalShortcut::ExportCyclePrev => "dialog.export.cycle_prev",
            LocalShortcut::ExportCycleNext => "dialog.export.cycle_next",
            LocalShortcut::ExportColumnPrev => "dialog.export.column_prev",
            LocalShortcut::ExportColumnNext => "dialog.export.column_next",
            LocalShortcut::ExportColumnStart => "dialog.export.column_start",
            LocalShortcut::ExportColumnEnd => "dialog.export.column_end",
            LocalShortcut::ExportColumnToggle => "dialog.export.column_toggle",
            LocalShortcut::ExportColumnsToggleAll => "dialog.export.columns_toggle_all",
            LocalShortcut::SqlExecute => "editor.sql.execute",
            LocalShortcut::SqlExplain => "editor.sql.explain",
            LocalShortcut::SqlClear => "editor.sql.clear",
            LocalShortcut::SqlAutocompleteTrigger => "editor.sql.autocomplete_trigger",
            LocalShortcut::SqlAutocompleteConfirm => "editor.sql.autocomplete_confirm",
            LocalShortcut::SqlHistoryPrev => "editor.sql.history_prev",
            LocalShortcut::SqlHistoryNext => "editor.sql.history_next",
            LocalShortcut::SqlHistoryBrowse => "editor.sql.history_browse",
            LocalShortcut::ImportRefresh => "dialog.import.refresh",
            LocalShortcut::ImportFormatSql => "dialog.import.format_sql",
            LocalShortcut::ImportFormatCsv => "dialog.import.format_csv",
            LocalShortcut::ImportFormatTsv => "dialog.import.format_tsv",
            LocalShortcut::ImportFormatJson => "dialog.import.format_json",
            LocalShortcut::ImportCyclePrev => "dialog.import.cycle_prev",
            LocalShortcut::ImportCycleNext => "dialog.import.cycle_next",
            LocalShortcut::ConnectionTypeSqlite => "dialog.connection.type_sqlite",
            LocalShortcut::ConnectionTypePostgres => "dialog.connection.type_postgres",
            LocalShortcut::ConnectionTypeMySql => "dialog.connection.type_mysql",
            LocalShortcut::ConnectionTypePrev => "dialog.connection.type_prev",
            LocalShortcut::ConnectionTypeNext => "dialog.connection.type_next",
            LocalShortcut::DdlColumnPrev => "dialog.ddl.column_prev",
            LocalShortcut::DdlColumnNext => "dialog.ddl.column_next",
            LocalShortcut::DdlColumnStart => "dialog.ddl.column_start",
            LocalShortcut::DdlColumnEnd => "dialog.ddl.column_end",
            LocalShortcut::DdlColumnDelete => "dialog.ddl.column_delete",
            LocalShortcut::DdlColumnAddBelow => "dialog.ddl.column_add_below",
            LocalShortcut::DdlColumnAddAbove => "dialog.ddl.column_add_above",
            LocalShortcut::DdlColumnTogglePrimaryKey => "dialog.ddl.column_toggle_primary_key",
            LocalShortcut::SqliteBrowseFile => "dialog.connection.sqlite_browse_file",
            LocalShortcut::FormatSelectionCycle => "dialog.common.format_selection_cycle",
            LocalShortcut::HistoryClear => "dialog.history.clear",
            LocalShortcut::HistoryPrev => "dialog.history.prev",
            LocalShortcut::HistoryNext => "dialog.history.next",
            LocalShortcut::HistoryStart => "dialog.history.start",
            LocalShortcut::HistoryEnd => "dialog.history.end",
            LocalShortcut::HistoryPageUp => "dialog.history.page_up",
            LocalShortcut::HistoryPageDown => "dialog.history.page_down",
            LocalShortcut::HistoryUse => "dialog.history.use",
        }
    }

    pub fn bindings(self) -> Vec<LocalBinding> {
        if let Some(bindings) = current_local_shortcut_overrides()
            .read()
            .get(self.config_key())
        {
            return bindings.clone();
        }
        self.default_bindings()
    }

    pub fn default_keybindings(self) -> Vec<KeyBinding> {
        self.default_bindings()
            .into_iter()
            .map(LocalBinding::key_binding)
            .collect()
    }

    pub fn bindings_for(self, keybindings: &KeyBindings) -> Vec<KeyBinding> {
        keybindings
            .local_bindings_for(self.config_key())
            .map(|bindings| bindings.to_vec())
            .unwrap_or_else(|| self.default_keybindings())
    }

    pub fn is_overridden(self, keybindings: &KeyBindings) -> bool {
        keybindings.local_bindings_for(self.config_key()).is_some()
    }

    fn default_bindings(self) -> Vec<LocalBinding> {
        match self {
            LocalShortcut::Confirm => vec![LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE)],
            LocalShortcut::Cancel => vec![LocalBinding::new(KeyCode::Escape, KeyModifiers::NONE)],
            LocalShortcut::Dismiss => vec![
                LocalBinding::new(KeyCode::Escape, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Q, KeyModifiers::NONE),
            ],
            LocalShortcut::DangerConfirm => vec![
                LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Y, KeyModifiers::NONE),
            ],
            LocalShortcut::DangerCancel => vec![
                LocalBinding::new(KeyCode::Escape, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::N, KeyModifiers::NONE),
            ],
            LocalShortcut::HelpScrollUp => vec![
                LocalBinding::new(KeyCode::K, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::NONE),
            ],
            LocalShortcut::HelpScrollDown => vec![
                LocalBinding::new(KeyCode::J, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::NONE),
            ],
            LocalShortcut::HelpPageUp => vec![
                LocalBinding::new(KeyCode::PageUp, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::U, KeyModifiers::CTRL),
            ],
            LocalShortcut::HelpPageDown => vec![
                LocalBinding::new(KeyCode::PageDown, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::D, KeyModifiers::CTRL),
            ],
            LocalShortcut::SidebarItemPrev => vec![
                LocalBinding::new(KeyCode::K, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::NONE),
            ],
            LocalShortcut::SidebarItemNext => vec![
                LocalBinding::new(KeyCode::J, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::NONE),
            ],
            LocalShortcut::SidebarItemStart => {
                vec![LocalBinding::new(KeyCode::Home, KeyModifiers::NONE)]
            }
            LocalShortcut::SidebarItemEnd => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::End, KeyModifiers::NONE),
            ],
            LocalShortcut::SidebarMoveLeft => vec![
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
            ],
            LocalShortcut::SidebarMoveRight => vec![
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::SidebarToggle => {
                vec![LocalBinding::new(KeyCode::Space, KeyModifiers::NONE)]
            }
            LocalShortcut::SidebarDelete => {
                vec![LocalBinding::new(KeyCode::D, KeyModifiers::NONE)]
            }
            LocalShortcut::SidebarEdit => {
                vec![LocalBinding::new(KeyCode::E, KeyModifiers::NONE)]
            }
            LocalShortcut::SidebarRename => {
                vec![LocalBinding::new(KeyCode::R, KeyModifiers::NONE)]
            }
            LocalShortcut::SidebarRefresh => {
                vec![LocalBinding::new(KeyCode::R, KeyModifiers::SHIFT)]
            }
            LocalShortcut::SidebarActivate => {
                vec![LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterAdd => vec![
                LocalBinding::new(KeyCode::A, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::O, KeyModifiers::NONE),
            ],
            LocalShortcut::FilterDelete => {
                vec![LocalBinding::new(KeyCode::X, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterClearAll => {
                vec![LocalBinding::new(KeyCode::C, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterColumnNext => {
                vec![LocalBinding::new(KeyCode::W, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterColumnPrev => {
                vec![LocalBinding::new(KeyCode::B, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterOperatorNext => {
                vec![LocalBinding::new(KeyCode::N, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterOperatorPrev => {
                vec![LocalBinding::new(KeyCode::N, KeyModifiers::SHIFT)]
            }
            LocalShortcut::FilterLogicToggle => {
                vec![LocalBinding::new(KeyCode::T, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterFocusInput => {
                vec![LocalBinding::new(KeyCode::I, KeyModifiers::NONE)]
            }
            LocalShortcut::FilterCaseToggle => {
                vec![LocalBinding::new(KeyCode::S, KeyModifiers::NONE)]
            }
            LocalShortcut::ExportFormatCsv => {
                vec![LocalBinding::new(KeyCode::Num1, KeyModifiers::NONE)]
            }
            LocalShortcut::ExportFormatTsv => {
                vec![LocalBinding::new(KeyCode::Num2, KeyModifiers::NONE)]
            }
            LocalShortcut::ExportFormatSql => {
                vec![LocalBinding::new(KeyCode::Num3, KeyModifiers::NONE)]
            }
            LocalShortcut::ExportFormatJson => {
                vec![LocalBinding::new(KeyCode::Num4, KeyModifiers::NONE)]
            }
            LocalShortcut::ExportCyclePrev => vec![
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
            ],
            LocalShortcut::ExportCycleNext => vec![
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::ExportColumnPrev => vec![
                LocalBinding::new(KeyCode::K, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::NONE),
            ],
            LocalShortcut::ExportColumnNext => vec![
                LocalBinding::new(KeyCode::J, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::NONE),
            ],
            LocalShortcut::ExportColumnStart => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Home, KeyModifiers::NONE),
            ],
            LocalShortcut::ExportColumnEnd => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::End, KeyModifiers::NONE),
            ],
            LocalShortcut::ExportColumnToggle => {
                vec![LocalBinding::new(KeyCode::Space, KeyModifiers::NONE)]
            }
            LocalShortcut::ExportColumnsToggleAll => {
                vec![LocalBinding::new(KeyCode::A, KeyModifiers::NONE)]
            }
            LocalShortcut::SqlExecute => vec![
                LocalBinding::new(KeyCode::Enter, KeyModifiers::CTRL),
                LocalBinding::new(KeyCode::F5, KeyModifiers::NONE),
            ],
            LocalShortcut::SqlExplain => vec![LocalBinding::new(KeyCode::F6, KeyModifiers::NONE)],
            LocalShortcut::SqlClear => vec![LocalBinding::new(KeyCode::D, KeyModifiers::SHIFT)],
            LocalShortcut::SqlAutocompleteTrigger => vec![
                LocalBinding::new(KeyCode::Space, KeyModifiers::CTRL),
                LocalBinding::new(KeyCode::L, KeyModifiers::ALT),
            ],
            LocalShortcut::SqlAutocompleteConfirm => vec![
                LocalBinding::new(KeyCode::Tab, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE),
            ],
            LocalShortcut::SqlHistoryPrev => vec![
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::K, KeyModifiers::SHIFT),
            ],
            LocalShortcut::SqlHistoryNext => vec![
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::J, KeyModifiers::SHIFT),
            ],
            LocalShortcut::SqlHistoryBrowse => vec![
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::K, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::J, KeyModifiers::SHIFT),
            ],
            LocalShortcut::ImportRefresh => {
                vec![LocalBinding::new(KeyCode::R, KeyModifiers::CTRL)]
            }
            LocalShortcut::ImportFormatSql => {
                vec![LocalBinding::new(KeyCode::Num1, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportFormatCsv => {
                vec![LocalBinding::new(KeyCode::Num2, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportFormatTsv => {
                vec![LocalBinding::new(KeyCode::Num3, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportFormatJson => {
                vec![LocalBinding::new(KeyCode::Num4, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportCyclePrev => vec![
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
            ],
            LocalShortcut::ImportCycleNext => vec![
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::ConnectionTypeSqlite => {
                vec![LocalBinding::new(KeyCode::Num1, KeyModifiers::NONE)]
            }
            LocalShortcut::ConnectionTypePostgres => {
                vec![LocalBinding::new(KeyCode::Num2, KeyModifiers::NONE)]
            }
            LocalShortcut::ConnectionTypeMySql => {
                vec![LocalBinding::new(KeyCode::Num3, KeyModifiers::NONE)]
            }
            LocalShortcut::ConnectionTypePrev => vec![
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
            ],
            LocalShortcut::ConnectionTypeNext => vec![
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::DdlColumnPrev => vec![
                LocalBinding::new(KeyCode::K, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::NONE),
            ],
            LocalShortcut::DdlColumnNext => vec![
                LocalBinding::new(KeyCode::J, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::NONE),
            ],
            LocalShortcut::DdlColumnStart => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Home, KeyModifiers::NONE),
            ],
            LocalShortcut::DdlColumnEnd => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::End, KeyModifiers::NONE),
            ],
            LocalShortcut::DdlColumnDelete => vec![
                LocalBinding::new(KeyCode::D, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Delete, KeyModifiers::NONE),
            ],
            LocalShortcut::DdlColumnAddBelow => {
                vec![LocalBinding::new(KeyCode::O, KeyModifiers::NONE)]
            }
            LocalShortcut::DdlColumnAddAbove => {
                vec![LocalBinding::new(KeyCode::O, KeyModifiers::SHIFT)]
            }
            LocalShortcut::DdlColumnTogglePrimaryKey => {
                vec![LocalBinding::new(KeyCode::Space, KeyModifiers::NONE)]
            }
            LocalShortcut::SqliteBrowseFile => {
                vec![LocalBinding::new(KeyCode::O, KeyModifiers::CTRL)]
            }
            LocalShortcut::FormatSelectionCycle => vec![
                LocalBinding::new(KeyCode::Num1, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Num2, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Num3, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Num4, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::HistoryClear => {
                vec![LocalBinding::new(KeyCode::Delete, KeyModifiers::CTRL)]
            }
            LocalShortcut::HistoryPrev => vec![
                LocalBinding::new(KeyCode::K, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::NONE),
            ],
            LocalShortcut::HistoryNext => vec![
                LocalBinding::new(KeyCode::J, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::NONE),
            ],
            LocalShortcut::HistoryStart => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Home, KeyModifiers::NONE),
            ],
            LocalShortcut::HistoryEnd => vec![
                LocalBinding::new(KeyCode::G, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::End, KeyModifiers::NONE),
            ],
            LocalShortcut::HistoryPageUp => vec![
                LocalBinding::new(KeyCode::PageUp, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::U, KeyModifiers::CTRL),
            ],
            LocalShortcut::HistoryPageDown => vec![
                LocalBinding::new(KeyCode::PageDown, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::D, KeyModifiers::CTRL),
            ],
            LocalShortcut::HistoryUse => vec![
                LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
            ],
        }
    }
}

fn current_local_shortcut_overrides() -> &'static RwLock<HashMap<&'static str, Vec<LocalBinding>>> {
    static LOCAL_SHORTCUT_OVERRIDES: OnceLock<RwLock<HashMap<&'static str, Vec<LocalBinding>>>> =
        OnceLock::new();
    LOCAL_SHORTCUT_OVERRIDES.get_or_init(|| RwLock::new(HashMap::new()))
}

pub fn sync_runtime_local_shortcuts(keybindings: &KeyBindings) {
    let mut overrides = HashMap::new();
    for shortcut in LocalShortcut::all() {
        if let Some(bindings) = keybindings.local_bindings_for(shortcut.config_key()) {
            let local_bindings = bindings
                .iter()
                .map(LocalBinding::from_key_binding)
                .collect::<Vec<_>>();
            if !local_bindings.is_empty() {
                overrides.insert(shortcut.config_key(), local_bindings);
            }
        }
    }
    *current_local_shortcut_overrides().write() = overrides;
}

/// 仅使用给定快捷键列表生成提示。
pub fn shortcut_tooltip(label: &str, shortcuts: &[&str]) -> String {
    if shortcuts.is_empty() {
        return label.to_string();
    }

    format!("{}\n快捷键: {}", label, shortcuts.join(" / "))
}

/// 基于动作与当前绑定生成提示。
pub fn action_tooltip(keybindings: &KeyBindings, action: Action) -> String {
    action_tooltip_with_extras(keybindings, action, action.description(), &[])
}

/// 基于动作、标签与额外快捷键生成提示。
pub fn action_tooltip_with_extras(
    keybindings: &KeyBindings,
    action: Action,
    label: &str,
    extra_shortcuts: &[&str],
) -> String {
    let mut shortcuts: Vec<String> = extra_shortcuts.iter().map(|s| (*s).to_string()).collect();
    let action_shortcut = keybindings.display(action);
    if !action_shortcut.is_empty() && !shortcuts.iter().any(|item| item == &action_shortcut) {
        shortcuts.push(action_shortcut);
    }

    let mut lines = vec![label.to_string()];
    if action.description() != label {
        lines.push(format!("功能: {}", action.description()));
    }
    if !shortcuts.is_empty() {
        lines.push(format!("快捷键: {}", shortcuts.join(" / ")));
    }

    lines.join("\n")
}

pub fn local_shortcut_text(shortcut: LocalShortcut) -> String {
    local_bindings_text(shortcut.bindings())
}

pub fn local_shortcuts_text(shortcuts: &[LocalShortcut]) -> String {
    let mut values: Vec<String> = Vec::new();
    for shortcut in shortcuts {
        for value in local_binding_strings(shortcut.bindings()) {
            if !values.iter().any(|item| item == &value) {
                values.push(value);
            }
        }
    }
    values.join(" / ")
}

pub fn local_shortcut_tooltip(label: &str, shortcut: LocalShortcut) -> String {
    let shortcuts = local_binding_strings(shortcut.bindings());
    let shortcut_refs: Vec<&str> = shortcuts.iter().map(String::as_str).collect();
    shortcut_tooltip(label, &shortcut_refs)
}

pub fn local_shortcuts_tooltip(label: &str, shortcuts: &[LocalShortcut]) -> String {
    let values = local_shortcuts_text(shortcuts);
    let refs: Vec<&str> = if values.is_empty() {
        Vec::new()
    } else {
        values.split(" / ").collect()
    };
    shortcut_tooltip(label, &refs)
}

pub fn local_shortcut_pressed(ctx: &egui::Context, shortcut: LocalShortcut) -> bool {
    shortcut
        .bindings()
        .into_iter()
        .any(|binding| binding.is_pressed(ctx))
}

pub fn consume_local_shortcut(input: &mut InputState, shortcut: LocalShortcut) -> bool {
    shortcut
        .bindings()
        .into_iter()
        .any(|binding| binding.consume(input))
}

fn local_binding_strings(bindings: Vec<LocalBinding>) -> Vec<String> {
    let mut values = Vec::new();
    for binding in bindings {
        let display = binding.display();
        if !values.iter().any(|item| item == &display) {
            values.push(display);
        }
    }
    values
}

fn local_bindings_text(bindings: Vec<LocalBinding>) -> String {
    local_binding_strings(bindings).join(" / ")
}

#[cfg(test)]
mod tests {
    use super::{
        LocalShortcut, action_tooltip_with_extras, local_shortcut_pressed, local_shortcut_text,
        local_shortcut_tooltip, local_shortcuts_text, local_shortcuts_tooltip, shortcut_tooltip,
        sync_runtime_local_shortcuts,
    };
    use crate::core::{Action, KeyBinding, KeyBindings, KeyCode};

    #[test]
    fn action_tooltip_appends_current_binding() {
        let keybindings = KeyBindings::default();
        let tooltip =
            action_tooltip_with_extras(&keybindings, Action::AddFilter, "打开筛选面板", &["/"]);

        assert!(tooltip.contains("打开筛选面板"));
        assert!(tooltip.contains("Ctrl+F"));
        assert!(tooltip.contains("/"));
    }

    #[test]
    fn shortcut_tooltip_keeps_local_shortcuts() {
        let tooltip = shortcut_tooltip("取消", &["Esc"]);

        assert_eq!(tooltip, "取消\n快捷键: Esc");
    }

    #[test]
    fn local_shortcut_text_formats_combined_shortcuts() {
        let text = local_shortcut_text(LocalShortcut::SqlExecute);

        assert_eq!(text, "Ctrl+Enter / F5");
    }

    #[test]
    fn local_shortcut_tooltip_uses_named_shortcut_set() {
        let tooltip = local_shortcut_tooltip("关闭对话框", LocalShortcut::Dismiss);

        assert_eq!(tooltip, "关闭对话框\n快捷键: Esc / Q");
    }

    #[test]
    fn local_shortcuts_tooltip_deduplicates_entries() {
        let tooltip = local_shortcuts_tooltip(
            "补全",
            &[
                LocalShortcut::SqlAutocompleteTrigger,
                LocalShortcut::SqlAutocompleteConfirm,
                LocalShortcut::Confirm,
            ],
        );

        assert_eq!(tooltip, "补全\n快捷键: Ctrl+Space / Alt+L / Tab / Enter");
    }

    #[test]
    fn local_shortcut_text_uses_actual_binding_order() {
        let text = local_shortcut_text(LocalShortcut::SqlHistoryBrowse);

        assert_eq!(text, "Shift+Up / Shift+Down / Shift+K / Shift+J");
    }

    #[test]
    fn local_shortcuts_text_deduplicates_and_preserves_order() {
        let text = local_shortcuts_text(&[
            LocalShortcut::ExportFormatCsv,
            LocalShortcut::ExportCyclePrev,
            LocalShortcut::ExportCycleNext,
        ]);

        assert_eq!(text, "1 / H / Left / L / Right");
    }

    #[test]
    fn local_shortcut_pressed_is_false_without_input() {
        let ctx = egui::Context::default();

        assert!(!local_shortcut_pressed(&ctx, LocalShortcut::SqlExecute));
    }

    #[test]
    fn runtime_local_shortcut_override_changes_display_text() {
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_bindings(
            "dialog.common.dismiss",
            vec![
                KeyBinding::key_only(KeyCode::Escape),
                KeyBinding::key_only(KeyCode::X),
            ],
        );

        sync_runtime_local_shortcuts(&keybindings);

        assert_eq!(local_shortcut_text(LocalShortcut::Dismiss), "Esc / X");
        assert_eq!(
            local_shortcut_tooltip("关闭对话框", LocalShortcut::Dismiss),
            "关闭对话框\n快捷键: Esc / X"
        );

        sync_runtime_local_shortcuts(&KeyBindings::default());
    }
}
