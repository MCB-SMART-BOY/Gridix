//! UI 状态层（Layer 3）
//!
//! 所有渲染状态，不包含任何数据库逻辑。
//! 被 ui/ Layer 使用。目前作为类型容器，完整提取将在后续提交中完成。

// 重导出 UI 基础类型（这些类型应该在未来移到 state/ 或 types.rs）
//! UI 状态层（Layer 3）
//!
//! 所有渲染状态，不包含任何数据库逻辑。
//! 被 ui/ Layer 使用。完整提取将在后续提交中完成。

use crate::core::{HighlightColors, NotificationManager, ProgressManager, ThemeManager};
use crate::ui::{
    DataGridState, EditorMode, ERDiagramState, ExportConfig, FocusArea,
    ImportState, SidebarPanelState, SidebarSection,
};

/// UI 状态聚合结构体
///
/// 目标：DbManagerApp 持有 `state: UiState` 字段，
/// 将所有渲染状态从平铺字段减少到 1 个。
pub struct UiState {
    pub focus_area: FocusArea,
    pub sidebar_section: SidebarSection,
    pub editor_mode: EditorMode,
    pub show_sidebar: bool,
    pub sidebar_width: f32,
    pub sidebar_panel_state: SidebarPanelState,
    pub show_sql_editor: bool,
    pub sql_editor_height: f32,
    pub show_autocomplete: bool,
    pub selected_completion: usize,
    pub show_connection_dialog: bool,
    pub show_delete_confirm: bool,
    pub show_export_dialog: bool,
    pub show_import_dialog: bool,
    pub connection_dialog_show_advanced: bool,
    pub show_er_diagram: bool,
    pub er_diagram_state: ERDiagramState,
    pub grid_state: DataGridState,
    pub export_config: ExportConfig,
    pub import_state: ImportState,
    pub theme_manager: ThemeManager,
    pub highlight_colors: HighlightColors,
    pub ui_scale: f32,
    pub base_pixels_per_point: f32,
    pub notifications: NotificationManager,
    pub progress: ProgressManager,
}

impl Default for UiState {
    fn default() -> Self {
        let theme_manager = ThemeManager::default();
        let highlight_colors = HighlightColors::from_theme(&theme_manager.colors());
        Self {
            focus_area: FocusArea::DataGrid,
            sidebar_section: SidebarSection::Tables,
            editor_mode: EditorMode::Normal,
            show_sidebar: true,
            sidebar_width: 280.0,
            sidebar_panel_state: SidebarPanelState::default(),
            show_sql_editor: true,
            sql_editor_height: 200.0,
            show_autocomplete: false,
            selected_completion: 0,
            show_connection_dialog: false,
            show_delete_confirm: false,
            show_export_dialog: false,
            show_import_dialog: false,
            connection_dialog_show_advanced: false,
            show_er_diagram: false,
            er_diagram_state: ERDiagramState::default(),
            grid_state: DataGridState::default(),
            export_config: ExportConfig::default(),
            import_state: ImportState::default(),
            theme_manager,
            highlight_colors,
            ui_scale: 1.0,
            base_pixels_per_point: 1.0,
            notifications: NotificationManager::default(),
            progress: ProgressManager::default(),
        }
    }
}
