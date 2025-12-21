//! 侧边栏状态定义

use crate::database::{RoutineInfo, TriggerInfo};

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
    /// 存储过程/函数列表选中索引
    pub routines: usize,
    /// 筛选条件选中索引
    pub filters: usize,
}

impl SidebarSelectionState {
    /// 重置数据库相关的选中索引（切换连接时调用）
    pub fn reset_for_connection_change(&mut self) {
        self.databases = 0;
        self.tables = 0;
        self.triggers = 0;
        self.routines = 0;
    }
    
    /// 重置表相关的选中索引（切换数据库时调用）
    pub fn reset_for_database_change(&mut self) {
        self.tables = 0;
        self.triggers = 0;
        self.routines = 0;
    }
}

/// 侧边栏面板状态
#[derive(Debug, Clone)]
pub struct SidebarPanelState {
    // ===== 连接面板 =====
    /// 连接列表面板是否显示
    pub show_connections: bool,
    /// 连接面板高度比例 (相对于可用空间)
    pub connections_ratio: f32,
    
    // ===== 触发器面板 =====
    /// 触发器面板是否显示
    pub show_triggers: bool,
    /// 触发器面板高度比例
    pub triggers_ratio: f32,
    /// 触发器列表
    pub triggers: Vec<TriggerInfo>,
    /// 触发器列表中的选中索引（保留向后兼容）
    pub trigger_selected_index: usize,
    /// 是否正在加载触发器
    pub loading_triggers: bool,
    
    // ===== 存储过程面板 =====
    /// 存储过程/函数面板是否显示
    pub show_routines: bool,
    /// 存储过程面板高度比例
    pub routines_ratio: f32,
    /// 存储过程/函数列表
    pub routines: Vec<RoutineInfo>,
    /// 存储过程/函数选中索引
    pub routine_selected_index: usize,
    /// 是否正在加载存储过程
    pub loading_routines: bool,
    
    // ===== 筛选面板 =====
    /// 筛选面板是否显示
    pub show_filters: bool,
    /// 筛选面板高度比例
    pub filters_ratio: f32,
    
    // ===== 其他状态 =====
    /// 各区域的选中状态
    pub selection: SidebarSelectionState,
    /// 当前正在拖动的分割条索引 (0=连接/触发器, 1=触发器/存储过程)
    pub dragging_divider: Option<usize>,
    /// 命令缓冲区（用于多键命令如 gs）
    pub command_buffer: String,
}

impl Default for SidebarPanelState {
    fn default() -> Self {
        Self {
            // 连接面板 - 默认显示，占 40%
            show_connections: true,
            connections_ratio: 0.4,
            
            // 触发器面板 - 默认显示，占 20%
            show_triggers: true,
            triggers_ratio: 0.2,
            triggers: Vec::new(),
            trigger_selected_index: 0,
            loading_triggers: false,
            
            // 存储过程面板 - 默认显示，占 20%
            show_routines: true,
            routines_ratio: 0.2,
            routines: Vec::new(),
            routine_selected_index: 0,
            loading_routines: false,
            
            // 筛选面板 - 默认显示，占 20%
            show_filters: true,
            filters_ratio: 0.2,
            
            selection: SidebarSelectionState::default(),
            dragging_divider: None,
            command_buffer: String::new(),
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
    
    /// 清空存储过程/函数列表
    pub fn clear_routines(&mut self) {
        self.routines.clear();
        self.routine_selected_index = 0;
        self.selection.routines = 0;
    }
    
    /// 设置存储过程/函数列表
    pub fn set_routines(&mut self, routines: Vec<RoutineInfo>) {
        self.routines = routines;
        self.routine_selected_index = 0;
        self.selection.routines = 0;
        self.loading_routines = false;
    }
}
