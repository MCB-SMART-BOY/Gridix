use super::history::QueryHistory;
use super::keybindings::KeyBindings;
use super::theme::ThemePreset;
// NOTE: Layer violation — core/L0 depends on data/L1 for ConnectionConfig.
// AppConfig naturally owns connection configurations. The password migration
// (decrypt/load/store) is transitional and will be removed once v4→v6 migration
// window closes. This is an intentional exception, not a design flaw.
use crate::data::{
    ConnectionConfig, decrypt_password, load_password_secret, store_password_secret,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 引入 workbench 持久化配置后的配置文件版本。
pub const CONFIG_VERSION_WORKBENCH: u32 = 3;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchActivity {
    #[default]
    Explorer,
    Filters,
    Objects,
    History,
    Help,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkbenchDensity {
    Comfortable,
    #[default]
    Compact,
    Dense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum BottomPanelTab {
    #[default]
    Results,
    Messages,
    Explain,
    History,
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RightInspectorTab {
    #[default]
    Properties,
    Schema,
    Row,
    Cell,
    ErSelection,
    Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TableOpenMode {
    #[default]
    ReuseActiveTableView,
    OpenNewTableView,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResultPlacement {
    #[default]
    BottomPanel,
    EditorTab,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimarySidebarConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_primary_sidebar_width")]
    pub width: f32,
    #[serde(default = "default_primary_sidebar_min_width")]
    pub min_width: f32,
    #[serde(default = "default_primary_sidebar_max_width")]
    pub max_width: f32,
    #[serde(default = "default_true")]
    pub edge_transfer: bool,
}

impl Default for PrimarySidebarConfig {
    fn default() -> Self {
        Self {
            visible: true,
            width: default_primary_sidebar_width(),
            min_width: default_primary_sidebar_min_width(),
            max_width: default_primary_sidebar_max_width(),
            edge_transfer: true,
        }
    }
}

impl PrimarySidebarConfig {
    pub fn normalize(&mut self) {
        normalize_dimension_triplet(
            &mut self.width,
            &mut self.min_width,
            &mut self.max_width,
            default_primary_sidebar_width(),
            default_primary_sidebar_min_width(),
            default_primary_sidebar_max_width(),
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BottomPanelConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_bottom_panel_height")]
    pub height: f32,
    #[serde(default = "default_bottom_panel_min_height")]
    pub min_height: f32,
    #[serde(default = "default_bottom_panel_max_ratio")]
    pub max_height_ratio: f32,
    #[serde(default)]
    pub active_tab: BottomPanelTab,
    #[serde(default = "default_true")]
    pub auto_open_on_query: bool,
    #[serde(default = "default_true")]
    pub auto_focus_results_on_execute: bool,
}

impl Default for BottomPanelConfig {
    fn default() -> Self {
        Self {
            visible: true,
            height: default_bottom_panel_height(),
            min_height: default_bottom_panel_min_height(),
            max_height_ratio: default_bottom_panel_max_ratio(),
            active_tab: BottomPanelTab::default(),
            auto_open_on_query: true,
            auto_focus_results_on_execute: true,
        }
    }
}

impl BottomPanelConfig {
    pub fn normalize(&mut self) {
        self.min_height = sanitize_dimension(self.min_height, default_bottom_panel_min_height());
        self.max_height_ratio =
            sanitize_ratio(self.max_height_ratio, default_bottom_panel_max_ratio());
        self.height =
            sanitize_dimension(self.height, default_bottom_panel_height()).max(self.min_height);
    }

    pub fn normalize_for_height(&mut self, viewport_height: f32) {
        self.normalize();
        let viewport_height = sanitize_dimension(viewport_height, default_bottom_panel_height());
        let effective_max_height = viewport_height * self.max_height_ratio;
        let min_height = self.min_height.min(effective_max_height);
        self.height = self.height.clamp(min_height, effective_max_height);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RightInspectorConfig {
    #[serde(default)]
    pub visible: bool,
    #[serde(default = "default_right_inspector_width")]
    pub width: f32,
    #[serde(default = "default_right_inspector_min_width")]
    pub min_width: f32,
    #[serde(default = "default_right_inspector_max_width")]
    pub max_width: f32,
    #[serde(default)]
    pub active_tab: RightInspectorTab,
    #[serde(default = "default_true")]
    pub auto_open_on_inspect: bool,
}

impl Default for RightInspectorConfig {
    fn default() -> Self {
        Self {
            visible: false,
            width: default_right_inspector_width(),
            min_width: default_right_inspector_min_width(),
            max_width: default_right_inspector_max_width(),
            active_tab: RightInspectorTab::default(),
            auto_open_on_inspect: true,
        }
    }
}

impl RightInspectorConfig {
    pub fn normalize(&mut self) {
        normalize_dimension_triplet(
            &mut self.width,
            &mut self.min_width,
            &mut self.max_width,
            default_right_inspector_width(),
            default_right_inspector_min_width(),
            default_right_inspector_max_width(),
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorAreaConfig {
    #[serde(default)]
    pub table_open_mode: TableOpenMode,
    #[serde(default)]
    pub result_placement: ResultPlacement,
    #[serde(default = "default_true")]
    pub restore_open_editors: bool,
    #[serde(default = "default_true")]
    pub show_welcome_when_empty: bool,
}

impl Default for EditorAreaConfig {
    fn default() -> Self {
        Self {
            table_open_mode: TableOpenMode::default(),
            result_placement: ResultPlacement::default(),
            restore_open_editors: true,
            show_welcome_when_empty: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarConfig {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub show_focus_area: bool,
    #[serde(default = "default_true")]
    pub show_query_time: bool,
    #[serde(default = "default_true")]
    pub show_row_count: bool,
}

impl Default for StatusBarConfig {
    fn default() -> Self {
        Self {
            visible: true,
            show_focus_area: true,
            show_query_time: true,
            show_row_count: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchBehaviorConfig {
    #[serde(default = "default_true")]
    pub command_palette_prefers_context: bool,
    #[serde(default = "default_true")]
    pub close_empty_panels_on_escape: bool,
    #[serde(default = "default_true")]
    pub reveal_sidebar_on_filter_action: bool,
    #[serde(default = "default_true")]
    pub reveal_inspector_on_schema_action: bool,
}

impl Default for WorkbenchBehaviorConfig {
    fn default() -> Self {
        Self {
            command_palette_prefers_context: true,
            close_empty_panels_on_escape: true,
            reveal_sidebar_on_filter_action: true,
            reveal_inspector_on_schema_action: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchConfig {
    #[serde(default = "default_workbench_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub activity: WorkbenchActivity,
    #[serde(default)]
    pub density: WorkbenchDensity,
    #[serde(default)]
    pub sidebar: PrimarySidebarConfig,
    #[serde(default)]
    pub bottom_panel: BottomPanelConfig,
    #[serde(default)]
    pub right_inspector: RightInspectorConfig,
    #[serde(default)]
    pub editor: EditorAreaConfig,
    #[serde(default)]
    pub status_bar: StatusBarConfig,
    #[serde(default)]
    pub behavior: WorkbenchBehaviorConfig,
}

impl Default for WorkbenchConfig {
    fn default() -> Self {
        Self {
            schema_version: default_workbench_schema_version(),
            activity: WorkbenchActivity::default(),
            density: WorkbenchDensity::default(),
            sidebar: PrimarySidebarConfig::default(),
            bottom_panel: BottomPanelConfig::default(),
            right_inspector: RightInspectorConfig::default(),
            editor: EditorAreaConfig::default(),
            status_bar: StatusBarConfig::default(),
            behavior: WorkbenchBehaviorConfig::default(),
        }
    }
}

impl WorkbenchConfig {
    pub fn normalize(&mut self) {
        self.sidebar.normalize();
        self.bottom_panel.normalize();
        self.right_inspector.normalize();
        if self.schema_version == 0 {
            self.schema_version = default_workbench_schema_version();
        }
    }

    pub fn normalize_for_viewport(&mut self, viewport_width: f32, viewport_height: f32) {
        self.normalize();
        self.sidebar.normalize();
        self.bottom_panel.normalize_for_height(viewport_height);
        self.right_inspector.normalize();

        let viewport_width = sanitize_dimension(viewport_width, 1200.0);
        let combined_width = self.sidebar.width + self.right_inspector.width;
        if combined_width > viewport_width {
            self.sidebar.width = self.sidebar.width.min(viewport_width * 0.45);
            self.right_inspector.width = self.right_inspector.width.min(viewport_width * 0.35);
            self.sidebar.normalize();
            self.right_inspector.normalize();
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AppConfig {
    /// 配置格式版本 — 用于未来格式迁移。当前版本：3。
    /// 旧版本（无 version 字段）默认解析为当前版本。
    #[serde(default = "default_config_version")]
    pub version: u32,
    #[serde(default)]
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
    /// 编辑器式 workbench 布局配置
    #[serde(default)]
    pub workbench: WorkbenchConfig,
}

fn default_ui_scale() -> f32 {
    1.0
}

fn default_config_version() -> u32 {
    CONFIG_VERSION_WORKBENCH
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

fn default_true() -> bool {
    true
}

fn default_workbench_schema_version() -> u32 {
    crate::core::constants::ui::workbench::SCHEMA_VERSION
}

fn default_primary_sidebar_width() -> f32 {
    crate::core::constants::ui::workbench::PRIMARY_SIDEBAR_WIDTH
}

fn default_primary_sidebar_min_width() -> f32 {
    crate::core::constants::ui::workbench::PRIMARY_SIDEBAR_MIN_WIDTH
}

fn default_primary_sidebar_max_width() -> f32 {
    crate::core::constants::ui::workbench::PRIMARY_SIDEBAR_MAX_WIDTH
}

fn default_bottom_panel_height() -> f32 {
    crate::core::constants::ui::workbench::BOTTOM_PANEL_HEIGHT
}

fn default_bottom_panel_min_height() -> f32 {
    crate::core::constants::ui::workbench::BOTTOM_PANEL_MIN_HEIGHT
}

fn default_bottom_panel_max_ratio() -> f32 {
    crate::core::constants::ui::workbench::BOTTOM_PANEL_MAX_HEIGHT_RATIO
}

fn default_right_inspector_width() -> f32 {
    crate::core::constants::ui::workbench::RIGHT_INSPECTOR_WIDTH
}

fn default_right_inspector_min_width() -> f32 {
    crate::core::constants::ui::workbench::RIGHT_INSPECTOR_MIN_WIDTH
}

fn default_right_inspector_max_width() -> f32 {
    crate::core::constants::ui::workbench::RIGHT_INSPECTOR_MAX_WIDTH
}

fn sanitize_dimension(value: f32, default: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        default
    }
}

fn sanitize_ratio(value: f32, default: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.1, 0.9)
    } else {
        default
    }
}

fn normalize_dimension_triplet(
    value: &mut f32,
    min_value: &mut f32,
    max_value: &mut f32,
    default_value: f32,
    default_min: f32,
    default_max: f32,
) {
    *min_value = sanitize_dimension(*min_value, default_min);
    *max_value = sanitize_dimension(*max_value, default_max);
    if *max_value < *min_value {
        *min_value = default_min;
        *max_value = default_max;
    }
    *value = sanitize_dimension(*value, default_value).clamp(*min_value, *max_value);
}

#[derive(Debug, Deserialize)]
struct PrimarySidebarConfigWire {
    #[serde(default = "default_true")]
    visible: bool,
    #[serde(default = "default_primary_sidebar_width")]
    width: f32,
    #[serde(default = "default_primary_sidebar_min_width")]
    min_width: f32,
    #[serde(default = "default_primary_sidebar_max_width")]
    max_width: f32,
    #[serde(default)]
    edge_transfer: Option<bool>,
}

impl Default for PrimarySidebarConfigWire {
    fn default() -> Self {
        Self {
            visible: true,
            width: default_primary_sidebar_width(),
            min_width: default_primary_sidebar_min_width(),
            max_width: default_primary_sidebar_max_width(),
            edge_transfer: None,
        }
    }
}

impl PrimarySidebarConfigWire {
    fn into_config(self) -> PrimarySidebarConfig {
        let mut config = PrimarySidebarConfig {
            visible: self.visible,
            width: self.width,
            min_width: self.min_width,
            max_width: self.max_width,
            edge_transfer: self.edge_transfer.unwrap_or_else(default_true),
        };
        config.normalize();
        config
    }
}

#[derive(Debug, Deserialize)]
struct WorkbenchConfigWire {
    #[serde(default = "default_workbench_schema_version")]
    schema_version: u32,
    #[serde(default)]
    activity: WorkbenchActivity,
    #[serde(default)]
    density: WorkbenchDensity,
    #[serde(default)]
    sidebar: PrimarySidebarConfigWire,
    #[serde(default)]
    bottom_panel: BottomPanelConfig,
    #[serde(default)]
    right_inspector: RightInspectorConfig,
    #[serde(default)]
    editor: EditorAreaConfig,
    #[serde(default)]
    status_bar: StatusBarConfig,
    #[serde(default)]
    behavior: WorkbenchBehaviorConfig,
}

impl Default for WorkbenchConfigWire {
    fn default() -> Self {
        Self {
            schema_version: default_workbench_schema_version(),
            activity: WorkbenchActivity::default(),
            density: WorkbenchDensity::default(),
            sidebar: PrimarySidebarConfigWire::default(),
            bottom_panel: BottomPanelConfig::default(),
            right_inspector: RightInspectorConfig::default(),
            editor: EditorAreaConfig::default(),
            status_bar: StatusBarConfig::default(),
            behavior: WorkbenchBehaviorConfig::default(),
        }
    }
}

impl WorkbenchConfigWire {
    fn into_config(self) -> WorkbenchConfig {
        let mut config = WorkbenchConfig {
            schema_version: self.schema_version,
            activity: self.activity,
            density: self.density,
            sidebar: self.sidebar.into_config(),
            bottom_panel: self.bottom_panel,
            right_inspector: self.right_inspector,
            editor: self.editor,
            status_bar: self.status_bar,
            behavior: self.behavior,
        };
        config.normalize();
        config
    }
}

#[derive(Debug, Deserialize)]
struct AppConfigWire {
    #[serde(default = "default_config_version")]
    version: u32,
    #[serde(default)]
    connections: Vec<ConnectionConfig>,
    #[serde(default)]
    theme_preset: ThemePreset,
    #[serde(default = "default_light_theme")]
    light_theme: ThemePreset,
    #[serde(default = "default_dark_theme")]
    dark_theme: ThemePreset,
    #[serde(default = "default_dark_mode")]
    is_dark_mode: bool,
    #[serde(default)]
    query_history: QueryHistory,
    #[serde(default)]
    command_history: HashMap<String, Vec<String>>,
    #[serde(default = "default_ui_scale")]
    ui_scale: f32,
    #[serde(default)]
    keybindings: KeyBindings,
    #[serde(default)]
    onboarding: OnboardingProgress,
    #[serde(default = "default_connection_dialog_show_advanced")]
    connection_dialog_show_advanced: bool,
    #[serde(default)]
    sidebar: SidebarConfig,
    #[serde(default)]
    workbench: WorkbenchConfigWire,
}

impl AppConfigWire {
    fn into_config(self) -> AppConfig {
        let workbench_sidebar_edge_transfer = self.workbench.sidebar.edge_transfer;
        let mut workbench = self.workbench.into_config();
        if workbench_sidebar_edge_transfer.is_none() {
            workbench.sidebar.edge_transfer = self.sidebar.edge_transfer;
        }

        let mut config = AppConfig {
            version: self.version,
            connections: self.connections,
            theme_preset: self.theme_preset,
            light_theme: self.light_theme,
            dark_theme: self.dark_theme,
            is_dark_mode: self.is_dark_mode,
            query_history: self.query_history,
            command_history: self.command_history,
            ui_scale: self.ui_scale,
            keybindings: self.keybindings,
            onboarding: self.onboarding,
            connection_dialog_show_advanced: self.connection_dialog_show_advanced,
            sidebar: self.sidebar,
            workbench,
        };
        config.normalize();
        config
    }
}

impl<'de> Deserialize<'de> for AppConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(AppConfigWire::deserialize(deserializer)?.into_config())
    }
}

fn backup_invalid_config(path: &Path) -> bool {
    let backup_path = path.with_extension("toml.bak");
    match fs::copy(path, &backup_path) {
        Ok(_) => {
            tracing::warn!(original = ?path, backup = ?backup_path, "已备份无法解析的配置文件");
            true
        }
        Err(e) => {
            tracing::error!(error = %e, original = ?path, backup = ?backup_path, "备份损坏配置文件失败，原文件将被保留");
            false
        }
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
            version: default_config_version(),
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
            workbench: WorkbenchConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn normalize(&mut self) {
        self.ui_scale = self.ui_scale.clamp(
            crate::core::constants::ui::UI_SCALE_MIN,
            crate::core::constants::ui::UI_SCALE_MAX,
        );
        self.workbench.normalize();
    }

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

        // 防止非法 UI/布局值（用户可能手动编辑配置）
        config.normalize();

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

        self.version = self.version.max(CONFIG_VERSION_WORKBENCH);
        self.normalize();

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
    use super::{
        AppConfig, BottomPanelTab, CONFIG_VERSION_WORKBENCH, ResultPlacement, TableOpenMode,
        WorkbenchActivity, WorkbenchDensity,
    };

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

    #[test]
    fn empty_config_deserializes_to_workbench_defaults() {
        let config: AppConfig = toml::from_str("").expect("parse empty config");

        assert_eq!(config.version, CONFIG_VERSION_WORKBENCH);
        assert_eq!(config.workbench.activity, WorkbenchActivity::Explorer);
        assert_eq!(config.workbench.density, WorkbenchDensity::Compact);
        assert_eq!(config.workbench.sidebar.width, 280.0);
        assert_eq!(
            config.workbench.bottom_panel.active_tab,
            BottomPanelTab::Results
        );
        assert_eq!(
            config.workbench.editor.table_open_mode,
            TableOpenMode::ReuseActiveTableView
        );
        assert_eq!(
            config.workbench.editor.result_placement,
            ResultPlacement::BottomPanel
        );
        assert!(config.workbench.status_bar.visible);
    }

    #[test]
    fn missing_workbench_uses_defaults_and_legacy_sidebar_edge_transfer() {
        let content = r#"
            version = 2
            connections = []

            [sidebar]
            edge_transfer = false
        "#;

        let config: AppConfig = toml::from_str(content).expect("parse config");

        assert_eq!(config.version, 2);
        assert!(!config.sidebar.edge_transfer);
        assert!(!config.workbench.sidebar.edge_transfer);
        assert_eq!(config.workbench.sidebar.width, 280.0);
    }

    #[test]
    fn explicit_workbench_sidebar_edge_transfer_overrides_legacy_sidebar() {
        let content = r#"
            connections = []

            [sidebar]
            edge_transfer = false

            [workbench.sidebar]
            edge_transfer = true
        "#;

        let config: AppConfig = toml::from_str(content).expect("parse config");

        assert!(!config.sidebar.edge_transfer);
        assert!(config.workbench.sidebar.edge_transfer);
    }

    #[test]
    fn workbench_dimensions_are_normalized_and_clamped_for_viewport() {
        let content = r#"
            connections = []

            [workbench.sidebar]
            width = -10.0
            min_width = 500.0
            max_width = 300.0

            [workbench.bottom_panel]
            height = 9999.0
            min_height = -1.0
            max_height_ratio = 2.0

            [workbench.right_inspector]
            width = 10.0
            min_width = 500.0
            max_width = 300.0
        "#;

        let mut config: AppConfig = toml::from_str(content).expect("parse config");
        config.workbench.normalize_for_viewport(800.0, 400.0);

        assert_eq!(config.workbench.sidebar.min_width, 220.0);
        assert_eq!(config.workbench.sidebar.max_width, 460.0);
        assert_eq!(config.workbench.sidebar.width, 280.0);
        assert_eq!(config.workbench.bottom_panel.min_height, 140.0);
        assert_eq!(config.workbench.bottom_panel.max_height_ratio, 0.9);
        assert_eq!(config.workbench.bottom_panel.height, 360.0);
        assert_eq!(config.workbench.right_inspector.min_width, 260.0);
        assert_eq!(config.workbench.right_inspector.max_width, 480.0);
        assert_eq!(config.workbench.right_inspector.width, 260.0);
    }

    #[test]
    fn default_config_serializes_workbench_section() {
        let toml = toml::to_string_pretty(&AppConfig::default()).expect("serialize config");

        assert!(toml.contains("version = 3"));
        assert!(toml.contains("[workbench]"));
        assert!(toml.contains("[workbench.sidebar]"));
        assert!(toml.contains("[workbench.bottom_panel]"));
        assert!(toml.contains("[workbench.right_inspector]"));
        assert!(toml.contains("table_open_mode = \"reuse_active_table_view\""));
        assert!(toml.contains("result_placement = \"bottom_panel\""));
    }
}
