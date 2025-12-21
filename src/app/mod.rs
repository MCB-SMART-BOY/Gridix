//! 主应用程序模块
//!
//! 包含 `DbManagerApp` 结构体，实现了 eframe::App trait，
//! 负责管理应用程序的整体状态和渲染逻辑。
//!
//! ## 子模块
//!
//! - `database`: 数据库连接和查询操作
//! - `dialogs`: 对话框渲染和处理
//! - `er_diagram`: ER 关系图数据加载
//! - `export`: 数据导出功能
//! - `handler`: 异步消息处理
//! - `import`: 数据导入功能
//! - `keyboard`: 键盘快捷键处理
//! - `message`: 异步消息定义
//! - `render`: UI 渲染和操作处理
//! - `state`: 应用状态定义

mod database;
mod dialogs;
mod er_diagram;
mod export;
mod handler;
mod import;
mod keyboard;
mod message;
mod render;
pub mod state;

use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::core::{
    clear_highlight_cache, constants, AppConfig, AutoComplete, HighlightColors,
    KeyBindings, NotificationManager, ProgressManager, QueryHistory, ThemeManager, ThemePreset,
};
use crate::database::{ConnectionConfig, ConnectionManager, QueryResult};
use crate::ui::{
    self, DdlDialogState, ExportConfig, KeyBindingsDialogState, QueryTabManager,
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
    /// SQL 编辑器模式 (Normal/Insert)
    editor_mode: ui::EditorMode,

    // ==================== UI 显示状态 ====================
    /// SQL 编辑器是否展开显示
    show_sql_editor: bool,
    /// SQL 编辑器是否需要获取焦点
    focus_sql_editor: bool,
    /// 侧边栏是否显示
    show_sidebar: bool,
    /// 全局焦点区域（侧边栏/SQL 编辑器/数据表格）
    focus_area: ui::FocusArea,
    /// 工具栏当前选中项索引（用于键盘导航）
    toolbar_index: usize,
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
    /// SQL 编辑器高度（用于可调整大小）
    sql_editor_height: f32,
    /// 待执行的切换日/夜模式操作（由键盘快捷键设置）
    pending_toggle_dark_mode: bool,
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
                tracing::warn!(error = %e, "多线程运行时创建失败，降级到单线程模式");
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
            editor_mode: ui::EditorMode::Normal,
            show_sql_editor: false,
            focus_sql_editor: false,
            show_sidebar: false,
            focus_area: ui::FocusArea::DataGrid,
            toolbar_index: 0,
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
            sql_editor_height: 200.0,  // 默认 SQL 编辑器高度
            pending_toggle_dark_mode: false,
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
    
    // 注意：handle_messages 已移至 handler.rs 模块
    
    // 注意：load_er_diagram_data, infer_relationships_from_columns 已移至 er_diagram.rs 模块

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

    fn load_routines(&mut self) {
        if let Some(conn) = self.manager.get_active() {
            let config = conn.config.clone();
            let tx = self.tx.clone();
            
            self.sidebar_panel_state.loading_routines = true;
            self.sidebar_panel_state.clear_routines();
            
            self.runtime.spawn(async move {
                let result = crate::database::get_routines(&config).await;
                let _ = tx.send(Message::RoutinesFetched(result.map_err(|e| e.to_string())));
            });
        }
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

        // 检测焦点切换快捷键
        self.handle_focus_shortcuts(ctx, &mut toolbar_actions);

        // ===== 对话框 =====
        let dialog_results = self.render_dialogs(ctx);
        let save_connection = dialog_results.save_connection;
        self.handle_dialog_results(dialog_results);

        // SQL 编辑器操作（将在主内容区内部渲染）
        let mut sql_editor_actions = SqlEditorActions::default();

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
                            
                            // 获取当前查询结果的列信息
                            let columns: Vec<String> = self
                                .result
                                .as_ref()
                                .map(|r| r.columns.clone())
                                .unwrap_or_default();

                            let (actions, filter_changed) = ui::Sidebar::show_in_ui(
                                ui,
                                &mut self.manager,
                                &mut self.selected_table,
                                &mut self.show_connection_dialog,
                                is_sidebar_focused,
                                self.sidebar_section,
                                &mut self.sidebar_panel_state,
                                sidebar_width,
                                &mut self.grid_state.filters,
                                &columns,
                            );
                            sidebar_actions = actions;

                            // 如果筛选条件改变，使缓存失效
                            if filter_changed {
                                self.grid_state.filter_cache.invalidate();
                            }
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
                                let is_toolbar_focused = self.focus_area == ui::FocusArea::Toolbar;
                                let cancel_task_id = ui::Toolbar::show_with_focus(
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
                                    is_toolbar_focused,
                                    self.toolbar_index,
                                );
                                
                                // 处理进度任务取消
                                if let Some(id) = cancel_task_id {
                                    self.progress.cancel(id);
                                }
                                
                                // 如果焦点在工具栏，处理键盘输入
                                if self.focus_area == ui::FocusArea::Toolbar {
                                    ui::Toolbar::handle_keyboard(
                                        ui,
                                        &mut self.toolbar_index,
                                        &mut toolbar_actions,
                                    );
                                    
                                    // 显示焦点提示
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("工具栏焦点").small().color(self.highlight_colors.keyword));
                                        ui.label(egui::RichText::new(" h/l:移动 j:Tab栏 Enter:选择").small().color(egui::Color32::GRAY));
                                    });
                                    
                                    // 处理焦点转移
                                    if let Some(transfer) = toolbar_actions.focus_transfer {
                                        match transfer {
                                            ui::ToolbarFocusTransfer::ToQueryTabs => {
                                                self.focus_area = ui::FocusArea::QueryTabs;
                                            }
                                        }
                                    }
                                }

                                ui.separator();

                                // Tab 栏（多查询窗口）
                                let mut tab_actions = ui::QueryTabBar::show(
                                    ui,
                                    &self.tab_manager.tabs,
                                    self.tab_manager.active_index,
                                    &self.highlight_colors,
                                );
                                
                                // 如果焦点在Tab栏，处理键盘输入
                                if self.focus_area == ui::FocusArea::QueryTabs {
                                    ui::QueryTabBar::handle_keyboard(
                                        ui,
                                        self.tab_manager.tabs.len(),
                                        self.tab_manager.active_index,
                                        &mut tab_actions,
                                    );
                                    
                                    // 显示焦点提示
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("TAB焦点").small().color(self.highlight_colors.keyword));
                                        ui.label(egui::RichText::new(" h/l:切换 j:表格 k:工具栏 d:删除").small().color(egui::Color32::GRAY));
                                    });
                                }
                                
                                // 处理Tab栏焦点转移
                                if let Some(transfer) = tab_actions.focus_transfer {
                                    match transfer {
                                        ui::TabBarFocusTransfer::ToToolbar => {
                                            self.focus_area = ui::FocusArea::Toolbar;
                                        }
                                        ui::TabBarFocusTransfer::ToDataGrid => {
                                            self.focus_area = ui::FocusArea::DataGrid;
                                            self.grid_state.focused = true;
                                        }
                                    }
                                }
                                
                                self.handle_tab_actions(tab_actions);

                                ui.separator();

                                // 计算数据表格和 SQL 编辑器的高度分配
                                let total_content_height = ui.available_height();
                                let sql_editor_height = if self.show_sql_editor {
                                    self.sql_editor_height.clamp(100.0, total_content_height * 0.6)
                                } else {
                                    0.0
                                };
                                let divider_height = if self.show_sql_editor { 6.0 } else { 0.0 };
                                let data_grid_height = total_content_height - sql_editor_height - divider_height;

                                // 数据表格区域（支持左右分割显示 ER 图）
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), data_grid_height),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                if self.show_er_diagram {
                                    // 左右分割布局 - 使用 horizontal 和固定宽度的子区域
                                    let available_width = ui.available_width();
                                    let available_height = data_grid_height;
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
                                                        
                                                        // 处理打开筛选面板请求
                                                        if grid_actions.open_filter_panel {
                                                            self.show_sidebar = true;
                                                            self.sidebar_panel_state.show_filters = true;
                                                            self.sidebar_section = ui::SidebarSection::Filters;
                                                            self.focus_area = ui::FocusArea::Sidebar;
                                                        }
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
                                            && let Some(table) = &self.selected_table
                                            && let Ok(quoted_table) = ui::quote_identifier(table, self.is_mysql()) {
                                                let sql = format!("SELECT * FROM {} LIMIT {};", quoted_table, constants::database::DEFAULT_QUERY_LIMIT);
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
                                                ui::FocusTransfer::ToQueryTabs => {
                                                    self.focus_area = ui::FocusArea::QueryTabs;
                                                    self.grid_state.focused = false;
                                                }
                                            }
                                        }
                                        
                                        // 处理表格请求焦点（点击表格时）
                                        if grid_actions.request_focus && self.focus_area != ui::FocusArea::DataGrid {
                                            self.focus_area = ui::FocusArea::DataGrid;
                                            self.grid_state.focused = true;
                                        }
                                        
                                        // 处理打开筛选面板请求
                                        if grid_actions.open_filter_panel {
                                            self.show_sidebar = true;
                                            self.sidebar_panel_state.show_filters = true;
                                            self.sidebar_section = ui::SidebarSection::Filters;
                                            self.focus_area = ui::FocusArea::Sidebar;
                                        }
                                        
                                        // 处理切换Tab请求 (数字+Enter)
                                        if let Some(tab_idx) = grid_actions.switch_to_tab {
                                            if tab_idx < self.tab_manager.tabs.len() {
                                                self.tab_manager.set_active(tab_idx);
                                            }
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
                                    }
                                ); // allocate_ui_with_layout 数据表格区域结束

                                // ===== SQL 编辑器 =====
                                sql_editor_actions = self.render_sql_editor_in_ui(ui, total_content_height);
                            }); // Frame 闭包结束
                    }
                ); // allocate_ui_with_layout 主内容区结束
            }); // horizontal 布局结束
        }); // CentralPanel 闭包结束
        
        // ===== 处理各种操作 =====
        self.handle_toolbar_actions(ctx, toolbar_actions);
        self.handle_sidebar_actions(sidebar_actions);
        self.handle_sql_editor_actions(sql_editor_actions);

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
