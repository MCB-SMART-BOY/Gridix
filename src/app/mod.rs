//! 主应用程序模块
//!
//! 包含 `DbManagerApp` 结构体，实现了 eframe::App trait，
//! 负责管理应用程序的整体状态和渲染逻辑。
//!
//! ## 子模块
//!
//! - `database`: 数据库连接和查询操作
//! - `export`: 数据导出功能
//! - `import`: 数据导入功能
//! - `keyboard`: 键盘快捷键处理
//! - `message`: 异步消息定义

mod database;
mod dialogs;
mod export;
mod import;
mod keyboard;
mod message;
pub mod state;

use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::core::{
    clear_highlight_cache, constants, format_sql, AppConfig, AutoComplete, HighlightColors,
    KeyBindings, NotificationManager, ProgressManager, QueryHistory, ThemeManager, ThemePreset,
};
use crate::database::{ConnectionConfig, ConnectionManager, QueryResult};
use crate::ui::{
    self, DdlDialogState, ExportConfig, KeyBindingsDialogState, QueryTabBar, QueryTabManager,
    SqlEditorActions, ToolbarActions,
};

use message::Message;

/// 数据库管理器主应用结构体
///
/// 管理所有应用状态，包括数据库连接、查询结果、UI 状态等。
/// 实现了 `eframe::App` trait，作为 GUI 应用程序的入口点。
///
/// # 架构概述
///
/// - **连接管理**: 支持 SQLite、PostgreSQL、MySQL 三种数据库
/// - **异步执行**: 使用 tokio runtime 异步执行查询，避免阻塞 UI
/// - **消息通道**: 通过 mpsc 通道在异步任务和 UI 线程间通信
/// - **多 Tab 支持**: 支持同时打开多个查询标签页
///
/// # 状态分组
///
/// 字段按功能分为以下几组：
/// - 连接管理：数据库连接状态和配置
/// - 查询状态：SQL 编辑器、执行结果
/// - 异步通信：消息通道和运行时
/// - 配置历史：应用配置和查询历史
/// - UI 状态：对话框、面板的显示状态
pub struct DbManagerApp {
    // ==================== 连接管理 ====================
    /// 数据库连接管理器，维护所有连接配置和状态
    manager: ConnectionManager,
    /// 是否显示新建/编辑连接对话框
    show_connection_dialog: bool,
    /// 当前编辑的连接配置（用于新建/编辑对话框）
    new_config: ConnectionConfig,

    // ==================== 查询状态 ====================
    /// 当前选中的表名
    selected_table: Option<String>,
    /// 当前 SQL 编辑器内容
    sql: String,
    /// 当前查询结果
    result: Option<QueryResult>,
    /// 多 Tab 查询管理器，支持多个独立查询
    tab_manager: QueryTabManager,

    // ==================== 异步通信 ====================
    /// 消息发送端，用于从异步任务发送结果到 UI
    tx: Sender<Message>,
    /// 消息接收端，UI 线程轮询获取异步结果
    rx: Receiver<Message>,
    /// Tokio 异步运行时
    runtime: tokio::runtime::Runtime,
    /// 是否正在建立连接
    connecting: bool,
    /// 是否正在执行查询
    executing: bool,

    // ==================== 配置和历史 ====================
    /// 应用程序配置（主题、UI 缩放等）
    app_config: AppConfig,
    /// 查询历史记录（用于历史面板）
    query_history: QueryHistory,
    /// 当前连接的命令历史（用于 ↑/↓ 导航）
    command_history: Vec<String>,
    /// 命令历史导航索引
    history_index: Option<usize>,
    /// 通知管理器（替代原来的 last_message）
    notifications: NotificationManager,
    /// 进度管理器
    progress: ProgressManager,
    /// 当前历史记录对应的连接名（用于切换连接时保存/恢复）
    current_history_connection: Option<String>,

    // ==================== 搜索和选择 ====================
    /// 表格搜索文本
    search_text: String,
    /// 搜索限定的列名
    search_column: Option<String>,
    /// 当前选中的行索引
    selected_row: Option<usize>,
    /// 当前选中的单元格 (行, 列)
    selected_cell: Option<(usize, usize)>,
    /// 数据表格状态（筛选、排序、编辑等）
    grid_state: ui::DataGridState,

    // ==================== 对话框状态 ====================
    /// 是否显示导出对话框
    show_export_dialog: bool,
    /// 导出配置
    export_config: ExportConfig,
    /// 导出操作结果
    export_status: Option<Result<String, String>>,
    /// 是否显示导入对话框
    show_import_dialog: bool,
    /// 导入状态（文件、预览、配置）
    import_state: ui::ImportState,
    /// 是否显示历史面板
    show_history_panel: bool,
    /// 历史面板状态
    history_panel_state: ui::HistoryPanelState,
    /// 是否显示删除确认对话框
    show_delete_confirm: bool,
    /// 待删除的连接名
    pending_delete_name: Option<String>,

    // ==================== 主题和外观 ====================
    /// 主题管理器
    theme_manager: ThemeManager,
    /// 语法高亮颜色配置
    highlight_colors: HighlightColors,
    /// 上次查询耗时（毫秒）
    last_query_time_ms: Option<u64>,

    // ==================== 自动补全 ====================
    /// 自动补全引擎
    autocomplete: AutoComplete,
    /// 是否显示自动补全列表
    show_autocomplete: bool,
    /// 当前选中的补全项索引
    selected_completion: usize,

    // ==================== UI 显示状态 ====================
    /// SQL 编辑器是否展开显示
    show_sql_editor: bool,
    /// SQL 编辑器是否需要获取焦点
    focus_sql_editor: bool,
    /// 侧边栏是否显示
    show_sidebar: bool,
    /// 全局焦点区域（侧边栏/SQL 编辑器/数据表格）
    focus_area: ui::FocusArea,
    /// 侧边栏当前焦点子区域（连接/数据库/表）
    sidebar_section: ui::SidebarSection,
    /// 侧边栏面板状态（上下分割、触发器列表、选中索引等）
    sidebar_panel_state: ui::SidebarPanelState,
    /// 侧边栏宽度
    sidebar_width: f32,
    /// 是否显示帮助面板
    show_help: bool,
    /// 帮助面板滚动位置
    help_scroll_offset: f32,
    /// 是否显示关于对话框
    show_about: bool,
    /// 用户设置的 UI 缩放比例
    ui_scale: f32,
    /// 系统基础 DPI 缩放
    base_pixels_per_point: f32,
    /// DDL 对话框状态（新建表等）
    ddl_dialog_state: DdlDialogState,
    /// 新建数据库对话框状态
    create_db_dialog_state: ui::CreateDbDialogState,
    /// 新建用户对话框状态
    create_user_dialog_state: ui::CreateUserDialogState,
    /// 快捷键绑定
    keybindings: KeyBindings,
    /// 快捷键设置对话框状态
    keybindings_dialog_state: KeyBindingsDialogState,
    /// 中央面板左右分割比例 (0.0-1.0, 左侧占比)
    central_panel_ratio: f32,
    /// 是否显示 ER 图面板
    show_er_diagram: bool,
    /// ER 图状态
    er_diagram_state: ui::ERDiagramState,
}

impl DbManagerApp {
    /// 检查是否有任何模态对话框打开
    /// 用于在对话框打开时禁用其他区域的键盘响应
    fn has_modal_dialog_open(&self) -> bool {
        self.show_connection_dialog
            || self.show_export_dialog
            || self.show_import_dialog
            || self.show_delete_confirm
            || self.show_help
            || self.show_about
            || self.show_history_panel
            || self.ddl_dialog_state.show
            || self.create_db_dialog_state.show
            || self.create_user_dialog_state.show
            || self.keybindings_dialog_state.show
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        
        // 创建 tokio runtime，优先多线程，失败则降级到单线程
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .or_else(|e| {
                eprintln!("[warn] 多线程运行时创建失败: {}，降级到单线程模式", e);
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
            })
            .expect("无法创建 tokio 运行时，系统资源可能不足");

        // 加载配置
        let app_config = AppConfig::load();
        let theme_manager = ThemeManager::new(app_config.theme_preset);
        let highlight_colors = HighlightColors::from_theme(&theme_manager.colors);
        let query_history = QueryHistory::new(100);

        // 应用主题
        theme_manager.apply(&cc.egui_ctx);

        // 获取基础 DPI 缩放并应用用户缩放设置
        let base_pixels_per_point = cc.egui_ctx.pixels_per_point();
        let ui_scale = app_config.ui_scale.clamp(constants::ui::UI_SCALE_MIN, constants::ui::UI_SCALE_MAX);
        cc.egui_ctx.set_pixels_per_point(base_pixels_per_point * ui_scale);

        // 从配置恢复连接
        let mut manager = ConnectionManager::default();
        for config in &app_config.connections {
            manager.add(config.clone());
        }

        Self {
            manager,
            show_connection_dialog: false,
            new_config: ConnectionConfig::default(),
            selected_table: None,
            sql: String::new(),
            result: None,
            tab_manager: QueryTabManager::new(),
            tx,
            rx,
            runtime,
            connecting: false,
            executing: false,
            app_config,
            query_history,
            command_history: Vec::new(),
            history_index: None,
            notifications: NotificationManager::new(),
            progress: ProgressManager::new(),
            current_history_connection: None,
            search_text: String::new(),
            search_column: None,
            selected_row: None,
            selected_cell: None,
            grid_state: ui::DataGridState::new(),
            show_export_dialog: false,
            export_config: ExportConfig::default(),
            export_status: None,
            show_import_dialog: false,
            import_state: ui::ImportState::new(),
            show_history_panel: false,
            history_panel_state: ui::HistoryPanelState::default(),
            show_delete_confirm: false,
            pending_delete_name: None,
            theme_manager,
            highlight_colors,
            last_query_time_ms: None,
            autocomplete: AutoComplete::new(),
            show_autocomplete: false,
            selected_completion: 0,
            show_sql_editor: false,
            focus_sql_editor: false,
            show_sidebar: false,
            focus_area: ui::FocusArea::DataGrid,
            sidebar_section: ui::SidebarSection::Connections,
            sidebar_panel_state: ui::SidebarPanelState::default(),
            sidebar_width: 280.0,  // 默认侧边栏宽度
            show_help: false,
            help_scroll_offset: 0.0,
            show_about: false,
            ui_scale,
            base_pixels_per_point,
            ddl_dialog_state: DdlDialogState::default(),
            create_db_dialog_state: ui::CreateDbDialogState::new(),
            create_user_dialog_state: ui::CreateUserDialogState::new(),
            keybindings: KeyBindings::default(),
            keybindings_dialog_state: KeyBindingsDialogState::default(),
            central_panel_ratio: 0.65,
            show_er_diagram: false,
            er_diagram_state: ui::ERDiagramState::new(),
        }
    }

    /// 设置 UI 缩放比例
    fn set_ui_scale(&mut self, ctx: &egui::Context, scale: f32) {
        let scale = scale.clamp(constants::ui::UI_SCALE_MIN, constants::ui::UI_SCALE_MAX);
        self.ui_scale = scale;
        self.app_config.ui_scale = scale;
        ctx.set_pixels_per_point(self.base_pixels_per_point * scale);
        let _ = self.app_config.save();
    }

    /// 检查当前连接是否是 MySQL（用于选择 SQL 引号类型）
    fn is_mysql(&self) -> bool {
        self.manager.get_active()
            .map(|c| matches!(c.config.db_type, crate::database::DatabaseType::MySQL))
            .unwrap_or(false)
    }

    fn set_theme(&mut self, ctx: &egui::Context, preset: ThemePreset) {
        self.theme_manager.set_theme(preset);
        self.theme_manager.apply(ctx);
        self.highlight_colors = HighlightColors::from_theme(&self.theme_manager.colors);
        self.app_config.theme_preset = preset;
        // 清除语法高亮缓存，确保使用新主题颜色
        clear_highlight_cache();
        let _ = self.app_config.save();
    }

    fn save_config(&mut self) {
        // 保存当前连接的历史记录
        self.save_current_history();

        self.app_config.connections = self
            .manager
            .connections
            .values()
            .map(|c| c.config.clone())
            .collect();
        let _ = self.app_config.save();
    }

    /// 保存当前连接的历史记录到配置
    fn save_current_history(&mut self) {
        if let Some(conn_name) = &self.current_history_connection {
            self.app_config
                .command_history
                .insert(conn_name.clone(), self.command_history.clone());
        }
    }

    /// 加载指定连接的历史记录
    fn load_history_for_connection(&mut self, conn_name: &str) {
        // 先保存当前连接的历史
        self.save_current_history();

        // 加载新连接的历史
        self.command_history = self
            .app_config
            .command_history
            .get(conn_name)
            .cloned()
            .unwrap_or_default();
        self.current_history_connection = Some(conn_name.to_string());
        self.history_index = None;
    }

    // 注意：connect, select_database, disconnect, delete_connection, execute,
    // fetch_primary_key, handle_connection_error 已移至 database.rs 模块

    fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::ConnectedWithTables(name, result) => {
                    // SQLite 模式：直接获得表列表
                    self.connecting = false;
                    match result {
                        Ok(tables) => {
                            self.notifications.success(
                                format!("已连接到 {} ({} 张表)", name, tables.len())
                            );
                            self.load_history_for_connection(&name);
                            self.autocomplete.set_tables(tables.clone());
                            if let Some(conn) = self.manager.connections.get_mut(&name) {
                                conn.set_connected(tables);
                            }
                            // 重置侧边栏选中索引
                            self.sidebar_panel_state.selection.reset_for_connection_change();
                            // 自动加载触发器（SQLite 直接连接后加载）
                            self.load_triggers();
                        }
                        Err(e) => self.handle_connection_error(&name, e),
                    }
                    ctx.request_repaint();
                }
                Message::ConnectedWithDatabases(name, result) => {
                    // MySQL/PostgreSQL 模式：获得数据库列表
                    self.connecting = false;
                    match result {
                        Ok(databases) => {
                            self.notifications.success(
                                format!("已连接到 {} ({} 个数据库)", name, databases.len())
                            );
                            self.load_history_for_connection(&name);
                            self.autocomplete.clear();
                            if let Some(conn) = self.manager.connections.get_mut(&name) {
                                conn.set_connected_with_databases(databases);
                            }
                            // 重置侧边栏选中索引
                            self.sidebar_panel_state.selection.reset_for_connection_change();
                        }
                        Err(e) => self.handle_connection_error(&name, e),
                    }
                    ctx.request_repaint();
                }
                Message::DatabaseSelected(conn_name, db_name, result) => {
                    // 数据库选择完成：获得表列表
                    self.connecting = false;
                    match result {
                        Ok(tables) => {
                            self.notifications.success(
                                format!("已选择数据库 {} ({} 张表)", db_name, tables.len())
                            );
                            // 更新自动补全的表列表
                            self.autocomplete.set_tables(tables.clone());
                            if let Some(conn) = self.manager.connections.get_mut(&conn_name) {
                                conn.set_database(db_name, tables);
                            }
                            // 重置表和触发器的选中索引
                            self.sidebar_panel_state.selection.reset_for_database_change();
                            // 自动加载触发器
                            self.load_triggers();
                        }
                        Err(e) => {
                            self.notifications.error(format!("选择数据库失败: {}", e));
                        }
                    }
                    // 清空已选择的表
                    self.selected_table = None;
                    self.result = None;
                    ctx.request_repaint();
                }
                Message::QueryDone(sql, result, elapsed_ms) => {
                    self.executing = false;
                    self.last_query_time_ms = Some(elapsed_ms);

                    // 提前检测 SQL 类型（在 sql 被移动之前）
                    let sql_lower = sql.trim().to_lowercase();
                    let is_update_or_delete =
                        sql_lower.starts_with("update") || sql_lower.starts_with("delete");
                    let is_insert = sql_lower.starts_with("insert");

                    let db_type = self
                        .manager
                        .get_active()
                        .map(|c| c.config.db_type.display_name().to_string())
                        .unwrap_or_default();

                    match result {
                        Ok(mut res) => {
                            // 限制结果集大小，防止内存溢出
                            let original_rows = res.rows.len();
                            let was_truncated = original_rows > constants::database::MAX_RESULT_SET_ROWS;
                            if was_truncated {
                                res.rows.truncate(constants::database::MAX_RESULT_SET_ROWS);
                                res.truncated = true;
                                res.original_row_count = Some(original_rows);
                            }

                            self.query_history.add(
                                sql,
                                db_type,
                                true,
                                if res.affected_rows > 0 {
                                    Some(res.affected_rows)
                                } else {
                                    None
                                },
                            );

                            let msg = if res.columns.is_empty() {
                                format!(
                                    "执行成功，影响 {} 行 ({}ms)",
                                    res.affected_rows, elapsed_ms
                                )
                            } else if was_truncated {
                                format!(
                                    "查询完成，返回 {} 行（已截断，原始 {} 行，建议使用 LIMIT）({}ms)",
                                    res.rows.len(), original_rows, elapsed_ms
                                )
                            } else {
                                format!(
                                    "查询完成，返回 {} 行 ({}ms)",
                                    res.rows.len(),
                                    elapsed_ms
                                )
                            };
                            self.notifications.success(&msg);

                            self.selected_row = None;
                            self.selected_cell = None;
                            self.search_text.clear();

                            // 根据 SQL 类型设置光标和滚动位置
                            // UPDATE/DELETE: 保持当前位置
                            // INSERT: 光标和滚动都到最后一行
                            if is_update_or_delete {
                                // 保持当前光标位置
                                self.grid_state.scroll_to_row = Some(self.grid_state.cursor.0);
                            } else if is_insert {
                                // 光标移动到最后一行（新插入的数据）
                                let last_row = res.rows.len().saturating_sub(1);
                                self.grid_state.cursor = (last_row, 0);
                                self.grid_state.scroll_to_row = Some(last_row);
                            }
                            
                            // 保留用户当前的焦点位置，不强制转移焦点
                            // 只有当焦点不在任何特定区域时才默认到数据表格
                            if self.focus_area == ui::FocusArea::DataGrid {
                                self.grid_state.focused = true;
                            }

                            // 同步结果到当前 Tab（先 clone 给 tab，再移动给 self）
                            if let Some(tab) = self.tab_manager.get_active_mut() {
                                tab.result = Some(res.clone());
                                tab.executing = false;
                                tab.query_time_ms = Some(elapsed_ms);
                                tab.last_message = Some(msg);
                            }
                            
                            // 更新列名到自动补全（当有选中表且返回了列信息时）
                            if let Some(table) = &self.selected_table
                                && !res.columns.is_empty() {
                                    self.autocomplete.set_columns(table.clone(), res.columns.clone());
                                }
                            
                            self.result = Some(res);
                        }
                        Err(e) => {
                            self.query_history.add(sql, db_type, false, None);
                            let err_msg = format!("错误: {}", e);
                            self.notifications.error(&err_msg);
                            self.result = Some(QueryResult::default());

                            // 同步错误到当前 Tab
                            if let Some(tab) = self.tab_manager.get_active_mut() {
                                tab.executing = false;
                                tab.last_message = Some(err_msg);
                            }
                        }
                    }
                    ctx.request_repaint();
                }
                Message::PrimaryKeyFetched(table_name, pk_column) => {
                    // 如果当前选中的表与返回的表匹配，设置主键列索引
                    if self.selected_table.as_deref() == Some(&table_name) {
                        if let Some(pk_name) = pk_column {
                            // 在当前结果的列中查找主键列的索引
                            if let Some(result) = &self.result
                                && let Some(idx) = result.columns.iter().position(|c| c == &pk_name) {
                                    self.grid_state.primary_key_column = Some(idx);
                                }
                        } else {
                            self.grid_state.primary_key_column = None;
                        }
                    }
                    ctx.request_repaint();
                }
                Message::TriggersFetched(result) => {
                    self.sidebar_panel_state.loading_triggers = false;
                    match result {
                        Ok(triggers) => {
                            self.sidebar_panel_state.set_triggers(triggers);
                        }
                        Err(e) => {
                            self.notifications.error(format!("加载触发器失败: {}", e));
                        }
                    }
                    ctx.request_repaint();
                }
                Message::ForeignKeysFetched(result) => {
                    match result {
                        Ok(fks) => {
                            // 更新表中列的外键标记
                            for fk in &fks {
                                if let Some(table) = self.er_diagram_state.tables.iter_mut().find(|t| t.name == fk.from_table)
                                    && let Some(col) = table.columns.iter_mut().find(|c| c.name == fk.from_column) {
                                        col.is_foreign_key = true;
                                    }
                            }
                            
                            // 将外键信息转换为 ER 图关系
                            let mut relationships: Vec<ui::Relationship> = fks
                                .into_iter()
                                .map(|fk| ui::Relationship {
                                    from_table: fk.from_table,
                                    from_column: fk.from_column,
                                    to_table: fk.to_table,
                                    to_column: fk.to_column,
                                    relation_type: ui::RelationType::OneToMany,
                                })
                                .collect();
                            
                            // 如果没有外键约束，尝试基于列名推断关系
                            if relationships.is_empty() {
                                relationships = self.infer_relationships_from_columns();
                            }
                            
                            let rel_count = relationships.len();
                            self.er_diagram_state.relationships = relationships;
                            self.er_diagram_state.loading = false;
                            
                            if rel_count > 0 {
                                self.notifications.info(format!(
                                    "ER图: {} 张表, {} 个关系",
                                    self.er_diagram_state.tables.len(),
                                    rel_count
                                ));
                            } else {
                                self.notifications.info(format!(
                                    "ER图: {} 张表（未发现外键关系）",
                                    self.er_diagram_state.tables.len()
                                ));
                            }
                        }
                        Err(e) => {
                            self.er_diagram_state.loading = false;
                            self.notifications.error(format!("加载外键关系失败: {}", e));
                        }
                    }
                    ctx.request_repaint();
                }
                Message::ERTableColumnsFetched(table_name, result) => {
                    match result {
                        Ok(columns) => {
                            // 找到对应的表并更新列信息
                            if let Some(er_table) = self.er_diagram_state.tables.iter_mut().find(|t| t.name == table_name) {
                                er_table.columns = columns.into_iter().map(|c| ui::ERColumn {
                                    name: c.name,
                                    data_type: c.data_type,
                                    is_primary_key: c.is_primary_key,
                                    is_foreign_key: false, // 外键标记会在 ForeignKeysFetched 中更新
                                    nullable: c.is_nullable,
                                    default_value: c.default_value,
                                }).collect();
                            }
                            
                            // 检查是否所有表都加载完成
                            let all_loaded = self.er_diagram_state.tables.iter().all(|t| !t.columns.is_empty());
                            if all_loaded && !self.er_diagram_state.tables.is_empty() {
                                // 重新应用布局（因为表尺寸变化了）
                                ui::grid_layout(
                                    &mut self.er_diagram_state.tables,
                                    4,
                                    egui::Vec2::new(60.0, 50.0),
                                );
                                
                                // 如果还没有关系，尝试推断
                                if self.er_diagram_state.relationships.is_empty() {
                                    let inferred = self.infer_relationships_from_columns();
                                    if !inferred.is_empty() {
                                        self.er_diagram_state.relationships = inferred;
                                        self.notifications.info(format!(
                                            "ER图: 推断出 {} 个关系",
                                            self.er_diagram_state.relationships.len()
                                        ));
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            self.notifications.warning(format!("获取表 {} 结构失败: {}", table_name, e));
                        }
                    }
                    ctx.request_repaint();
                }
            }
        }
    }
    
    /// 加载当前数据库的触发器
    fn load_triggers(&mut self) {
        if let Some(conn) = self.manager.get_active() {
            let config = conn.config.clone();
            let tx = self.tx.clone();
            
            self.sidebar_panel_state.loading_triggers = true;
            self.sidebar_panel_state.clear_triggers();
            
            self.runtime.spawn(async move {
                let result = crate::database::get_triggers(&config).await;
                let _ = tx.send(Message::TriggersFetched(result.map_err(|e| e.to_string())));
            });
        }
    }

    /// 加载 ER 图数据
    fn load_er_diagram_data(&mut self) {
        // 清空旧数据
        self.er_diagram_state.clear();
        self.er_diagram_state.loading = true;
        
        if let Some(conn) = self.manager.get_active() {
            // 从现有表信息构建 ER 表
            let tables = conn.tables.clone();
            let db_name = conn.selected_database.clone().unwrap_or_else(|| "未选择".to_string());
            let config = conn.config.clone();
            
            if tables.is_empty() {
                self.notifications.warning(format!("数据库 {} 没有表，请先选择数据库", db_name));
                self.er_diagram_state.loading = false;
                return;
            }
            
            for table_name in &tables {
                let er_table = ui::ERTable::new(table_name.clone());
                self.er_diagram_state.tables.push(er_table);
            }
            
            // 应用初始布局（增加间距）
            ui::grid_layout(
                &mut self.er_diagram_state.tables,
                4,
                egui::Vec2::new(60.0, 50.0),
            );
            
            self.notifications.info(format!("ER图: 加载 {} 张表，正在获取结构... ({})", tables.len(), db_name));
            
            // 异步加载每个表的列信息
            for table_name in &tables {
                let tx = self.tx.clone();
                let config_clone = config.clone();
                let table_clone = table_name.clone();
                self.runtime.spawn(async move {
                    let result = crate::database::get_table_columns(&config_clone, &table_clone).await;
                    let _ = tx.send(Message::ERTableColumnsFetched(table_clone, result.map_err(|e| e.to_string())));
                });
            }
            
            // 异步加载外键关系
            let tx = self.tx.clone();
            self.runtime.spawn(async move {
                let result = crate::database::get_foreign_keys(&config).await;
                let _ = tx.send(Message::ForeignKeysFetched(result.map_err(|e| e.to_string())));
            });
        } else {
            self.notifications.warning("请先连接数据库");
            self.er_diagram_state.loading = false;
        }
        
        self.er_diagram_state.needs_layout = false;
    }
    
    /// 基于列名推断表之间的关系
    /// 规则：如果列名是 `xxx_id` 或 `xxxid`，尝试匹配名为 `xxx` 或 `xxxs` 的表
    fn infer_relationships_from_columns(&self) -> Vec<ui::Relationship> {
        let mut relationships = Vec::new();
        let table_names: Vec<&str> = self.er_diagram_state.tables.iter().map(|t| t.name.as_str()).collect();
        
        for table in &self.er_diagram_state.tables {
            for col in &table.columns {
                // 跳过主键列
                if col.is_primary_key {
                    continue;
                }
                
                let col_lower = col.name.to_lowercase();
                
                // 检查是否是可能的外键列（以 id 结尾或包含 _id）
                let potential_ref = if col_lower.ends_with("_id") {
                    // xxx_id -> xxx
                    Some(col_lower.trim_end_matches("_id").to_string())
                } else if col_lower.ends_with("id") && col_lower.len() > 2 {
                    // xxxid -> xxx
                    Some(col_lower.trim_end_matches("id").to_string())
                } else {
                    None
                };
                
                if let Some(ref_name) = potential_ref {
                    // 尝试匹配表名
                    for &target_table in &table_names {
                        if target_table == table.name {
                            continue; // 跳过自引用
                        }
                        
                        let target_lower = target_table.to_lowercase();
                        
                        // 匹配：user, users, user_info 等
                        if target_lower == ref_name 
                            || target_lower == format!("{}s", ref_name)
                            || target_lower == format!("{}_info", ref_name)
                            || target_lower.starts_with(&format!("{}_", ref_name))
                        {
                            // 找到匹配，创建关系
                            relationships.push(ui::Relationship {
                                from_table: table.name.clone(),
                                from_column: col.name.clone(),
                                to_table: target_table.to_string(),
                                to_column: "id".to_string(),
                                relation_type: ui::RelationType::OneToMany,
                            });
                            break; // 每列只匹配一个表
                        }
                    }
                }
            }
        }
        
        relationships
    }

    fn handle_export_with_config(&mut self, config: ExportConfig) {
        let table_name = self
            .selected_table
            .clone()
            .unwrap_or_else(|| "query_result".to_string());

        if let Some(result) = &self.result {
            let filter_name = format!("{} 文件", config.format.display_name());
            let filter_ext = config.format.extension();

            let file_dialog = rfd::FileDialog::new()
                .set_file_name(format!("{}.{}", table_name, filter_ext))
                .add_filter(&filter_name, &[filter_ext]);

            if let Some(path) = file_dialog.save_file() {
                // 使用导出模块执行导出
                self.export_status =
                    Some(export::execute_export(result, &table_name, &path, &config));
            }
        }
    }

    // 注意：handle_import, select_import_file, refresh_import_preview, 
    // execute_import 已移至 import.rs 模块

    // 注意：handle_keyboard_shortcuts, handle_zoom_shortcuts 已移至 keyboard.rs 模块
}

impl eframe::App for DbManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_messages(ctx);
        self.handle_keyboard_shortcuts(ctx);
        self.handle_zoom_shortcuts(ctx);
        
        // 清理过期通知，如果有通知被清理则请求重绘
        if self.notifications.tick() {
            ctx.request_repaint();
        }

        let mut toolbar_actions = ToolbarActions::default();

        // 检测下拉框快捷键（仅在没有对话框打开时响应）
        let has_dialog = self.has_modal_dialog_open();
        ctx.input(|i| {
            // Ctrl+Shift+T: 打开主题选择器（改为 Shift 避免与新建查询冲突）
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::T) {
                toolbar_actions.open_theme_selector = true;
            }
            
            // 对话框打开时跳过以下快捷键
            if has_dialog {
                return;
            }
            
            // Ctrl+T: 新建查询标签页
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::T) {
                self.tab_manager.new_tab();
                // 同步当前 Tab 的状态
                if let Some(tab) = self.tab_manager.get_active() {
                    self.sql = tab.sql.clone();
                    self.result = tab.result.clone();
                }
            }
            // Ctrl+1: 聚焦侧边栏连接列表
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num1) {
                self.show_sidebar = true;
                self.focus_area = ui::FocusArea::Sidebar;
                self.sidebar_section = ui::SidebarSection::Connections;
                self.grid_state.focused = false;
                self.focus_sql_editor = false;
                self.notifications.info("切换到: 连接列表");
            }
            // Ctrl+2: 聚焦侧边栏数据库列表
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num2) {
                self.show_sidebar = true;
                self.focus_area = ui::FocusArea::Sidebar;
                self.sidebar_section = ui::SidebarSection::Databases;
                self.grid_state.focused = false;
                self.focus_sql_editor = false;
                self.notifications.info("切换到: 数据库列表");
            }
            // Ctrl+3: 聚焦侧边栏表列表
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num3) {
                self.show_sidebar = true;
                self.focus_area = ui::FocusArea::Sidebar;
                self.sidebar_section = ui::SidebarSection::Tables;
                self.grid_state.focused = false;
                self.focus_sql_editor = false;
                self.notifications.info("切换到: 表列表");
            }
            // Ctrl+4: 聚焦侧边栏触发器列表
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num4) {
                self.show_sidebar = true;
                self.focus_area = ui::FocusArea::Sidebar;
                self.sidebar_section = ui::SidebarSection::Triggers;
                self.grid_state.focused = false;
                self.focus_sql_editor = false;
                self.notifications.info("切换到: 触发器列表");
            }
            // Ctrl+D: 切换日/夜模式（不与 Ctrl+Shift+D 新建数据库冲突）
            if i.modifiers.ctrl && !i.modifiers.shift && i.key_pressed(egui::Key::D) {
                toolbar_actions.toggle_dark_mode = true;
            }
        });

        // ===== 对话框 =====
        let dialog_results = self.render_dialogs(ctx);
        let save_connection = dialog_results.save_connection;
        self.handle_dialog_results(dialog_results);

        // ===== 底部 SQL 编辑器 =====
        let mut sql_editor_actions = SqlEditorActions::default();

        if self.show_sql_editor {
            // 只有在没有对话框打开时，SQL 编辑器才响应快捷键
            let is_editor_focused = self.focus_area == ui::FocusArea::SqlEditor
                && !self.has_modal_dialog_open();
            // 可拖动调整大小的编辑器面板
            egui::TopBottomPanel::bottom("sql_editor_panel")
                .resizable(true)
                .min_height(150.0)
                .max_height(500.0)
                .default_height(280.0)
                .show(ctx, |ui| {
                    // 获取最新通知消息用于状态栏显示
                    let latest_msg = self.notifications.latest_message().map(|s| s.to_string());
                    sql_editor_actions = ui::SqlEditor::show(
                        ui,
                        &mut self.sql,
                        &self.command_history,
                        &mut self.history_index,
                        self.executing,
                        &latest_msg,
                        &self.highlight_colors,
                        self.last_query_time_ms,
                        &self.autocomplete,
                        &mut self.show_autocomplete,
                        &mut self.selected_completion,
                        &mut self.focus_sql_editor,
                        is_editor_focused,
                    );
                });
        }

        // ===== 中心面板 =====
        let central_frame = egui::Frame::NONE
            .fill(ctx.style().visuals.panel_fill)
            .inner_margin(egui::Margin::same(0));
        
        // 侧边栏操作结果（在 CentralPanel 外声明）
        let mut sidebar_actions = ui::SidebarActions::default();
        
        egui::CentralPanel::default().frame(central_frame).show(ctx, |ui| {
            // 准备连接、数据库和表列表数据（提前克隆以避免借用冲突）
            let connections: Vec<String> = self.manager.connections.keys().cloned().collect();
            let active_connection = self.manager.active.clone();
            let (databases, selected_database, tables): (Vec<String>, Option<String>, Vec<String>) = self
                .manager
                .get_active()
                .map(|c| {
                    (
                        c.databases.clone(),
                        c.selected_database.clone(),
                        c.tables.clone(),
                    )
                })
                .unwrap_or_default();
            let selected_table_for_toolbar = self.selected_table.clone();

            // 使用 horizontal 布局：侧边栏 + 分割条 + 主内容区
            let available_width = ui.available_width();
            let available_height = ui.available_height();
            let divider_width = 8.0;
            
            // 计算侧边栏和主内容区的宽度
            let sidebar_width = if self.show_sidebar { self.sidebar_width } else { 0.0 };
            let main_width = if self.show_sidebar {
                available_width - sidebar_width - divider_width
            } else {
                available_width
            };
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                
                // ===== 侧边栏区域 =====
                if self.show_sidebar {
                    ui.allocate_ui_with_layout(
                        egui::vec2(sidebar_width, available_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.set_min_size(egui::vec2(sidebar_width, available_height));
                            
                            // 只有在没有对话框打开时，侧边栏才响应键盘
                            let is_sidebar_focused = self.focus_area == ui::FocusArea::Sidebar 
                                && !self.has_modal_dialog_open();
                            
                            sidebar_actions = ui::Sidebar::show_in_ui(
                                ui,
                                &mut self.manager,
                                &mut self.selected_table,
                                &mut self.show_connection_dialog,
                                is_sidebar_focused,
                                self.sidebar_section,
                                &mut self.sidebar_panel_state,
                                sidebar_width,
                            );
                        }
                    );
                    
                    // 可拖动的垂直分割条（与 ER 图分割条相同风格）
                    let (divider_rect, divider_response) = ui.allocate_exact_size(
                        egui::vec2(divider_width, available_height),
                        egui::Sense::drag(),
                    );
                    
                    // 绘制分割条
                    let divider_color = if divider_response.dragged() || divider_response.hovered() {
                        egui::Color32::from_rgb(100, 150, 255)
                    } else {
                        egui::Color32::from_rgba_unmultiplied(128, 128, 128, 80)
                    };
                    
                    ui.painter().rect_filled(
                        divider_rect.shrink2(egui::vec2(2.0, 4.0)),
                        egui::CornerRadius::same(2),
                        divider_color,
                    );
                    
                    // 中间的拖动指示器（三个小点）
                    let center = divider_rect.center();
                    for offset in [-10.0, 0.0, 10.0] {
                        ui.painter().circle_filled(
                            egui::pos2(center.x, center.y + offset),
                            2.0,
                            egui::Color32::from_gray(180),
                        );
                    }
                    
                    // 处理拖动调整侧边栏宽度
                    if divider_response.dragged() {
                        let delta = divider_response.drag_delta().x;
                        self.sidebar_width = (self.sidebar_width + delta).clamp(
                            constants::ui::SIDEBAR_MIN_WIDTH_PX,
                            constants::ui::SIDEBAR_MAX_WIDTH_PX,
                        );
                    }
                    
                    // 鼠标光标
                    if divider_response.hovered() || divider_response.dragged() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                    }
                }
                
                // ===== 主内容区 =====
                ui.allocate_ui_with_layout(
                    egui::vec2(main_width, available_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_min_size(egui::vec2(main_width, available_height));
                        
                        // 添加左边距（仅当侧边栏显示时不需要，否则需要）
                        let content_margin = if self.show_sidebar { 0 } else { 8 };
                        egui::Frame::NONE
                            .inner_margin(egui::Margin {
                                left: content_margin,
                                right: 8,
                                top: 8,
                                bottom: 8,
                            })
                            .show(ui, |ui| {
                                // 工具栏
                                let cancel_task_id = ui::Toolbar::show(
                                    ui,
                                    &self.theme_manager,
                                    self.result.is_some(),
                                    self.show_sidebar,
                                    self.show_sql_editor,
                                    self.app_config.is_dark_mode,
                                    &mut toolbar_actions,
                                    &connections,
                                    active_connection.as_deref(),
                                    &databases,
                                    selected_database.as_deref(),
                                    &tables,
                                    selected_table_for_toolbar.as_deref(),
                                    self.ui_scale,
                                    &self.progress,
                                );
                                
                                // 处理进度任务取消
                                if let Some(id) = cancel_task_id {
                                    self.progress.cancel(id);
                                }

                                ui.separator();

                                // Tab 栏（多查询窗口）
                                let tab_actions = QueryTabBar::show(
                                    ui,
                                    &self.tab_manager.tabs,
                                    self.tab_manager.active_index,
                                    &self.highlight_colors,
                                );

                                // 处理 Tab 栏操作
                                if tab_actions.new_tab {
                                    self.tab_manager.new_tab();
                                }
                                if let Some(idx) = tab_actions.switch_to {
                                    self.tab_manager.set_active(idx);
                                    // 同步当前 Tab 的 SQL 和结果到主状态
                                    if let Some(tab) = self.tab_manager.get_active() {
                                        self.sql = tab.sql.clone();
                                        self.result = tab.result.clone();
                                    }
                                }
                                if let Some(idx) = tab_actions.close_tab {
                                    self.tab_manager.close_tab(idx);
                                    // 同步当前 Tab 的状态
                                    if let Some(tab) = self.tab_manager.get_active() {
                                        self.sql = tab.sql.clone();
                                        self.result = tab.result.clone();
                                    }
                                }
                                if tab_actions.close_others {
                                    self.tab_manager.close_other_tabs();
                                }
                                if tab_actions.close_right {
                                    self.tab_manager.close_tabs_to_right();
                                }

                                ui.separator();

                                // 搜索栏
                                let columns = self
                                    .result
                                    .as_ref()
                                    .map(|r| r.columns.clone())
                                    .unwrap_or_default();

                                // 使用统一的搜索计数函数
                                let result_count = self.result.as_ref().map(|r| {
                                    ui::count_search_matches(r, &self.search_text, &self.search_column)
                                });

                                ui.add_space(4.0);
                                ui::SearchBar::show(
                                    ui,
                                    &mut self.search_text,
                                    &mut self.search_column,
                                    &columns,
                                    result_count,
                                );
                                ui.add_space(4.0);

                                ui.separator();

                                // 数据表格区域（支持左右分割显示 ER 图）
                                if self.show_er_diagram {
                                    // 左右分割布局 - 使用 horizontal 和固定宽度的子区域
                                    let available_width = ui.available_width();
                                    let available_height = ui.available_height();
                                    let divider_width = 8.0;
                                    let left_width = (available_width - divider_width) * self.central_panel_ratio;
                                    let right_width = available_width - left_width - divider_width;
                                    let theme_preset = self.theme_manager.current;
                                    
                                    ui.horizontal(|ui| {
                                        // 左侧：数据表格
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(left_width, available_height),
                                            egui::Layout::top_down(egui::Align::LEFT),
                                            |ui| {
                                                ui.set_min_size(egui::vec2(left_width, available_height));
                                                
                                                if let Some(result) = &self.result {
                                                    if !result.columns.is_empty() {
                                                        self.grid_state.focused = self.focus_area == ui::FocusArea::DataGrid 
                                                            && !self.has_modal_dialog_open();
                                                        
                                                        let table_name = self.selected_table.as_deref();
                                                        let _ = ui::DataGrid::show_editable(
                                                            ui,
                                                            result,
                                                            &self.search_text,
                                                            &self.search_column,
                                                            &mut self.selected_row,
                                                            &mut self.selected_cell,
                                                            &mut self.grid_state,
                                                            table_name,
                                                        );
                                                    } else {
                                                        ui.centered_and_justified(|ui| {
                                                            ui.label("暂无数据");
                                                        });
                                                    }
                                                } else {
                                                    ui.centered_and_justified(|ui| {
                                                        ui.label("请执行查询");
                                                    });
                                                }
                                            }
                                        );
                                        
                                        // 可拖动的垂直分割条
                                        let (divider_rect, divider_response) = ui.allocate_exact_size(
                                            egui::vec2(divider_width, available_height),
                                            egui::Sense::drag(),
                                        );
                                        
                                        // 绘制分割条
                                        let divider_color = if divider_response.dragged() || divider_response.hovered() {
                                            egui::Color32::from_rgb(100, 150, 255)
                                        } else {
                                            egui::Color32::from_rgba_unmultiplied(128, 128, 128, 80)
                                        };
                                        
                                        ui.painter().rect_filled(
                                            divider_rect.shrink2(egui::vec2(2.0, 4.0)),
                                            egui::CornerRadius::same(2),
                                            divider_color,
                                        );
                                        
                                        // 中间的拖动指示器（三个小点）
                                        let center = divider_rect.center();
                                        for offset in [-10.0, 0.0, 10.0] {
                                            ui.painter().circle_filled(
                                                egui::pos2(center.x, center.y + offset),
                                                2.0,
                                                egui::Color32::from_gray(180),
                                            );
                                        }
                                        
                                        // 处理拖动
                                        if divider_response.dragged() {
                                            let delta = divider_response.drag_delta().x;
                                            let delta_ratio = delta / available_width;
                                            self.central_panel_ratio = (self.central_panel_ratio + delta_ratio).clamp(0.2, 0.8);
                                        }
                                        
                                        // 鼠标光标
                                        if divider_response.hovered() || divider_response.dragged() {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                        }
                                        
                                        // 右侧：ER 关系图
                                        ui.allocate_ui_with_layout(
                                            egui::vec2(right_width, available_height),
                                            egui::Layout::top_down(egui::Align::LEFT),
                                            |ui| {
                                                ui.set_min_size(egui::vec2(right_width, available_height));
                                                
                                                let er_response = self.er_diagram_state.show(ui, &theme_preset);
                                                
                                                if er_response.refresh_requested {
                                                    self.load_er_diagram_data();
                                                }
                                                if er_response.layout_requested {
                                                    ui::force_directed_layout(
                                                        &mut self.er_diagram_state.tables,
                                                        &self.er_diagram_state.relationships,
                                                        50,
                                                    );
                                                }
                                                if er_response.fit_view_requested {
                                                    self.er_diagram_state.fit_to_view(ui.available_size());
                                                }
                                            }
                                        );
                                    });
                                } else if let Some(result) = &self.result {
                                    if !result.columns.is_empty() {
                                        // 同步焦点状态：只有当全局焦点在 DataGrid 且没有对话框打开时才响应键盘
                                        self.grid_state.focused = self.focus_area == ui::FocusArea::DataGrid 
                                            && !self.has_modal_dialog_open();
                                        
                                        let table_name = self.selected_table.as_deref();
                                        let (grid_actions, _) = ui::DataGrid::show_editable(
                                            ui,
                                            result,
                                            &self.search_text,
                                            &self.search_column,
                                            &mut self.selected_row,
                                            &mut self.selected_cell,
                                            &mut self.grid_state,
                                            table_name,
                                        );

                                        // 处理表格操作
                                        if let Some(msg) = grid_actions.message {
                                            self.notifications.info(msg);
                                        }

                                        // 执行生成的 SQL
                                        for sql in grid_actions.sql_to_execute {
                                            self.execute(sql);
                                        }

                                        // 处理刷新请求
                                        if grid_actions.refresh_requested
                                            && let Some(table) = &self.selected_table {
                                                let sql = format!("SELECT * FROM {}", table);
                                                self.execute(sql);
                                            }
                                        
                                        // 处理焦点转移请求
                                        if let Some(transfer) = grid_actions.focus_transfer {
                                            match transfer {
                                                ui::FocusTransfer::ToSidebar => {
                                                    self.show_sidebar = true;
                                                    self.focus_area = ui::FocusArea::Sidebar;
                                                    self.grid_state.focused = false;
                                                }
                                                ui::FocusTransfer::ToSqlEditor => {
                                                    self.show_sql_editor = true;
                                                    self.focus_area = ui::FocusArea::SqlEditor;
                                                    self.grid_state.focused = false;
                                                    self.focus_sql_editor = true;
                                                }
                                            }
                                        }
                                        
                                        // 处理表格请求焦点（点击表格时）
                                        if grid_actions.request_focus && self.focus_area != ui::FocusArea::DataGrid {
                                            self.focus_area = ui::FocusArea::DataGrid;
                                            self.grid_state.focused = true;
                                        }
                                    } else if result.affected_rows > 0 {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(50.0);
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "执行成功，影响 {} 行",
                                                    result.affected_rows
                                                ))
                                                .color(ui::styles::SUCCESS)
                                                .size(16.0),
                                            );
                                        });
                                    } else {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(50.0);
                                            ui.label(egui::RichText::new("暂无数据").color(ui::styles::GRAY));
                                        });
                                    }
                                } else if self.manager.connections.is_empty() {
                                    ui::Welcome::show(ui);
                                } else if self.manager.active.is_some() {
                                    // 有连接但没有结果
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(50.0);
                                        ui.label("在底部命令行输入 SQL 查询");
                                        ui.add_space(8.0);

                                        if let Some(table) = &self.selected_table
                                            && ui.button(format!("查询表 {} 的数据", table)).clicked()
                                                && let Ok(quoted_table) = ui::quote_identifier(table, self.is_mysql()) {
                                                    self.sql = format!("SELECT * FROM {} LIMIT {};", quoted_table, constants::database::DEFAULT_QUERY_LIMIT);
                                                    sql_editor_actions.execute = true;
                                                }
                                    });
                                } else {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(50.0);
                                        ui.label("请先在左侧选择或创建数据库连接");
                                    });
                                }
                            }); // Frame 闭包结束
                    }
                ); // allocate_ui_with_layout 主内容区结束
            }); // horizontal 布局结束
        }); // CentralPanel 闭包结束
        
        // 处理侧边栏焦点转移
        if let Some(transfer) = sidebar_actions.focus_transfer {
            match transfer {
                ui::SidebarFocusTransfer::ToDataGrid => {
                    self.focus_area = ui::FocusArea::DataGrid;
                    self.grid_state.focused = true;
                }
            }
        }

        // 处理侧边栏层级导航（左右键切换 section）
        if let Some(new_section) = sidebar_actions.section_change {
            self.sidebar_section = new_section;
        }

        // ===== 处理各种操作 =====

        // 处理工具栏操作
        if toolbar_actions.toggle_sidebar {
            self.show_sidebar = !self.show_sidebar;
        }

        if toolbar_actions.toggle_editor {
            self.show_sql_editor = !self.show_sql_editor;
        }

        if toolbar_actions.refresh_tables
            && let Some(name) = self.manager.active.clone() {
                self.connect(name);
            }

        // 处理连接切换
        if let Some(conn_name) = toolbar_actions.switch_connection
            && self.manager.active.as_deref() != Some(&conn_name) {
                self.connect(conn_name);
                self.selected_table = None;
                self.result = None;
            }

        // 处理数据库切换
        if let Some(db_name) = toolbar_actions.switch_database {
            self.select_database(db_name);
        }

        // 处理表切换
        if let Some(table_name) = toolbar_actions.switch_table {
            self.selected_table = Some(table_name.clone());
            self.grid_state.primary_key_column = None; // 先清空主键信息
            if let Ok(quoted_table) = ui::quote_identifier(&table_name, self.is_mysql()) {
                let query_sql = format!("SELECT * FROM {} LIMIT {};", quoted_table, constants::database::DEFAULT_QUERY_LIMIT);
                self.execute(query_sql);
            }
            // 异步获取主键列
            self.fetch_primary_key(&table_name);
            // 切换表后清空编辑区，不残留自动生成的查询语句
            self.sql.clear();
        }

        if toolbar_actions.export {
            self.show_export_dialog = true;
            self.export_status = None;
        }

        if toolbar_actions.import {
            self.handle_import();
        }

        if toolbar_actions.create_table {
            let db_type = self.manager.get_active()
                .map(|c| c.config.db_type)
                .unwrap_or_default();
            self.ddl_dialog_state.open_create_table(db_type);
        }

        if toolbar_actions.create_database {
            let db_type = self.manager.get_active()
                .map(|c| c.config.db_type)
                .unwrap_or_default();
            self.create_db_dialog_state.open(db_type);
        }

        if toolbar_actions.create_user {
            if let Some(conn) = self.manager.get_active() {
                let db_type = conn.config.db_type;
                // SQLite 不支持用户管理
                if matches!(db_type, crate::database::DatabaseType::SQLite) {
                    self.notifications.warning("SQLite 不支持用户管理");
                } else {
                    // 获取可用数据库列表
                    let databases = conn.databases.clone();
                    self.create_user_dialog_state.open(db_type, databases);
                }
            } else {
                self.notifications.warning("请先连接数据库");
            }
        }

        if toolbar_actions.toggle_er_diagram {
            self.show_er_diagram = !self.show_er_diagram;
            if self.show_er_diagram {
                self.load_er_diagram_data();
                self.notifications.info("ER 关系图已打开");
            } else {
                self.notifications.info("ER 关系图已关闭");
            }
        }

        if let Some(preset) = toolbar_actions.theme_changed {
            // 更新当前模式对应的主题
            if self.app_config.is_dark_mode {
                self.app_config.dark_theme = preset;
            } else {
                self.app_config.light_theme = preset;
            }
            self.set_theme(ctx, preset);
        }

        if toolbar_actions.toggle_dark_mode {
            self.app_config.is_dark_mode = !self.app_config.is_dark_mode;
            let new_theme = if self.app_config.is_dark_mode {
                self.app_config.dark_theme
            } else {
                self.app_config.light_theme
            };
            self.set_theme(ctx, new_theme);
        }

        // 处理缩放操作
        if toolbar_actions.zoom_in {
            self.set_ui_scale(ctx, self.ui_scale + 0.1);
        }
        if toolbar_actions.zoom_out {
            self.set_ui_scale(ctx, self.ui_scale - 0.1);
        }
        if toolbar_actions.zoom_reset {
            self.set_ui_scale(ctx, 1.0);
        }

        if toolbar_actions.show_history {
            self.show_history_panel = true;
        }

        if toolbar_actions.show_help {
            self.show_help = true;
        }
        
        if toolbar_actions.show_about {
            self.show_about = true;
        }

        if toolbar_actions.show_keybindings {
            self.keybindings_dialog_state.open(&self.keybindings);
        }

        // 处理侧边栏操作
        if let Some(name) = sidebar_actions.connect {
            self.connect(name);
        }

        if let Some(name) = sidebar_actions.disconnect {
            self.disconnect(name);
        }

        // 处理侧边栏数据库选择
        if let Some(db_name) = sidebar_actions.select_database {
            self.select_database(db_name);
        }

        // 处理删除请求
        if let Some(name) = sidebar_actions.delete {
            self.pending_delete_name = Some(name);
            self.show_delete_confirm = true;
        }

        // 处理查看表结构
        if let Some(table) = sidebar_actions.show_table_schema {
            self.selected_table = Some(table.clone());
            // 根据数据库类型生成查看表结构的 SQL
            if let Some(conn) = self.manager.get_active() {
                // 对于 PRAGMA 和 information_schema 查询，表名作为字符串参数更安全
                let schema_sql = match conn.config.db_type {
                    crate::database::DatabaseType::SQLite => {
                        // SQLite PRAGMA 使用单引号包裹的字符串
                        let escaped = table.replace('\'', "''");
                        format!("PRAGMA table_info('{}');", escaped)
                    }
                    crate::database::DatabaseType::PostgreSQL => {
                        // PostgreSQL information_schema 使用字符串参数
                        let escaped = table.replace('\'', "''");
                        format!(
                            "SELECT column_name, data_type, is_nullable, column_default \
                             FROM information_schema.columns \
                             WHERE table_name = '{}' \
                             ORDER BY ordinal_position;",
                            escaped
                        )
                    }
                    crate::database::DatabaseType::MySQL => {
                        // MySQL DESCRIBE 使用反引号，同时禁止点号防止跨库访问
                        let escaped = table.replace('`', "``").replace('.', "_");
                        format!("DESCRIBE `{}`;", escaped)
                    }
                };
                self.execute(schema_sql);
                // 不在编辑区残留自动生成的查询语句
                self.sql.clear();
            }
        }

        // 处理查询表数据（从侧边栏双击表）
        if let Some(table) = sidebar_actions.query_table {
            self.selected_table = Some(table.clone());
            self.grid_state.primary_key_column = None; // 先清空主键信息
            if let Ok(quoted_table) = ui::quote_identifier(&table, self.is_mysql()) {
                let query_sql = format!("SELECT * FROM {} LIMIT {};", quoted_table, constants::database::DEFAULT_QUERY_LIMIT);
                self.execute(query_sql);
            }
            // 异步获取主键列
            self.fetch_primary_key(&table);
            // 不在编辑区残留自动生成的查询语句
            self.sql.clear();
        }

        // 处理 SQL 编辑器操作
        if sql_editor_actions.execute && !self.sql.is_empty() {
            let sql = self.sql.clone();
            self.execute(sql);
            // 执行后清空编辑器
            self.sql.clear();
        }

        if sql_editor_actions.format {
            self.sql = format_sql(&self.sql);
        }

        if sql_editor_actions.clear {
            self.sql.clear();
            self.notifications.dismiss_all();
            self.last_query_time_ms = None;
        }

        // 编辑器焦点转移到表格
        if sql_editor_actions.focus_to_grid {
            self.focus_area = ui::FocusArea::DataGrid;
            self.grid_state.focused = true;
        }
        
        // 编辑器请求焦点（点击编辑器时）
        if sql_editor_actions.request_focus && self.focus_area != ui::FocusArea::SqlEditor {
            self.focus_area = ui::FocusArea::SqlEditor;
            self.grid_state.focused = false;
            self.focus_sql_editor = true;
        }

        // 保存新连接
        if save_connection {
            let config = std::mem::take(&mut self.new_config);
            let name = config.name.clone();
            self.manager.add(config);
            self.save_config();
            self.connect(name);
        }

        // 渲染通知 toast
        ui::NotificationToast::show(ctx, &self.notifications);
        
        // 持续刷新（有活动任务或有通知时需要刷新）
        if self.connecting || self.executing || !self.notifications.is_empty() {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
        
        // 清理连接池，确保所有数据库连接正确关闭
        self.runtime.block_on(async {
            crate::database::POOL_MANAGER.clear_all().await;
        });
    }
}
