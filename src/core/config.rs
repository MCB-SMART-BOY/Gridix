use super::history::QueryHistory;
use super::theme::ThemePreset;
use crate::database::ConnectionConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    pub connections: Vec<ConnectionConfig>,
    #[serde(default)]
    pub theme_preset: ThemePreset,
    /// 日间模式主题
    #[serde(default = "default_light_theme")]
    pub light_theme: ThemePreset,
    /// 夜间模式主题
    #[serde(default = "default_dark_theme")]
    pub dark_theme: ThemePreset,
    /// 当前是否为夜间模式
    #[serde(default = "default_dark_mode")]
    pub is_dark_mode: bool,
    #[serde(default)]
    pub query_history: QueryHistory,
    /// 每个连接的 SQL 命令历史记录 (连接名 -> SQL 列表)
    #[serde(default)]
    pub command_history: HashMap<String, Vec<String>>,
    /// UI 缩放比例 (0.5 - 2.0)
    #[serde(default = "default_ui_scale")]
    pub ui_scale: f32,
}

fn default_ui_scale() -> f32 {
    1.0
}

fn default_light_theme() -> ThemePreset {
    ThemePreset::TokyoNightLight
}

fn default_dark_theme() -> ThemePreset {
    ThemePreset::TokyoNight
}

fn default_dark_mode() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            connections: Vec::new(),
            theme_preset: ThemePreset::default(),
            light_theme: default_light_theme(),
            dark_theme: default_dark_theme(),
            is_dark_mode: default_dark_mode(),
            query_history: QueryHistory::new(100),
            command_history: HashMap::new(),
            ui_scale: default_ui_scale(),
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("rust-db-manager"))
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir().ok_or("无法找到配置目录")?;
        let path = Self::config_path().ok_or("无法找到配置路径")?;

        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let toml_str = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(&path, toml_str).map_err(|e| e.to_string())?;

        Ok(())
    }
}
