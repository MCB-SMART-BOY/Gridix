//! 侧边栏状态定义

use crate::database::TriggerInfo;

/// 侧边栏各区域的选中索引
#[derive(Debug, Clone, Default)]
pub struct SidebarSelectionState {
    /// 连接列表选中索引
    pub connections: usize,
    /// 数据库列表选中索引
    pub databases: usize,
    /// 表列表选中索引
    pub tables: usize,
    /// 触发器列表选中索引
    pub triggers: usize,
}

impl SidebarSelectionState {
    /// 重置数据库相关的选中索引（切换连接时调用）
    pub fn reset_for_connection_change(&mut self) {
        self.databases = 0;
        self.tables = 0;
        self.triggers = 0;
    }
    
    /// 重置表相关的选中索引（切换数据库时调用）
    pub fn reset_for_database_change(&mut self) {
        self.tables = 0;
        self.triggers = 0;
    }
}

/// 侧边栏面板状态
#[derive(Debug, Clone)]
pub struct SidebarPanelState {
    /// 上下分割比例 (0.0-1.0)，表示上部占比
    pub divider_ratio: f32,
    /// 上部（连接列表）是否折叠
    pub upper_collapsed: bool,
    /// 下部（触发器）是否折叠
    pub lower_collapsed: bool,
    /// 触发器列表
    pub triggers: Vec<TriggerInfo>,
    /// 触发器列表中的选中索引（保留向后兼容）
    pub trigger_selected_index: usize,
    /// 是否正在加载触发器
    pub loading_triggers: bool,
    /// 是否正在拖动分割条
    pub(crate) dragging_divider: bool,
    /// 各区域的选中状态
    pub selection: SidebarSelectionState,
}

impl Default for SidebarPanelState {
    fn default() -> Self {
        Self {
            divider_ratio: 0.7,  // 默认上部占 70%
            upper_collapsed: false,
            lower_collapsed: false,
            triggers: Vec::new(),
            trigger_selected_index: 0,
            loading_triggers: false,
            dragging_divider: false,
            selection: SidebarSelectionState::default(),
        }
    }
}

impl SidebarPanelState {
    /// 清空触发器列表
    pub fn clear_triggers(&mut self) {
        self.triggers.clear();
        self.trigger_selected_index = 0;
        self.selection.triggers = 0;
    }
    
    /// 设置触发器列表
    pub fn set_triggers(&mut self, triggers: Vec<TriggerInfo>) {
        self.triggers = triggers;
        self.trigger_selected_index = 0;
        self.selection.triggers = 0;
        self.loading_triggers = false;
    }
}
