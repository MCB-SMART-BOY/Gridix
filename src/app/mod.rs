//! 主应用程序模块
//!
//! 包含 `DbManagerApp` 结构体，实现了 eframe::App trait，
//! 负责管理应用程序的整体状态和渲染逻辑。
//!
//! ## 子模块
//!
//! - `action`: 应用动作语义和命令面板
//! - `input`: focus-aware 输入路由和键盘兼容层
//! - `runtime`: 数据库请求、异步消息、元数据和请求生命周期
//! - `surfaces`: 主渲染循环、对话框和偏好设置
//! - `workflow`: 导入导出、帮助和欢迎页用户流程

mod action;
pub(crate) mod dialogs;
mod input;
pub(crate) mod runtime;
mod surfaces;
mod workflow;

use eframe::egui;
use std::collections::HashMap;
use std::sync::mpsc::channel;

use crate::core::{
    AppConfig, HighlightColors, KeyBindings, ThemeManager, constants,
};
use crate::data::{ConnectionConfig, DatabaseType, QueryResult};
use crate::ui::{self, DdlDialogState, ExportConfig, KeyBindingsDialogState, QueryTabManager};

use action::command_palette::CommandPaletteState;
use dialogs::host::DialogId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(in crate::app) struct GridWorkspaceId {
    tab_id: String,
    connection_name: String,
    database_name: Option<String>,
    table_name: String,
}

#[derive(Default)]
pub(in crate::app) struct GridWorkspaceStore {
    states: HashMap<GridWorkspaceId, ui::DataGridState>,
}

impl GridWorkspaceStore {
    fn save(&mut self, workspace_id: GridWorkspaceId, state: &ui::DataGridState) {
        self.states.insert(workspace_id, state.clone());
    }

    fn load(&self, workspace_id: &GridWorkspaceId) -> Option<ui::DataGridState> {
        self.states.get(workspace_id).cloned()
    }

    fn remove_connection(&mut self, connection_name: &str) {
        self.states
            .retain(|workspace_id, _| workspace_id.connection_name != connection_name);
    }

    fn remove_tab(&mut self, tab_id: &str) {
        self.states
            .retain(|workspace_id, _| workspace_id.tab_id != tab_id);
    }

    fn remove_table(
        &mut self,
        connection_name: &str,
        database_name: &Option<String>,
        table_name: &str,
    ) {
        self.states.retain(|workspace_id, _| {
            workspace_id.connection_name != connection_name
                || &workspace_id.database_name != database_name
                || workspace_id.table_name != table_name
        });
    }

    fn remove_database(&mut self, connection_name: &str, database_name: &str) {
        self.states.retain(|workspace_id, _| {
            workspace_id.connection_name != connection_name
                || workspace_id.database_name.as_deref() != Some(database_name)
        });
    }
}

/// Gridix 主应用结构体 — `eframe::App` 实现。
///
/// # 架构：已迁移字段
///
/// DB 连接、异步基础设施、请求追踪 → `self.session` (Session)
/// 主题、缩放、高亮颜色 → `self.state` (UiState)
///
/// # 剩余字段（待迁移）
///
/// 对话框状态、Grid 状态、搜索/选择、ER 图、UI 显示
///
/// 目标：4 字段 { session, state, config, keybindings }
/// 当前：~47 字段（已迁移 ~53）
pub struct DbManagerApp {
    // ==================== 核心聚合 ====================
    /// 会话状态 — 聚合 DB 连接、异步基础设施、请求追踪（渐进迁移中）
    session: crate::session::Session,
    /// UI 状态 — 聚合渲染状态（渐进迁移中）
    pub state: crate::state::UiState,

    // ==================== 连接对话框 ====================

    // ==================== 查询状态 ====================

    // ==================== 配置和历史 ====================
    app_config: AppConfig,

    // ==================== 搜索和选择 ====================
    /// 表格搜索文本
    /// 搜索限定的列名
    /// 当前选中的行索引
    /// 当前选中的单元格 (行, 列)
    /// 数据表格状态（筛选、排序、编辑等）
    /// 按表实例隔离的表格工作区状态
    grid_workspaces: GridWorkspaceStore,
    /// 当前活动 surface 是否使用持久化 grid workspace
    active_grid_workspace_enabled: bool,

    // ==================== 对话框状态 ====================
    /// 是否显示导出对话框
    /// 导出配置
    /// 导出操作结果
    /// 是否显示导入对话框
    /// 导入状态（文件、预览、配置）
    /// 是否显示历史面板
    /// 历史面板状态
    /// 是否显示删除确认对话框
    /// 待删除目标（连接 / 数据库 / 表）
    /// 待处理的表删除（request_id -> (连接名, 表名)）
    /// 侧边栏请求聚焦的筛选输入框索引

    // ==================== 自动补全 ====================
    /// 当前选中的补全项索引
    /// SQL 编辑器模式 (Normal/Insert)

    // ==================== UI 显示状态 ====================
    /// egui_dock 布局状态（管理主工作区的面板布局）
    dock_state: egui_dock::DockState<ui::dock_tabs::DockTab>,
    /// SQL 编辑器是否展开显示
    /// SQL 编辑器是否需要获取焦点
    /// 侧边栏是否显示
    /// 全局焦点区域（侧边栏/SQL 编辑器/数据表格）
    /// 最近一个非 ER 的 workspace 主区域（仅记录 Sidebar / DataGrid / SqlEditor）
    /// 工具栏当前选中项索引（用于键盘导航）
    /// 侧边栏当前焦点子区域（连接/数据库/表）
    /// 侧边栏面板状态（上下分割、触发器列表、选中索引等）
    /// 侧边栏宽度
    /// 欢迎页数据库环境检测状态
    /// 是否显示欢迎页安装/初始化引导
    /// 当前引导目标数据库
    /// 欢迎页安装/初始化引导当前选中的动作索引
    /// 是否显示帮助面板
    /// 帮助面板滚动位置
    /// 帮助面板分类与学习主题状态
    /// 是否显示关于对话框
    /// 当前显式 dialog owner（输入/渲染优先走这里，再兼容回退到可见性采样）
    /// DDL 对话框状态（新建表等）
    /// 新建数据库对话框状态
    /// 新建用户对话框状态
    /// 快捷键绑定
    keybindings: KeyBindings,
    /// 快捷键设置对话框状态
    /// 顶部工具栏“操作”菜单状态
    /// 顶部工具栏“新建”菜单状态
    /// 顶部工具栏“主题”菜单状态
    /// 命令面板状态
    command_palette_state: CommandPaletteState,
    /// 是否显示 ER 图面板
    /// ER 图状态
    /// SQL 编辑器高度（用于可调整大小）
    /// 待执行的切换日/夜模式操作（由键盘快捷键设置）
    pending_toggle_dark_mode: bool,
    /// Config save throttling
    config_dirty: bool,
    last_config_save: std::time::Instant,
}

// ===== SQL 编辑器访问方法（委托给 tab_manager，消除 self.sql 双源）=====

impl DbManagerApp {
    /// 获取当前活动 Tab 的 SQL（只读）
    pub(crate) fn active_sql(&self) -> &str {
        self.session.tab_manager
            .get_active()
            .map(|t| t.sql.as_str())
            .unwrap_or("")
    }

    /// 设置编辑器 SQL（如无 tab 则自动创建）
    pub(crate) fn set_active_sql(&mut self, sql: String) {
        if self.session.tab_manager.tabs.is_empty() {
            self.session.tab_manager.new_tab();
        }
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            tab.sql = sql;
        }
    }
    /// Clear result from both mirror and active tab
    pub(crate) fn clear_result(&mut self) {
        self.state.result = None;
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            tab.result = None;
        }
    }

    /// Clear search from both mirror and active tab
    pub(crate) fn clear_search(&mut self) {
        self.state.search_text.clear();
        self.state.search_column = None;
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            tab.search_text.clear();
            tab.search_column = None;
        }
    }

    // ── Config save throttling ──

    /// Mark config as dirty — batch save on next tick
    pub(crate) fn save_config_debounced(&mut self) {
        self.config_dirty = true;
    }

    /// Periodically flush dirty config (called each frame)
    pub(crate) fn tick_config_save(&mut self) {
        if self.config_dirty
            && self.last_config_save.elapsed() > std::time::Duration::from_secs(5)
        {
            self.save_config();
            self.config_dirty = false;
            self.last_config_save = std::time::Instant::now();
        }
    }
}

impl DbManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let app_config = AppConfig::load();
        let keybindings = KeyBindings::load_or_init(&app_config.keybindings);
        Self::new_with_loaded_config(cc, app_config, keybindings)
    }

    fn new_with_loaded_config(
        cc: &eframe::CreationContext<'_>,
        app_config: AppConfig,
        keybindings: KeyBindings,
    ) -> Self {
        let (tx, rx) = channel();

        // 创建 tokio runtime，优先多线程，失败则降级到单线程
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .or_else(|e| {
                tracing::warn!(error = %e, "多线程运行时创建失败，降级到单线程模式");
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
            })
            .expect("无法创建 tokio 运行时，系统资源可能不足");

        // 创建 Session（Layer 2 聚合结构体）
        let query_history = app_config.query_history.clone();
        let mut session = crate::session::Session::new(runtime, tx.clone(), rx, query_history);

        ui::sync_runtime_local_shortcuts(&keybindings);

        // 主题和外观
        let theme_manager = ThemeManager::new(app_config.theme_preset);
        let highlight_colors = HighlightColors::from_theme(&theme_manager.colors);
        theme_manager.apply(&cc.egui_ctx);

        let base_pixels_per_point = cc.egui_ctx.pixels_per_point();
        let ui_scale = app_config
            .ui_scale
            .clamp(constants::ui::UI_SCALE_MIN, constants::ui::UI_SCALE_MAX);
        cc.egui_ctx
            .set_pixels_per_point(base_pixels_per_point * ui_scale);

        // 从配置恢复连接
        for config in &app_config.connections {
            session.manager.add(config.clone());
        }

        let mut sidebar_panel_state = ui::SidebarPanelState::default();
        sidebar_panel_state.workflow.edge_transfer = app_config.sidebar.edge_transfer;

        let mut app = Self {
            connection_dialog_show_advanced: app_config.connection_dialog_show_advanced,
            session,
            state: {
                let mut s = crate::state::UiState::default();
                s.theme_manager = theme_manager;
                s.highlight_colors = highlight_colors;
                s.ui_scale = ui_scale;
                s.base_pixels_per_point = base_pixels_per_point;
                s
            },
            app_config,
            grid_workspaces: GridWorkspaceStore::default(),
            active_grid_workspace_enabled: false,
            dock_state: ui::dock_tabs::default_layout(),
            sidebar_width: 280.0, // 默认侧边栏宽度
            ddl_dialog_state: DdlDialogState::default(),
            create_db_dialog_state: ui::CreateDbDialogState::new(),
            create_user_dialog_state: ui::CreateUserDialogState::new(),
            keybindings,
            command_palette_state: CommandPaletteState::default(),
            pending_toggle_dark_mode: false,
            config_dirty: false,
            last_config_save: std::time::Instant::now(),
        };
        app.refresh_welcome_environment_status();
        app
    }

    // ==================== egui_dock 集成访问器 ====================

    pub(crate) fn tab_manager(&self) -> &QueryTabManager {
        &self.session.tab_manager
    }

    pub(crate) fn tab_manager_mut(&mut self) -> &mut QueryTabManager {
        &mut self.session.tab_manager
    }

    pub(crate) fn show_er_diagram(&self) -> bool {
        self.state.show_er_diagram
    }

    pub(crate) fn show_sql_editor(&self) -> bool {
        self.state.show_sql_editor
    }

    /// Dock tab 关闭时的清理：持久化状态、取消查询、移除工作区
    pub(crate) fn on_dock_tab_close(&mut self, tab_index: usize) {
        self.persist_active_tab_state_for_navigation();
        // Clone needed values before mutable operations
        let pending_id = self.session.tab_manager.tabs
            .get(tab_index)
            .and_then(|tab| tab.pending_request_id);
        let tab_id = self.session.tab_manager.tabs
            .get(tab_index)
            .map(|tab| tab.id.clone());
        if let Some(request_id) = pending_id {
            self.cancel_query_request_silently(request_id);
        }
        if let Some(ref id) = tab_id {
            self.remove_grid_workspaces_for_tab(id);
        }
        self.session.tab_manager.close_tab(tab_index);
        self.sync_from_active_tab();
    }

    #[cfg(test)]
    pub(crate) fn new_for_test() -> Self {
        let cc = eframe::CreationContext::_new_kittest(egui::Context::default());
        Self::new_with_loaded_config(&cc, AppConfig::default(), KeyBindings::default())
    }

    pub(in crate::app) fn grid_workspace_id_for_table(
        &self,
        table_name: &str,
    ) -> Option<GridWorkspaceId> {
        let tab_id = self.session.tab_manager.get_active()?.id.clone();
        let connection_name = self.session.manager.active.clone()?;
        let database_name = self
            .session.manager
            .get_active()
            .and_then(|connection| connection.selected_database.clone());
        Some(GridWorkspaceId {
            tab_id,
            connection_name,
            database_name,
            table_name: table_name.to_string(),
        })
    }

    pub(in crate::app) fn active_grid_workspace_id(&self) -> Option<GridWorkspaceId> {
        if !self.active_grid_workspace_enabled {
            return None;
        }
        let table_name = self.state.selected_table.as_deref()?;
        self.grid_workspace_id_for_table(table_name)
    }

    pub(in crate::app) fn persist_active_grid_workspace(&mut self) {
        let Some(workspace_id) = self.active_grid_workspace_id() else {
            return;
        };
        self.grid_workspaces.save(workspace_id, &self.state.grid_state);
    }

    fn sync_active_grid_focus(&mut self) {
        self.state.grid_state.focused = self.state.focus_area == ui::FocusArea::DataGrid;
    }

    pub(in crate::app) fn sync_active_surface_binding_to_tab(&mut self) {
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            tab.selected_table = self.state.selected_table.clone();
            tab.search_text = self.state.search_text.clone();
            tab.search_column = self.state.search_column.clone();
            tab.uses_grid_workspace = self.active_grid_workspace_enabled;
        }
    }

    pub(in crate::app) fn restore_grid_surface_from_active_tab(&mut self) {
        self.state.grid_state = match self.active_grid_workspace_id() {
            Some(workspace_id) => self.grid_workspaces.load(&workspace_id).unwrap_or_default(),
            None => ui::DataGridState::new(),
        };
        self.sync_active_grid_focus();
    }

    pub(in crate::app) fn switch_grid_workspace(&mut self, next_table: Option<String>) {
        self.persist_active_grid_workspace();
        self.state.selected_table = next_table;
        self.active_grid_workspace_enabled = self.state.selected_table.is_some();
        self.restore_grid_surface_from_active_tab();
        self.sync_active_surface_binding_to_tab();
    }

    pub(in crate::app) fn reset_grid_workspace_for_transient_surface(
        &mut self,
        selected_table: Option<String>,
    ) {
        self.persist_active_grid_workspace();
        self.state.selected_table = selected_table;
        self.active_grid_workspace_enabled = false;
        self.restore_grid_surface_from_active_tab();
        self.sync_active_surface_binding_to_tab();
    }

    pub(in crate::app) fn remove_grid_workspaces_for_connection(&mut self, connection_name: &str) {
        self.grid_workspaces.remove_connection(connection_name);
    }

    pub(crate) fn remove_grid_workspaces_for_tab(&mut self, tab_id: &str) {
        self.grid_workspaces.remove_tab(tab_id);
    }

    pub(in crate::app) fn remove_grid_workspace_for_table(&mut self, table_name: &str) {
        let Some(connection_name) = self.session.manager.active.clone() else {
            return;
        };
        let database_name = self
            .session.manager
            .get_active()
            .and_then(|connection| connection.selected_database.clone());
        self.grid_workspaces
            .remove_table(&connection_name, &database_name, table_name);
    }

    pub(in crate::app) fn remove_grid_workspaces_for_database(&mut self, database_name: &str) {
        let Some(connection_name) = self.session.manager.active.clone() else {
            return;
        };
        self.grid_workspaces
            .remove_database(&connection_name, database_name);
    }

    // 注意：connect, select_database, disconnect, delete_connection, execute,
    // fetch_primary_key, handle_connection_error 已移至 database.rs 模块

    // 注意：handle_messages 已移至 handler.rs 模块

    // 注意：load_er_diagram_data, infer_relationships_from_columns 已移至 er_diagram.rs 模块

    fn handle_export_with_config(&mut self, config: ExportConfig) {
        let table_name = self
            .selected_table
            .clone()
            .unwrap_or_else(|| "query_result".to_string());

        if let Some(result) = &self.state.result {
            let filter_name = format!("{} 文件", config.format.display_name());
            let filter_ext = config.format.extension();

            let file_dialog = rfd::FileDialog::new()
                .set_file_name(format!("{}.{}", table_name, filter_ext))
                .add_filter(&filter_name, &[filter_ext]);

            if let Some(path) = file_dialog.save_file() {
                // 使用导出模块执行导出
                let db_type = self
                    .session.manager
                    .get_active()
                    .map(|connection| connection.config.db_type)
                    .unwrap_or(crate::data::DatabaseType::SQLite);
                self.state.export_status = Some(workflow::export::execute_export(
                    result,
                    &table_name,
                    &path,
                    &config,
                    db_type,
                ));
            }
        }
    }

    // 注意：handle_import, select_import_file, refresh_import_preview,
    // execute_import 已移至 import.rs 模块

    // 注意：handle_keyboard_shortcuts, handle_zoom_shortcuts 已移至 keyboard.rs 模块
}

impl eframe::App for DbManagerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.run_frame(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Flush any pending config changes before exit
        if self.config_dirty {
            self.save_config();
            self.config_dirty = false;
        }
        for (_, handle) in self.session.pending_query_tasks.drain() {
            handle.abort();
        }
        self.session.pending_query_connections.clear();
        self.session.pending_query_cancellers.clear();

        // 清理连接池，确保所有数据库连接正确关闭
        self.session.runtime.block_on(async {
            crate::data::ssh_tunnel::SSH_TUNNEL_MANAGER
                .stop_all()
                .await;
            crate::data::POOL_MANAGER.clear_all().await;
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{GridWorkspaceId, GridWorkspaceStore};
    use crate::ui::DataGridState;

    fn workspace(
        tab_id: &str,
        connection: &str,
        database: Option<&str>,
        table: &str,
    ) -> GridWorkspaceId {
        GridWorkspaceId {
            tab_id: tab_id.to_string(),
            connection_name: connection.to_string(),
            database_name: database.map(str::to_string),
            table_name: table.to_string(),
        }
    }

    #[test]
    fn grid_workspace_store_keeps_tables_isolated() {
        let mut store = GridWorkspaceStore::default();
        let mut users_state = DataGridState::new();
        users_state.new_rows.push(vec!["draft-user".to_string()]);
        users_state.cursor = (5, 0);

        let mut orders_state = DataGridState::new();
        orders_state
            .new_rows
            .push(vec!["draft-order".to_string(), "2".to_string()]);
        orders_state.cursor = (8, 1);

        let users_id = workspace("tab-1", "local", Some("main"), "users");
        let orders_id = workspace("tab-1", "local", Some("main"), "orders");

        store.save(users_id.clone(), &users_state);
        store.save(orders_id.clone(), &orders_state);

        let restored_users = store.load(&users_id).expect("users workspace should exist");
        let restored_orders = store
            .load(&orders_id)
            .expect("orders workspace should exist");

        assert_eq!(restored_users.new_rows.len(), 1);
        assert_eq!(restored_users.new_rows[0].len(), 1);
        assert_eq!(restored_users.cursor, (5, 0));
        assert_eq!(restored_orders.new_rows.len(), 1);
        assert_eq!(restored_orders.new_rows[0].len(), 2);
        assert_eq!(restored_orders.cursor, (8, 1));
    }

    #[test]
    fn grid_workspace_store_can_drop_whole_connection() {
        let mut store = GridWorkspaceStore::default();
        let state = DataGridState::new();
        let left = workspace("tab-left", "left", Some("main"), "users");
        let right = workspace("tab-right", "right", Some("main"), "users");

        store.save(left.clone(), &state);
        store.save(right.clone(), &state);
        store.remove_connection("left");

        assert!(store.load(&left).is_none());
        assert!(store.load(&right).is_some());
    }

    #[test]
    fn grid_workspace_store_can_drop_whole_database() {
        let mut store = GridWorkspaceStore::default();
        let state = DataGridState::new();
        let users = workspace("tab-1", "local", Some("main"), "users");
        let orders = workspace("tab-2", "local", Some("main"), "orders");
        let analytics = workspace("tab-3", "local", Some("analytics"), "events");

        store.save(users.clone(), &state);
        store.save(orders.clone(), &state);
        store.save(analytics.clone(), &state);
        store.remove_database("local", "main");

        assert!(store.load(&users).is_none());
        assert!(store.load(&orders).is_none());
        assert!(store.load(&analytics).is_some());
    }

    #[test]
    fn grid_workspace_store_keeps_same_table_isolated_per_tab() {
        let mut store = GridWorkspaceStore::default();
        let mut left_tab = DataGridState::new();
        left_tab.new_rows.push(vec!["draft-a".to_string()]);
        left_tab.cursor = (3, 0);

        let mut right_tab = DataGridState::new();
        right_tab.new_rows.push(vec!["draft-b".to_string()]);
        right_tab.cursor = (9, 1);

        let left_id = workspace("tab-left", "local", Some("main"), "users");
        let right_id = workspace("tab-right", "local", Some("main"), "users");

        store.save(left_id.clone(), &left_tab);
        store.save(right_id.clone(), &right_tab);

        let restored_left = store
            .load(&left_id)
            .expect("left tab workspace should exist");
        let restored_right = store
            .load(&right_id)
            .expect("right tab workspace should exist");

        assert_eq!(restored_left.cursor, (3, 0));
        assert_eq!(restored_left.new_rows[0][0], "draft-a");
        assert_eq!(restored_right.cursor, (9, 1));
        assert_eq!(restored_right.new_rows[0][0], "draft-b");
    }

    #[test]
    fn grid_workspace_store_can_drop_whole_tab() {
        let mut store = GridWorkspaceStore::default();
        let state = DataGridState::new();
        let keep = workspace("tab-keep", "local", Some("main"), "users");
        let drop = workspace("tab-drop", "local", Some("main"), "users");

        store.save(keep.clone(), &state);
        store.save(drop.clone(), &state);
        store.remove_tab("tab-drop");

        assert!(store.load(&keep).is_some());
        assert!(store.load(&drop).is_none());
    }
}
