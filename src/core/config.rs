use super::history::QueryHistory;
use super::keybindings::KeyBindings;
use super::theme::ThemePreset;
use crate::database::{
    ConnectionConfig, decrypt_password, load_password_secret, store_password_secret,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OnboardingProgress {
    /// 已完成环境检测
    #[serde(default)]
    pub environment_checked: bool,
    /// 已创建至少一个连接
    #[serde(default)]
    pub connection_created: bool,
    /// 已完成数据库初始化（创建数据库）
    #[serde(default)]
    pub database_initialized: bool,
    /// 已完成创建用户（仅 MySQL/PostgreSQL）
    #[serde(default)]
    pub user_created: bool,
    /// 已执行首条查询
    #[serde(default)]
    pub first_query_executed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarConfig {
    /// 是否允许在侧边栏列表边界通过 j/k 跨 panel 转移
    #[serde(default = "default_sidebar_edge_transfer")]
    pub edge_transfer: bool,
}

impl Default for SidebarConfig {
    fn default() -> Self {
        Self {
            edge_transfer: default_sidebar_edge_transfer(),
        }
    }
}

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
    /// 兼容旧版本：从 config.toml 读取的内联快捷键绑定。
    ///
    /// TODO(v4 compatibility window): 保持只读一个兼容发布周期，后续迁移为
    /// 显式导入流程并移除此字段。
    #[serde(default, skip_serializing)]
    pub keybindings: KeyBindings,
    /// 首次启动引导进度
    #[serde(default)]
    pub onboarding: OnboardingProgress,
    /// 连接对话框是否展开高级配置
    #[serde(default = "default_connection_dialog_show_advanced")]
    pub connection_dialog_show_advanced: bool,
    /// 侧边栏工作流配置
    #[serde(default)]
    pub sidebar: SidebarConfig,
}

fn default_ui_scale() -> f32 {
    1.0
}

fn default_light_theme() -> ThemePreset {
    ThemePreset::TokyoNightLight
}

fn default_dark_theme() -> ThemePreset {
    ThemePreset::TokyoNightStorm
}

fn default_dark_mode() -> bool {
    true
}

fn default_connection_dialog_show_advanced() -> bool {
    false
}

fn default_sidebar_edge_transfer() -> bool {
    true
}

fn backup_invalid_config(path: &Path) {
    let backup_path = path.with_extension("toml.bak");
    if let Err(e) = fs::copy(path, &backup_path) {
        tracing::warn!(error = %e, original = ?path, backup = ?backup_path, "备份损坏配置文件失败");
    } else {
        tracing::warn!(original = ?path, backup = ?backup_path, "已备份无法解析的配置文件");
    }
}

fn hydrate_connection_passwords(connections: &mut [ConnectionConfig]) -> bool {
    let mut migration_needed = false;

    for connection in connections {
        if let Some(password_ref) = connection.password_ref.clone() {
            match load_password_secret(&password_ref) {
                Ok(Some(password)) => {
                    connection.password = password;
                }
                Ok(None) => {
                    tracing::warn!(
                        connection = %connection.name,
                        password_ref = %password_ref,
                        "系统密钥链中缺少保存的密码，需要重新输入"
                    );
                    connection.password.clear();
                }
                Err(e) => {
                    tracing::warn!(
                        connection = %connection.name,
                        password_ref = %password_ref,
                        error = %e,
                        "读取系统密钥链中的密码失败，需要重新输入"
                    );
                    connection.password.clear();
                }
            }
            continue;
        }

        if connection.password.is_empty() {
            continue;
        }

        migration_needed = true;
        match decrypt_password(&connection.password) {
            Ok(password) => {
                connection.password = password;
            }
            Err(e) => {
                tracing::warn!(
                    connection = %connection.name,
                    error = %e,
                    "读取旧版密码字段失败，已保留连接配置但需要重新输入密码"
                );
                connection.password.clear();
            }
        }
    }

    migration_needed
}

fn persist_connection_passwords(connections: &mut [ConnectionConfig]) -> Result<(), String> {
    let mut warnings = Vec::new();

    for connection in connections {
        if connection.password.is_empty() {
            continue;
        }

        let password_ref = connection
            .password_ref
            .clone()
            .unwrap_or_else(|| format!("connection:{}", uuid::Uuid::new_v4()));

        match store_password_secret(&password_ref, &connection.password) {
            Ok(()) => {
                connection.password_ref = Some(password_ref);
            }
            Err(e) => {
                tracing::warn!(
                    connection = %connection.name,
                    error = %e,
                    "无法将密码写入系统密钥链；连接配置仍会保存，但下次启动可能需要重新输入密码"
                );
                warnings.push(format!("{}: {}", connection.name, e));
            }
        }
    }

    if warnings.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "以下连接的密码未能写入系统密钥链: {}",
            warnings.join("; ")
        ))
    }
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
            keybindings: KeyBindings::default(),
            onboarding: OnboardingProgress::default(),
            connection_dialog_show_advanced: default_connection_dialog_show_advanced(),
            sidebar: SidebarConfig::default(),
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
            return Self::default();
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "读取配置文件失败");
                return Self::default();
            }
        };

        let mut config: Self = match toml::from_str(&content) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!(error = %e, path = ?path, "解析配置文件失败");
                backup_invalid_config(&path);
                return Self::default();
            }
        };

        let migration_needed = hydrate_connection_passwords(&mut config.connections);
        if migration_needed && let Err(e) = config.save() {
            tracing::warn!(error = %e, "迁移旧版密码到系统密钥链失败");
        }

        config
    }

    pub fn save(&mut self) -> Result<(), String> {
        let dir = Self::config_dir().ok_or("无法找到配置目录")?;
        let path = Self::config_path().ok_or("无法找到配置路径")?;

        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let password_warning = persist_connection_passwords(&mut self.connections).err();
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

        if let Some(warning) = password_warning {
            Err(warning)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AppConfig;

    #[test]
    fn load_from_toml_preserves_connections_when_legacy_password_is_broken() {
        let content = r#"
            [[connections]]
            name = "primary"
            db_type = "PostgreSQL"
            host = "db.example.com"
            port = 5432
            username = "app"
            password = "v1:not-valid-base64"
            database = "appdb"

            theme_preset = "TokyoNightStorm"
            is_dark_mode = true
        "#;

        let mut config: AppConfig = toml::from_str(content).expect("parse config");
        super::hydrate_connection_passwords(&mut config.connections);

        assert_eq!(config.connections.len(), 1);
        assert_eq!(config.connections[0].name, "primary");
        assert!(config.connections[0].password.is_empty());
        assert!(config.is_dark_mode);
    }

    #[test]
    fn load_from_toml_reads_sidebar_edge_transfer() {
        let content = r#"
            connections = []

            [sidebar]
            edge_transfer = false
        "#;

        let config: AppConfig = toml::from_str(content).expect("parse config");

        assert!(!config.sidebar.edge_transfer);
    }
}
