//! 主应用程序模块
//!
//! 包含 `DbManagerApp` 结构体，实现了 eframe::App trait，
//! 负责管理应用程序的整体状态和渲染逻辑。

use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Instant;

use crate::core::{
    export_to_csv, export_to_json, export_to_sql, format_sql, import_sql_file, AppConfig,
    AutoComplete, ExportFormat, HighlightColors, QueryHistory, ThemeManager, ThemePreset,
};
use crate::database::{
    connect_database, execute_query, get_tables_for_database, ConnectResult,
    ConnectionConfig, ConnectionManager, QueryResult,
};
use crate::ui::{self, SqlEditorActions, ToolbarActions};

/// 异步任务完成后发送的消息
enum Message {
    /// 数据库连接完成 - SQLite 模式 (连接名, 表列表结果)
    ConnectedWithTables(String, Result<Vec<String>, String>),
    /// 数据库连接完成 - MySQL/PostgreSQL 模式 (连接名, 数据库列表结果)
    ConnectedWithDatabases(String, Result<Vec<String>, String>),
    /// 数据库选择完成 (连接名, 数据库名, 表列表结果)
    DatabaseSelected(String, String, Result<Vec<String>, String>),
    /// 查询执行完成 (SQL语句, 查询结果, 耗时毫秒)
    QueryDone(String, Result<QueryResult, String>, u64),
}

/// 数据库管理器主应用结构体
///
/// 管理所有应用状态，包括数据库连接、查询结果、UI 状态等。
pub struct DbManagerApp {
    // 连接管理
    manager: ConnectionManager,
    show_connection_dialog: bool,
    new_config: ConnectionConfig,

    // 查询状态
    selected_table: Option<String>,
    sql: String,
    result: Option<QueryResult>,

    // 异步通信
    tx: Sender<Message>,
    rx: Receiver<Message>,
    runtime: tokio::runtime::Runtime,
    connecting: bool,
    executing: bool,

    // 配置和历史
    app_config: AppConfig,
    query_history: QueryHistory,

    // 命令行历史 (当前连接的历史记录)
    command_history: Vec<String>,
    history_index: Option<usize>,
    last_message: Option<String>,
    // 当前历史记录对应的连接名
    current_history_connection: Option<String>,

    // 搜索
    search_text: String,
    search_column: Option<String>,

    // 表格选择
    selected_row: Option<usize>,
    selected_cell: Option<(usize, usize)>,

    // 表格编辑状态
    grid_state: ui::DataGridState,

    // UI 状态
    show_export_dialog: bool,
    export_format: ExportFormat,
    export_status: Option<Result<String, String>>,
    show_history_panel: bool,

    // 确认删除对话框
    show_delete_confirm: bool,
    pending_delete_name: Option<String>,

    // 主题
    theme_manager: ThemeManager,

    // 语法高亮
    highlight_colors: HighlightColors,

    // 查询耗时
    last_query_time_ms: Option<u64>,

    // 自动补全
    autocomplete: AutoComplete,
    show_autocomplete: bool,
    selected_completion: usize,

    // SQL 编辑器显示状态
    show_sql_editor: bool,
    // SQL 编辑器需要聚焦
    focus_sql_editor: bool,
    // 侧边栏显示状态
    show_sidebar: bool,
    // 帮助面板显示状态
    show_help: bool,
    // 帮助面板滚动位置
    help_scroll_offset: f32,
    // UI 缩放比例
    ui_scale: f32,
    // 基础 DPI 缩放（系统设置）
    base_pixels_per_point: f32,
}

impl DbManagerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let (tx, rx) = channel();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create tokio runtime");

        // 加载配置
        let app_config = AppConfig::load();
        let theme_manager = ThemeManager::new(app_config.theme_preset);
        let highlight_colors = HighlightColors::from_theme(&theme_manager.colors);
        let query_history = QueryHistory::new(100);

        // 应用主题
        theme_manager.apply(&cc.egui_ctx);

        // 获取基础 DPI 缩放并应用用户缩放设置
        let base_pixels_per_point = cc.egui_ctx.pixels_per_point();
        let ui_scale = app_config.ui_scale.clamp(0.5, 2.0);
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
            tx,
            rx,
            runtime,
            connecting: false,
            executing: false,
            app_config,
            query_history,
            command_history: Vec::new(),
            history_index: None,
            last_message: None,
            current_history_connection: None,
            search_text: String::new(),
            search_column: None,
            selected_row: None,
            selected_cell: None,
            grid_state: ui::DataGridState::new(),
            show_export_dialog: false,
            export_format: ExportFormat::Csv,
            export_status: None,
            show_history_panel: false,
            show_delete_confirm: false,
            pending_delete_name: None,
            theme_manager,
            highlight_colors,
            last_query_time_ms: None,
            autocomplete: AutoComplete::new(),
            show_autocomplete: false,
            selected_completion: 0,
            show_sql_editor: true,
            focus_sql_editor: false,
            show_sidebar: true,
            show_help: false,
            help_scroll_offset: 0.0,
            ui_scale,
            base_pixels_per_point,
        }
    }

    /// 设置 UI 缩放比例
    fn set_ui_scale(&mut self, ctx: &egui::Context, scale: f32) {
        let scale = scale.clamp(0.5, 2.0);
        self.ui_scale = scale;
        self.app_config.ui_scale = scale;
        ctx.set_pixels_per_point(self.base_pixels_per_point * scale);
        let _ = self.app_config.save();
    }

    fn set_theme(&mut self, ctx: &egui::Context, preset: ThemePreset) {
        self.theme_manager.set_theme(preset);
        self.theme_manager.apply(ctx);
        self.highlight_colors = HighlightColors::from_theme(&self.theme_manager.colors);
        self.app_config.theme_preset = preset;
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

    fn connect(&mut self, name: String) {
        if let Some(conn) = self.manager.connections.get(&name) {
            let config = conn.config.clone();
            let tx = self.tx.clone();

            self.connecting = true;
            self.manager.active = Some(name.clone());

            self.runtime.spawn(async move {
                use tokio::time::{timeout, Duration};
                // 30秒连接超时
                let result = timeout(Duration::from_secs(30), connect_database(&config)).await;
                match result {
                    Ok(Ok(ConnectResult::Tables(tables))) => {
                        tx.send(Message::ConnectedWithTables(name, Ok(tables))).ok();
                    }
                    Ok(Ok(ConnectResult::Databases(databases))) => {
                        tx.send(Message::ConnectedWithDatabases(name, Ok(databases))).ok();
                    }
                    Ok(Err(e)) => {
                        tx.send(Message::ConnectedWithTables(name, Err(e.to_string()))).ok();
                    }
                    Err(_) => {
                        tx.send(Message::ConnectedWithTables(name, Err("连接超时".to_string()))).ok();
                    }
                }
            });
        }
    }

    /// 选择数据库（MySQL/PostgreSQL）
    fn select_database(&mut self, database: String) {
        if let Some(active_name) = self.manager.active.clone() {
            if let Some(conn) = self.manager.connections.get(&active_name) {
                let config = conn.config.clone();
                let tx = self.tx.clone();

                self.connecting = true;

                self.runtime.spawn(async move {
                    use tokio::time::{timeout, Duration};
                    let result = timeout(
                        Duration::from_secs(30),
                        get_tables_for_database(&config, &database),
                    )
                    .await;
                    let tables_result = match result {
                        Ok(Ok(tables)) => Ok(tables),
                        Ok(Err(e)) => Err(e.to_string()),
                        Err(_) => Err("获取表列表超时".to_string()),
                    };
                    tx.send(Message::DatabaseSelected(active_name, database, tables_result)).ok();
                });
            }
        }
    }

    fn disconnect(&mut self, name: String) {
        self.manager.disconnect(&name);
        if self.manager.active.as_deref() == Some(&name) {
            self.manager.active = None;
            self.selected_table = None;
            self.result = None;
        }
    }

    fn delete_connection(&mut self, name: &str) {
        self.manager.connections.remove(name);
        // 删除该连接的历史记录
        self.app_config.command_history.remove(name);
        // 如果删除的是当前连接，清空当前状态
        if self.manager.active.as_deref() == Some(name) {
            self.manager.active = None;
            self.selected_table = None;
            self.result = None;
            self.command_history.clear();
            self.current_history_connection = None;
        }
        self.save_config();
    }

    fn execute(&mut self, sql: String) {
        if sql.trim().is_empty() {
            return;
        }

        if let Some(active_name) = self.manager.active.clone() {
            if let Some(conn) = self.manager.connections.get(&active_name) {
                let config = conn.config.clone();
                let tx = self.tx.clone();

                // 添加到命令历史
                if self.command_history.first() != Some(&sql) {
                    self.command_history.insert(0, sql.clone());
                    // 限制每个连接最多保存100条历史记录
                    if self.command_history.len() > 100 {
                        self.command_history.pop();
                    }
                    // 保存历史记录到配置文件
                    self.save_current_history();
                    let _ = self.app_config.save();
                }
                self.history_index = None;

                self.executing = true;
                self.result = None;
                self.last_query_time_ms = None;

                self.runtime.spawn(async move {
                    use tokio::time::{timeout, Duration};
                    let start = Instant::now();
                    // 查询超时 5 分钟
                    let result = timeout(Duration::from_secs(300), execute_query(&config, &sql)).await;
                    let elapsed_ms = start.elapsed().as_millis() as u64;
                    let query_result = match result {
                        Ok(Ok(res)) => Ok(res),
                        Ok(Err(e)) => Err(e.to_string()),
                        Err(_) => Err("查询超时".to_string()),
                    };
                    tx.send(Message::QueryDone(sql, query_result, elapsed_ms)).ok();
                });
            }
        } else {
            self.last_message = Some("请先连接数据库".to_string());
        }
    }

    fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::ConnectedWithTables(name, result) => {
                    // SQLite 模式：直接获得表列表
                    self.connecting = false;
                    match result {
                        Ok(tables) => {
                            self.last_message =
                                Some(format!("已连接到 {} ({} 张表)", name, tables.len()));
                            // 加载该连接的历史记录
                            self.load_history_for_connection(&name);
                            // 更新自动补全的表列表
                            self.autocomplete.set_tables(tables.clone());
                            // 设置连接状态
                            if let Some(conn) = self.manager.connections.get_mut(&name) {
                                conn.set_connected(tables);
                            }
                        }
                        Err(e) => {
                            self.last_message = Some(format!("连接失败: {}", e));
                            self.autocomplete.clear();
                            if let Some(conn) = self.manager.connections.get_mut(&name) {
                                conn.set_error(e);
                            }
                        }
                    }
                    ctx.request_repaint();
                }
                Message::ConnectedWithDatabases(name, result) => {
                    // MySQL/PostgreSQL 模式：获得数据库列表
                    self.connecting = false;
                    match result {
                        Ok(databases) => {
                            self.last_message =
                                Some(format!("已连接到 {} ({} 个数据库)", name, databases.len()));
                            // 加载该连接的历史记录
                            self.load_history_for_connection(&name);
                            // 清空表的自动补全，等选择数据库后再设置
                            self.autocomplete.clear();
                            if let Some(conn) = self.manager.connections.get_mut(&name) {
                                conn.set_connected_with_databases(databases);
                            }
                        }
                        Err(e) => {
                            self.last_message = Some(format!("连接失败: {}", e));
                            self.autocomplete.clear();
                            if let Some(conn) = self.manager.connections.get_mut(&name) {
                                conn.set_error(e);
                            }
                        }
                    }
                    ctx.request_repaint();
                }
                Message::DatabaseSelected(conn_name, db_name, result) => {
                    // 数据库选择完成：获得表列表
                    self.connecting = false;
                    match result {
                        Ok(tables) => {
                            self.last_message =
                                Some(format!("已选择数据库 {} ({} 张表)", db_name, tables.len()));
                            // 更新自动补全的表列表
                            self.autocomplete.set_tables(tables.clone());
                            if let Some(conn) = self.manager.connections.get_mut(&conn_name) {
                                conn.set_database(db_name, tables);
                            }
                        }
                        Err(e) => {
                            self.last_message = Some(format!("选择数据库失败: {}", e));
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

                    let db_type = self
                        .manager
                        .get_active()
                        .map(|c| c.config.db_type.display_name().to_string())
                        .unwrap_or_default();

                    match result {
                        Ok(res) => {
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

                            if res.columns.is_empty() {
                                self.last_message = Some(format!(
                                    "执行成功，影响 {} 行 ({}ms)",
                                    res.affected_rows, elapsed_ms
                                ));
                            } else {
                                self.last_message = Some(format!(
                                    "查询完成，返回 {} 行 ({}ms)",
                                    res.rows.len(),
                                    elapsed_ms
                                ));
                            }

                            self.result = Some(res);
                            self.selected_row = None;
                            self.selected_cell = None;
                            self.search_text.clear();
                        }
                        Err(e) => {
                            self.query_history.add(sql, db_type, false, None);
                            self.last_message = Some(format!("错误: {}", e));
                            self.result = Some(QueryResult::default());
                        }
                    }
                    ctx.request_repaint();
                }
            }
        }
    }

    fn handle_export(&mut self, format: ExportFormat) {
        let table_name = self
            .selected_table
            .clone()
            .unwrap_or_else(|| "query_result".to_string());

        if let Some(result) = &self.result {
            let filter_name = format!("{} 文件", format.display_name());
            let filter_ext = format.extension();

            let file_dialog = rfd::FileDialog::new()
                .set_file_name(format!("{}.{}", table_name, filter_ext))
                .add_filter(&filter_name, &[filter_ext]);

            if let Some(path) = file_dialog.save_file() {
                let export_result = match format {
                    ExportFormat::Csv => export_to_csv(result, &path),
                    ExportFormat::Sql => export_to_sql(result, &table_name, &path),
                    ExportFormat::Json => export_to_json(result, &path),
                };

                self.export_status = Some(match export_result {
                    Ok(()) => Ok(format!("已导出到 {}", path.display())),
                    Err(e) => Err(e),
                });
            }
        }
    }

    fn handle_import(&mut self) {
        let file_dialog = rfd::FileDialog::new()
            .add_filter("SQL 文件", &["sql"])
            .add_filter("所有文件", &["*"]);

        if let Some(path) = file_dialog.pick_file() {
            match import_sql_file(&path) {
                Ok(content) => {
                    self.sql = content;
                    self.last_message = Some(format!("已导入: {}", path.display()));
                }
                Err(e) => {
                    self.last_message = Some(format!("导入错误: {}", e));
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            // Ctrl+N: 新建连接
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                self.show_connection_dialog = true;
            }

            // Ctrl+E: 导出
            if i.modifiers.ctrl && i.key_pressed(egui::Key::E) && self.result.is_some() {
                self.show_export_dialog = true;
                self.export_status = None;
            }

            // Ctrl+I: 导入
            if i.modifiers.ctrl && i.key_pressed(egui::Key::I) {
                self.handle_import();
            }

            // Ctrl+H: 历史记录
            if i.modifiers.ctrl && i.key_pressed(egui::Key::H) {
                self.show_history_panel = !self.show_history_panel;
            }

            // F5: 刷新表列表
            if i.key_pressed(egui::Key::F5) {
                if let Some(name) = self.manager.active.clone() {
                    self.connect(name);
                }
            }

            // Ctrl+L: 清空命令行
            if i.modifiers.ctrl && i.key_pressed(egui::Key::L) {
                self.sql.clear();
                self.last_message = None;
            }

            // Escape: 关闭对话框
            if i.key_pressed(egui::Key::Escape) {
                self.show_connection_dialog = false;
                self.show_export_dialog = false;
                self.show_history_panel = false;
                self.show_delete_confirm = false;
            }

            // Ctrl+J: 切换 SQL 编辑器显示
            if i.modifiers.ctrl && i.key_pressed(egui::Key::J) {
                self.show_sql_editor = !self.show_sql_editor;
                if self.show_sql_editor {
                    // 打开时自动聚焦到编辑器
                    self.focus_sql_editor = true;
                    self.grid_state.focused = false;
                } else {
                    // 关闭时将焦点还给数据表格
                    self.grid_state.focused = true;
                }
            }

            // Ctrl+B: 切换侧边栏显示
            if i.modifiers.ctrl && i.key_pressed(egui::Key::B) {
                self.show_sidebar = !self.show_sidebar;
            }

            // Ctrl+K: 清空搜索
            if i.modifiers.ctrl && i.key_pressed(egui::Key::K) {
                self.search_text.clear();
            }

            // F1: 帮助
            if i.key_pressed(egui::Key::F1) {
                self.show_help = !self.show_help;
            }

            // Ctrl+F: 添加筛选条件
            if i.modifiers.ctrl && i.key_pressed(egui::Key::F) && !i.modifiers.shift {
                if let Some(result) = &self.result {
                    if let Some(col) = result.columns.first() {
                        self.grid_state.filters.push(ui::components::ColumnFilter::new(col.clone()));
                    }
                }
            }

            // Ctrl+Shift+F: 清空筛选条件
            if i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(egui::Key::F) {
                self.grid_state.filters.clear();
            }

            // Ctrl+S: 触发保存表格修改
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                // 标记需要保存
                self.grid_state.pending_save = true;
            }

            // Ctrl+G: 跳转到行
            if i.modifiers.ctrl && i.key_pressed(egui::Key::G) {
                self.grid_state.show_goto_dialog = true;
            }
        });
    }

    /// 处理缩放快捷键（需要在 update 中调用以获取 ctx）
    fn handle_zoom_shortcuts(&mut self, ctx: &egui::Context) {
        let zoom_delta = ctx.input(|i| {
            let mut delta = 0.0f32;

            // Ctrl++ 或 Ctrl+= 放大
            if i.modifiers.ctrl && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
                delta = 0.1;
            }

            // Ctrl+- 缩小
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Minus) {
                delta = -0.1;
            }

            // Ctrl+0 重置缩放
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num0) {
                return Some(-999.0); // 特殊值表示重置
            }

            // Ctrl+滚轮缩放
            if i.modifiers.ctrl && i.raw_scroll_delta.y != 0.0 {
                delta = i.raw_scroll_delta.y * 0.001;
            }

            if delta != 0.0 {
                Some(delta)
            } else {
                None
            }
        });

        if let Some(delta) = zoom_delta {
            if delta == -999.0 {
                // 重置为 1.0
                self.set_ui_scale(ctx, 1.0);
            } else {
                let new_scale = self.ui_scale + delta;
                self.set_ui_scale(ctx, new_scale);
            }
        }
    }
}

impl eframe::App for DbManagerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_messages(ctx);
        self.handle_keyboard_shortcuts(ctx);
        self.handle_zoom_shortcuts(ctx);

        let mut save_connection = false;
        let mut toolbar_actions = ToolbarActions::default();

        // 检测下拉框快捷键
        ctx.input(|i| {
            // Ctrl+T: 打开主题选择器
            if i.modifiers.ctrl && i.key_pressed(egui::Key::T) {
                toolbar_actions.open_theme_selector = true;
            }
            // Ctrl+1: 打开连接选择器
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num1) {
                toolbar_actions.open_connection_selector = true;
            }
            // Ctrl+2: 打开数据库选择器
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num2) {
                toolbar_actions.open_database_selector = true;
            }
            // Ctrl+3: 打开表选择器
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Num3) {
                toolbar_actions.open_table_selector = true;
            }
            // Ctrl+D: 切换日/夜模式
            if i.modifiers.ctrl && i.key_pressed(egui::Key::D) {
                toolbar_actions.toggle_dark_mode = true;
            }
        });

        // ===== 对话框 =====

        // 连接对话框
        ui::ConnectionDialog::show(
            ctx,
            &mut self.show_connection_dialog,
            &mut self.new_config,
            &mut save_connection,
        );

        // 删除确认对话框
        let mut confirm_delete = false;
        let delete_msg = self
            .pending_delete_name
            .as_ref()
            .map(|n| format!("确定要删除连接 '{}' 吗？", n))
            .unwrap_or_default();
        ui::ConfirmDialog::show(
            ctx,
            &mut self.show_delete_confirm,
            "删除连接",
            &delete_msg,
            "删除",
            &mut confirm_delete,
        );

        if confirm_delete {
            if let Some(name) = self.pending_delete_name.take() {
                self.delete_connection(&name);
            }
        }

        // 导出对话框
        let mut export_action: Option<ExportFormat> = None;
        let table_name = self
            .selected_table
            .clone()
            .unwrap_or_else(|| "result".to_string());
        ui::ExportDialog::show(
            ctx,
            &mut self.show_export_dialog,
            &mut self.export_format,
            &table_name,
            &mut export_action,
            &self.export_status,
        );

        if let Some(format) = export_action {
            self.handle_export(format);
        }

        // 历史记录面板
        let mut history_selected_sql: Option<String> = None;
        let mut clear_history = false;
        ui::HistoryPanel::show(
            ctx,
            &mut self.show_history_panel,
            &self.query_history,
            &mut history_selected_sql,
            &mut clear_history,
        );

        if let Some(sql) = history_selected_sql {
            self.sql = sql;
        }

        if clear_history {
            self.query_history.clear();
        }

        // 帮助面板（带 Helix 键位支持）
        ui::HelpDialog::show_with_scroll(ctx, &mut self.show_help, &mut self.help_scroll_offset);

        // ===== 侧边栏 =====
        let sidebar_actions = if self.show_sidebar {
            ui::Sidebar::show(
                ctx,
                &mut self.manager,
                &mut self.selected_table,
                &mut self.show_connection_dialog,
            )
        } else {
            ui::SidebarActions::default()
        };

        // ===== 底部 SQL 编辑器 =====
        let mut sql_editor_actions = SqlEditorActions::default();

        if self.show_sql_editor {
            // 获取屏幕高度，按比例计算编辑器高度
            let screen_height = ctx.screen_rect().height();
            let default_height = (screen_height * 0.30).clamp(180.0, 450.0); // 30% 屏幕高度
            let min_height = (screen_height * 0.18).clamp(120.0, 250.0); // 18% 最小
            let max_height = (screen_height * 0.60).clamp(300.0, 700.0); // 60% 最大

            egui::TopBottomPanel::bottom("sql_editor_panel")
                .resizable(true)
                .min_height(min_height)
                .max_height(max_height)
                .default_height(default_height)
                .show(ctx, |ui| {
                    sql_editor_actions = ui::SqlEditor::show(
                        ui,
                        &mut self.sql,
                        &self.command_history,
                        &mut self.history_index,
                        self.executing,
                        &self.last_message,
                        &self.highlight_colors,
                        self.last_query_time_ms,
                        &self.autocomplete,
                        &mut self.show_autocomplete,
                        &mut self.selected_completion,
                        &mut self.focus_sql_editor,
                    );
                });
        }

        // ===== 中心面板 =====
        egui::CentralPanel::default().show(ctx, |ui| {
            // 准备连接、数据库和表列表数据
            let connections: Vec<String> = self.manager.connections.keys().cloned().collect();
            let active_connection = self.manager.active.as_deref();
            let (databases, selected_database, tables): (Vec<String>, Option<&str>, Vec<String>) = self
                .manager
                .get_active()
                .map(|c| {
                    (
                        c.databases.clone(),
                        c.selected_database.as_deref(),
                        c.tables.clone(),
                    )
                })
                .unwrap_or_default();
            let selected_table = self.selected_table.as_deref();

            // 工具栏
            ui::Toolbar::show(
                ui,
                &self.theme_manager,
                self.result.is_some(),
                self.show_sidebar,
                self.show_sql_editor,
                self.app_config.is_dark_mode,
                &mut toolbar_actions,
                &connections,
                active_connection,
                &databases,
                selected_database,
                &tables,
                selected_table,
                self.ui_scale,
            );

            ui.separator();

            // 搜索栏
            let columns = self
                .result
                .as_ref()
                .map(|r| r.columns.clone())
                .unwrap_or_default();

            // 预先计算过滤后的行数（避免不必要的 clone）
            let result_count = if let Some(r) = &self.result {
                let total = r.rows.len();
                if self.search_text.is_empty() {
                    Some((total, total))
                } else {
                    let search_lower = self.search_text.to_lowercase();
                    // 预先查找列索引，避免在循环中重复查找
                    let col_idx = self
                        .search_column
                        .as_ref()
                        .and_then(|col_name| r.columns.iter().position(|c| c == col_name));

                    let filtered = r
                        .rows
                        .iter()
                        .filter(|row| match col_idx {
                            Some(idx) => row
                                .get(idx)
                                .map(|cell| cell.to_lowercase().contains(&search_lower))
                                .unwrap_or(false),
                            None => row
                                .iter()
                                .any(|cell| cell.to_lowercase().contains(&search_lower)),
                        })
                        .count();
                    Some((filtered, total))
                }
            } else {
                None
            };

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

            // 数据表格区域
            if let Some(result) = &self.result {
                if !result.columns.is_empty() {
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
                        self.last_message = Some(msg);
                    }

                    // 执行生成的 SQL
                    for sql in grid_actions.sql_to_execute {
                        self.execute(sql);
                    }
                } else if result.affected_rows > 0 {
                    ui.vertical_centered(|ui| {
                        ui.add_space(50.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "✓ 执行成功，影响 {} 行",
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

                    if let Some(table) = &self.selected_table {
                        if ui.button(format!("查询表 {} 的数据", table)).clicked() {
                            self.sql = format!("SELECT * FROM {} LIMIT 100;", table);
                            sql_editor_actions.execute = true;
                        }
                    }
                });
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(50.0);
                    ui.label("请先在左侧选择或创建数据库连接");
                });
            }
        });

        // ===== 处理各种操作 =====

        // 处理工具栏操作
        if toolbar_actions.toggle_sidebar {
            self.show_sidebar = !self.show_sidebar;
        }

        if toolbar_actions.toggle_editor {
            self.show_sql_editor = !self.show_sql_editor;
        }

        if toolbar_actions.refresh_tables {
            if let Some(name) = self.manager.active.clone() {
                self.connect(name);
            }
        }

        // 处理连接切换
        if let Some(conn_name) = toolbar_actions.switch_connection {
            if self.manager.active.as_deref() != Some(&conn_name) {
                self.connect(conn_name);
                self.selected_table = None;
                self.result = None;
            }
        }

        // 处理数据库切换
        if let Some(db_name) = toolbar_actions.switch_database {
            self.select_database(db_name);
        }

        // 处理表切换
        if let Some(table_name) = toolbar_actions.switch_table {
            self.selected_table = Some(table_name.clone());
            let query_sql = format!("SELECT * FROM {} LIMIT 100;", table_name);
            self.execute(query_sql);
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
                let schema_sql = match conn.config.db_type {
                    crate::database::DatabaseType::SQLite => {
                        format!("PRAGMA table_info('{}');", table)
                    }
                    crate::database::DatabaseType::PostgreSQL => {
                        format!(
                            "SELECT column_name, data_type, is_nullable, column_default \
                             FROM information_schema.columns \
                             WHERE table_name = '{}' \
                             ORDER BY ordinal_position;",
                            table
                        )
                    }
                    crate::database::DatabaseType::MySQL => {
                        format!("DESCRIBE `{}`;", table)
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
            let query_sql = format!("SELECT * FROM {} LIMIT 100;", table);
            self.execute(query_sql);
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
            self.last_message = None;
            self.last_query_time_ms = None;
        }

        // 保存新连接
        if save_connection {
            let config = std::mem::take(&mut self.new_config);
            let name = config.name.clone();
            self.manager.add(config);
            self.save_config();
            self.connect(name);
        }

        // 持续刷新
        if self.connecting || self.executing {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
    }
}
