//! 数据导入处理模块
//!
//! 处理 CSV、TSV、JSON、SQL 文件的统一传输逻辑。

use crate::core::{plan_import_transfer, preview_import_transfer};
use crate::data::execute_import_batch;
use crate::ui;

use super::{DbManagerApp, message::Message};
use crate::app::dialogs::host::DialogId;

impl DbManagerApp {
    /// 打开导入对话框
    pub(in crate::app) fn handle_import(&mut self) {
        self.open_dialog(DialogId::Import);
        self.import_state.clear();
    }

    /// 选择导入文件
    pub(in crate::app) fn select_import_file(&mut self) {
        let file_dialog = rfd::FileDialog::new()
            .add_filter("SQL 文件", &["sql"])
            .add_filter("CSV 文件", &["csv"])
            .add_filter("TSV 文件", &["tsv", "tab"])
            .add_filter("JSON 文件", &["json"])
            .add_filter("所有文件", &["*"]);

        if let Some(path) = file_dialog.pick_file() {
            self.import_state.set_file(path);
        }
    }

    /// 刷新导入预览
    pub(in crate::app) fn refresh_import_preview(&mut self) {
        let Some(ref path) = self.import_state.file_path else {
            return;
        };
        let session = self.import_state.to_transfer_session(self.is_mysql());

        self.import_state.loading = true;
        self.import_state.error = None;
        self.import_state.preview = None;

        match preview_import_transfer(path, &session) {
            Ok(preview) => {
                self.import_state.preview = Some(ui::ImportPreview::from_transfer_preview(preview));
            }
            Err(error) => {
                self.import_state.error = Some(error);
            }
        }

        self.import_state.loading = false;
    }

    /// 执行导入（直接执行 SQL）
    pub(in crate::app) fn execute_import(&mut self) {
        let Some(ref path) = self.import_state.file_path else {
            return;
        };
        let session = self.import_state.to_transfer_session(self.is_mysql());

        let plan = match plan_import_transfer(path, &session) {
            Ok(plan) => plan,
            Err(error) => {
                self.session.notifications
                    .error(format!("导入计划生成失败: {}", error));
                return;
            }
        };

        let valid_statements: Vec<String> = match plan.into_sql_statements() {
            Ok(statements) => statements
                .into_iter()
                .filter(|statement| !statement.trim().is_empty())
                .collect(),
            Err(error) => {
                self.session.notifications.error(error);
                return;
            }
        };
        let valid_count = valid_statements.len();

        if valid_count == 0 {
            self.session.notifications.warning("没有有效的 SQL 语句");
            return;
        }

        let Some(active_name) = self.session.manager.active.clone() else {
            self.session.notifications.warning("请先连接数据库");
            return;
        };
        let Some(conn) = self.session.manager.connections.get(&active_name) else {
            self.session.notifications.warning("请先连接数据库");
            return;
        };
        let config = conn.config.clone();

        self.close_dialog(DialogId::Import);

        let use_transaction = self.import_state.sql_config.use_transaction;
        let stop_on_error = self.import_state.sql_config.stop_on_error;

        let tx = self.session.tx.clone();
        self.session.import_executing = true;
        self.session.refresh_executing_flag();
        self.session.last_query_time_ms = None;

        self.session.runtime.spawn(async move {
            let start = std::time::Instant::now();
            let result =
                execute_import_batch(&config, valid_statements, use_transaction, stop_on_error)
                    .await
                    .map_err(|e| e.to_string());
            let elapsed_ms = start.elapsed().as_millis() as u64;

            if tx.send(Message::ImportDone(result, elapsed_ms)).is_err() {
                tracing::warn!("无法发送导入结果：接收端已关闭");
            }
        });

        self.session.notifications.info(format!(
            "导入已开始：共 {} 条语句（事务: {}，遇错停止: {}）",
            valid_count,
            if use_transaction { "是" } else { "否" },
            if stop_on_error { "是" } else { "否" },
        ));

        self.import_state.clear();
    }
}
