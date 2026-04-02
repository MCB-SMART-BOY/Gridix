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
mod help;
mod import;
mod input_router;
mod keyboard;
mod message;
mod metadata;
mod preferences;
mod render;
mod request_lifecycle;
pub mod state;
mod welcome;

use eframe::egui;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::core::{
    AppConfig, AutoComplete, HighlightColors, KeyBindings, NotificationManager, ProgressManager,
    QueryHistory, ThemeManager, constants,
};
use crate::database::{ConnectionConfig, ConnectionManager, DatabaseType, QueryResult};
use crate::ui::{self, DdlDialogState, ExportConfig, KeyBindingsDialogState, QueryTabManager};

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
    /// 连接对话框是否展开高级配置
    connection_dialog_show_advanced: bool,
    /// 当前编辑的连接配置（用于新建/编辑对话框）
    new_config: ConnectionConfig,
    /// 当前正在编辑的连接名（None 表示新建模式）
    editing_connection_name: Option<String>,

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
    /// 是否正在执行导入
    import_executing: bool,
    /// 连接请求自增序列（用于丢弃过期连接/选库回包）
    next_connect_request_id: u64,
    /// 查询请求自增序列（用于丢弃过期回包）
    next_query_request_id: u64,
    /// 元数据请求自增序列（触发器/存储过程）
    next_metadata_request_id: u64,
    /// 各连接最新连接请求 ID
    pending_connect_requests: HashMap<String, u64>,
    /// 各连接最新数据库切换请求 (database, request_id)
    pending_database_requests: HashMap<String, (String, u64)>,
    /// 触发器请求上下文 (连接名, 数据库名, 请求ID)
    pending_triggers_request: Option<(String, Option<String>, u64)>,
    /// 存储过程请求上下文 (连接名, 数据库名, 请求ID)
    pending_routines_request: Option<(String, Option<String>, u64)>,
    /// 进行中的查询任务句柄
    pending_query_tasks: HashMap<u64, tokio::task::JoinHandle<()>>,
    /// 查询请求与连接映射（用于按连接取消）
    pending_query_connections: HashMap<u64, String>,
    /// 查询取消信号发送器
    pending_query_cancellers: HashMap<u64, Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>>,

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
    /// 待删除目标（连接名或 `table:<表名>`）
    pending_delete_name: Option<String>,
    /// 待处理的表删除（request_id -> (连接名, 表名)）
    pending_drop_requests: HashMap<u64, (String, String)>,
    /// 侧边栏请求聚焦的筛选输入框索引
    pending_filter_input_focus: Option<usize>,

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
    /// 欢迎页数据库环境检测状态
    welcome_status: ui::WelcomeStatusSummary,
    /// 是否显示欢迎页安装/初始化引导
    show_welcome_setup_dialog: bool,
    /// 当前引导目标数据库
    welcome_setup_target: DatabaseType,
    /// 是否显示帮助面板
    show_help: bool,
    /// 帮助面板滚动位置
    help_scroll_offset: f32,
    /// 帮助面板分类与学习主题状态
    help_state: ui::HelpState,
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
        let keybindings = KeyBindings::load_or_init(&app_config.keybindings);
        let theme_manager = ThemeManager::new(app_config.theme_preset);
        let highlight_colors = HighlightColors::from_theme(&theme_manager.colors);
        let query_history = QueryHistory::new(100);

        // 应用主题
        theme_manager.apply(&cc.egui_ctx);

        // 获取基础 DPI 缩放并应用用户缩放设置
        let base_pixels_per_point = cc.egui_ctx.pixels_per_point();
        let ui_scale = app_config
            .ui_scale
            .clamp(constants::ui::UI_SCALE_MIN, constants::ui::UI_SCALE_MAX);
        cc.egui_ctx
            .set_pixels_per_point(base_pixels_per_point * ui_scale);

        // 从配置恢复连接
        let mut manager = ConnectionManager::default();
        for config in &app_config.connections {
            manager.add(config.clone());
        }

        let mut app = Self {
            manager,
            show_connection_dialog: false,
            connection_dialog_show_advanced: app_config.connection_dialog_show_advanced,
            new_config: ConnectionConfig::default(),
            editing_connection_name: None,
            selected_table: None,
            sql: String::new(),
            result: None,
            tab_manager: QueryTabManager::new(),
            tx,
            rx,
            runtime,
            connecting: false,
            executing: false,
            import_executing: false,
            next_connect_request_id: 0,
            next_query_request_id: 0,
            next_metadata_request_id: 0,
            pending_connect_requests: HashMap::new(),
            pending_database_requests: HashMap::new(),
            pending_triggers_request: None,
            pending_routines_request: None,
            pending_query_tasks: HashMap::new(),
            pending_query_connections: HashMap::new(),
            pending_query_cancellers: HashMap::new(),
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
            pending_drop_requests: HashMap::new(),
            pending_filter_input_focus: None,
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
            sidebar_width: 280.0, // 默认侧边栏宽度
            welcome_status: ui::WelcomeStatusSummary::default(),
            show_welcome_setup_dialog: false,
            welcome_setup_target: DatabaseType::SQLite,
            show_help: false,
            help_scroll_offset: 0.0,
            help_state: ui::HelpState::default(),
            show_about: false,
            ui_scale,
            base_pixels_per_point,
            ddl_dialog_state: DdlDialogState::default(),
            create_db_dialog_state: ui::CreateDbDialogState::new(),
            create_user_dialog_state: ui::CreateUserDialogState::new(),
            keybindings,
            keybindings_dialog_state: KeyBindingsDialogState::default(),
            central_panel_ratio: 0.65,
            show_er_diagram: false,
            er_diagram_state: ui::ERDiagramState::new(),
            sql_editor_height: 200.0, // 默认 SQL 编辑器高度
            pending_toggle_dark_mode: false,
        };
        app.refresh_welcome_environment_status();
        app
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.run_frame(ui);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.save_config();
        for (_, handle) in self.pending_query_tasks.drain() {
            handle.abort();
        }
        self.pending_query_connections.clear();
        self.pending_query_cancellers.clear();

        // 清理连接池，确保所有数据库连接正确关闭
        self.runtime.block_on(async {
            crate::database::ssh_tunnel::SSH_TUNNEL_MANAGER
                .stop_all()
                .await;
            crate::database::POOL_MANAGER.clear_all().await;
        });
    }
}
