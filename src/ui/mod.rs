//! UI 模块 - 用户界面组件

pub mod components;
pub mod dialogs;
pub mod panels;
mod shortcut_tooltip;
pub mod styles;

// 重新导出常用组件
#[allow(unused_imports)] // 公开 API，供外部使用
pub use components::{
    ColumnFilter,
    DataGrid,
    DataGridState,
    // 其他组件
    EditorMode,
    FilterCache,
    FilterLogic,
    FilterOperator,
    FocusTransfer,
    GridMode,
    // 通知组件
    NotificationToast,
    // 进度指示器
    ProgressIndicator,
    // 多 Tab 查询
    QueryTab,
    QueryTabBar,
    QueryTabManager,
    SqlEditor,
    SqlEditorActions,
    TabBarActions,
    TabBarFocusTransfer,
    Toolbar,
    ToolbarActions,
    ToolbarFocusTransfer,
    Welcome,
    WelcomeAction,
    WelcomeOnboardingStatus,
    WelcomeOnboardingStep,
    WelcomeServiceState,
    WelcomeStatusSummary,
    // 数据表格相关
    check_filter_match,
    // ER 关系图
    er_diagram::{
        ERColumn, ERDiagramResponse, ERDiagramState, ERTable, RelationType, Relationship,
        calculate_table_size, force_directed_layout, grid_layout,
    },
    escape_identifier,
    escape_value,
    filter_rows_cached,
    quote_identifier,
};
#[allow(unused_imports)] // 公开 API，供外部使用
pub use dialogs::{
    // 其他对话框
    AboutDialog,
    // DDL 对话框
    ColumnDefinition,
    ColumnType,
    ConfirmDialog,
    ConnectionDialog,
    // 新建数据库/用户对话框
    CreateDatabaseRequest,
    CreateDbDialog,
    CreateDbDialogResult,
    CreateDbDialogState,
    CreateUserDialog,
    CreateUserDialogResult,
    CreateUserDialogState,
    DdlDialog,
    DdlDialogState,
    DialogShortcutContext,
    ExportConfig,
    ExportDialog,
    HelpAction,
    HelpContext,
    HelpDialog,
    HelpOnboardingStep,
    HelpState,
    HelpTab,
    ImportAction,
    ImportDialog,
    ImportFormat,
    ImportPreview,
    ImportState,
    // 快捷键设置对话框
    KeyBindingsDialog,
    KeyBindingsDialogState,
    LearningTopic,
    TableDefinition,
    // 导入对话框
    parse_sql_file,
};
pub use panels::{
    HistoryPanel, HistoryPanelState, Sidebar, SidebarActions, SidebarFilterInsertMode,
    SidebarFilterWorkspaceMode, SidebarFocusTransfer, SidebarPanelState, SidebarWorkflowState,
};
#[allow(unused_imports)] // 公开 API，供外部使用
pub use shortcut_tooltip::{
    LocalShortcut, action_tooltip, action_tooltip_with_extras, consume_local_shortcut,
    consume_local_shortcut_with_text_priority, consume_scoped_command_with_text_priority,
    local_shortcut_pressed, local_shortcut_text, local_shortcut_tooltip, local_shortcuts_text,
    local_shortcuts_tooltip, scoped_command_text, shortcut_tooltip, sync_runtime_local_shortcuts,
    text_entry_has_priority,
};

/// 全局焦点区域
///
/// 控制键盘输入应该被哪个区域接收，确保同时只有一个区域响应键盘操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusArea {
    /// 顶部工具栏
    Toolbar,
    /// 查询Tab栏
    QueryTabs,
    /// 侧边栏（连接/数据库/表列表）
    Sidebar,
    /// 数据表格
    #[default]
    DataGrid,
    /// SQL 编辑器
    SqlEditor,
    /// 对话框（连接、导出等模态对话框打开时，预留扩展）
    #[allow(dead_code)] // 预留变体，用于未来对话框焦点管理
    Dialog,
}

/// 侧边栏焦点子区域
///
/// 用于 Ctrl+1/2/3/4/5/6 快捷键切换侧边栏不同区域的焦点
/// 顺序：连接 -> 数据库 -> 表 -> 筛选 -> 触发器 -> 存储过程
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarSection {
    /// 1. 连接列表
    #[default]
    Connections,
    /// 2. 数据库列表
    Databases,
    /// 3. 表列表
    Tables,
    /// 4. 筛选条件
    Filters,
    /// 5. 触发器列表
    Triggers,
    /// 6. 存储过程/函数列表
    Routines,
}
