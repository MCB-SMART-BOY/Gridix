//! UI 状态层（Layer 3）
//!
//! 已从 DbManagerApp 迁移的 UI 渲染状态。
//! 更多字段将在后续提交中逐步迁移。

use crate::core::{HighlightColors, ThemeManager};
use crate::ui::{EditorMode, ERDiagramState, FocusArea, SidebarSection};

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
            show_er_diagram: false,
            er_diagram_state: ERDiagramState::default(),
        }
    }
}
