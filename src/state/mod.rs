//! UI 状态层（Layer 3）
//!
//! 已从 DbManagerApp 迁移的 UI 渲染状态。
//! 更多字段将在后续提交中逐步迁移。

use crate::core::{HighlightColors, ThemeManager};
use crate::ui::{EditorMode, ERDiagramState, ExportConfig, FocusArea, ImportState, SidebarSection};
use crate::ui::{CreateDbDialogState, CreateUserDialogState, DdlDialogState, HelpState};
use crate::ui::{HistoryPanelState, KeyBindingsDialogState};
use crate::ui::{ToolbarMenuDialogState, ToolbarThemeDialogState};

/// UI 状态（逐步从 DbManagerApp 提取中）
pub struct UiState {
    pub theme_manager: ThemeManager,
    pub highlight_colors: HighlightColors,
    pub ui_scale: f32,
    pub base_pixels_per_point: f32,
    pub focus_area: FocusArea,
    pub last_non_er_workspace_focus: FocusArea,
    pub sidebar_section: SidebarSection,
    pub editor_mode: EditorMode,
    pub show_sidebar: bool,
    pub sidebar_width: f32,
    pub show_sql_editor: bool,
    pub focus_sql_editor: bool,
    pub sql_editor_height: f32,
    pub show_autocomplete: bool,
    pub selected_completion: usize,
    pub show_connection_dialog: bool,
    pub connection_dialog_show_advanced: bool,
    pub show_export_dialog: bool,
    pub show_import_dialog: bool,
    pub show_delete_confirm: bool,
    pub show_history_panel: bool,
    pub show_help: bool,
    pub show_about: bool,
    pub export_config: ExportConfig,
    pub import_state: ImportState,
    pub ddl_dialog_state: DdlDialogState,
    pub create_db_dialog_state: CreateDbDialogState,
    pub history_panel_state: HistoryPanelState,
    pub keybindings_dialog_state: KeyBindingsDialogState,
    pub toolbar_actions_menu_state: ToolbarMenuDialogState,
    pub toolbar_create_menu_state: ToolbarMenuDialogState,
    pub toolbar_theme_dialog_state: ToolbarThemeDialogState,
    pub help_state: HelpState,
    pub create_user_dialog_state: CreateUserDialogState,
    pub show_welcome_setup_dialog: bool,
    pub show_er_diagram: bool,
    pub er_diagram_state: ERDiagramState,
}

impl Default for UiState {
    fn default() -> Self {
        let theme_manager = ThemeManager::default();
        let highlight_colors = HighlightColors::from_theme(&theme_manager.colors);
        Self {
            theme_manager,
            highlight_colors,
            ui_scale: 1.0,
            base_pixels_per_point: 1.0,
            focus_area: FocusArea::DataGrid,
            last_non_er_workspace_focus: FocusArea::DataGrid,
            sidebar_section: SidebarSection::Tables,
            editor_mode: EditorMode::Normal,
            show_sidebar: true,
            sidebar_width: 280.0,
            show_sql_editor: true,
            focus_sql_editor: false,
            sql_editor_height: 200.0,
            show_autocomplete: false,
            selected_completion: 0,
            show_connection_dialog: false,
            connection_dialog_show_advanced: false,
            show_export_dialog: false,
            show_import_dialog: false,
            show_delete_confirm: false,
            show_history_panel: false,
            show_help: false,
            show_about: false,
            export_config: ExportConfig::default(),
            import_state: ImportState::default(),
            ddl_dialog_state: DdlDialogState::default(),
            create_db_dialog_state: CreateDbDialogState::default(),
            history_panel_state: HistoryPanelState::default(),
            keybindings_dialog_state: KeyBindingsDialogState::default(),
            toolbar_actions_menu_state: ToolbarMenuDialogState::default(),
            toolbar_create_menu_state: ToolbarMenuDialogState::default(),
            toolbar_theme_dialog_state: ToolbarThemeDialogState::default(),
            help_state: HelpState::default(),
            create_user_dialog_state: CreateUserDialogState::default(),
            show_welcome_setup_dialog: false,
            show_er_diagram: false,
            er_diagram_state: ERDiagramState::default(),
        }
    }
}
