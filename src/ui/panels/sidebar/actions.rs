//! 侧边栏操作和事件定义

use crate::ui::SidebarSection;

/// 焦点转移方向（从侧边栏转出）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFocusTransfer {
    /// 转移到数据表格
    ToDataGrid,
}

/// 侧边栏操作
#[derive(Default)]
pub struct SidebarActions {
    pub connect: Option<String>,
    pub disconnect: Option<String>,
    pub delete: Option<String>,
    pub select_database: Option<String>,
    pub show_table_schema: Option<String>,
    pub query_table: Option<String>,
    /// 在 SQL 编辑器中显示触发器定义
    pub show_trigger_definition: Option<String>,
    /// 在 SQL 编辑器中显示存储过程/函数定义
    pub show_routine_definition: Option<String>,
    /// 焦点转移请求（转出侧边栏）
    pub focus_transfer: Option<SidebarFocusTransfer>,
    /// Section 切换请求（侧边栏内部层级导航）
    pub section_change: Option<SidebarSection>,
    /// 编辑连接配置（打开连接对话框）
    pub edit_connection: Option<String>,
    /// 重命名项目（连接/表等）
    pub rename_item: Option<(SidebarSection, String)>,
    /// 刷新当前列表
    pub refresh: bool,
    /// 筛选条件已更改
    pub filter_changed: bool,
    /// 添加新的筛选条件
    pub add_filter: bool,
    /// 清空所有筛选条件
    pub clear_filters: bool,
    /// 切换筛选条件的逻辑运算符 (AND/OR)
    pub toggle_filter_logic: Option<usize>,
    /// 聚焦到指定筛选条件的输入框
    pub focus_filter_input: Option<usize>,
    /// 切换筛选条件的列 (索引, true=下一个/false=上一个)
    pub cycle_filter_column: Option<(usize, bool)>,
}

#[allow(dead_code)] // 公开 API，供外部使用
impl SidebarActions {
    /// 检查是否有任何操作
    #[inline]
    pub fn has_action(&self) -> bool {
        self.connect.is_some()
            || self.disconnect.is_some()
            || self.delete.is_some()
            || self.select_database.is_some()
            || self.show_table_schema.is_some()
            || self.query_table.is_some()
    }
}
