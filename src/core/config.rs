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
        dirs::config_dir().map(|p| p.join("gridix"))
    }

    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("config.toml"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            tracing::warn!("无法获取配置文件路径");
            return Self::default();
        };
        
        if !path.exists() {
            // 首次启动，配置文件不存在是正常的
            return Self::default();
        }
        
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "读取配置文件失败");
                return Self::default();
            }
        };
        
        match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!(error = %e, path = ?path, "解析配置文件失败");
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::config_dir().ok_or("无法找到配置目录")?;
        let path = Self::config_path().ok_or("无法找到配置路径")?;

        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let toml_str = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        
        // 原子写入：先写入临时文件，再重命名
        // 这样即使程序在写入过程中崩溃，原配置文件也不会损坏
        let temp_path = path.with_extension("toml.tmp");
        fs::write(&temp_path, &toml_str).map_err(|e| format!("写入临时文件失败: {}", e))?;
        
        // 设置临时文件权限（在重命名之前）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&temp_path, permissions).map_err(|e| e.to_string())?;
        }
        
        // 原子重命名（在同一文件系统上是原子操作）
        fs::rename(&temp_path, &path).map_err(|e| format!("重命名配置文件失败: {}", e))?;

        // Windows 上无法设置类似权限，记录警告（仅首次）
        #[cfg(windows)]
        {
            use std::sync::Once;
            static WARN_ONCE: Once = Once::new();
            WARN_ONCE.call_once(|| {
                tracing::warn!("Windows 上配置文件权限无法限制为私有，请确保配置目录安全");
            });
        }

        Ok(())
    }
}
