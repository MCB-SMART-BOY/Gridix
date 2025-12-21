use crate::core::ThemePreset;

/// 主题下拉框状态
#[derive(Default, Clone)]
pub struct ThemeComboState {
    pub selected_index: usize,
    pub is_open: bool,
}

/// 下拉菜单状态
#[derive(Default, Clone)]
pub struct DropdownState {
    pub is_open: bool,
    pub selected_index: usize,
}

/// 工具栏焦点转移方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolbarFocusTransfer {
    /// 转移到查询Tab栏
    ToQueryTabs,
}

/// 工具栏操作结果
#[derive(Default)]
pub struct ToolbarActions {
    pub refresh_tables: bool,
    pub export: bool,
    pub import: bool,
    pub show_history: bool,
    pub show_help: bool,
    pub toggle_sidebar: bool,
    pub toggle_editor: bool,
    pub show_editor: bool,
    pub theme_changed: Option<ThemePreset>,
    pub toggle_dark_mode: bool,
    pub switch_connection: Option<String>,
    pub switch_database: Option<String>,
    pub switch_table: Option<String>,
    // 快捷键触发的下拉框打开
    pub open_theme_selector: bool,
    // 缩放操作
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
    // DDL 操作
    pub create_table: bool,
    pub create_database: bool,
    pub create_user: bool,
    // ER 图
    pub toggle_er_diagram: bool,
    // 关于对话框
    pub show_about: bool,
    // 快捷键设置
    pub show_keybindings: bool,
    // 焦点转移
    pub focus_transfer: Option<ToolbarFocusTransfer>,
}
