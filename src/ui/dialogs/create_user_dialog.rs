//! 新建用户对话框
//!
//! 提供创建新数据库用户的 UI，支持 MySQL 和 PostgreSQL。
//! SQLite 不支持用户管理。
//! 支持 Helix 风格的键盘导航。

use super::keyboard::{self, DialogAction};
use crate::database::DatabaseType;
use egui::{self, Color32, RichText, TextEdit};

// ============================================================================
// 对话框结果
// ============================================================================

/// 创建用户对话框的结果
pub enum CreateUserDialogResult {
    /// 无操作
    None,
    /// 用户确认创建（返回 SQL 语句列表）
    Create(Vec<String>),
    /// 用户取消
    Cancelled,
}

// ============================================================================
// 权限定义
// ============================================================================

/// 数据库权限
#[derive(Debug, Clone, PartialEq)]
pub struct Privilege {
    pub name: &'static str,
    pub description: &'static str,
    pub selected: bool,
}

impl Privilege {
    fn new(name: &'static str, description: &'static str) -> Self {
        Self {
            name,
            description,
            selected: false,
        }
    }
}

// ============================================================================
// 对话框状态
// ============================================================================

/// 创建用户对话框状态
pub struct CreateUserDialogState {
    /// 用户名
    pub username: String,
    /// 密码
    pub password: String,
    /// 确认密码
    pub confirm_password: String,
    /// 主机 (MySQL)
    pub host: String,
    /// 授权的数据库
    pub grant_database: String,
    /// 权限列表
    pub privileges: Vec<Privilege>,
    /// 是否授予所有权限
    pub grant_all: bool,
    /// 是否显示对话框
    pub show: bool,
    /// 当前数据库类型
    pub db_type: DatabaseType,
    /// 可用数据库列表
    pub available_databases: Vec<String>,
    /// 错误信息
    pub error: Option<String>,
}

impl Default for CreateUserDialogState {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            confirm_password: String::new(),
            host: "localhost".to_string(),
            grant_database: String::new(),
            privileges: Vec::new(),
            grant_all: true,
            show: false,
            db_type: DatabaseType::MySQL,
            available_databases: Vec::new(),
            error: None,
        }
    }
}

impl CreateUserDialogState {
    /// 创建新的对话框状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 打开对话框
    pub fn open(&mut self, db_type: DatabaseType, databases: Vec<String>) {
        self.reset();
        self.db_type = db_type.clone();
        self.available_databases = databases;
        self.show = true;
        
        // 初始化权限列表
        self.privileges = match db_type {
            DatabaseType::MySQL => vec![
                Privilege::new("SELECT", "查询数据"),
                Privilege::new("INSERT", "插入数据"),
                Privilege::new("UPDATE", "更新数据"),
                Privilege::new("DELETE", "删除数据"),
                Privilege::new("CREATE", "创建表/数据库"),
                Privilege::new("DROP", "删除表/数据库"),
                Privilege::new("ALTER", "修改表结构"),
                Privilege::new("INDEX", "创建/删除索引"),
                Privilege::new("REFERENCES", "创建外键"),
                Privilege::new("CREATE VIEW", "创建视图"),
            ],
            DatabaseType::PostgreSQL => vec![
                Privilege::new("SELECT", "查询数据"),
                Privilege::new("INSERT", "插入数据"),
                Privilege::new("UPDATE", "更新数据"),
                Privilege::new("DELETE", "删除数据"),
                Privilege::new("TRUNCATE", "清空表"),
                Privilege::new("REFERENCES", "创建外键"),
                Privilege::new("TRIGGER", "创建触发器"),
                Privilege::new("CREATE", "创建对象"),
                Privilege::new("CONNECT", "连接数据库"),
                Privilege::new("TEMPORARY", "创建临时表"),
            ],
            DatabaseType::SQLite => vec![], // SQLite 不支持用户管理
        };
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.show = false;
        self.reset();
    }

    /// 重置状态
    fn reset(&mut self) {
        self.username.clear();
        self.password.clear();
        self.confirm_password.clear();
        self.host = "localhost".to_string();
        self.grant_database.clear();
        self.privileges.clear();
        self.grant_all = true;
        self.error = None;
    }

    /// 验证输入
    fn validate(&self) -> Result<(), String> {
        if self.username.is_empty() {
            return Err("用户名不能为空".to_string());
        }

        if !self.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return Err("用户名只能包含字母、数字和下划线".to_string());
        }

        if self.password.is_empty() {
            return Err("密码不能为空".to_string());
        }

        if self.password != self.confirm_password {
            return Err("两次输入的密码不一致".to_string());
        }

        if self.password.len() < 4 {
            return Err("密码长度至少为 4 位".to_string());
        }

        Ok(())
    }

    /// 生成 SQL 语句
    pub fn generate_sql(&self) -> Result<Vec<String>, String> {
        self.validate()?;

        match self.db_type {
            DatabaseType::MySQL => self.generate_mysql_sql(),
            DatabaseType::PostgreSQL => self.generate_postgres_sql(),
            DatabaseType::SQLite => Err("SQLite 不支持用户管理".to_string()),
        }
    }

    fn generate_mysql_sql(&self) -> Result<Vec<String>, String> {
        let mut statements = Vec::new();

        // CREATE USER 语句
        let escaped_password = self.password.replace('\'', "''");
        statements.push(format!(
            "CREATE USER '{}'@'{}' IDENTIFIED BY '{}';",
            self.username, self.host, escaped_password
        ));

        // GRANT 语句
        if !self.grant_database.is_empty() {
            let privileges = if self.grant_all {
                "ALL PRIVILEGES".to_string()
            } else {
                let selected: Vec<&str> = self.privileges
                    .iter()
                    .filter(|p| p.selected)
                    .map(|p| p.name)
                    .collect();
                if selected.is_empty() {
                    return Err("请至少选择一个权限".to_string());
                }
                selected.join(", ")
            };

            statements.push(format!(
                "GRANT {} ON `{}`.* TO '{}'@'{}';",
                privileges, self.grant_database, self.username, self.host
            ));
        }

        // FLUSH PRIVILEGES
        statements.push("FLUSH PRIVILEGES;".to_string());

        Ok(statements)
    }

    fn generate_postgres_sql(&self) -> Result<Vec<String>, String> {
        let mut statements = Vec::new();

        // CREATE USER 语句
        let escaped_password = self.password.replace('\'', "''");
        statements.push(format!(
            "CREATE USER \"{}\" WITH PASSWORD '{}';",
            self.username, escaped_password
        ));

        // GRANT 语句
        if !self.grant_database.is_empty() {
            if self.grant_all {
                statements.push(format!(
                    "GRANT ALL PRIVILEGES ON DATABASE \"{}\" TO \"{}\";",
                    self.grant_database, self.username
                ));
            } else {
                let selected: Vec<&str> = self.privileges
                    .iter()
                    .filter(|p| p.selected)
                    .map(|p| p.name)
                    .collect();
                if selected.is_empty() {
                    return Err("请至少选择一个权限".to_string());
                }
                
                // PostgreSQL 权限授予比较复杂，这里简化处理
                for priv_name in selected {
                    statements.push(format!(
                        "GRANT {} ON DATABASE \"{}\" TO \"{}\";",
                        priv_name, self.grant_database, self.username
                    ));
                }
            }
        }

        Ok(statements)
    }
}

// ============================================================================
// 对话框 UI
// ============================================================================

/// 创建用户对话框
pub struct CreateUserDialog;

impl CreateUserDialog {
    /// 显示对话框
    pub fn show(
        ctx: &egui::Context,
        state: &mut CreateUserDialogState,
    ) -> CreateUserDialogResult {
        if !state.show {
            return CreateUserDialogResult::None;
        }

        // SQLite 不支持
        if matches!(state.db_type, DatabaseType::SQLite) {
            state.close();
            return CreateUserDialogResult::None;
        }

        let mut result = CreateUserDialogResult::None;
        let mut should_close = false;

        // 键盘快捷键处理
        if !keyboard::has_text_focus(ctx) {
            // Esc/q 关闭
            if keyboard::handle_close_keys(ctx) {
                state.close();
                return CreateUserDialogResult::Cancelled;
            }

            // Enter 确认
            if let DialogAction::Confirm = keyboard::handle_dialog_keys(ctx) {
                match state.generate_sql() {
                    Ok(statements) => {
                        result = CreateUserDialogResult::Create(statements);
                        should_close = true;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }
        }

        let title = match state.db_type {
            DatabaseType::MySQL => "新建 MySQL 用户",
            DatabaseType::PostgreSQL => "新建 PostgreSQL 用户",
            DatabaseType::SQLite => "新建用户", // 不会显示
        };

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .min_width(450.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // 基本信息
                    ui.group(|ui| {
                        ui.label(RichText::new("基本信息").strong());
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("用户名:");
                            ui.add(
                                TextEdit::singleline(&mut state.username)
                                    .desired_width(200.0)
                                    .hint_text("输入用户名"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("密  码:");
                            ui.add(
                                TextEdit::singleline(&mut state.password)
                                    .password(true)
                                    .desired_width(200.0)
                                    .hint_text("输入密码"),
                            );
                        });

                        ui.horizontal(|ui| {
                            ui.label("确  认:");
                            ui.add(
                                TextEdit::singleline(&mut state.confirm_password)
                                    .password(true)
                                    .desired_width(200.0)
                                    .hint_text("再次输入密码"),
                            );
                        });

                        // MySQL 特有：主机
                        if matches!(state.db_type, DatabaseType::MySQL) {
                            ui.horizontal(|ui| {
                                ui.label("主  机:");
                                egui::ComboBox::from_id_salt("host")
                                    .selected_text(&state.host)
                                    .width(150.0)
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut state.host,
                                            "localhost".to_string(),
                                            "localhost",
                                        );
                                        ui.selectable_value(
                                            &mut state.host,
                                            "%".to_string(),
                                            "% (所有主机)",
                                        );
                                        ui.selectable_value(
                                            &mut state.host,
                                            "127.0.0.1".to_string(),
                                            "127.0.0.1",
                                        );
                                    });
                            });
                        }
                    });

                    ui.add_space(8.0);

                    // 权限设置
                    ui.group(|ui| {
                        ui.label(RichText::new("权限设置").strong());
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label("授权数据库:");
                            egui::ComboBox::from_id_salt("grant_db")
                                .selected_text(if state.grant_database.is_empty() {
                                    "选择数据库（可选）"
                                } else {
                                    &state.grant_database
                                })
                                .width(200.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut state.grant_database,
                                        String::new(),
                                        "不授权",
                                    );
                                    for db in &state.available_databases {
                                        ui.selectable_value(
                                            &mut state.grant_database,
                                            db.clone(),
                                            db,
                                        );
                                    }
                                });
                        });

                        if !state.grant_database.is_empty() {
                            ui.add_space(4.0);
                            
                            ui.checkbox(&mut state.grant_all, "授予所有权限 (ALL PRIVILEGES)");

                            if !state.grant_all {
                                ui.add_space(4.0);
                                ui.label(
                                    RichText::new("选择权限:")
                                        .small()
                                        .color(Color32::from_rgb(150, 150, 150)),
                                );

                                egui::ScrollArea::vertical()
                                    .max_height(150.0)
                                    .show(ui, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            for priv_item in &mut state.privileges {
                                                ui.checkbox(&mut priv_item.selected, priv_item.name)
                                                    .on_hover_text(priv_item.description);
                                            }
                                        });
                                    });
                            }
                        }
                    });

                    ui.add_space(8.0);
                    ui.separator();

                    // 预览 SQL
                    ui.collapsing("预览 SQL", |ui| {
                        let sql = state.generate_sql()
                            .map(|stmts| stmts.join("\n"))
                            .unwrap_or_default();
                        ui.add(
                            TextEdit::multiline(&mut sql.as_str())
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(4),
                        );
                    });

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
                                Ok(statements) => {
                                    result = CreateUserDialogResult::Create(statements);
                                    should_close = true;
                                }
                                Err(e) => {
                                    state.error = Some(e);
                                }
                            }
                        }

                        if ui.button("取消 [Esc]").clicked() {
                            result = CreateUserDialogResult::Cancelled;
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
}
