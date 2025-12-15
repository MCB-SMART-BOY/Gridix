//! UI 模块 - 用户界面组件

pub mod components;
pub mod dialogs;
pub mod panels;
pub mod styles;

// 重新导出常用组件
pub use components::{
    count_search_matches, quote_identifier, DataGrid, DataGridState, SearchBar, SqlEditor,
    SqlEditorActions, Toolbar, ToolbarActions, Welcome,
    // 多 Tab 查询
    QueryTabBar, QueryTabManager,
    // 焦点转移
    FocusTransfer,
};
pub use dialogs::{
    AboutDialog, ConfirmDialog, ConnectionDialog, DdlDialog, DdlDialogState, ExportConfig, ExportDialog, HelpDialog,
    // 导入对话框
    ImportDialog, ImportState, ImportAction, ImportPreview, ImportFormat,
    parse_sql_file,
};
pub use panels::{HistoryPanel, Sidebar, SidebarActions, SidebarFocusTransfer};

/// 全局焦点区域
/// 
/// 控制键盘输入应该被哪个区域接收，确保同时只有一个区域响应键盘操作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FocusArea {
    /// 侧边栏（连接/数据库/表列表）
    Sidebar,
    /// 数据表格
    #[default]
    DataGrid,
    /// SQL 编辑器
    SqlEditor,
    /// 对话框（连接、导出等模态对话框打开时）
    #[allow(dead_code)]
    Dialog,
}

/// 侧边栏焦点子区域
/// 
/// 用于 Ctrl+1/2/3 快捷键切换侧边栏不同区域的焦点
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SidebarSection {
    /// 连接列表
    #[default]
    Connections,
    /// 数据库列表
    Databases,
    /// 表列表
    Tables,
}
