//! 新建数据库对话框
//!
//! 提供创建新数据库的 UI，支持 MySQL、PostgreSQL 和 SQLite。
//! 支持 Helix 风格的键盘导航。

use super::keyboard::{self, DialogAction};
use crate::database::DatabaseType;
use egui::{self, Color32, RichText, TextEdit};

// ============================================================================
// 对话框结果
// ============================================================================

/// 创建数据库对话框的结果
pub enum CreateDbDialogResult {
    /// 无操作
    None,
    /// 用户确认创建
    Create(String), // SQL 语句
    /// 用户取消
    Cancelled,
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
        self.db_type = db_type.clone();
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

    /// 生成 SQL 语句
    pub fn generate_sql(&self) -> Result<String, String> {
        // 验证数据库名
        if self.db_name.is_empty() {
            return Err("数据库名称不能为空".to_string());
        }

        // 验证数据库名格式
        if !self.db_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("数据库名只能包含字母、数字和下划线".to_string());
        }

        match self.db_type {
            DatabaseType::MySQL => self.generate_mysql_sql(),
            DatabaseType::PostgreSQL => self.generate_postgres_sql(),
            DatabaseType::SQLite => self.generate_sqlite_sql(),
        }
    }

    fn generate_mysql_sql(&self) -> Result<String, String> {
        let mut sql = format!("CREATE DATABASE `{}`", self.db_name);
        
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
        let mut sql = format!("CREATE DATABASE \"{}\"", self.db_name);
        
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

    fn generate_sqlite_sql(&self) -> Result<String, String> {
        // SQLite 不需要 CREATE DATABASE 语句
        // 只需要连接到新文件即可创建
        if self.sqlite_path.is_empty() && self.db_name.is_empty() {
            return Err("请指定数据库文件路径或名称".to_string());
        }
        
        // 返回文件路径作为特殊标记
        let path = if self.sqlite_path.is_empty() {
            format!("{}.db", self.db_name)
        } else {
            self.sqlite_path.clone()
        };
        
        Ok(format!("SQLITE_CREATE:{}", path))
    }
}

// ============================================================================
// 对话框 UI
// ============================================================================

/// 创建数据库对话框
pub struct CreateDbDialog;

impl CreateDbDialog {
    /// 显示对话框
    pub fn show(
        ctx: &egui::Context,
        state: &mut CreateDbDialogState,
    ) -> CreateDbDialogResult {
        if !state.show {
            return CreateDbDialogResult::None;
        }

        let mut result = CreateDbDialogResult::None;
        let mut should_close = false;

        // 键盘快捷键处理
        if !keyboard::has_text_focus(ctx) {
            // Esc/q 关闭
            if keyboard::handle_close_keys(ctx) {
                state.close();
                return CreateDbDialogResult::Cancelled;
            }

            // Enter 确认
            if let DialogAction::Confirm = keyboard::handle_dialog_keys(ctx) {
                match state.generate_sql() {
                    Ok(sql) => {
                        result = CreateDbDialogResult::Create(sql);
                        should_close = true;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
        }

        let title = match state.db_type {
            DatabaseType::MySQL => "新建 MySQL 数据库",
            DatabaseType::PostgreSQL => "新建 PostgreSQL 数据库",
            DatabaseType::SQLite => "新建 SQLite 数据库",
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .min_width(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
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
                        ui.label(RichText::new(err).color(Color32::from_rgb(255, 100, 100)));
                    }

                    ui.add_space(8.0);

                    // 快捷键提示
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("快捷键: Esc/q 关闭 | Enter 创建")
                                .small()
                                .color(Color32::from_rgb(120, 120, 120)),
                        );
                    });

                    ui.add_space(4.0);

                    // 按钮
                    ui.horizontal(|ui| {
                        if ui.button("创建 [Enter]").clicked() {
                            match state.generate_sql() {
                                Ok(sql) => {
                                    result = CreateDbDialogResult::Create(sql);
                                    should_close = true;
                                }
                                Err(e) => {
                                    state.error = Some(e);
                                }
                            }
                        }

                        if ui.button("取消 [Esc]").clicked() {
                            result = CreateDbDialogResult::Cancelled;
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            state.close();
        }

        result
    }

    fn show_mysql_options(ui: &mut egui::Ui, state: &mut CreateDbDialogState) {
        ui.group(|ui| {
            ui.label(RichText::new("MySQL 选项").strong());
            ui.add_space(4.0);

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
        ui.group(|ui| {
            ui.label(RichText::new("PostgreSQL 选项").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("编码:");
                egui::ComboBox::from_id_salt("encoding")
                    .selected_text(&state.encoding)
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.encoding, "UTF8".to_string(), "UTF8");
                        ui.selectable_value(&mut state.encoding, "LATIN1".to_string(), "LATIN1");
                        ui.selectable_value(&mut state.encoding, "SQL_ASCII".to_string(), "SQL_ASCII");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("模板:");
                egui::ComboBox::from_id_salt("template")
                    .selected_text(&state.template)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut state.template, "template0".to_string(), "template0");
                        ui.selectable_value(&mut state.template, "template1".to_string(), "template1");
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
        ui.group(|ui| {
            ui.label(RichText::new("SQLite 选项").strong());
            ui.add_space(4.0);

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
