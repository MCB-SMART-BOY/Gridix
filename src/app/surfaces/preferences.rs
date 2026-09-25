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
        self.state.ui_scale = scale;
        self.app_config.ui_scale = scale;
        ctx.set_pixels_per_point(self.state.base_pixels_per_point * scale);
        if let Err(e) = self.app_config.save() {
            tracing::warn!(%e, "保存配置失败");
        }
    }

    /// 检查当前连接是否是 MySQL（用于选择 SQL 引号类型）
    pub(in crate::app) fn is_mysql(&self) -> bool {
        self.session
            .manager
            .get_active()
            .map(|c| matches!(c.config.db_type, crate::data::DatabaseType::MySQL))
            .unwrap_or(false)
    }

    pub(in crate::app) fn set_theme(&mut self, ctx: &egui::Context, preset: ThemePreset) {
        self.state.theme_manager.set_theme(preset);
        self.state.theme_manager.apply(ctx);
        self.state.highlight_colors = HighlightColors::from_theme(&self.state.theme_manager.colors);
        self.app_config.theme_preset = preset;
        // 清除语法高亮缓存，确保使用新主题颜色
        clear_highlight_cache();
        if let Err(e) = self.app_config.save() {
            tracing::warn!(%e, "保存配置失败");
        }
    }

    /// 应用由系统主题推导出的主题，不改变用户固定的主题选择，也不立即写盘。
    fn apply_effective_theme(&mut self, ctx: &egui::Context, preset: ThemePreset) {
        if self.state.theme_manager.current == preset {
            return;
        }
        self.state.theme_manager.set_theme(preset);
        self.state.theme_manager.apply(ctx);
        self.state.highlight_colors = HighlightColors::from_theme(&self.state.theme_manager.colors);
        clear_highlight_cache();
        self.session.needs_repaint = true;
    }

    /// 跟随系统亮暗模式：系统主题变化时切换 `light_theme` / `dark_theme`。
    ///
    /// 每帧调用，但只在解析结果与生效主题不一致时应用样式（`apply_effective_theme`
    /// 内部按 `ThemeManager::current` 早退），因此不会每帧重建 egui 样式与高亮缓存。
    /// 同时校正启动时可能残留的不一致：启动样式来自 `theme_preset`，而跟随期间
    /// 生效的是 `is_dark_mode` 推导出的预设（评审 finding-1）。
    pub(in crate::app) fn sync_system_theme(&mut self, ctx: &egui::Context) {
        if !self.app_config.theme_follows_system {
            return;
        }
        let system_is_dark = ctx.system_theme().map(|theme| theme == egui::Theme::Dark);
        let is_dark = crate::core::resolve_dark_mode(system_is_dark, self.app_config.is_dark_mode);
        let mode_changed = is_dark != self.app_config.is_dark_mode;
        self.app_config.is_dark_mode = is_dark;
        let preset = if is_dark {
            self.app_config.dark_theme
        } else {
            self.app_config.light_theme
        };
        self.apply_effective_theme(ctx, preset);
        if mode_changed {
            self.save_config_debounced();
        }
    }

    pub(in crate::app) fn save_config(&mut self) {
        // 保存当前连接的历史记录
        self.save_current_history();

        self.app_config.connections = self
            .session
            .manager
            .connections
            .values()
            .map(|c| c.config.clone())
            .collect();
        self.app_config.query_history = self.session.query_history.clone();
        self.app_config.connection_dialog_show_advanced =
            self.state.connection_dialog_show_advanced;
        if let Err(e) = self.app_config.save() {
            tracing::warn!(%e, "保存配置失败");
        }

        for saved_config in &self.app_config.connections {
            if let Some(connection) = self.session.manager.connections.get_mut(&saved_config.name) {
                connection.config.password_ref = saved_config.password_ref.clone();
            }
        }
    }

    /// 保存当前连接的历史记录到配置
    pub(in crate::app) fn save_current_history(&mut self) {
        if let Some(conn_name) = &self.session.current_history_connection {
            self.app_config
                .command_history
                .insert(conn_name.clone(), self.session.command_history.clone());
        }
    }

    /// 加载指定连接的历史记录
    pub(in crate::app) fn load_history_for_connection(&mut self, conn_name: &str) {
        // 先保存当前连接的历史
        self.save_current_history();

        // 加载新连接的历史
        self.session.command_history = self
            .app_config
            .command_history
            .get(conn_name)
            .cloned()
            .unwrap_or_default();
        self.session.current_history_connection = Some(conn_name.to_string());
        self.session.history_index = None;
    }
}

#[cfg(test)]
mod tests {
    use crate::core::{ThemeManager, ThemePreset};

    /// 跟随模式下重启：启动样式来自 `theme_preset`，与 `is_dark_mode` 不一致时必须被纠正。
    #[test]
    fn follow_mode_reapplies_stored_mode_after_restart() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        app.app_config.theme_follows_system = true;
        app.app_config.dark_theme = ThemePreset::TokyoNightStorm;
        app.app_config.light_theme = ThemePreset::TokyoNightLight;
        app.app_config.is_dark_mode = false;

        // 启动路径：按 theme_preset 初始化（暗），与 is_dark_mode(亮) 矛盾。
        app.state
            .theme_manager
            .set_theme(ThemePreset::TokyoNightStorm);
        app.state.theme_manager.apply(&ctx);
        assert_eq!(
            app.state.theme_manager.current,
            ThemePreset::TokyoNightStorm
        );

        // 无头 context 不提供系统主题，解析结果保持 is_dark_mode=false，即亮色。
        app.sync_system_theme(&ctx);

        assert_eq!(
            app.state.theme_manager.current,
            ThemePreset::TokyoNightLight,
            "跟随模式必须把启动时残留的主题纠正回 is_dark_mode"
        );
    }

    /// 应用主题必须钉住 egui 主题槽，否则系统主题翻转时 egui 会切换到未写入的槽。
    #[test]
    fn applying_a_theme_pins_the_egui_theme_slot() {
        let ctx = egui::Context::default();
        let mut manager = ThemeManager::new(ThemePreset::TokyoNightLight);
        manager.apply(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Light);

        manager.set_theme(ThemePreset::TokyoNightStorm);
        manager.apply(&ctx);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
    }
}
