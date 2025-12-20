//! 对话框渲染逻辑
//!
//! 将对话框的渲染和事件处理从主 update 循环中分离出来。

use crate::core::KeyBindings;
use crate::ui::{self, ExportConfig, KeyBindingsDialog};
use super::DbManagerApp;

/// 对话框处理结果
#[derive(Default)]
pub struct DialogResults {
    /// 是否需要保存连接
    pub save_connection: bool,
    /// 导出配置（如果触发导出）
    pub export_action: Option<ExportConfig>,
    /// 导入操作
    pub import_action: ui::ImportAction,
    /// DDL 创建 SQL
    pub ddl_sql: Option<String>,
    /// 创建数据库 SQL
    pub create_db_sql: Option<String>,
    /// 创建用户 SQL
    pub create_user_sql: Option<Vec<String>>,
    /// 历史记录选中的 SQL
    pub history_selected_sql: Option<String>,
    /// 是否清空历史
    pub clear_history: bool,
    /// 更新后的快捷键绑定
    pub updated_keybindings: Option<KeyBindings>,
}

impl DbManagerApp {
    /// 渲染所有对话框并返回处理结果
    pub fn render_dialogs(&mut self, ctx: &egui::Context) -> DialogResults {
        let mut results = DialogResults::default();

        // 连接对话框
        ui::ConnectionDialog::show(
            ctx,
            &mut self.show_connection_dialog,
            &mut self.new_config,
            &mut results.save_connection,
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

        if confirm_delete
            && let Some(name) = self.pending_delete_name.take()
        {
            self.delete_connection(&name);
        }

        // 导出对话框
        let table_name = self
            .selected_table
            .clone()
            .unwrap_or_else(|| "result".to_string());
        ui::ExportDialog::show(
            ctx,
            &mut self.show_export_dialog,
            &mut self.export_config,
            &table_name,
            self.result.as_ref(),
            &mut results.export_action,
            &self.export_status,
        );

        // 导入对话框
        let is_mysql = self.is_mysql();
        results.import_action = ui::ImportDialog::show(
            ctx,
            &mut self.show_import_dialog,
            &mut self.import_state,
            is_mysql,
        );

        // DDL 对话框（创建表）
        results.ddl_sql = ui::DdlDialog::show_create_table(
            ctx,
            &mut self.ddl_dialog_state,
        );

        // 新建数据库对话框
        let create_db_result = ui::CreateDbDialog::show(
            ctx,
            &mut self.create_db_dialog_state,
        );
        match create_db_result {
            ui::CreateDbDialogResult::Create(sql) => {
                results.create_db_sql = Some(sql);
            }
            ui::CreateDbDialogResult::Cancelled | ui::CreateDbDialogResult::None => {}
        }

        // 新建用户对话框
        let create_user_result = ui::CreateUserDialog::show(
            ctx,
            &mut self.create_user_dialog_state,
        );
        match create_user_result {
            ui::CreateUserDialogResult::Create(statements) => {
                results.create_user_sql = Some(statements);
            }
            ui::CreateUserDialogResult::Cancelled | ui::CreateUserDialogResult::None => {}
        }

        // 历史记录面板
        ui::HistoryPanel::show(
            ctx,
            &mut self.show_history_panel,
            &self.query_history,
            &mut results.history_selected_sql,
            &mut results.clear_history,
            &mut self.history_panel_state,
        );

        // 帮助面板
        ui::HelpDialog::show_with_scroll(ctx, &mut self.show_help, &mut self.help_scroll_offset);
        
        // 关于对话框
        ui::AboutDialog::show(ctx, &mut self.show_about);

        // 快捷键设置对话框
        results.updated_keybindings = KeyBindingsDialog::show(ctx, &mut self.keybindings_dialog_state);

        results
    }

    /// 处理对话框结果
    pub fn handle_dialog_results(&mut self, results: DialogResults) {
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
                self.focus_sql_editor = true;
                self.show_import_dialog = false;
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
            self.focus_sql_editor = true;
        }

        // 处理创建数据库
        if let Some(sql) = results.create_db_sql {
            if sql.starts_with("SQLITE_CREATE:") {
                let path = sql.trim_start_matches("SQLITE_CREATE:");
                self.notifications.info(format!("SQLite 数据库将创建于: {}", path));
            } else {
                self.sql = sql;
                self.show_sql_editor = true;
                self.focus_sql_editor = true;
                self.notifications.info("SQL 已生成，按 Ctrl+Enter 执行");
            }
        }

        // 处理创建用户
        if let Some(statements) = results.create_user_sql {
            self.sql = statements.join("\n");
            self.show_sql_editor = true;
            self.focus_sql_editor = true;
            self.notifications.info("SQL 已生成，按 Ctrl+Enter 执行");
        }

        // 处理历史记录
        if let Some(sql) = results.history_selected_sql {
            self.sql = sql;
        }

        if results.clear_history {
            self.query_history.clear();
        }

        // 处理快捷键更新
        if let Some(keybindings) = results.updated_keybindings {
            self.keybindings = keybindings;
            self.notifications.success("快捷键设置已保存");
        }
    }
}
