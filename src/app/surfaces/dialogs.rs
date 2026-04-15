//! 对话框渲染逻辑
//!
//! 将对话框的渲染和事件处理从主 update 循环中分离出来。

use std::path::{Path, PathBuf};

use super::DbManagerApp;
use super::action_system::AppAction;
use crate::app::dialogs::host::DialogId;
use crate::core::{KeyBindings, ThemePreset};
use crate::ui::{
    self, ExportConfig, KeyBindingsDialog, LocalShortcut, ToolbarMenuDialogEntry,
    ToolbarMenuItemId, local_shortcut_text,
};

/// 对话框处理结果
#[derive(Default)]
pub(in crate::app) struct DialogResults {
    /// 是否需要保存连接
    pub save_connection: bool,
    /// 导出配置（如果触发导出）
    pub export_action: Option<ExportConfig>,
    /// 导入操作
    pub import_action: ui::ImportAction,
    /// DDL 创建 SQL
    pub ddl_sql: Option<String>,
    /// 创建数据库 workflow 请求
    pub create_database_request: Option<ui::CreateDatabaseRequest>,
    /// 创建用户 SQL
    pub create_user_sql: Option<Vec<String>>,
    /// 历史记录选中的 SQL
    pub history_selected_sql: Option<String>,
    /// 是否清空历史
    pub clear_history: bool,
    /// 帮助面板动作
    pub help_action: Option<ui::HelpAction>,
    /// 更新后的快捷键绑定
    pub updated_keybindings: Option<KeyBindings>,
    /// 主题选择器返回的主题
    pub theme_preset: Option<ThemePreset>,
    /// 顶部工具栏菜单触发的 app action
    pub toolbar_menu_action: Option<AppAction>,
}

impl DbManagerApp {
    fn toolbar_menu_entry(
        &self,
        id: ToolbarMenuItemId,
        icon: &'static str,
        title: &'static str,
        description: &'static str,
        action: AppAction,
    ) -> ToolbarMenuDialogEntry {
        let availability = self.action_availability(action);
        ToolbarMenuDialogEntry {
            id,
            icon,
            title,
            description,
            shortcut: self.shortcut_label_for_action(action).unwrap_or_default(),
            enabled: availability.enabled,
            disabled_reason: availability.reason.map(str::to_owned),
        }
    }

    fn map_toolbar_menu_item_to_action(item: ToolbarMenuItemId) -> AppAction {
        match item {
            ToolbarMenuItemId::Export => AppAction::OpenExportDialog,
            ToolbarMenuItemId::Import => AppAction::OpenImportDialog,
            ToolbarMenuItemId::ToggleErDiagram => AppAction::ToggleErDiagram,
            ToolbarMenuItemId::ShowHistory => AppAction::OpenHistoryPanel,
            ToolbarMenuItemId::NewTable => AppAction::NewTable,
            ToolbarMenuItemId::NewDatabase => AppAction::NewDatabase,
            ToolbarMenuItemId::NewUser => AppAction::NewUser,
        }
    }

    /// 渲染所有对话框并返回处理结果
    pub(in crate::app) fn render_dialogs(&mut self, ctx: &egui::Context) -> DialogResults {
        self.reconcile_active_dialog_owner();
        let mut results = DialogResults::default();
        let active_dialog = self.active_dialog_id();

        // 连接对话框
        if active_dialog == Some(DialogId::Connection) {
            let old_show_advanced = self.connection_dialog_show_advanced;
            ui::ConnectionDialog::show(
                ctx,
                &mut self.show_connection_dialog,
                &mut self.connection_dialog_show_advanced,
                &mut self.new_config,
                &mut results.save_connection,
                self.editing_connection_name.is_some(),
            );
            if old_show_advanced != self.connection_dialog_show_advanced {
                self.app_config.connection_dialog_show_advanced =
                    self.connection_dialog_show_advanced;
                if let Err(e) = self.app_config.save() {
                    self.notifications
                        .error(format!("保存连接对话框模式失败: {}", e));
                }
            }
        }

        // 删除确认对话框
        if active_dialog == Some(DialogId::DeleteConfirm) {
            let mut confirm_delete = false;
            let (delete_title, delete_msg) = match self.pending_delete_target.as_ref() {
                Some(ui::SidebarDeleteTarget::Connection(connection)) => (
                    "删除连接",
                    format!(
                        "确定要删除连接 '{}' 吗？这只会移除保存的连接配置。",
                        connection
                    ),
                ),
                Some(ui::SidebarDeleteTarget::Database {
                    connection_name,
                    database_name,
                }) => (
                    "删除数据库",
                    format!(
                        "确定要删除连接 '{}' 下的数据库 '{}' 吗？这会真正执行 DROP DATABASE，且不可撤销。",
                        connection_name, database_name
                    ),
                ),
                Some(ui::SidebarDeleteTarget::Table {
                    connection_name,
                    table_name,
                }) => (
                    "删除表",
                    format!(
                        "确定要删除连接 '{}' 下的表 '{}' 吗？该操作不可撤销。",
                        connection_name, table_name
                    ),
                ),
                None => ("删除", String::new()),
            };
            ui::ConfirmDialog::show(
                ctx,
                &mut self.show_delete_confirm,
                delete_title,
                &delete_msg,
                "删除",
                &mut confirm_delete,
            );

            if confirm_delete {
                self.dispatch_app_action(ctx, AppAction::ConfirmPendingDelete);
            }
        }

        // 导出对话框
        if active_dialog == Some(DialogId::Export) {
            let table_name = self
                .selected_table
                .clone()
                .unwrap_or_else(|| "result".to_string());
            let export_db_type = self
                .manager
                .get_active()
                .map(|connection| connection.config.db_type)
                .unwrap_or(crate::database::DatabaseType::SQLite);
            ui::ExportDialog::show(
                ctx,
                &mut self.show_export_dialog,
                &mut self.export_config,
                &table_name,
                self.result.as_ref(),
                export_db_type,
                &mut results.export_action,
                &self.export_status,
            );
        }

        // 导入对话框
        if active_dialog == Some(DialogId::Import) {
            let is_mysql = self.is_mysql();
            results.import_action = ui::ImportDialog::show(
                ctx,
                &mut self.show_import_dialog,
                &mut self.import_state,
                is_mysql,
            );
        }

        // DDL 对话框（创建表）
        if active_dialog == Some(DialogId::Ddl) {
            results.ddl_sql = ui::DdlDialog::show_create_table(ctx, &mut self.ddl_dialog_state);
        }

        // 新建数据库对话框
        if active_dialog == Some(DialogId::CreateDatabase) {
            let create_db_result = ui::CreateDbDialog::show(ctx, &mut self.create_db_dialog_state);
            match create_db_result {
                ui::CreateDbDialogResult::Create(request) => {
                    results.create_database_request = Some(request);
                }
                ui::CreateDbDialogResult::Cancelled | ui::CreateDbDialogResult::None => {}
            }
        }

        // 新建用户对话框
        if active_dialog == Some(DialogId::CreateUser) {
            let create_user_result =
                ui::CreateUserDialog::show(ctx, &mut self.create_user_dialog_state);
            match create_user_result {
                ui::CreateUserDialogResult::Create(statements) => {
                    results.create_user_sql = Some(statements);
                }
                ui::CreateUserDialogResult::Cancelled | ui::CreateUserDialogResult::None => {}
            }
        }

        // 历史记录面板
        if active_dialog == Some(DialogId::History) {
            ui::HistoryPanel::show(
                ctx,
                &mut self.show_history_panel,
                &self.query_history,
                &mut results.history_selected_sql,
                &mut results.clear_history,
                &mut self.history_panel_state,
            );
        }

        // 帮助面板
        if active_dialog == Some(DialogId::Help) {
            let onboarding = self.welcome_onboarding_status();
            let help_context = ui::HelpContext {
                keybindings: self.keybindings.clone(),
                active_connection_name: self.manager.active.clone(),
                selected_table: self.selected_table.clone(),
                has_result: self.result.is_some(),
                show_sql_editor: self.show_sql_editor,
                show_er_diagram: self.show_er_diagram,
                onboarding_environment_checked: onboarding.environment_checked,
                onboarding_connection_created: onboarding.connection_created,
                onboarding_database_initialized: onboarding.database_initialized,
                onboarding_user_created: onboarding.user_created,
                onboarding_first_query_executed: onboarding.first_query_executed,
                onboarding_require_user_step: onboarding.require_user_step,
            };
            results.help_action = ui::HelpDialog::show_with_scroll(
                ctx,
                &mut self.show_help,
                &mut self.help_scroll_offset,
                &mut self.help_state,
                &help_context,
            );
        }

        // 关于对话框
        if active_dialog == Some(DialogId::About) {
            ui::AboutDialog::show(ctx, &mut self.show_about);
        }

        // 快捷键设置对话框
        if active_dialog == Some(DialogId::Keybindings) {
            results.updated_keybindings =
                KeyBindingsDialog::show(ctx, &mut self.keybindings_dialog_state);
        }

        if active_dialog == Some(DialogId::ToolbarActionsMenu) {
            let entries = vec![
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::Export,
                    "⇪",
                    "导出当前结果",
                    "把当前结果集导出到 CSV / TSV / SQL / JSON。",
                    AppAction::OpenExportDialog,
                ),
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::Import,
                    "⇩",
                    "导入数据文件",
                    "打开统一导入流程，预览并执行导入。",
                    AppAction::OpenImportDialog,
                ),
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::ToggleErDiagram,
                    "⊞",
                    "切换 ER 图",
                    "打开或关闭当前连接的 ER 关系图工作区。",
                    AppAction::ToggleErDiagram,
                ),
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::ShowHistory,
                    "🕘",
                    "打开查询历史",
                    "查看并回填当前连接的查询历史记录。",
                    AppAction::OpenHistoryPanel,
                ),
            ];

            if let Some(item) = ui::ToolbarMenuDialog::show(
                ctx,
                &mut self.toolbar_actions_menu_state,
                "操作菜单",
                "保留顶部按钮作为 trigger，把真正的选择与确认放到显式 overlay owner 里。",
                "打开",
                &entries,
            ) {
                results.toolbar_menu_action = Some(Self::map_toolbar_menu_item_to_action(item));
            }
        }

        if active_dialog == Some(DialogId::ToolbarCreateMenu) {
            let entries = vec![
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::NewTable,
                    "#",
                    "新建表",
                    "打开建表工作流，并把生成的 SQL 带回当前会话。",
                    AppAction::NewTable,
                ),
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::NewDatabase,
                    "+",
                    "新建数据库",
                    "打开数据库初始化工作流，继续当前连接的创建过程。",
                    AppAction::NewDatabase,
                ),
                self.toolbar_menu_entry(
                    ToolbarMenuItemId::NewUser,
                    "@",
                    "新建用户",
                    "为支持用户管理的连接生成创建用户 SQL。",
                    AppAction::NewUser,
                ),
            ];

            if let Some(item) = ui::ToolbarMenuDialog::show(
                ctx,
                &mut self.toolbar_create_menu_state,
                "新建菜单",
                "让新建动作进入固定 footer 的选择对话框，而不是继续停留在 toolbar 内部 popup。",
                "打开",
                &entries,
            ) {
                results.toolbar_menu_action = Some(Self::map_toolbar_menu_item_to_action(item));
            }
        }

        if active_dialog == Some(DialogId::ToolbarThemeMenu) {
            results.theme_preset = ui::ToolbarThemeDialog::show(
                ctx,
                &mut self.toolbar_theme_dialog_state,
                self.theme_manager.current,
                self.app_config.is_dark_mode,
            );
        }

        self.reconcile_active_dialog_owner();
        results
    }

    /// 处理对话框结果
    pub(in crate::app) fn handle_dialog_results(
        &mut self,
        ctx: &egui::Context,
        results: DialogResults,
    ) {
        // 处理导出
        if let Some(config) = results.export_action {
            self.handle_export_with_config(config);
        }

        // 处理导入
        match results.import_action {
            ui::ImportAction::SelectFile => {
                self.select_import_file();
                if self.import_state.file_path.is_some() {
                    self.refresh_import_preview();
                }
            }
            ui::ImportAction::RefreshPreview => {
                self.refresh_import_preview();
            }
            ui::ImportAction::Execute => {
                self.execute_import();
            }
            ui::ImportAction::CopyToEditor(sql) => {
                self.sql = sql;
                self.show_sql_editor = true;
                self.set_focus_area(ui::FocusArea::SqlEditor);
                self.close_dialog(DialogId::Import);
                self.import_state.clear();
                self.notifications.success("SQL 已复制到编辑器");
            }
            ui::ImportAction::Close => {
                self.import_state.clear();
            }
            ui::ImportAction::None => {}
        }

        // 处理 DDL
        if let Some(create_sql) = results.ddl_sql {
            self.sql = create_sql;
            self.show_sql_editor = true;
            self.set_focus_area(ui::FocusArea::SqlEditor);
        }

        // 处理创建数据库
        if let Some(request) = results.create_database_request {
            match request {
                ui::CreateDatabaseRequest::SqliteFile(path) => match self
                    .initialize_sqlite_database(&path)
                {
                    Ok(created_path) => {
                        self.notifications
                            .success(format!("SQLite 数据库已初始化: {}", created_path.display()));
                        self.mark_onboarding_database_initialized();
                    }
                    Err(error) => {
                        self.notifications
                            .error(format!("SQLite 初始化失败: {}", error));
                    }
                },
                ui::CreateDatabaseRequest::Sql(sql) => {
                    self.sql = sql;
                    self.show_sql_editor = true;
                    self.set_focus_area(ui::FocusArea::SqlEditor);
                    self.notifications.info(format!(
                        "SQL 已生成，按 {} 执行",
                        local_shortcut_text(LocalShortcut::SqlExecute)
                    ));
                }
            }
        }

        // 处理创建用户
        if let Some(statements) = results.create_user_sql {
            self.sql = statements.join("\n");
            self.show_sql_editor = true;
            self.set_focus_area(ui::FocusArea::SqlEditor);
            self.notifications.info(format!(
                "SQL 已生成，按 {} 执行",
                local_shortcut_text(LocalShortcut::SqlExecute)
            ));
        }

        // 处理历史记录
        if let Some(sql) = results.history_selected_sql {
            self.sql = sql;
        }

        if results.clear_history {
            self.query_history.clear();
            self.app_config.query_history.clear();
            let _ = self.app_config.save();
        }

        if let Some(action) = results.help_action {
            self.handle_help_action(ctx, action);
        }

        // 处理快捷键更新
        if let Some(keybindings) = results.updated_keybindings {
            self.keybindings = keybindings;
            ui::sync_runtime_local_shortcuts(&self.keybindings);
            if let Err(e) = self.keybindings.save_to_disk() {
                self.notifications.error(format!("快捷键保存失败: {}", e));
            } else {
                self.notifications.success("快捷键设置已保存");
            }
        }

        if let Some(preset) = results.theme_preset {
            if self.app_config.is_dark_mode {
                self.app_config.dark_theme = preset;
            } else {
                self.app_config.light_theme = preset;
            }
            self.set_theme(ctx, preset);
        }

        if let Some(action) = results.toolbar_menu_action {
            self.dispatch_app_action(ctx, action);
        }
    }

    pub(in crate::app) fn confirm_pending_delete(&mut self) {
        self.close_dialog(DialogId::DeleteConfirm);
        if let Some(target) = self.pending_delete_target.take() {
            match target {
                ui::SidebarDeleteTarget::Connection(connection) => {
                    self.delete_connection(&connection);
                }
                ui::SidebarDeleteTarget::Database {
                    connection_name,
                    database_name,
                } => {
                    self.delete_database(&connection_name, &database_name);
                }
                ui::SidebarDeleteTarget::Table {
                    connection_name,
                    table_name,
                } => {
                    self.delete_table(&connection_name, &table_name);
                }
            }
        }
    }

    fn initialize_sqlite_database(&self, path: &Path) -> Result<PathBuf, String> {
        if path.as_os_str().is_empty() {
            return Err("路径为空".to_string());
        }

        let path = path.to_path_buf();
        if let Some(parent) = path.parent()
            && parent != Path::new("")
        {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("创建目录失败 ({}): {}", parent.display(), e))?;
        }

        rusqlite::Connection::open(&path)
            .map_err(|e| format!("创建/打开 SQLite 文件失败 ({}): {}", path.display(), e))?;

        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::dialogs::host::DialogId;
    use crate::database::{Connection, ConnectionConfig, DatabaseType, QueryResult};
    use crate::ui::{FocusArea, SidebarDeleteTarget};

    fn prime_active_connection_with_tables(app: &mut DbManagerApp, tables: &[&str]) {
        let mut connection = Connection::new(ConnectionConfig::new("demo", DatabaseType::SQLite));
        connection.connected = true;
        connection.selected_database = Some("main".to_string());
        connection.tables = tables.iter().map(|name| (*name).to_string()).collect();
        app.manager
            .connections
            .insert("demo".to_string(), connection);
        app.manager.active = Some("demo".to_string());
    }

    #[test]
    fn confirm_pending_delete_database_target_hits_database_branch_and_clears_state() {
        let mut app = DbManagerApp::new_for_test();
        app.pending_delete_target = Some(SidebarDeleteTarget::database("missing", "analytics"));
        app.open_dialog(DialogId::DeleteConfirm);

        app.confirm_pending_delete();

        assert!(app.pending_delete_target.is_none());
        assert!(!app.show_delete_confirm);
        assert_eq!(app.active_dialog_id(), None);
        assert_eq!(app.notifications.latest_message(), Some("目标连接已失效"));
    }

    #[test]
    fn confirm_pending_delete_table_target_hits_table_branch_and_clears_state() {
        let mut app = DbManagerApp::new_for_test();
        app.pending_delete_target = Some(SidebarDeleteTarget::table("missing", "users"));
        app.open_dialog(DialogId::DeleteConfirm);

        app.confirm_pending_delete();

        assert!(app.pending_delete_target.is_none());
        assert!(!app.show_delete_confirm);
        assert_eq!(app.active_dialog_id(), None);
        assert_eq!(app.notifications.latest_message(), Some("目标连接已失效"));
    }

    #[test]
    fn toolbar_menu_toggle_er_diagram_action_focuses_er_from_data_grid() {
        let ctx = egui::Context::default();
        let mut app = DbManagerApp::new_for_test();
        prime_active_connection_with_tables(&mut app, &["customers", "orders"]);
        app.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        app.selected_table = Some("customers".to_string());
        app.set_focus_area(FocusArea::DataGrid);

        app.handle_dialog_results(
            &ctx,
            DialogResults {
                toolbar_menu_action: Some(AppAction::ToggleErDiagram),
                ..Default::default()
            },
        );

        assert!(app.show_er_diagram);
        assert_eq!(app.focus_area, FocusArea::ErDiagram);
        assert!(!app.grid_state.focused);
        assert_eq!(
            app.er_diagram_state.selected_table_name(),
            Some("customers")
        );
    }
}
