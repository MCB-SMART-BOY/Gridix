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
    /// 焦点转移请求（转出侧边栏）
    pub focus_transfer: Option<SidebarFocusTransfer>,
    /// Section 切换请求（侧边栏内部层级导航）
    pub section_change: Option<SidebarSection>,
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
