//! 应用动作语义层
//!
//! 将快捷键、命令面板、按钮入口统一到同一套动作定义和执行边界上。

use eframe::egui;

use crate::core::{Action as ShortcutAction, constants};
use crate::database::DatabaseType;
use crate::ui::{self, FocusArea};

use super::DbManagerApp;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::app) enum AppAction {
    OpenCommandPalette,
    OpenConnectionDialog,
    OpenConnectionDialogFor(DatabaseType),
    OpenExportDialog,
    OpenImportDialog,
    OpenHelpPanel,
    ToggleHelpPanel,
    OpenHistoryPanel,
    ToggleHistoryPanel,
    OpenAboutDialog,
    OpenKeybindingsDialog,
    ToggleSidebar,
    ToggleSqlEditor,
    ToggleErDiagram,
    ToggleDarkMode,
    RefreshActiveConnection,
    RefreshSelectedTable,
    NewTable,
    NewDatabase,
    NewUser,
    NewQueryTab,
    SwitchToQueryTab(usize),
    NextQueryTab,
    PrevQueryTab,
    CloseActiveQueryTab,
    RunCurrentSql,
    ClearCommandLine,
    ClearSearch,
    AddFilter,
    ClearFilters,
    OpenFilterWorkspace,
    SaveGridChanges,
    GotoLine,
    FocusSidebar,
    FocusGrid,
    FocusEditor,
    FocusToolbar,
    FocusQueryTabs,
    QuerySelectedTable,
    ShowSelectedTableSchema,
    RecheckEnvironment,
    OpenLearningSample,
    EnsureLearningSample { reset: bool, notify: bool },
    ConfirmPendingDelete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) enum CommandScope {
    Global,
    Connection,
    Sidebar,
    Grid,
    Editor,
    Tabs,
    Onboarding,
    Appearance,
}

impl CommandScope {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Global => "全局",
            Self::Connection => "连接",
            Self::Sidebar => "侧边栏",
            Self::Grid => "表格",
            Self::Editor => "编辑器",
            Self::Tabs => "标签页",
            Self::Onboarding => "引导",
            Self::Appearance => "外观",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(in crate::app) struct CommandDescriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub category: &'static str,
    pub scope: CommandScope,
    pub action: AppAction,
    pub shortcut: Option<ShortcutAction>,
    pub keywords: &'static [&'static str],
}

impl CommandDescriptor {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        id: &'static str,
        title: &'static str,
        subtitle: &'static str,
        category: &'static str,
        scope: CommandScope,
        action: AppAction,
        shortcut: Option<ShortcutAction>,
        keywords: &'static [&'static str],
    ) -> Self {
        Self {
            id,
            title,
            subtitle,
            category,
            scope,
            action,
            shortcut,
            keywords,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::app) struct ActionAvailability {
    pub enabled: bool,
    pub reason: Option<&'static str>,
}

impl ActionAvailability {
    pub const fn enabled() -> Self {
        Self {
            enabled: true,
            reason: None,
        }
    }

    pub const fn disabled(reason: &'static str) -> Self {
        Self {
            enabled: false,
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::app) struct ActionContext {
    pub has_any_connection: bool,
    pub has_active_connection: bool,
    pub active_db_type: Option<DatabaseType>,
    pub has_result: bool,
    pub result_has_rows: bool,
    pub selected_table: Option<String>,
    pub has_sql: bool,
    pub has_search_text: bool,
    pub grid_has_changes: bool,
    pub query_tab_count: usize,
    pub focus_area: FocusArea,
    pub show_sidebar: bool,
    pub show_sql_editor: bool,
    pub show_er_diagram: bool,
    pub can_confirm_pending_delete: bool,
}

impl ActionContext {
    fn from_app(app: &DbManagerApp) -> Self {
        Self {
            has_any_connection: !app.manager.connections.is_empty(),
            has_active_connection: app.manager.active.is_some(),
            active_db_type: app.manager.get_active().map(|conn| conn.config.db_type),
            has_result: app.result.is_some(),
            result_has_rows: app
                .result
                .as_ref()
                .is_some_and(|result| !result.rows.is_empty()),
            selected_table: app.selected_table.clone(),
            has_sql: !app.sql.trim().is_empty(),
            has_search_text: !app.search_text.trim().is_empty(),
            grid_has_changes: app.grid_state.has_changes(),
            query_tab_count: app.tab_manager.tabs.len(),
            focus_area: app.focus_area,
            show_sidebar: app.show_sidebar,
            show_sql_editor: app.show_sql_editor,
            show_er_diagram: app.show_er_diagram,
            can_confirm_pending_delete: app.show_delete_confirm
                && app.pending_delete_target.is_some(),
        }
    }

    fn scope_bonus(&self, scope: CommandScope) -> usize {
        match (scope, self.focus_area) {
            (CommandScope::Sidebar, FocusArea::Sidebar) => 18,
            (CommandScope::Grid, FocusArea::DataGrid) => 18,
            (CommandScope::Editor, FocusArea::SqlEditor) => 18,
            (CommandScope::Tabs, FocusArea::QueryTabs) => 18,
            (CommandScope::Global, FocusArea::Toolbar) => 6,
            (CommandScope::Appearance, FocusArea::Toolbar) => 10,
            _ => 0,
        }
    }

    pub(in crate::app) fn status_line(&self) -> String {
        let connection = if self.has_active_connection {
            "已连接"
        } else if self.has_any_connection {
            "未激活连接"
        } else {
            "无连接"
        };
        let table = self.selected_table.as_deref().unwrap_or("无表");
        let focus = match self.focus_area {
            FocusArea::Toolbar => "工具栏",
            FocusArea::QueryTabs => "标签页",
            FocusArea::Sidebar => "侧边栏",
            FocusArea::DataGrid => "表格",
            FocusArea::SqlEditor => "编辑器",
            FocusArea::Dialog => "对话框",
        };
        let sidebar = if self.show_sidebar {
            "侧边栏开"
        } else {
            "侧边栏关"
        };
        let editor = if self.show_sql_editor {
            "编辑器开"
        } else {
            "编辑器关"
        };
        let er_diagram = if self.show_er_diagram {
            "ER 开"
        } else {
            "ER 关"
        };
        format!(
            "{} · 焦点 {} · 当前表 {} · {} / {} / {}",
            connection, focus, table, sidebar, editor, er_diagram
        )
    }
}

#[derive(Debug, Clone)]
pub(in crate::app) struct CommandMatch {
    pub descriptor: &'static CommandDescriptor,
    pub availability: ActionAvailability,
    pub score: usize,
}

#[derive(Debug, Clone)]
pub(in crate::app) enum AppEffect {
    Connect(String),
    ExecuteSql(String),
    FetchPrimaryKey(String),
    RefreshWelcomeEnvironment,
    EnsureLearningSample {
        reset: bool,
        notify: bool,
        mark_onboarding: bool,
        show_setup: bool,
    },
}

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor::new(
        "open_command_palette",
        "打开命令面板",
        "从当前上下文搜索并执行 Gridix 动作。",
        "全局",
        CommandScope::Global,
        AppAction::OpenCommandPalette,
        Some(ShortcutAction::CommandPalette),
        &["command", "palette", "action", "run command"],
    ),
    CommandDescriptor::new(
        "new_connection",
        "新建连接",
        "打开连接对话框，开始新的数据库会话。",
        "连接",
        CommandScope::Connection,
        AppAction::OpenConnectionDialog,
        Some(ShortcutAction::NewConnection),
        &["connect", "connection", "database", "db", "open connection"],
    ),
    CommandDescriptor::new(
        "open_export",
        "导出当前结果",
        "把当前结果集导出到 CSV / TSV / SQL / JSON。",
        "传输",
        CommandScope::Grid,
        AppAction::OpenExportDialog,
        Some(ShortcutAction::Export),
        &["export", "download", "transfer", "csv", "json", "sql"],
    ),
    CommandDescriptor::new(
        "open_import",
        "导入数据文件",
        "打开统一导入流程，预览并执行导入。",
        "传输",
        CommandScope::Connection,
        AppAction::OpenImportDialog,
        Some(ShortcutAction::Import),
        &["import", "upload", "transfer", "csv", "json", "sql", "tsv"],
    ),
    CommandDescriptor::new(
        "refresh_connection",
        "刷新当前连接",
        "重新加载当前连接的数据库或表列表。",
        "连接",
        CommandScope::Connection,
        AppAction::RefreshActiveConnection,
        Some(ShortcutAction::Refresh),
        &["refresh", "reload", "reconnect", "tables", "databases"],
    ),
    CommandDescriptor::new(
        "refresh_selected_table",
        "刷新当前表",
        "重新查询当前选中的表数据，保持当前工作区语义。",
        "查询",
        CommandScope::Grid,
        AppAction::RefreshSelectedTable,
        None,
        &[
            "refresh table",
            "reload table",
            "current table",
            "query table",
        ],
    ),
    CommandDescriptor::new(
        "toggle_sidebar",
        "切换侧边栏",
        "显示或隐藏 Connections / Filters 等侧边栏工作区。",
        "布局",
        CommandScope::Sidebar,
        AppAction::ToggleSidebar,
        Some(ShortcutAction::ToggleSidebar),
        &[
            "sidebar",
            "panel",
            "toggle sidebar",
            "connections",
            "filters",
        ],
    ),
    CommandDescriptor::new(
        "toggle_editor",
        "切换 SQL 编辑器",
        "显示或隐藏底部 SQL 编辑器工作区。",
        "布局",
        CommandScope::Editor,
        AppAction::ToggleSqlEditor,
        Some(ShortcutAction::ToggleEditor),
        &["editor", "sql editor", "query editor", "toggle editor"],
    ),
    CommandDescriptor::new(
        "toggle_er_diagram",
        "切换 ER 图",
        "显示或隐藏当前连接的 ER 关系图。",
        "布局",
        CommandScope::Global,
        AppAction::ToggleErDiagram,
        Some(ShortcutAction::ToggleErDiagram),
        &["er", "diagram", "relationship", "graph"],
    ),
    CommandDescriptor::new(
        "run_current_sql",
        "执行当前 SQL",
        "执行当前编辑器里的 SQL 内容。",
        "查询",
        CommandScope::Editor,
        AppAction::RunCurrentSql,
        None,
        &["run", "execute", "query", "sql", "current sql"],
    ),
    CommandDescriptor::new(
        "query_selected_table",
        "查询当前表",
        "对当前选中的表执行 SELECT * LIMIT 结果预览。",
        "查询",
        CommandScope::Sidebar,
        AppAction::QuerySelectedTable,
        None,
        &[
            "query table",
            "open table",
            "browse table",
            "select current table",
        ],
    ),
    CommandDescriptor::new(
        "show_table_schema",
        "查看当前表结构",
        "显示当前选中表的列定义和类型信息。",
        "查询",
        CommandScope::Sidebar,
        AppAction::ShowSelectedTableSchema,
        None,
        &["schema", "describe", "table info", "ddl", "columns"],
    ),
    CommandDescriptor::new(
        "add_filter",
        "添加筛选条件",
        "向 Filters 工作区添加一条新的筛选规则。",
        "筛选",
        CommandScope::Sidebar,
        AppAction::AddFilter,
        None,
        &["filter", "add filter", "where", "condition"],
    ),
    CommandDescriptor::new(
        "open_filter_workspace",
        "打开筛选工作区",
        "打开侧边栏 Filters 工作区，继续用键盘编辑筛选条件。",
        "筛选",
        CommandScope::Sidebar,
        AppAction::OpenFilterWorkspace,
        None,
        &["filter", "filters workspace", "open filters", "where"],
    ),
    CommandDescriptor::new(
        "clear_filters",
        "清空筛选条件",
        "移除当前结果集上的所有筛选规则。",
        "筛选",
        CommandScope::Sidebar,
        AppAction::ClearFilters,
        None,
        &["clear filters", "reset filters", "filter"],
    ),
    CommandDescriptor::new(
        "save_grid_changes",
        "保存表格修改",
        "把 DataGrid 中尚未保存的编辑写回数据库。",
        "表格",
        CommandScope::Grid,
        AppAction::SaveGridChanges,
        Some(ShortcutAction::Save),
        &["save", "commit", "grid changes", "write back"],
    ),
    CommandDescriptor::new(
        "goto_grid_row",
        "跳转到行",
        "在结果表格中快速跳转到指定行号。",
        "表格",
        CommandScope::Grid,
        AppAction::GotoLine,
        Some(ShortcutAction::GotoLine),
        &["goto", "line", "row", "jump"],
    ),
    CommandDescriptor::new(
        "new_query_tab",
        "新建查询标签页",
        "创建一个新的 SQL 工作标签页。",
        "标签页",
        CommandScope::Tabs,
        AppAction::NewQueryTab,
        Some(ShortcutAction::NewTab),
        &["new tab", "query tab", "tab"],
    ),
    CommandDescriptor::new(
        "next_query_tab",
        "下一个查询标签页",
        "切换到下一个查询标签页。",
        "标签页",
        CommandScope::Tabs,
        AppAction::NextQueryTab,
        Some(ShortcutAction::NextTab),
        &["next tab", "tab", "cycle tab"],
    ),
    CommandDescriptor::new(
        "prev_query_tab",
        "上一个查询标签页",
        "切换到上一个查询标签页。",
        "标签页",
        CommandScope::Tabs,
        AppAction::PrevQueryTab,
        Some(ShortcutAction::PrevTab),
        &["previous tab", "prev tab", "tab"],
    ),
    CommandDescriptor::new(
        "close_query_tab",
        "关闭当前标签页",
        "关闭当前查询标签页并清理挂起请求。",
        "标签页",
        CommandScope::Tabs,
        AppAction::CloseActiveQueryTab,
        Some(ShortcutAction::CloseTab),
        &["close tab", "tab", "remove tab"],
    ),
    CommandDescriptor::new(
        "focus_sidebar",
        "聚焦侧边栏",
        "切换焦点到侧边栏工作区。",
        "焦点",
        CommandScope::Sidebar,
        AppAction::FocusSidebar,
        None,
        &["focus sidebar", "sidebar", "connections", "filters"],
    ),
    CommandDescriptor::new(
        "focus_grid",
        "聚焦结果表格",
        "切换焦点到 DataGrid 结果工作区。",
        "焦点",
        CommandScope::Grid,
        AppAction::FocusGrid,
        None,
        &["focus grid", "grid", "results", "table"],
    ),
    CommandDescriptor::new(
        "focus_editor",
        "聚焦 SQL 编辑器",
        "显示并聚焦 SQL 编辑器。",
        "焦点",
        CommandScope::Editor,
        AppAction::FocusEditor,
        None,
        &["focus editor", "editor", "sql"],
    ),
    CommandDescriptor::new(
        "focus_toolbar",
        "聚焦工具栏",
        "切换焦点到顶部工具栏。",
        "焦点",
        CommandScope::Global,
        AppAction::FocusToolbar,
        None,
        &["focus toolbar", "toolbar", "top bar"],
    ),
    CommandDescriptor::new(
        "focus_query_tabs",
        "聚焦标签栏",
        "切换焦点到查询标签栏。",
        "焦点",
        CommandScope::Tabs,
        AppAction::FocusQueryTabs,
        None,
        &["focus tabs", "query tabs", "tab bar"],
    ),
    CommandDescriptor::new(
        "open_history",
        "打开历史记录",
        "显示查询历史面板。",
        "面板",
        CommandScope::Global,
        AppAction::OpenHistoryPanel,
        Some(ShortcutAction::ShowHistory),
        &["history", "query history", "recent queries"],
    ),
    CommandDescriptor::new(
        "open_help",
        "打开帮助",
        "显示内置帮助和学习指南。",
        "面板",
        CommandScope::Global,
        AppAction::OpenHelpPanel,
        Some(ShortcutAction::ShowHelp),
        &["help", "docs", "guide", "manual"],
    ),
    CommandDescriptor::new(
        "open_keybindings",
        "打开快捷键设置",
        "查看和编辑当前 keymap 绑定。",
        "设置",
        CommandScope::Global,
        AppAction::OpenKeybindingsDialog,
        Some(ShortcutAction::OpenKeybindingsDialog),
        &["keybindings", "shortcuts", "keymap", "bindings"],
    ),
    CommandDescriptor::new(
        "open_about",
        "打开关于窗口",
        "查看版本、项目说明和版权信息。",
        "设置",
        CommandScope::Global,
        AppAction::OpenAboutDialog,
        None,
        &["about", "version", "info"],
    ),
    CommandDescriptor::new(
        "new_table",
        "新建表",
        "打开 DDL 工作流，创建新表。",
        "创建",
        CommandScope::Connection,
        AppAction::NewTable,
        Some(ShortcutAction::NewTable),
        &["new table", "create table", "ddl"],
    ),
    CommandDescriptor::new(
        "new_database",
        "新建数据库",
        "打开创建数据库工作流。",
        "创建",
        CommandScope::Connection,
        AppAction::NewDatabase,
        Some(ShortcutAction::NewDatabase),
        &["new database", "create database", "database"],
    ),
    CommandDescriptor::new(
        "new_user",
        "新建用户",
        "打开创建数据库用户工作流。",
        "创建",
        CommandScope::Connection,
        AppAction::NewUser,
        Some(ShortcutAction::NewUser),
        &["new user", "create user", "grant", "role"],
    ),
    CommandDescriptor::new(
        "clear_command_line",
        "清空命令行",
        "清空当前 SQL 编辑器内容并收起通知。",
        "编辑",
        CommandScope::Editor,
        AppAction::ClearCommandLine,
        Some(ShortcutAction::ClearCommandLine),
        &["clear command", "clear sql", "editor"],
    ),
    CommandDescriptor::new(
        "clear_search",
        "清空搜索",
        "清空当前结果搜索关键字。",
        "编辑",
        CommandScope::Grid,
        AppAction::ClearSearch,
        Some(ShortcutAction::ClearSearch),
        &["clear search", "search", "filter search"],
    ),
    CommandDescriptor::new(
        "toggle_dark_mode",
        "切换明暗主题",
        "在浅色和深色主题之间切换。",
        "外观",
        CommandScope::Appearance,
        AppAction::ToggleDarkMode,
        Some(ShortcutAction::ToggleDarkMode),
        &["theme", "dark mode", "light mode", "appearance"],
    ),
    CommandDescriptor::new(
        "recheck_environment",
        "重新检测数据库环境",
        "重新检测本机 PostgreSQL / MySQL 可用状态。",
        "引导",
        CommandScope::Onboarding,
        AppAction::RecheckEnvironment,
        None,
        &["environment", "setup", "check", "postgres", "mysql"],
    ),
    CommandDescriptor::new(
        "open_learning_sample",
        "打开学习示例库",
        "创建或打开内置学习数据库并进入引导流程。",
        "引导",
        CommandScope::Onboarding,
        AppAction::OpenLearningSample,
        None,
        &["learning", "sample", "demo", "onboarding", "sqlite"],
    ),
];

impl AppAction {
    pub(in crate::app) fn from_shortcut_action(action: ShortcutAction) -> Option<Self> {
        Some(match action {
            ShortcutAction::NextFocusArea | ShortcutAction::PrevFocusArea => {
                return None;
            }
            ShortcutAction::CommandPalette => Self::OpenCommandPalette,
            ShortcutAction::NewConnection => Self::OpenConnectionDialog,
            ShortcutAction::OpenKeybindingsDialog => Self::OpenKeybindingsDialog,
            ShortcutAction::ToggleSidebar => Self::ToggleSidebar,
            ShortcutAction::ToggleDarkMode => Self::ToggleDarkMode,
            ShortcutAction::ToggleEditor => Self::ToggleSqlEditor,
            ShortcutAction::ToggleErDiagram => Self::ToggleErDiagram,
            ShortcutAction::ShowHistory => Self::ToggleHistoryPanel,
            ShortcutAction::Export => Self::OpenExportDialog,
            ShortcutAction::Import => Self::OpenImportDialog,
            ShortcutAction::Refresh => Self::RefreshActiveConnection,
            ShortcutAction::ClearCommandLine => Self::ClearCommandLine,
            ShortcutAction::ClearSearch => Self::ClearSearch,
            ShortcutAction::NewTable => Self::NewTable,
            ShortcutAction::NewDatabase => Self::NewDatabase,
            ShortcutAction::NewUser => Self::NewUser,
            ShortcutAction::NewTab => Self::NewQueryTab,
            ShortcutAction::CloseTab => Self::CloseActiveQueryTab,
            ShortcutAction::NextTab => Self::NextQueryTab,
            ShortcutAction::PrevTab => Self::PrevQueryTab,
            ShortcutAction::Save => Self::SaveGridChanges,
            ShortcutAction::GotoLine => Self::GotoLine,
            ShortcutAction::ShowHelp => Self::ToggleHelpPanel,
            ShortcutAction::OpenThemeSelector
            | ShortcutAction::FocusSidebarConnections
            | ShortcutAction::FocusSidebarDatabases
            | ShortcutAction::FocusSidebarTables
            | ShortcutAction::FocusSidebarFilters
            | ShortcutAction::FocusSidebarTriggers
            | ShortcutAction::FocusSidebarRoutines
            | ShortcutAction::ZoomIn
            | ShortcutAction::ZoomOut
            | ShortcutAction::ZoomReset => {
                return None;
            }
        })
    }
}

pub(in crate::app) fn command_descriptors() -> &'static [CommandDescriptor] {
    COMMANDS
}

pub(in crate::app) fn search_commands(context: &ActionContext, query: &str) -> Vec<CommandMatch> {
    let mut matches: Vec<CommandMatch> = command_descriptors()
        .iter()
        .filter_map(|descriptor| {
            let score = command_match_score(descriptor, context, query)?;
            Some(CommandMatch {
                descriptor,
                availability: availability_for_action(context, descriptor.action),
                score,
            })
        })
        .collect();

    matches.sort_by(|left, right| {
        right
            .availability
            .enabled
            .cmp(&left.availability.enabled)
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| left.descriptor.title.cmp(right.descriptor.title))
    });
    matches
}

fn command_match_score(
    descriptor: &CommandDescriptor,
    context: &ActionContext,
    query: &str,
) -> Option<usize> {
    let query = query.trim().to_ascii_lowercase();
    let mut score = context.scope_bonus(descriptor.scope);

    if query.is_empty() {
        return Some(score);
    }

    let title = descriptor.title.to_ascii_lowercase();
    let subtitle = descriptor.subtitle.to_ascii_lowercase();
    let id = descriptor.id.to_ascii_lowercase();
    let category = descriptor.category.to_ascii_lowercase();
    let keywords: Vec<String> = descriptor
        .keywords
        .iter()
        .map(|keyword| keyword.to_ascii_lowercase())
        .collect();

    for token in query.split_whitespace() {
        let token_score = if title == token {
            220
        } else if title.starts_with(token) {
            160
        } else if id.starts_with(token) {
            150
        } else if keywords.iter().any(|keyword| keyword == token) {
            140
        } else if title.contains(token) {
            120
        } else if keywords.iter().any(|keyword| keyword.starts_with(token)) {
            100
        } else if subtitle.contains(token) {
            80
        } else if category.contains(token) || id.contains(token) {
            60
        } else if keywords.iter().any(|keyword| keyword.contains(token)) {
            50
        } else {
            return None;
        };
        score += token_score;
    }

    Some(score)
}

fn availability_for_action(context: &ActionContext, action: AppAction) -> ActionAvailability {
    match action {
        AppAction::OpenCommandPalette
        | AppAction::OpenConnectionDialog
        | AppAction::OpenConnectionDialogFor(_)
        | AppAction::OpenImportDialog
        | AppAction::OpenHelpPanel
        | AppAction::ToggleHelpPanel
        | AppAction::OpenHistoryPanel
        | AppAction::ToggleHistoryPanel
        | AppAction::OpenAboutDialog
        | AppAction::OpenKeybindingsDialog
        | AppAction::ToggleSidebar
        | AppAction::ToggleSqlEditor
        | AppAction::ToggleErDiagram
        | AppAction::ToggleDarkMode
        | AppAction::NewTable
        | AppAction::NewDatabase
        | AppAction::NewQueryTab
        | AppAction::AddFilter
        | AppAction::ClearFilters
        | AppAction::FocusSidebar
        | AppAction::FocusGrid
        | AppAction::FocusEditor
        | AppAction::FocusToolbar
        | AppAction::FocusQueryTabs
        | AppAction::RecheckEnvironment
        | AppAction::OpenLearningSample
        | AppAction::EnsureLearningSample { .. } => ActionAvailability::enabled(),
        AppAction::ConfirmPendingDelete => {
            if context.can_confirm_pending_delete {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前没有待确认的删除操作")
            }
        }
        AppAction::OpenExportDialog => {
            if context.has_result {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前没有可导出的结果集")
            }
        }
        AppAction::RefreshActiveConnection => {
            if context.has_active_connection {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("请先连接数据库")
            }
        }
        AppAction::RefreshSelectedTable => {
            if context.has_active_connection && context.selected_table.is_some() {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("请先选中一张表")
            }
        }
        AppAction::NewUser => {
            if !context.has_active_connection {
                ActionAvailability::disabled("请先连接支持用户管理的数据库")
            } else if matches!(context.active_db_type, Some(DatabaseType::SQLite)) {
                ActionAvailability::disabled("SQLite 不支持数据库用户管理")
            } else {
                ActionAvailability::enabled()
            }
        }
        AppAction::SwitchToQueryTab(index) => {
            if index < context.query_tab_count {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("目标查询标签页不存在")
            }
        }
        AppAction::NextQueryTab | AppAction::PrevQueryTab | AppAction::CloseActiveQueryTab => {
            if context.query_tab_count > 1 {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前只有一个查询标签页")
            }
        }
        AppAction::RunCurrentSql => {
            if context.has_sql {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("SQL 编辑器里还没有可执行内容")
            }
        }
        AppAction::ClearCommandLine => {
            if context.has_sql {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前 SQL 编辑器已经是空的")
            }
        }
        AppAction::ClearSearch => {
            if context.has_search_text {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前没有可清空的搜索关键字")
            }
        }
        AppAction::OpenFilterWorkspace => {
            if context.has_result {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前没有可筛选的结果集")
            }
        }
        AppAction::SaveGridChanges => {
            if context.grid_has_changes {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前表格没有待保存的修改")
            }
        }
        AppAction::GotoLine => {
            if context.result_has_rows {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("当前结果表格没有可跳转的行")
            }
        }
        AppAction::QuerySelectedTable | AppAction::ShowSelectedTableSchema => {
            if context.has_active_connection && context.selected_table.is_some() {
                ActionAvailability::enabled()
            } else {
                ActionAvailability::disabled("请先在侧边栏选中一张表")
            }
        }
    }
}

impl DbManagerApp {
    pub(in crate::app) fn action_context(&self) -> ActionContext {
        ActionContext::from_app(self)
    }

    pub(in crate::app) fn action_availability(&self, action: AppAction) -> ActionAvailability {
        availability_for_action(&self.action_context(), action)
    }

    pub(in crate::app) fn shortcut_label_for_action(&self, action: AppAction) -> Option<String> {
        command_descriptors()
            .iter()
            .find(|descriptor| descriptor.action == action)
            .and_then(|descriptor| descriptor.shortcut)
            .and_then(|shortcut| self.keybindings.get(shortcut))
            .map(|binding| binding.display())
    }

    pub(in crate::app) fn dispatch_app_action(&mut self, ctx: &egui::Context, action: AppAction) {
        let availability = self.action_availability(action);
        if !availability.enabled {
            if let Some(reason) = availability.reason {
                self.notifications.warning(reason);
            }
            return;
        }

        let effects = self.reduce_app_action(ctx, action);
        self.apply_app_effects(effects);
    }

    fn reduce_app_action(&mut self, ctx: &egui::Context, action: AppAction) -> Vec<AppEffect> {
        match action {
            AppAction::OpenCommandPalette => {
                self.command_palette_state.open();
                Vec::new()
            }
            AppAction::OpenConnectionDialog => {
                self.new_config = crate::database::ConnectionConfig::default();
                self.editing_connection_name = None;
                self.show_connection_dialog = true;
                Vec::new()
            }
            AppAction::OpenConnectionDialogFor(db_type) => {
                self.welcome_setup_target = db_type;
                self.open_connection_dialog_for(db_type);
                Vec::new()
            }
            AppAction::OpenExportDialog => {
                self.open_export_dialog();
                Vec::new()
            }
            AppAction::OpenImportDialog => {
                self.open_import_dialog();
                Vec::new()
            }
            AppAction::OpenHelpPanel => {
                self.show_help = true;
                Vec::new()
            }
            AppAction::ToggleHelpPanel => {
                self.show_help = !self.show_help;
                Vec::new()
            }
            AppAction::OpenHistoryPanel => {
                self.set_history_panel_visible(true);
                Vec::new()
            }
            AppAction::ToggleHistoryPanel => {
                self.toggle_history_panel();
                Vec::new()
            }
            AppAction::OpenAboutDialog => {
                self.show_about = true;
                Vec::new()
            }
            AppAction::OpenKeybindingsDialog => {
                self.keybindings_dialog_state
                    .open_with_legacy(&self.keybindings, &self.app_config.keybindings);
                Vec::new()
            }
            AppAction::ToggleSidebar => {
                self.toggle_sidebar_visibility();
                Vec::new()
            }
            AppAction::ToggleSqlEditor => {
                self.toggle_sql_editor_visibility();
                Vec::new()
            }
            AppAction::ToggleErDiagram => {
                self.toggle_er_diagram_visibility();
                Vec::new()
            }
            AppAction::ToggleDarkMode => {
                self.app_config.is_dark_mode = !self.app_config.is_dark_mode;
                let new_theme = if self.app_config.is_dark_mode {
                    self.app_config.dark_theme
                } else {
                    self.app_config.light_theme
                };
                self.set_theme(ctx, new_theme);
                Vec::new()
            }
            AppAction::RefreshActiveConnection => self
                .manager
                .active
                .clone()
                .map(AppEffect::Connect)
                .into_iter()
                .collect(),
            AppAction::RefreshSelectedTable => {
                self.selected_table_query_effects(false, false, false)
            }
            AppAction::NewTable => {
                self.open_create_table_dialog();
                Vec::new()
            }
            AppAction::NewDatabase => {
                self.open_create_database_dialog();
                Vec::new()
            }
            AppAction::NewUser => {
                self.open_create_user_dialog();
                Vec::new()
            }
            AppAction::NewQueryTab => {
                self.open_new_query_tab();
                Vec::new()
            }
            AppAction::SwitchToQueryTab(index) => {
                self.switch_to_query_tab(index);
                Vec::new()
            }
            AppAction::NextQueryTab => {
                self.select_next_query_tab();
                Vec::new()
            }
            AppAction::PrevQueryTab => {
                self.select_previous_query_tab();
                Vec::new()
            }
            AppAction::CloseActiveQueryTab => {
                self.close_active_query_tab();
                Vec::new()
            }
            AppAction::RunCurrentSql => vec![AppEffect::ExecuteSql(self.sql.clone())],
            AppAction::ClearCommandLine => {
                self.sql.clear();
                self.notifications.dismiss_all();
                Vec::new()
            }
            AppAction::ClearSearch => {
                self.search_text.clear();
                Vec::new()
            }
            AppAction::AddFilter => {
                self.add_sidebar_filter();
                Vec::new()
            }
            AppAction::ClearFilters => {
                self.clear_sidebar_filters();
                Vec::new()
            }
            AppAction::OpenFilterWorkspace => {
                self.open_filter_workspace();
                Vec::new()
            }
            AppAction::SaveGridChanges => {
                self.grid_state.pending_save = true;
                Vec::new()
            }
            AppAction::GotoLine => {
                self.grid_state.show_goto_dialog = true;
                Vec::new()
            }
            AppAction::FocusSidebar => {
                self.ensure_sidebar_workspace_visible();
                self.set_focus_area(FocusArea::Sidebar);
                Vec::new()
            }
            AppAction::FocusGrid => {
                self.set_focus_area(FocusArea::DataGrid);
                Vec::new()
            }
            AppAction::FocusEditor => {
                self.show_sql_editor = true;
                self.set_focus_area(FocusArea::SqlEditor);
                Vec::new()
            }
            AppAction::FocusToolbar => {
                self.set_focus_area(FocusArea::Toolbar);
                Vec::new()
            }
            AppAction::FocusQueryTabs => {
                self.set_focus_area(FocusArea::QueryTabs);
                Vec::new()
            }
            AppAction::QuerySelectedTable => self.selected_table_query_effects(true, true, true),
            AppAction::ShowSelectedTableSchema => {
                let Some(table) = self.selected_table.clone() else {
                    return Vec::new();
                };
                self.selected_table = Some(table.clone());
                let Some(conn) = self.manager.get_active() else {
                    return Vec::new();
                };
                let schema_sql = match conn.config.db_type {
                    DatabaseType::SQLite => {
                        let escaped = table.replace('\'', "''");
                        format!("PRAGMA table_info('{}');", escaped)
                    }
                    DatabaseType::PostgreSQL => {
                        let escaped = table.replace('\'', "''");
                        format!(
                            "SELECT column_name, data_type, is_nullable, column_default \
                             FROM information_schema.columns \
                             WHERE table_name = '{}' \
                             ORDER BY ordinal_position;",
                            escaped
                        )
                    }
                    DatabaseType::MySQL => {
                        let escaped = table.replace('`', "``").replace('.', "_");
                        format!("DESCRIBE `{}`;", escaped)
                    }
                };
                self.sql.clear();
                vec![AppEffect::ExecuteSql(schema_sql)]
            }
            AppAction::RecheckEnvironment => vec![AppEffect::RefreshWelcomeEnvironment],
            AppAction::OpenLearningSample => {
                self.welcome_setup_target = DatabaseType::SQLite;
                vec![AppEffect::EnsureLearningSample {
                    reset: false,
                    notify: true,
                    mark_onboarding: true,
                    show_setup: true,
                }]
            }
            AppAction::EnsureLearningSample { reset, notify } => {
                self.welcome_setup_target = DatabaseType::SQLite;
                vec![AppEffect::EnsureLearningSample {
                    reset,
                    notify,
                    mark_onboarding: false,
                    show_setup: false,
                }]
            }
            AppAction::ConfirmPendingDelete => {
                self.confirm_pending_delete();
                Vec::new()
            }
        }
    }

    fn selected_table_query_effects(
        &mut self,
        reset_primary_key: bool,
        clear_sql: bool,
        fetch_primary_key: bool,
    ) -> Vec<AppEffect> {
        let Some(table) = self.selected_table.clone() else {
            return Vec::new();
        };
        self.switch_grid_workspace(Some(table.clone()));
        if reset_primary_key {
            self.grid_state.primary_key_column = None;
        }

        let query_sql = match ui::quote_identifier(&table, self.is_mysql()) {
            Ok(quoted_table) => format!(
                "SELECT * FROM {} LIMIT {};",
                quoted_table,
                constants::database::DEFAULT_QUERY_LIMIT
            ),
            Err(error) => {
                self.notifications.error(format!("表名无效: {}", error));
                return Vec::new();
            }
        };

        if clear_sql {
            self.sql.clear();
        }

        let mut effects = vec![AppEffect::ExecuteSql(query_sql)];
        if fetch_primary_key {
            effects.push(AppEffect::FetchPrimaryKey(table));
        }
        effects
    }

    fn switch_to_query_tab(&mut self, index: usize) {
        self.sync_sql_to_active_tab();
        self.tab_manager.set_active(index);
        self.sync_from_active_tab();
    }

    fn open_filter_workspace(&mut self) {
        self.show_sidebar = true;
        self.sidebar_panel_state.show_filters = true;
        self.sidebar_section = ui::SidebarSection::Filters;
        self.set_focus_area(FocusArea::Sidebar);
    }

    fn apply_app_effects(&mut self, effects: Vec<AppEffect>) {
        for effect in effects {
            match effect {
                AppEffect::Connect(name) => {
                    self.connect(name);
                }
                AppEffect::ExecuteSql(sql) => {
                    let _ = self.execute(sql);
                }
                AppEffect::FetchPrimaryKey(table) => {
                    self.fetch_primary_key(&table);
                }
                AppEffect::RefreshWelcomeEnvironment => {
                    self.refresh_welcome_environment_status();
                    self.notifications.info("已重新检测本机数据库环境");
                }
                AppEffect::EnsureLearningSample {
                    reset,
                    notify,
                    mark_onboarding,
                    show_setup,
                } => match self.ensure_learning_connection(reset, notify) {
                    Ok(()) => {
                        if mark_onboarding {
                            self.mark_onboarding_connection_created();
                            self.mark_onboarding_database_initialized();
                        }
                        if show_setup {
                            self.show_welcome_setup_dialog = true;
                        }
                    }
                    Err(error) => {
                        self.notifications.error(error);
                    }
                },
            }
        }
    }

    fn ensure_sidebar_workspace_visible(&mut self) {
        self.show_sidebar = true;
        match self.sidebar_section {
            ui::SidebarSection::Connections
            | ui::SidebarSection::Databases
            | ui::SidebarSection::Tables => {
                self.sidebar_panel_state.show_connections = true;
            }
            ui::SidebarSection::Filters => {
                self.sidebar_panel_state.show_filters = true;
            }
            ui::SidebarSection::Triggers => {
                self.sidebar_panel_state.show_triggers = true;
            }
            ui::SidebarSection::Routines => {
                self.sidebar_panel_state.show_routines = true;
            }
        }

        if !self.sidebar_panel_state.show_connections
            && !self.sidebar_panel_state.show_filters
            && !self.sidebar_panel_state.show_triggers
            && !self.sidebar_panel_state.show_routines
        {
            self.sidebar_panel_state.show_connections = true;
            self.sidebar_section = ui::SidebarSection::Connections;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_context() -> ActionContext {
        ActionContext {
            has_any_connection: false,
            has_active_connection: false,
            active_db_type: None,
            has_result: false,
            result_has_rows: false,
            selected_table: None,
            has_sql: false,
            has_search_text: false,
            grid_has_changes: false,
            query_tab_count: 1,
            focus_area: FocusArea::DataGrid,
            show_sidebar: false,
            show_sql_editor: false,
            show_er_diagram: false,
            can_confirm_pending_delete: false,
        }
    }

    #[test]
    fn registry_exposes_plenty_of_palette_commands() {
        assert!(command_descriptors().len() >= 20);
    }

    #[test]
    fn search_matches_keyword_aliases() {
        let context = base_context();
        let matches = search_commands(&context, "ddl");
        assert!(
            matches
                .iter()
                .any(|entry| entry.descriptor.action == AppAction::NewTable)
        );
    }

    #[test]
    fn search_prefers_scope_local_commands() {
        let mut context = base_context();
        context.focus_area = FocusArea::SqlEditor;
        let matches = search_commands(&context, "run sql");
        assert_eq!(
            matches.first().map(|entry| entry.descriptor.action),
            Some(AppAction::RunCurrentSql)
        );
    }

    #[test]
    fn learning_sample_command_uses_onboarding_action() {
        let descriptor = command_descriptors()
            .iter()
            .find(|entry| entry.id == "open_learning_sample")
            .expect("learning sample command should be registered");

        assert_eq!(descriptor.action, AppAction::OpenLearningSample);
    }

    #[test]
    fn availability_tracks_connection_and_result_requirements() {
        let context = base_context();
        assert_eq!(
            availability_for_action(&context, AppAction::OpenExportDialog),
            ActionAvailability::disabled("当前没有可导出的结果集")
        );
        assert_eq!(
            availability_for_action(&context, AppAction::RefreshActiveConnection),
            ActionAvailability::disabled("请先连接数据库")
        );
        assert_eq!(
            availability_for_action(&context, AppAction::RefreshSelectedTable),
            ActionAvailability::disabled("请先选中一张表")
        );
        assert_eq!(
            availability_for_action(&context, AppAction::OpenFilterWorkspace),
            ActionAvailability::disabled("当前没有可筛选的结果集")
        );
        assert_eq!(
            availability_for_action(&context, AppAction::SwitchToQueryTab(1)),
            ActionAvailability::disabled("目标查询标签页不存在")
        );

        let mut connected = base_context();
        connected.has_active_connection = true;
        connected.has_result = true;
        connected.result_has_rows = true;
        connected.selected_table = Some("users".to_string());
        connected.query_tab_count = 2;
        assert!(availability_for_action(&connected, AppAction::OpenExportDialog).enabled);
        assert!(availability_for_action(&connected, AppAction::QuerySelectedTable).enabled);
        assert!(availability_for_action(&connected, AppAction::RefreshSelectedTable).enabled);
        assert!(availability_for_action(&connected, AppAction::OpenFilterWorkspace).enabled);
        assert!(availability_for_action(&connected, AppAction::SwitchToQueryTab(1)).enabled);
    }

    #[test]
    fn confirm_pending_delete_requires_pending_target() {
        let context = base_context();
        assert_eq!(
            availability_for_action(&context, AppAction::ConfirmPendingDelete),
            ActionAvailability::disabled("当前没有待确认的删除操作")
        );

        let mut with_pending = base_context();
        with_pending.can_confirm_pending_delete = true;
        assert_eq!(
            availability_for_action(&with_pending, AppAction::ConfirmPendingDelete),
            ActionAvailability::enabled()
        );
    }
}
