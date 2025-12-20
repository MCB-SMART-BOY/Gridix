//! 应用状态结构定义
//!
//! 将 DbManagerApp 的状态字段按功能分组，提高代码可维护性。

#![allow(dead_code)] // 公开 API，供未来使用

use crate::database::{ConnectionConfig, ConnectionManager, QueryResult};
use crate::ui::{
    DataGridState, ExportConfig, FocusArea, HistoryPanelState, ImportState,
    QueryTabManager, SidebarPanelState, SidebarSection,
};
use crate::ui::{
    CreateDbDialogState, CreateUserDialogState, DdlDialogState, ERDiagramState,
};
use crate::core::{
    AppConfig, AutoComplete, HighlightColors, NotificationManager, ProgressManager,
    QueryHistory, ThemeManager,
};
use std::sync::mpsc::{Sender, Receiver};
use super::message::Message;

/// 连接相关状态
#[derive(Default)]
pub struct ConnectionState {
    /// 数据库连接管理器
    pub manager: ConnectionManager,
    /// 是否正在建立连接
    pub connecting: bool,
    /// 是否显示连接对话框
    pub show_connection_dialog: bool,
    /// 当前编辑的连接配置
    pub new_config: ConnectionConfig,
}

/// 查询相关状态
pub struct QueryState {
    /// 当前选中的表名
    pub selected_table: Option<String>,
    /// 当前 SQL 编辑器内容
    pub sql: String,
    /// 当前查询结果
    pub result: Option<QueryResult>,
    /// 多 Tab 查询管理器
    pub tab_manager: QueryTabManager,
    /// 是否正在执行查询
    pub executing: bool,
    /// 上次查询耗时（毫秒）
    pub last_query_time_ms: Option<u64>,
    /// 自动补全引擎
    pub autocomplete: AutoComplete,
    /// 是否显示自动补全列表
    pub show_autocomplete: bool,
    /// 当前选中的补全项索引
    pub selected_completion: usize,
}

impl Default for QueryState {
    fn default() -> Self {
        Self {
            selected_table: None,
            sql: String::new(),
            result: None,
            tab_manager: QueryTabManager::new(),
            executing: false,
            last_query_time_ms: None,
            autocomplete: AutoComplete::new(),
            show_autocomplete: false,
            selected_completion: 0,
        }
    }
}

/// 搜索和选择状态
#[derive(Default)]
pub struct SearchState {
    /// 表格搜索文本
    pub search_text: String,
    /// 搜索限定的列名
    pub search_column: Option<String>,
    /// 当前选中的行索引
    pub selected_row: Option<usize>,
    /// 当前选中的单元格 (行, 列)
    pub selected_cell: Option<(usize, usize)>,
    /// 数据表格状态
    pub grid_state: DataGridState,
}

impl SearchState {
    pub fn new() -> Self {
        Self {
            search_text: String::new(),
            search_column: None,
            selected_row: None,
            selected_cell: None,
            grid_state: DataGridState::new(),
        }
    }
}

/// UI 显示状态
pub struct UiState {
    /// 侧边栏是否显示
    pub show_sidebar: bool,
    /// SQL 编辑器是否展开
    pub show_sql_editor: bool,
    /// SQL 编辑器是否需要获取焦点
    pub focus_sql_editor: bool,
    /// 是否显示 ER 图
    pub show_er_diagram: bool,
    /// 全局焦点区域
    pub focus_area: FocusArea,
    /// 侧边栏焦点子区域
    pub sidebar_section: SidebarSection,
    /// 侧边栏面板状态（包含各区域的选中索引）
    pub sidebar_panel_state: SidebarPanelState,
    /// 侧边栏宽度
    pub sidebar_width: f32,
    /// 中央面板左右分割比例
    pub central_panel_ratio: f32,
    /// ER 图状态
    pub er_diagram_state: ERDiagramState,
    /// 用户设置的 UI 缩放比例
    pub ui_scale: f32,
    /// 系统基础 DPI 缩放
    pub base_pixels_per_point: f32,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            show_sidebar: false,
            show_sql_editor: false,
            focus_sql_editor: false,
            show_er_diagram: false,
            focus_area: FocusArea::default(),
            sidebar_section: SidebarSection::default(),
            sidebar_panel_state: SidebarPanelState::default(),
            sidebar_width: 280.0,
            central_panel_ratio: 0.65,
            er_diagram_state: ERDiagramState::new(),
            ui_scale: 1.0,
            base_pixels_per_point: 1.0,
        }
    }
}

/// 对话框状态（使用枚举减少布尔字段）
#[derive(Default)]
pub struct DialogState {
    /// 是否显示导出对话框
    pub show_export: bool,
    /// 导出配置
    pub export_config: ExportConfig,
    /// 导出操作结果
    pub export_status: Option<Result<String, String>>,
    /// 是否显示导入对话框
    pub show_import: bool,
    /// 导入状态
    pub import_state: ImportState,
    /// 是否显示历史面板
    pub show_history: bool,
    /// 历史面板状态
    pub history_panel_state: HistoryPanelState,
    /// 是否显示删除确认
    pub show_delete_confirm: bool,
    /// 待删除的连接名
    pub pending_delete_name: Option<String>,
    /// 是否显示帮助
    pub show_help: bool,
    /// 帮助滚动位置
    pub help_scroll_offset: f32,
    /// 是否显示关于
    pub show_about: bool,
    /// DDL 对话框状态
    pub ddl_state: DdlDialogState,
    /// 新建数据库对话框状态
    pub create_db_state: CreateDbDialogState,
    /// 新建用户对话框状态
    pub create_user_state: CreateUserDialogState,
}

impl DialogState {
    pub fn new() -> Self {
        Self {
            show_export: false,
            export_config: ExportConfig::default(),
            export_status: None,
            show_import: false,
            import_state: ImportState::new(),
            show_history: false,
            history_panel_state: HistoryPanelState::default(),
            show_delete_confirm: false,
            pending_delete_name: None,
            show_help: false,
            help_scroll_offset: 0.0,
            show_about: false,
            ddl_state: DdlDialogState::default(),
            create_db_state: CreateDbDialogState::new(),
            create_user_state: CreateUserDialogState::new(),
        }
    }
    
    /// 检查是否有任何模态对话框打开
    pub fn has_modal_open(&self) -> bool {
        self.show_export
            || self.show_import
            || self.show_delete_confirm
            || self.show_help
            || self.show_about
            || self.show_history
            || self.ddl_state.show
            || self.create_db_state.show
            || self.create_user_state.show
    }
}

/// 主题和外观状态
pub struct ThemeState {
    /// 主题管理器
    pub manager: ThemeManager,
    /// 语法高亮颜色
    pub highlight_colors: HighlightColors,
}

/// 历史和配置状态
pub struct HistoryState {
    /// 应用程序配置
    pub app_config: AppConfig,
    /// 查询历史记录
    pub query_history: QueryHistory,
    /// 当前连接的命令历史
    pub command_history: Vec<String>,
    /// 命令历史导航索引
    pub history_index: Option<usize>,
    /// 当前历史记录对应的连接名
    pub current_connection: Option<String>,
}

/// 异步通信状态
pub struct AsyncState {
    /// 消息发送端
    pub tx: Sender<Message>,
    /// 消息接收端
    pub rx: Receiver<Message>,
    /// Tokio 异步运行时
    pub runtime: tokio::runtime::Runtime,
}

/// 反馈状态（通知和进度）
pub struct FeedbackState {
    /// 通知管理器
    pub notifications: NotificationManager,
    /// 进度管理器
    pub progress: ProgressManager,
}

impl Default for FeedbackState {
    fn default() -> Self {
        Self {
            notifications: NotificationManager::new(),
            progress: ProgressManager::new(),
        }
    }
}
