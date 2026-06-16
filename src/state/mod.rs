//! UI 状态层（Layer 3）
//!
//! 已从 DbManagerApp 迁移的 UI 渲染状态。
//! 更多字段将在后续提交中逐步迁移。

use crate::core::{HighlightColors, ThemeManager};

/// UI 状态（逐步从 DbManagerApp 提取中）
///
/// 当前已迁移：主题、缩放、高亮颜色。
/// 其余字段仍在 DbManagerApp 上直接维护，
/// 将在完成渲染层解耦后迁移。
pub struct UiState {
    pub theme_manager: ThemeManager,
    pub highlight_colors: HighlightColors,
    pub ui_scale: f32,
    pub base_pixels_per_point: f32,
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
        }
    }
}
