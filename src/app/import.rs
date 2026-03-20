//! 数据导入处理模块
//!
//! 处理 CSV、JSON、SQL 文件的导入逻辑。

use crate::core::{
    CsvImportConfig, JsonImportConfig, import_csv_to_sql, import_json_to_sql, preview_csv,
    preview_json,
};
use crate::database::execute_import_batch;
use crate::ui;

use super::{DbManagerApp, message::Message};

impl DbManagerApp {
    /// 打开导入对话框
    pub(super) fn handle_import(&mut self) {
        self.show_import_dialog = true;
        self.import_state.clear();
    }

    /// 选择导入文件
    pub(super) fn select_import_file(&mut self) {
        let file_dialog = rfd::FileDialog::new()
            .add_filter("SQL 文件", &["sql"])
            .add_filter("CSV 文件", &["csv", "tsv"])
            .add_filter("JSON 文件", &["json"])
            .add_filter("所有文件", &["*"]);

        if let Some(path) = file_dialog.pick_file() {
            self.import_state.set_file(path);
        }
    }

    /// 刷新导入预览
    pub(super) fn refresh_import_preview(&mut self) {
        let Some(ref path) = self.import_state.file_path else {
            return;
        };

        self.import_state.loading = true;
        self.import_state.error = None;
        self.import_state.preview = None;

        match self.import_state.format {
            ui::ImportFormat::Sql => {
                // SQL 文件解析
                match std::fs::read_to_string(path) {
                    Ok(content) => {
                        let preview = ui::parse_sql_file(&content, &self.import_state.sql_config);
                        self.import_state.preview = Some(preview);
                    }
                    Err(e) => {
                        self.import_state.error = Some(format!("读取文件失败: {}", e));
                    }
                }
            }
            ui::ImportFormat::Csv => {
                // CSV 预览
                let config = CsvImportConfig {
                    delimiter: self.import_state.csv_config.delimiter,
                    skip_rows: self.import_state.csv_config.skip_rows,
                    has_header: self.import_state.csv_config.has_header,
                    quote_char: self.import_state.csv_config.quote_char,
                    ..Default::default()
                };

                match preview_csv(path, &config) {
                    Ok(preview) => {
                        self.import_state.preview = Some(ui::ImportPreview {
                            columns: preview.columns,
                            preview_rows: preview.preview_rows,
                            total_rows: preview.total_rows,
                            statement_count: 0,
                            warnings: preview.warnings,
                            sql_statements: Vec::new(),
                        });
                    }
                    Err(e) => {
                        self.import_state.error = Some(e);
                    }
                }
            }
            ui::ImportFormat::Json => {
                // JSON 预览
                let config = JsonImportConfig {
                    json_path: if self.import_state.json_config.json_path.is_empty() {
                        None
                    } else {
                        Some(self.import_state.json_config.json_path.clone())
                    },
                    flatten_nested: self.import_state.json_config.flatten_nested,
                    ..Default::default()
                };

                match preview_json(path, &config) {
                    Ok(preview) => {
                        self.import_state.preview = Some(ui::ImportPreview {
                            columns: preview.columns,
                            preview_rows: preview.preview_rows,
                            total_rows: preview.total_rows,
                            statement_count: 0,
                            warnings: preview.warnings,
                            sql_statements: Vec::new(),
                        });
                    }
                    Err(e) => {
                        self.import_state.error = Some(e);
                    }
                }
            }
        }

        self.import_state.loading = false;
    }

    /// 执行导入（直接执行 SQL）
    pub(super) fn execute_import(&mut self) {
        let Some(ref path) = self.import_state.file_path else {
            return;
        };

        let is_mysql = self.is_mysql();

        let statements: Vec<String> = match self.import_state.format {
            ui::ImportFormat::Sql => {
                if let Some(ref preview) = self.import_state.preview {
                    preview.sql_statements.clone()
                } else {
                    Vec::new()
                }
            }
            ui::ImportFormat::Csv => {
                let config = CsvImportConfig {
                    delimiter: self.import_state.csv_config.delimiter,
                    skip_rows: self.import_state.csv_config.skip_rows,
                    has_header: self.import_state.csv_config.has_header,
                    quote_char: self.import_state.csv_config.quote_char,
                    table_name: self.import_state.csv_config.table_name.clone(),
                    ..Default::default()
                };

                match import_csv_to_sql(path, &config, is_mysql) {
                    Ok(result) => result.sql_statements,
                    Err(e) => {
                        self.notifications.error(format!("CSV 转换失败: {}", e));
                        return;
                    }
                }
            }
            ui::ImportFormat::Json => {
                let config = JsonImportConfig {
                    json_path: if self.import_state.json_config.json_path.is_empty() {
                        None
                    } else {
                        Some(self.import_state.json_config.json_path.clone())
                    },
                    table_name: self.import_state.json_config.table_name.clone(),
                    flatten_nested: self.import_state.json_config.flatten_nested,
                    ..Default::default()
                };

                match import_json_to_sql(path, &config, is_mysql) {
                    Ok(result) => result.sql_statements,
                    Err(e) => {
                        self.notifications.error(format!("JSON 转换失败: {}", e));
                        return;
                    }
                }
            }
        };

        if statements.is_empty() {
            self.notifications.warning("没有可执行的 SQL 语句");
            return;
        }

        let Some(active_name) = self.manager.active.clone() else {
            self.notifications.warning("请先连接数据库");
            return;
        };
        let Some(conn) = self.manager.connections.get(&active_name) else {
            self.notifications.warning("请先连接数据库");
            return;
        };
        let config = conn.config.clone();

        // 关闭对话框
        self.show_import_dialog = false;

        // 根据配置决定是否使用事务
        let use_transaction = self.import_state.sql_config.use_transaction;
        let stop_on_error = self.import_state.sql_config.stop_on_error;

        // 执行 SQL
        let valid_statements: Vec<String> = statements
            .into_iter()
            .filter(|s| !s.trim().is_empty())
            .collect();
        let valid_count = valid_statements.len();

        if valid_count == 0 {
            self.notifications.warning("没有有效的 SQL 语句");
            return;
        }

        let tx = self.tx.clone();
        self.import_executing = true;
        self.refresh_executing_flag();
        self.last_query_time_ms = None;

        self.runtime.spawn(async move {
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

        self.notifications.info(format!(
            "导入已开始：共 {} 条语句（事务: {}，遇错停止: {}）",
            valid_count,
            if use_transaction { "是" } else { "否" },
            if stop_on_error { "是" } else { "否" },
        ));

        self.import_state.clear();
    }
}
