//! 主题、缩放与配置持久化
//!
//! 将与 UI 偏好和历史记录相关的逻辑从 `mod.rs` 拆分。

use eframe::egui;

use crate::core::{HighlightColors, ThemePreset, clear_highlight_cache, constants};

use super::DbManagerApp;

impl DbManagerApp {
    /// 设置 UI 缩放比例
    pub(in crate::app) fn set_ui_scale(&mut self, ctx: &egui::Context, scale: f32) {
        let scale = scale.clamp(constants::ui::UI_SCALE_MIN, constants::ui::UI_SCALE_MAX);
        self.ui_scale = scale;
        self.app_config.ui_scale = scale;
        ctx.set_pixels_per_point(self.base_pixels_per_point * scale);
        let _ = self.app_config.save();
    }

    /// 检查当前连接是否是 MySQL（用于选择 SQL 引号类型）
    pub(in crate::app) fn is_mysql(&self) -> bool {
        self.manager
            .get_active()
            .map(|c| matches!(c.config.db_type, crate::database::DatabaseType::MySQL))
            .unwrap_or(false)
    }

    pub(in crate::app) fn set_theme(&mut self, ctx: &egui::Context, preset: ThemePreset) {
        self.theme_manager.set_theme(preset);
        self.theme_manager.apply(ctx);
        self.highlight_colors = HighlightColors::from_theme(&self.theme_manager.colors);
        self.app_config.theme_preset = preset;
        // 清除语法高亮缓存，确保使用新主题颜色
        clear_highlight_cache();
        let _ = self.app_config.save();
    }

    pub(in crate::app) fn save_config(&mut self) {
        // 保存当前连接的历史记录
        self.save_current_history();

        self.app_config.connections = self
            .manager
            .connections
            .values()
            .map(|c| c.config.clone())
            .collect();
        self.app_config.query_history = self.query_history.clone();
        self.app_config.connection_dialog_show_advanced = self.connection_dialog_show_advanced;
        let _ = self.app_config.save();

        for saved_config in &self.app_config.connections {
            if let Some(connection) = self.manager.connections.get_mut(&saved_config.name) {
                connection.config.password_ref = saved_config.password_ref.clone();
            }
        }
    }

    /// 保存当前连接的历史记录到配置
    pub(in crate::app) fn save_current_history(&mut self) {
        if let Some(conn_name) = &self.current_history_connection {
            self.app_config
                .command_history
                .insert(conn_name.clone(), self.command_history.clone());
        }
    }

    /// 加载指定连接的历史记录
    pub(in crate::app) fn load_history_for_connection(&mut self, conn_name: &str) {
        // 先保存当前连接的历史
        self.save_current_history();

        // 加载新连接的历史
        self.command_history = self
            .app_config
            .command_history
            .get(conn_name)
            .cloned()
            .unwrap_or_default();
        self.current_history_connection = Some(conn_name.to_string());
        self.history_index = None;
    }
}
