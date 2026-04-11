//! 新建数据库对话框
//!
//! 提供创建新数据库的 UI，支持 MySQL、PostgreSQL 和 SQLite。

use std::path::PathBuf;

use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStyle, DialogWindow,
};
use crate::database::DatabaseType;
use crate::ui::{LocalShortcut, local_shortcut_text};
use egui::{self, Color32, RichText, TextEdit};

// ============================================================================
// 对话框结果
// ============================================================================

/// 创建数据库对话框的结果
pub enum CreateDbDialogResult {
    /// 无操作
    None,
    /// 用户确认创建
    Create(CreateDatabaseRequest),
    /// 用户取消
    Cancelled,
}

/// 创建数据库 workflow 的显式请求。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CreateDatabaseRequest {
    /// 需要复制到 SQL 编辑器并由用户执行的 SQL。
    Sql(String),
    /// SQLite 通过打开目标文件完成创建，不应伪装成 SQL 字符串。
    SqliteFile(PathBuf),
}

// ============================================================================
// 对话框状态
// ============================================================================

/// 创建数据库对话框状态
#[derive(Default)]
pub struct CreateDbDialogState {
    /// 数据库名称
    pub db_name: String,
    /// 字符集 (MySQL)
    pub charset: String,
    /// 排序规则 (MySQL)
    pub collation: String,
    /// 编码 (PostgreSQL)
    pub encoding: String,
    /// 模板 (PostgreSQL)
    pub template: String,
    /// 所有者 (PostgreSQL)
    pub owner: String,
    /// SQLite 文件路径
    pub sqlite_path: String,
    /// 是否显示对话框
    pub show: bool,
    /// 当前数据库类型
    pub db_type: DatabaseType,
    /// 错误信息
    pub error: Option<String>,
}

impl CreateDbDialogState {
    /// 创建新的对话框状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 打开对话框
    pub fn open(&mut self, db_type: DatabaseType) {
        self.reset();
        self.db_type = db_type;
        self.show = true;

        // 设置默认值
        match db_type {
            DatabaseType::MySQL => {
                self.charset = "utf8mb4".to_string();
                self.collation = "utf8mb4_unicode_ci".to_string();
            }
            DatabaseType::PostgreSQL => {
                self.encoding = "UTF8".to_string();
                self.template = "template0".to_string();
            }
            DatabaseType::SQLite => {
                // SQLite 使用文件路径
            }
        }
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.show = false;
        self.reset();
    }

    /// 重置状态
    fn reset(&mut self) {
        self.db_name.clear();
        self.charset.clear();
        self.collation.clear();
        self.encoding.clear();
        self.template.clear();
        self.owner.clear();
        self.sqlite_path.clear();
        self.error = None;
    }

    /// 生成 workflow 请求。
    pub fn generate_request(&self) -> Result<CreateDatabaseRequest, String> {
        match self.db_type {
            DatabaseType::SQLite => self.generate_sqlite_request(),
            DatabaseType::MySQL | DatabaseType::PostgreSQL => {
                self.generate_sql().map(CreateDatabaseRequest::Sql)
            }
        }
    }

    /// 生成 SQL 语句。SQLite 使用文件创建请求，不走 SQL 预览。
    pub fn generate_sql(&self) -> Result<String, String> {
        let db_name = self.db_name.trim();
        if db_name.is_empty() {
            return Err("数据库名称不能为空".to_string());
        }
        if !db_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("数据库名只能包含字母、数字和下划线".to_string());
        }

        match self.db_type {
            DatabaseType::MySQL => self.generate_mysql_sql(),
            DatabaseType::PostgreSQL => self.generate_postgres_sql(),
            DatabaseType::SQLite => {
                Err("SQLite 数据库通过文件创建，不生成 CREATE DATABASE SQL".to_string())
            }
        }
    }

    fn generate_mysql_sql(&self) -> Result<String, String> {
        let mut sql = format!("CREATE DATABASE `{}`", self.db_name.trim());

        if !self.charset.is_empty() {
            sql.push_str(&format!(" CHARACTER SET {}", self.charset));
        }

        if !self.collation.is_empty() {
            sql.push_str(&format!(" COLLATE {}", self.collation));
        }

        sql.push(';');
        Ok(sql)
    }

    fn generate_postgres_sql(&self) -> Result<String, String> {
        let mut sql = format!("CREATE DATABASE \"{}\"", self.db_name.trim());

        if !self.encoding.is_empty() {
            sql.push_str(&format!(" ENCODING '{}'", self.encoding));
        }

        if !self.template.is_empty() {
            sql.push_str(&format!(" TEMPLATE {}", self.template));
        }

        if !self.owner.is_empty() {
            sql.push_str(&format!(" OWNER \"{}\"", self.owner));
        }

        sql.push(';');
        Ok(sql)
    }

    fn generate_sqlite_request(&self) -> Result<CreateDatabaseRequest, String> {
        // SQLite 不需要 CREATE DATABASE 语句
        // 只需要连接到新文件即可创建
        let sqlite_path = self.sqlite_path.trim();
        let db_name = self.db_name.trim();
        if sqlite_path.is_empty() && db_name.is_empty() {
            return Err("请指定数据库文件路径或名称".to_string());
        }

        let path = if sqlite_path.is_empty() {
            let lower = db_name.to_ascii_lowercase();
            if lower.ends_with(".db") || lower.ends_with(".sqlite") || lower.ends_with(".sqlite3") {
                db_name.to_string()
            } else {
                format!("{db_name}.db")
            }
        } else {
            sqlite_path.to_string()
        };

        Ok(CreateDatabaseRequest::SqliteFile(PathBuf::from(path)))
    }
}

// ============================================================================
// 对话框 UI
// ============================================================================

/// 创建数据库对话框
pub struct CreateDbDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreateDbKeyAction {
    Confirm,
    Close,
}

impl CreateDbDialog {
    fn try_create(state: &mut CreateDbDialogState) -> Result<CreateDatabaseRequest, String> {
        match state.generate_request() {
            Ok(request) => {
                state.error = None;
                Ok(request)
            }
            Err(error) => {
                state.error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn detect_key_action(ctx: &egui::Context) -> Option<CreateDbKeyAction> {
        DialogShortcutContext::new(ctx).resolve_commands(&[
            (
                LocalShortcut::Dismiss.config_key(),
                CreateDbKeyAction::Close,
            ),
            (
                LocalShortcut::Confirm.config_key(),
                CreateDbKeyAction::Confirm,
            ),
        ])
    }

    /// 显示对话框
    pub fn show(ctx: &egui::Context, state: &mut CreateDbDialogState) -> CreateDbDialogResult {
        if !state.show {
            return CreateDbDialogResult::None;
        }

        let mut result = CreateDbDialogResult::None;
        let mut should_close = false;

        // 键盘快捷键处理（文本输入优先于普通命令键）
        if let Some(key_action) = Self::detect_key_action(ctx) {
            match key_action {
                CreateDbKeyAction::Close => {
                    state.close();
                    return CreateDbDialogResult::Cancelled;
                }
                CreateDbKeyAction::Confirm => {
                    if let Ok(request) = Self::try_create(state) {
                        result = CreateDbDialogResult::Create(request);
                        should_close = true;
                    }
                }
            }
        }

        let title = match state.db_type {
            DatabaseType::MySQL => "新建 MySQL 数据库",
            DatabaseType::PostgreSQL => "新建 PostgreSQL 数据库",
            DatabaseType::SQLite => "新建 SQLite 数据库",
        };

        let style = DialogStyle::MEDIUM;
        DialogWindow::standard(ctx, title, &style).show(ctx, |ui| {
            ui.vertical(|ui| {
                // 数据库名称
                ui.horizontal(|ui| {
                    ui.label("数据库名:");
                    ui.add(
                        TextEdit::singleline(&mut state.db_name)
                            .desired_width(200.0)
                            .hint_text("输入数据库名称"),
                    );
                });

                ui.add_space(8.0);

                // 根据数据库类型显示不同选项
                match state.db_type {
                    DatabaseType::MySQL => {
                        Self::show_mysql_options(ui, state);
                    }
                    DatabaseType::PostgreSQL => {
                        Self::show_postgres_options(ui, state);
                    }
                    DatabaseType::SQLite => {
                        Self::show_sqlite_options(ui, state);
                    }
                }

                ui.add_space(8.0);
                ui.separator();

                // 预览 SQL
                if !matches!(state.db_type, DatabaseType::SQLite) {
                    ui.collapsing("预览 SQL", |ui| {
                        let sql = state.generate_sql().unwrap_or_default();
                        ui.add(
                            TextEdit::multiline(&mut sql.as_str())
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(3),
                        );
                    });
                }

                // 错误信息
                if let Some(err) = &state.error {
                    ui.add_space(4.0);
                    DialogContent::error_text(ui, err);
                }

                ui.add_space(8.0);

                // 快捷键提示
                DialogContent::shortcut_hint(
                    ui,
                    &[
                        (local_shortcut_text(LocalShortcut::Dismiss).as_str(), "关闭"),
                        (local_shortcut_text(LocalShortcut::Confirm).as_str(), "创建"),
                    ],
                );

                let can_attempt_create = match state.db_type {
                    DatabaseType::SQLite => {
                        !state.db_name.trim().is_empty() || !state.sqlite_path.trim().is_empty()
                    }
                    DatabaseType::MySQL | DatabaseType::PostgreSQL => {
                        !state.db_name.trim().is_empty()
                    }
                };
                let footer = DialogFooter::show(
                    ui,
                    &format!("创建 [{}]", local_shortcut_text(LocalShortcut::Confirm)),
                    &format!("取消 [{}]", local_shortcut_text(LocalShortcut::Dismiss)),
                    can_attempt_create,
                    &style,
                );

                if footer.confirmed
                    && let Ok(request) = Self::try_create(state)
                {
                    result = CreateDbDialogResult::Create(request);
                    should_close = true;
                }
                if footer.cancelled {
                    result = CreateDbDialogResult::Cancelled;
                    should_close = true;
                }
            });
        });

        if should_close {
            state.close();
        }

        result
    }

    fn show_mysql_options(ui: &mut egui::Ui, state: &mut CreateDbDialogState) {
        DialogContent::section(ui, "MySQL 选项", |ui| {
            ui.horizontal(|ui| {
                ui.label("字符集:");
                egui::ComboBox::from_id_salt("charset")
                    .selected_text(&state.charset)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.charset, "utf8mb4".to_string(), "utf8mb4");
                        ui.selectable_value(&mut state.charset, "utf8".to_string(), "utf8");
                        ui.selectable_value(&mut state.charset, "latin1".to_string(), "latin1");
                        ui.selectable_value(&mut state.charset, "ascii".to_string(), "ascii");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("排序规则:");
                egui::ComboBox::from_id_salt("collation")
                    .selected_text(&state.collation)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.collation,
                            "utf8mb4_unicode_ci".to_string(),
                            "utf8mb4_unicode_ci",
                        );
                        ui.selectable_value(
                            &mut state.collation,
                            "utf8mb4_general_ci".to_string(),
                            "utf8mb4_general_ci",
                        );
                        ui.selectable_value(
                            &mut state.collation,
                            "utf8_general_ci".to_string(),
                            "utf8_general_ci",
                        );
                        ui.selectable_value(
                            &mut state.collation,
                            "latin1_swedish_ci".to_string(),
                            "latin1_swedish_ci",
                        );
                    });
            });
        });
    }

    fn show_postgres_options(ui: &mut egui::Ui, state: &mut CreateDbDialogState) {
        DialogContent::section(ui, "PostgreSQL 选项", |ui| {
            ui.horizontal(|ui| {
                ui.label("编码:");
                egui::ComboBox::from_id_salt("encoding")
                    .selected_text(&state.encoding)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.encoding, "UTF8".to_string(), "UTF8");
                        ui.selectable_value(&mut state.encoding, "LATIN1".to_string(), "LATIN1");
                        ui.selectable_value(
                            &mut state.encoding,
                            "SQL_ASCII".to_string(),
                            "SQL_ASCII",
                        );
                    });
            });

            ui.horizontal(|ui| {
                ui.label("模板:");
                egui::ComboBox::from_id_salt("template")
                    .selected_text(&state.template)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut state.template,
                            "template0".to_string(),
                            "template0",
                        );
                        ui.selectable_value(
                            &mut state.template,
                            "template1".to_string(),
                            "template1",
                        );
                    });
            });

            ui.horizontal(|ui| {
                ui.label("所有者:");
                ui.add(
                    TextEdit::singleline(&mut state.owner)
                        .desired_width(150.0)
                        .hint_text("可选，留空使用当前用户"),
                );
            });
        });
    }

    fn show_sqlite_options(ui: &mut egui::Ui, state: &mut CreateDbDialogState) {
        DialogContent::section(ui, "SQLite 选项", |ui| {
            ui.horizontal(|ui| {
                ui.label("文件路径:");
                ui.add(
                    TextEdit::singleline(&mut state.sqlite_path)
                        .desired_width(250.0)
                        .hint_text("输入完整路径，或留空使用数据库名.db"),
                );
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new("提示: SQLite 数据库将在指定路径创建新文件")
                    .small()
                    .color(Color32::from_rgb(120, 120, 120)),
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key, Modifiers, RawInput};

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        ctx.begin_pass(RawInput {
            events: vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
    }

    #[test]
    fn sqlite_accepts_path_without_db_name() {
        let mut state = CreateDbDialogState {
            db_type: DatabaseType::SQLite,
            ..Default::default()
        };
        state.sqlite_path = "/tmp/gridix-learning.sqlite3".to_string();
        let request = state
            .generate_request()
            .expect("sqlite path should be accepted");
        assert_eq!(
            request,
            CreateDatabaseRequest::SqliteFile(PathBuf::from("/tmp/gridix-learning.sqlite3"))
        );
    }

    #[test]
    fn sqlite_keeps_existing_extension() {
        let state = CreateDbDialogState {
            db_type: DatabaseType::SQLite,
            db_name: "demo.sqlite3".to_string(),
            ..Default::default()
        };
        let request = state
            .generate_request()
            .expect("sqlite db_name with extension should be accepted");
        assert_eq!(
            request,
            CreateDatabaseRequest::SqliteFile(PathBuf::from("demo.sqlite3"))
        );
    }

    #[test]
    fn mysql_requires_valid_db_name() {
        let state = CreateDbDialogState {
            db_type: DatabaseType::MySQL,
            db_name: "bad-name".to_string(),
            ..Default::default()
        };
        let err = state.generate_sql().expect_err("invalid name should fail");
        assert!(err.contains("数据库名只能包含"));
    }

    #[test]
    fn create_db_dialog_detects_confirm_shortcut_through_scoped_command_id() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Enter);

        let action = CreateDbDialog::detect_key_action(&ctx);

        assert_eq!(action, Some(CreateDbKeyAction::Confirm));

        let _ = ctx.end_pass();
    }
}
