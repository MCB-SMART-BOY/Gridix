//! 多 Tab 查询窗口组件
//!
//! 支持多个独立的 SQL 查询 Tab，每个 Tab 有自己的 SQL 编辑器和结果显示区域。

use crate::core::HighlightColors;
use crate::database::QueryResult;
use egui::{self, Color32, RichText, Ui};
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
    /// 查询耗时 (毫秒)
    pub query_time_ms: Option<u64>,
    /// 是否已修改 (未保存)
    pub modified: bool,
    /// 关联的表名 (如果有)
    pub table_name: Option<String>,
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
            query_time_ms: None,
            modified: false,
            table_name: None,
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

/// 多 Tab 管理器
pub struct QueryTabManager {
    /// 所有 Tab
    pub tabs: Vec<QueryTab>,
    /// 当前活动的 Tab 索引
    pub active_index: usize,
    /// 最大 Tab 数量
    pub max_tabs: usize,
    /// 下一个 Tab 的编号
    next_number: usize,
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
            next_number: 1,
        };
        // 创建初始 Tab
        manager.new_tab();
        manager
    }

    /// 创建新 Tab
    pub fn new_tab(&mut self) -> usize {
        if self.tabs.len() >= self.max_tabs {
            return self.active_index;
        }

        let mut tab = QueryTab::new();
        tab.title = format!("查询 {}", self.next_number);
        self.next_number += 1;
        
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.active_index
    }

    /// 创建带有 SQL 内容的新 Tab
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn new_tab_with_sql(&mut self, sql: &str) -> usize {
        if self.tabs.len() >= self.max_tabs {
            // 如果达到最大数量，使用当前 Tab
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
        // 检查是否已有该表的 Tab
        if let Some(idx) = self.tabs.iter().position(|t| t.table_name.as_deref() == Some(table_name)) {
            self.active_index = idx;
            return idx;
        }

        if self.tabs.len() >= self.max_tabs {
            if let Some(tab) = self.get_active_mut() {
                tab.sql = sql.to_string();
                tab.table_name = Some(table_name.to_string());
                tab.title = table_name.to_string();
            }
            return self.active_index;
        }

        let tab = QueryTab::from_table(table_name, sql);
        self.tabs.push(tab);
        self.active_index = self.tabs.len() - 1;
        self.active_index
    }

    /// 关闭 Tab
    pub fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 {
            // 至少保留一个 Tab
            return;
        }

        if index < self.tabs.len() {
            self.tabs.remove(index);
            
            // 调整活动索引
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

// ============================================================================
// Tab 栏 UI 组件
// ============================================================================

/// Tab 栏操作
#[derive(Default)]
pub struct TabBarActions {
    /// 新建 Tab
    pub new_tab: bool,
    /// 关闭指定 Tab
    pub close_tab: Option<usize>,
    /// 切换到指定 Tab
    pub switch_to: Option<usize>,
    /// 关闭其他
    pub close_others: bool,
    /// 关闭右侧
    pub close_right: bool,
}

/// Tab 栏 UI
pub struct QueryTabBar;

impl QueryTabBar {
    /// 显示 Tab 栏
    pub fn show(
        ui: &mut Ui,
        tabs: &[QueryTab],
        active_index: usize,
        highlight_colors: &HighlightColors,
    ) -> TabBarActions {
        let mut actions = TabBarActions::default();

        ui.horizontal(|ui| {
            // Tab 按钮
            for (idx, tab) in tabs.iter().enumerate() {
                let is_active = idx == active_index;
                
                // Tab 背景色
                let bg_color = if is_active {
                    Color32::from_rgba_unmultiplied(
                        highlight_colors.keyword.r(),
                        highlight_colors.keyword.g(),
                        highlight_colors.keyword.b(),
                        40,
                    )
                } else {
                    Color32::TRANSPARENT
                };

                let frame = egui::Frame::NONE
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .corner_radius(egui::CornerRadius::same(4));

                frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // 状态图标
                        if tab.executing {
                            ui.spinner();
                        } else if tab.modified {
                            ui.label(RichText::new("*").color(highlight_colors.number).small());
                        }

                        // Tab 标题
                        let title_color = if is_active {
                            highlight_colors.keyword
                        } else {
                            highlight_colors.default
                        };
                        
                        let title_response = ui.add(
                            egui::Label::new(
                                RichText::new(&tab.title)
                                    .color(title_color)
                                    .small()
                            )
                            .sense(egui::Sense::click()),
                        );

                        if title_response.clicked() {
                            actions.switch_to = Some(idx);
                        }

                        // 右键菜单
                        title_response.context_menu(|ui| {
                            if ui.button("关闭").clicked() {
                                actions.close_tab = Some(idx);
                                ui.close();
                            }
                            if ui.button("关闭其他").clicked() {
                                actions.close_others = true;
                                ui.close();
                            }
                            if ui.button("关闭右侧").clicked() {
                                actions.close_right = true;
                                ui.close();
                            }
                        });

                        // 关闭按钮
                        if tabs.len() > 1 {
                            let close_response = ui.add(
                                egui::Label::new(
                                    RichText::new("×")
                                        .color(highlight_colors.comment)
                                        .small()
                                )
                                .sense(egui::Sense::click()),
                            );
                            
                            if close_response.clicked() {
                                actions.close_tab = Some(idx);
                            }
                            
                            if close_response.hovered() {
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        }
                    });
                });

                // Tab 分隔符
                if idx < tabs.len() - 1 {
                    ui.separator();
                }
            }

            // 新建 Tab 按钮
            ui.add_space(4.0);
            if ui.small_button("+").on_hover_text("新建查询 [Ctrl+T]").clicked() {
                actions.new_tab = true;
            }
        });

        actions
    }
}

// ============================================================================
// 测试
// ============================================================================

