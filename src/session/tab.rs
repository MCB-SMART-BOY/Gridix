//! 查询 Tab 管理（纯数据，无 UI 依赖）
//!
//! QueryTab 和 QueryTabManager 是 Layer 2 的纯数据类型。
//! 渲染逻辑在 `src/ui/components/query_tab_bar.rs`。

use crate::data::QueryResult;
use uuid::Uuid;

// ============================================================================
// 查询 Tab 状态
// ============================================================================

/// 单个查询 Tab 的状态
#[allow(dead_code)] // id 预留用于持久化和 Tab 标识
#[derive(Clone)]
pub struct QueryTab {
    /// Tab 的唯一标识符
    pub id: String,
    /// Tab 标题
    pub title: String,
    /// SQL 内容
    pub sql: String,
    /// 查询结果
    pub result: Option<QueryResult>,
    /// 是否正在执行
    pub executing: bool,
    /// 最后一条消息
    pub last_message: Option<String>,
    /// 最近一次查询错误（仅用于当前 Tab 的结果区显示）
    pub last_error: Option<String>,
    /// 查询耗时 (毫秒)
    pub query_time_ms: Option<u64>,
    /// 是否已修改 (未保存)
    pub modified: bool,
    /// 关联的表名 (如果有)
    pub table_name: Option<String>,
    /// 当前 Tab 绑定的选中表（用于恢复 grid workspace）
    pub selected_table: Option<String>,
    /// 当前 Tab 的结果搜索文本
    pub search_text: String,
    /// 当前 Tab 的结果搜索列
    pub search_column: Option<String>,
    /// 当前 Tab 是否使用持久化的 grid workspace
    pub uses_grid_workspace: bool,
    /// 当前进行中的请求 ID（用于丢弃过期回包）
    pub pending_request_id: Option<u64>,
}

impl QueryTab {
    /// 创建新的查询 Tab
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            title: "新查询".to_string(),
            sql: String::new(),
            result: None,
            executing: false,
            last_message: None,
            last_error: None,
            query_time_ms: None,
            modified: false,
            table_name: None,
            selected_table: None,
            search_text: String::new(),
            search_column: None,
            uses_grid_workspace: false,
            pending_request_id: None,
        }
    }

    /// 从 SQL 内容创建 Tab
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn from_sql(sql: &str) -> Self {
        let mut tab = Self::new();
        tab.sql = sql.to_string();
        tab.title = Self::extract_title(sql);
        tab.modified = true;
        tab
    }

    /// 从表名和 SQL 创建 Tab
    pub fn from_table(table_name: &str, sql: &str) -> Self {
        let mut tab = Self::new();
        tab.sql = sql.to_string();
        tab.title = table_name.to_string();
        tab.table_name = Some(table_name.to_string());
        tab
    }

    /// 从 SQL 内容提取标题
    fn extract_title(sql: &str) -> String {
        let sql_upper = sql.trim().to_uppercase();

        // 尝试提取表名
        if let Some(from_pos) = sql_upper.find("FROM") {
            let after_from = &sql[from_pos + 4..].trim_start();
            let table_end = after_from
                .find(|c: char| c.is_whitespace() || c == ';' || c == ',' || c == ')')
                .unwrap_or(after_from.len());
            let table_name = &after_from[..table_end];
            if !table_name.is_empty() && table_name.len() <= 20 {
                return format!("查询 {}", table_name);
            }
        }

        // 根据 SQL 类型生成标题
        if sql_upper.starts_with("SELECT") {
            "SELECT 查询".to_string()
        } else if sql_upper.starts_with("INSERT") {
            "INSERT 操作".to_string()
        } else if sql_upper.starts_with("UPDATE") {
            "UPDATE 操作".to_string()
        } else if sql_upper.starts_with("DELETE") {
            "DELETE 操作".to_string()
        } else if sql_upper.starts_with("CREATE") {
            "CREATE 操作".to_string()
        } else if sql_upper.starts_with("ALTER") {
            "ALTER 操作".to_string()
        } else if sql_upper.starts_with("DROP") {
            "DROP 操作".to_string()
        } else {
            "新查询".to_string()
        }
    }

    /// 更新标题
    pub fn update_title(&mut self) {
        if self.table_name.is_none() {
            self.title = Self::extract_title(&self.sql);
        }
    }

    /// 获取标题
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn title(&self) -> &str {
        &self.title
    }
}

impl Default for QueryTab {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tab 管理器
// ============================================================================

/// 多 Tab 管理器（纯数据操作）
pub struct QueryTabManager {
    /// 所有 Tab
    pub tabs: Vec<QueryTab>,
    /// 当前活动的 Tab 索引
    pub active_index: usize,
    /// 最大 Tab 数量
    pub max_tabs: usize,
}

impl Default for QueryTabManager {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryTabManager {
    /// 创建新的 Tab 管理器
    pub fn new() -> Self {
        let mut manager = Self {
            tabs: Vec::new(),
            active_index: 0,
            max_tabs: 20,
        };
        // 创建初始 Tab
        manager.new_tab();
        manager
    }

    /// 找到下一个可用的Tab编号
    fn find_next_number(&self) -> usize {
        let mut used_numbers: Vec<usize> = self
            .tabs
            .iter()
            .filter_map(|tab| {
                if tab.title.starts_with("查询 ") {
                    tab.title[7..].parse::<usize>().ok()
                } else {
                    None
                }
            })
            .collect();
        used_numbers.sort();

        let mut next = 1;
        for num in used_numbers {
            if num == next {
                next += 1;
            } else if num > next {
                break;
            }
        }
        next
    }

    /// 创建新 Tab
    pub fn new_tab(&mut self) -> usize {
        if self.tabs.len() >= self.max_tabs {
            return self.active_index;
        }

        let mut tab = QueryTab::new();
        tab.title = format!("查询 {}", self.find_next_number());

        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.active_index
    }

    /// 创建带有 SQL 内容的新 Tab
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn new_tab_with_sql(&mut self, sql: &str) -> usize {
        if self.tabs.len() >= self.max_tabs {
            if let Some(tab) = self.get_active_mut() {
                tab.sql = sql.to_string();
                tab.update_title();
            }
            return self.active_index;
        }

        let tab = QueryTab::from_sql(sql);
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.active_index
    }

    /// 为表创建新 Tab（如果已存在则激活）
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn new_tab_for_table(&mut self, table_name: &str, sql: &str) -> usize {
        if let Some(idx) = self
            .tabs
            .iter()
            .position(|t| t.table_name.as_deref() == Some(table_name))
        {
            if let Some(tab) = self.tabs.get_mut(idx) {
                tab.selected_table = Some(table_name.to_string());
                tab.uses_grid_workspace = true;
            }
            self.active_index = idx;
            return idx;
        }

        if self.tabs.len() >= self.max_tabs {
            if let Some(tab) = self.get_active_mut() {
                tab.sql = sql.to_string();
                tab.table_name = Some(table_name.to_string());
                tab.title = table_name.to_string();
                tab.selected_table = Some(table_name.to_string());
                tab.uses_grid_workspace = true;
            }
            return self.active_index;
        }

        let mut tab = QueryTab::from_table(table_name, sql);
        tab.selected_table = Some(table_name.to_string());
        tab.uses_grid_workspace = true;
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.active_index
    }

    /// 关闭 Tab
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            return;
        }

        if index < self.tabs.len() {
            self.tabs.remove(index);

            if self.active_index >= self.tabs.len() {
                self.active_index = self.tabs.len() - 1;
            } else if self.active_index > index {
                self.active_index -= 1;
            }
        }
    }

    /// 关闭当前活动的 Tab
    pub fn close_active_tab(&mut self) {
        self.close_tab(self.active_index);
    }

    /// 关闭其他所有 Tab
    pub fn close_other_tabs(&mut self) {
        if let Some(active_tab) = self.tabs.get(self.active_index).cloned() {
            self.tabs = vec![active_tab];
            self.active_index = 0;
        }
    }

    /// 关闭右侧所有 Tab
    pub fn close_tabs_to_right(&mut self) {
        if self.active_index < self.tabs.len() - 1 {
            self.tabs.truncate(self.active_index + 1);
        }
    }

    /// 获取当前活动的 Tab
    pub fn get_active(&self) -> Option<&QueryTab> {
        self.tabs.get(self.active_index)
    }

    /// 获取当前活动的 Tab (可变)
    pub fn get_active_mut(&mut self) -> Option<&mut QueryTab> {
        self.tabs.get_mut(self.active_index)
    }

    /// 设置活动 Tab
    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = index;
        }
    }

    /// 切换到下一个 Tab
    pub fn next_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = (self.active_index + 1) % self.tabs.len();
        }
    }

    /// 切换到上一个 Tab
    pub fn prev_tab(&mut self) {
        if !self.tabs.is_empty() {
            self.active_index = if self.active_index == 0 {
                self.tabs.len() - 1
            } else {
                self.active_index - 1
            };
        }
    }

    /// 检查是否有未保存的修改
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn has_unsaved_changes(&self) -> bool {
        self.tabs.iter().any(|t| t.modified)
    }

    /// 获取 Tab 数量
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// 检查是否为空
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn is_empty(&self) -> bool {
        self.tabs.is_empty()
    }

    /// 获取当前活动索引
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn active_index(&self) -> usize {
        self.active_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_tab_new_creates_with_uuid() {
        let tab = QueryTab::new();
        assert!(!tab.id.is_empty());
        assert_eq!(tab.title, "新查询");
        assert!(!tab.executing);
    }

    #[test]
    fn query_tab_from_sql_extracts_title() {
        let tab = QueryTab::from_sql("SELECT * FROM users");
        assert!(tab.title.contains("users"));
        assert!(tab.modified);
    }

    #[test]
    fn query_tab_manager_starts_with_one_tab() {
        let manager = QueryTabManager::new();
        assert_eq!(manager.tabs.len(), 1);
    }

    #[test]
    fn query_tab_manager_close_preserves_at_least_one() {
        let mut manager = QueryTabManager::new();
        manager.close_tab(0);
        assert_eq!(manager.tabs.len(), 1);
    }
}
