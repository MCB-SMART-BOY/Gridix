//! 会话状态持久化
//!
//! 保存和恢复用户的工作会话，包括：
//! - 打开的查询 Tab
//! - 上次连接的数据库
//! - UI 布局状态

#![allow(dead_code)] // 公开 API，供未来使用

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Tab 状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabState {
    /// Tab 标题
    pub title: String,
    /// SQL 内容
    pub sql: String,
    /// 关联的表名（如果有）
    pub associated_table: Option<String>,
}

impl TabState {
    /// 创建新的 Tab 状态
    pub fn new(title: impl Into<String>, sql: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            sql: sql.into(),
            associated_table: None,
        }
    }

    /// 创建关联表的 Tab 状态
    pub fn with_table(title: impl Into<String>, sql: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            sql: sql.into(),
            associated_table: Some(table.into()),
        }
    }
}

/// 会话状态
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionState {
    /// 打开的 Tab 列表
    #[serde(default)]
    pub tabs: Vec<TabState>,
    
    /// 活动 Tab 索引
    #[serde(default)]
    pub active_tab_index: usize,
    
    /// 上次连接的连接名
    #[serde(default)]
    pub last_connection: Option<String>,
    
    /// 上次选择的数据库
    #[serde(default)]
    pub last_database: Option<String>,
    
    /// 上次选择的表
    #[serde(default)]
    pub last_table: Option<String>,
    
    /// 侧边栏宽度
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: f32,
    
    /// 中央面板分割比例（SQL 编辑器 vs 数据表格）
    #[serde(default = "default_central_panel_ratio")]
    pub central_panel_ratio: f32,
    
    /// 是否显示侧边栏
    #[serde(default = "default_show_sidebar")]
    pub show_sidebar: bool,
    
    /// 是否显示 SQL 编辑器
    #[serde(default = "default_show_sql_editor")]
    pub show_sql_editor: bool,
    
    /// 是否显示 ER 关系图
    #[serde(default)]
    pub show_er_diagram: bool,
    
    /// 窗口位置和大小（可选）
    #[serde(default)]
    pub window_state: Option<WindowState>,
}

/// 窗口状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    /// 窗口 X 坐标
    pub x: i32,
    /// 窗口 Y 坐标
    pub y: i32,
    /// 窗口宽度
    pub width: u32,
    /// 窗口高度
    pub height: u32,
    /// 是否最大化
    pub maximized: bool,
}

fn default_sidebar_width() -> f32 {
    250.0
}

fn default_central_panel_ratio() -> f32 {
    0.3  // SQL 编辑器占 30%
}

fn default_show_sidebar() -> bool {
    true
}

fn default_show_sql_editor() -> bool {
    true
}

impl SessionState {
    /// 创建新的会话状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取会话文件路径
    fn session_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("gridix").join("session.toml"))
    }

    /// 加载会话状态
    pub fn load() -> Option<Self> {
        let path = Self::session_path()?;
        
        if !path.exists() {
            return None;
        }
        
        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(error = %e, "读取会话文件失败");
                return None;
            }
        };
        
        match toml::from_str(&content) {
            Ok(session) => Some(session),
            Err(e) => {
                tracing::warn!(error = %e, "解析会话文件失败");
                None
            }
        }
    }

    /// 保存会话状态
    pub fn save(&self) -> Result<(), String> {
        let path = Self::session_path().ok_or("无法获取会话文件路径")?;
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("创建目录失败: {}", e))?;
        }
        
        let toml_str = toml::to_string_pretty(self).map_err(|e| format!("序列化失败: {}", e))?;
        
        // 原子写入
        let temp_path = path.with_extension("toml.tmp");
        fs::write(&temp_path, &toml_str).map_err(|e| format!("写入失败: {}", e))?;
        fs::rename(&temp_path, &path).map_err(|e| format!("重命名失败: {}", e))?;
        
        Ok(())
    }

    /// 清除会话文件
    pub fn clear() -> Result<(), String> {
        if let Some(path) = Self::session_path()
            && path.exists() {
                fs::remove_file(&path).map_err(|e| format!("删除失败: {}", e))?;
            }
        Ok(())
    }

    /// 添加 Tab
    pub fn add_tab(&mut self, tab: TabState) {
        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len().saturating_sub(1);
    }

    /// 移除 Tab
    pub fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.active_tab_index >= self.tabs.len() && !self.tabs.is_empty() {
                self.active_tab_index = self.tabs.len() - 1;
            }
        }
    }

    /// 更新 Tab 内容
    pub fn update_tab(&mut self, index: usize, sql: String) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.sql = sql;
        }
    }

    /// 设置活动 Tab
    pub fn set_active_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_tab_index = index;
        }
    }

    /// 记录最后访问的位置
    pub fn record_last_location(&mut self, connection: Option<String>, database: Option<String>, table: Option<String>) {
        self.last_connection = connection;
        self.last_database = database;
        self.last_table = table;
    }

    /// 记录 UI 布局
    pub fn record_layout(&mut self, sidebar_width: f32, central_panel_ratio: f32, show_sidebar: bool, show_sql_editor: bool) {
        self.sidebar_width = sidebar_width;
        self.central_panel_ratio = central_panel_ratio;
        self.show_sidebar = show_sidebar;
        self.show_sql_editor = show_sql_editor;
    }

    /// 是否有有效的会话数据
    pub fn has_valid_session(&self) -> bool {
        !self.tabs.is_empty() || self.last_connection.is_some()
    }

    /// 获取 Tab 数量
    pub fn tab_count(&self) -> usize {
        self.tabs.len()
    }
}

/// 会话管理器
/// 
/// 提供会话的自动保存和恢复功能
pub struct SessionManager {
    /// 当前会话状态
    state: SessionState,
    /// 是否有未保存的更改
    dirty: bool,
    /// 自动保存间隔（秒）
    auto_save_interval: u64,
    /// 上次保存时间
    last_save: std::time::Instant,
}

impl SessionManager {
    /// 创建新的会话管理器
    pub fn new() -> Self {
        let state = SessionState::load().unwrap_or_default();
        Self {
            state,
            dirty: false,
            auto_save_interval: 60,  // 默认 60 秒自动保存
            last_save: std::time::Instant::now(),
        }
    }

    /// 设置自动保存间隔
    pub fn set_auto_save_interval(&mut self, seconds: u64) {
        self.auto_save_interval = seconds;
    }

    /// 获取当前会话状态
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// 获取可变的会话状态
    pub fn state_mut(&mut self) -> &mut SessionState {
        self.dirty = true;
        &mut self.state
    }

    /// 标记为已修改
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// 检查并执行自动保存
    pub fn tick(&mut self) {
        if self.dirty && self.last_save.elapsed().as_secs() >= self.auto_save_interval {
            self.save();
        }
    }

    /// 保存会话
    pub fn save(&mut self) {
        if let Err(e) = self.state.save() {
            tracing::warn!(error = %e, "保存会话失败");
        } else {
            self.dirty = false;
            self.last_save = std::time::Instant::now();
        }
    }

    /// 强制保存
    pub fn force_save(&mut self) {
        self.dirty = true;
        self.save();
    }

    /// 重置会话
    pub fn reset(&mut self) {
        self.state = SessionState::default();
        self.dirty = true;
        if let Err(e) = SessionState::clear() {
            tracing::warn!(error = %e, "清除会话失败");
        }
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for SessionManager {
    fn drop(&mut self) {
        // 退出时保存会话
        if self.dirty {
            self.save();
        }
    }
}
