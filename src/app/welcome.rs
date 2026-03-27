//! 欢迎页交互与新手引导
//!
//! 负责欢迎页数据库环境检测、卡片动作处理、安装/初始化指引弹窗。

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use eframe::egui;

use crate::core::AppConfig;
use crate::database::{ConnectionConfig, DatabaseType};
use crate::ui;

use super::DbManagerApp;

impl DbManagerApp {
    /// 刷新欢迎页数据库环境检测状态
    pub(super) fn refresh_welcome_environment_status(&mut self) {
        self.welcome_status = ui::WelcomeStatusSummary {
            sqlite: ui::WelcomeServiceState::BuiltIn,
            postgres: Self::detect_postgres_status(),
            mysql: Self::detect_mysql_status(),
        };
        self.mark_onboarding_environment_checked();
    }

    /// 处理欢迎页动作
    pub(super) fn handle_welcome_action(&mut self, action: ui::WelcomeAction) {
        match action {
            ui::WelcomeAction::OpenConnection(db_type) => {
                self.welcome_setup_target = db_type;
                self.open_connection_dialog_for(db_type);
            }
            ui::WelcomeAction::OpenSetupGuide(db_type) => {
                self.welcome_setup_target = db_type;
                self.show_welcome_setup_dialog = true;
            }
            ui::WelcomeAction::RecheckEnvironment => {
                self.refresh_welcome_environment_status();
                self.notifications.info("已重新检测本机数据库环境");
            }
            ui::WelcomeAction::ContinueOnboarding(step) => {
                self.handle_onboarding_step(step);
            }
            ui::WelcomeAction::OpenLearningSample => {
                self.welcome_setup_target = DatabaseType::SQLite;
                match self.ensure_learning_connection(false, true) {
                    Ok(()) => {
                        self.mark_onboarding_connection_created();
                        self.mark_onboarding_database_initialized();
                        self.show_welcome_setup_dialog = true;
                    }
                    Err(error) => {
                        self.notifications.error(error);
                    }
                }
            }
        }
    }

    pub(super) fn welcome_onboarding_status(&self) -> ui::WelcomeOnboardingStatus {
        let connection_created =
            self.app_config.onboarding.connection_created || !self.manager.connections.is_empty();
        let require_user_step = !matches!(self.onboarding_target_db_type(), DatabaseType::SQLite);
        ui::WelcomeOnboardingStatus {
            environment_checked: self.app_config.onboarding.environment_checked,
            connection_created,
            database_initialized: self.app_config.onboarding.database_initialized,
            user_created: self.app_config.onboarding.user_created,
            first_query_executed: self.app_config.onboarding.first_query_executed,
            require_user_step,
        }
    }

    pub(super) fn mark_onboarding_connection_created(&mut self) {
        if self.app_config.onboarding.connection_created {
            return;
        }
        self.app_config.onboarding.connection_created = true;
        let _ = self.app_config.save();
    }

    pub(super) fn mark_onboarding_database_initialized(&mut self) {
        if self.app_config.onboarding.database_initialized {
            return;
        }
        self.app_config.onboarding.database_initialized = true;
        let _ = self.app_config.save();
    }

    pub(super) fn mark_onboarding_user_created(&mut self) {
        if self.app_config.onboarding.user_created {
            return;
        }
        self.app_config.onboarding.user_created = true;
        let _ = self.app_config.save();
    }

    pub(super) fn mark_onboarding_first_query_executed(&mut self) {
        if self.app_config.onboarding.first_query_executed {
            return;
        }
        self.app_config.onboarding.first_query_executed = true;
        let _ = self.app_config.save();
    }

    fn mark_onboarding_environment_checked(&mut self) {
        if self.app_config.onboarding.environment_checked {
            return;
        }
        self.app_config.onboarding.environment_checked = true;
        let _ = self.app_config.save();
    }

    fn handle_onboarding_step(&mut self, step: ui::WelcomeOnboardingStep) {
        match step {
            ui::WelcomeOnboardingStep::EnvironmentCheck => {
                self.refresh_welcome_environment_status();
                self.notifications.success("环境检测已完成");
            }
            ui::WelcomeOnboardingStep::CreateConnection => {
                self.open_connection_dialog_for(self.onboarding_target_db_type());
            }
            ui::WelcomeOnboardingStep::InitializeDatabase => {
                let db_type = self.onboarding_target_db_type();
                self.create_db_dialog_state.open(db_type);
            }
            ui::WelcomeOnboardingStep::CreateUser => {
                let db_type = self.onboarding_target_db_type();
                if matches!(db_type, DatabaseType::SQLite) {
                    self.notifications
                        .info("SQLite 无需创建用户，此步骤自动跳过");
                    self.mark_onboarding_user_created();
                    return;
                }
                let databases = self.pick_databases_for_user_dialog(db_type);
                self.create_user_dialog_state.open(db_type, databases);
            }
            ui::WelcomeOnboardingStep::RunFirstQuery => {
                self.run_onboarding_first_query();
            }
        }
    }

    fn onboarding_target_db_type(&self) -> DatabaseType {
        if let Some(conn) = self.manager.get_active() {
            return conn.config.db_type;
        }
        self.recommended_onboarding_db_type()
    }

    fn recommended_onboarding_db_type(&self) -> DatabaseType {
        if matches!(
            self.welcome_setup_target,
            DatabaseType::PostgreSQL | DatabaseType::MySQL
        ) {
            return self.welcome_setup_target;
        }

        match self.welcome_status.postgres {
            ui::WelcomeServiceState::Running | ui::WelcomeServiceState::InstalledNotRunning => {
                return DatabaseType::PostgreSQL;
            }
            ui::WelcomeServiceState::BuiltIn | ui::WelcomeServiceState::NotDetected => {}
        }

        match self.welcome_status.mysql {
            ui::WelcomeServiceState::Running | ui::WelcomeServiceState::InstalledNotRunning => {
                return DatabaseType::MySQL;
            }
            ui::WelcomeServiceState::BuiltIn | ui::WelcomeServiceState::NotDetected => {}
        }

        DatabaseType::SQLite
    }

    fn run_onboarding_first_query(&mut self) {
        if self.manager.active.is_none() {
            self.notifications
                .warning("请先创建并连接数据库，再执行首条查询");
            self.open_connection_dialog_for(self.onboarding_target_db_type());
            return;
        }

        let sql = "SELECT 1 AS hello;";
        self.sql = sql.to_string();
        self.show_sql_editor = true;
        self.focus_sql_editor = false;
        self.focus_area = ui::FocusArea::DataGrid;
        self.grid_state.focused = true;
        let _ = self.execute(sql.to_string());
        self.notifications.info("已执行首条查询示例");
    }

    /// 渲染欢迎页安装/初始化引导弹窗
    pub(super) fn show_welcome_setup_dialog_window(&mut self, ctx: &egui::Context) {
        if !self.show_welcome_setup_dialog {
            return;
        }

        let db_type = self.welcome_setup_target;
        let status = self.welcome_status.state_for(db_type);
        let mut keep_open = self.show_welcome_setup_dialog;
        let mut close_now = false;

        egui::Window::new(format!("{} 安装与初始化引导", db_type.display_name()))
            .open(&mut keep_open)
            .collapsible(false)
            .resizable(true)
            .default_width(640.0)
            .default_height(500.0)
            .min_width(560.0)
            .min_height(420.0)
            .show(ctx, |ui| {
                let onboarding = self.welcome_onboarding_status();
                ui.label(
                    egui::RichText::new(Self::status_summary_text(status))
                        .strong()
                        .color(Self::status_summary_color(status)),
                );
                ui.add_space(8.0);

                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        let completed = onboarding.completed_steps();
                        let total = onboarding.total_steps();
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("首启闭环进度").strong());
                            ui.label(format!("({}/{})", completed, total));
                        });
                        ui.add_space(4.0);
                        ui.add(
                            egui::ProgressBar::new(completed as f32 / total.max(1) as f32)
                                .desired_width(ui.available_width())
                                .show_percentage(),
                        );
                        ui.add_space(6.0);
                        for step in onboarding.steps() {
                            let done = onboarding.is_step_done(step);
                            let marker = if done { "✓" } else { "○" };
                            ui.label(format!(
                                "{} {}",
                                marker,
                                ui::WelcomeOnboardingStatus::step_label(step)
                            ));
                        }
                        if let Some(next_step) = onboarding.next_step() {
                            ui.add_space(8.0);
                            if ui
                                .button(ui::WelcomeOnboardingStatus::action_label(next_step))
                                .clicked()
                            {
                                self.handle_onboarding_step(next_step);
                            }
                        }
                    });

                ui.add_space(10.0);

                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("安装步骤").strong());
                        ui.add_space(6.0);
                        match db_type {
                            DatabaseType::SQLite => {
                                ui.label("SQLite 由 Gridix 内置支持，不需要单独安装服务。");
                                ui.label("你只需要创建一个本地 .db/.sqlite3 文件即可开始。");
                            }
                            DatabaseType::PostgreSQL => {
                                ui.label("Windows: 可用 winget 安装 PostgreSQL 官方包。");
                                ui.label("macOS: 可用 brew 安装并启动 postgresql。");
                                ui.label("Linux: 可用 apt/dnf/pacman 安装 postgresql 服务。");
                                ui.label(
                                    "安装后请确认 5432 端口服务已启动，再回到 Gridix 新建连接。",
                                );
                            }
                            DatabaseType::MySQL => {
                                ui.label("Windows: 可用 winget 安装 MySQL 或 MariaDB。");
                                ui.label("macOS: 可用 brew 安装 mysql 并启动服务。");
                                ui.label("Linux: 可用 apt/dnf/pacman 安装 mysql/mariadb 服务。");
                                ui.label(
                                    "安装后请确认 3306 端口服务已启动，再回到 Gridix 新建连接。",
                                );
                            }
                        }
                    });

                ui.add_space(10.0);
                egui::Frame::group(ui.style())
                    .inner_margin(egui::Margin::symmetric(12, 10))
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("初始化与账号").strong());
                        ui.add_space(6.0);
                        match db_type {
                            DatabaseType::SQLite => {
                                ui.label("SQLite 不需要用户管理。建议先创建一个学习数据库文件。");
                            }
                            DatabaseType::PostgreSQL | DatabaseType::MySQL => {
                                ui.label("安装完成后建议先创建业务数据库，再创建应用用户并授权。");
                                ui.label("如果你还没连接到数据库，先点击“打开新建连接”。");
                            }
                        }
                    });

                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("重新检测环境").clicked() {
                        self.refresh_welcome_environment_status();
                    }

                    if ui.button("打开新建连接").clicked() {
                        self.open_connection_dialog_for(db_type);
                        close_now = true;
                    }

                    if ui.button("初始化数据库").clicked() {
                        self.create_db_dialog_state.open(db_type);
                        close_now = true;
                    }

                    if matches!(db_type, DatabaseType::PostgreSQL | DatabaseType::MySQL) {
                        if ui.button("创建用户").clicked() {
                            let databases = self.pick_databases_for_user_dialog(db_type);
                            self.create_user_dialog_state.open(db_type, databases);
                            close_now = true;
                        }
                    }

                    if ui.button("执行首条查询").clicked() {
                        self.run_onboarding_first_query();
                        close_now = true;
                    }
                });
            });

        self.show_welcome_setup_dialog = keep_open && !close_now;
    }

    fn open_connection_dialog_for(&mut self, db_type: DatabaseType) {
        let mut config = match db_type {
            DatabaseType::SQLite => {
                let path = default_sqlite_path();
                let mut cfg = ConnectionConfig::new("SQLite 本地", DatabaseType::SQLite);
                cfg.database = path.to_string_lossy().into_owned();
                cfg
            }
            DatabaseType::PostgreSQL => {
                let mut cfg = ConnectionConfig::new("PostgreSQL 本地", DatabaseType::PostgreSQL);
                cfg.host = "localhost".to_string();
                cfg.port = 5432;
                cfg.username = "postgres".to_string();
                cfg.database = "postgres".to_string();
                cfg
            }
            DatabaseType::MySQL => {
                let mut cfg = ConnectionConfig::new("MySQL 本地", DatabaseType::MySQL);
                cfg.host = "localhost".to_string();
                cfg.port = 3306;
                cfg.username = "root".to_string();
                cfg.database = "mysql".to_string();
                cfg
            }
        };

        let candidate = config.name.clone();
        if self.manager.connections.contains_key(&candidate) {
            let mut idx = 2usize;
            while self
                .manager
                .connections
                .contains_key(&format!("{} {}", candidate, idx))
            {
                idx += 1;
            }
            config.name = format!("{} {}", candidate, idx);
        }

        self.new_config = config;
        self.editing_connection_name = None;
        self.show_connection_dialog = true;
    }

    fn pick_databases_for_user_dialog(&self, db_type: DatabaseType) -> Vec<String> {
        if let Some(conn) = self.manager.get_active()
            && conn.config.db_type == db_type
            && !conn.databases.is_empty()
        {
            return conn.databases.clone();
        }

        for conn in self.manager.connections.values() {
            if conn.config.db_type == db_type && !conn.databases.is_empty() {
                return conn.databases.clone();
            }
        }

        match db_type {
            DatabaseType::PostgreSQL => vec!["postgres".to_string()],
            DatabaseType::MySQL => vec!["mysql".to_string()],
            DatabaseType::SQLite => Vec::new(),
        }
    }

    fn detect_postgres_status() -> ui::WelcomeServiceState {
        let port_open = Self::probe_local_port(5432);
        let command_found = port_open
            || Self::command_available("psql", &["--version"])
            || Self::command_available("postgres", &["--version"])
            || Self::command_available("pg_ctl", &["--version"]);

        if port_open {
            ui::WelcomeServiceState::Running
        } else if command_found {
            ui::WelcomeServiceState::InstalledNotRunning
        } else {
            ui::WelcomeServiceState::NotDetected
        }
    }

    fn detect_mysql_status() -> ui::WelcomeServiceState {
        let port_open = Self::probe_local_port(3306);
        let command_found = port_open
            || Self::command_available("mysql", &["--version"])
            || Self::command_available("mysqld", &["--version"])
            || Self::command_available("mariadb", &["--version"]);

        if port_open {
            ui::WelcomeServiceState::Running
        } else if command_found {
            ui::WelcomeServiceState::InstalledNotRunning
        } else {
            ui::WelcomeServiceState::NotDetected
        }
    }

    fn probe_local_port(port: u16) -> bool {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        TcpStream::connect_timeout(&addr, Duration::from_millis(220)).is_ok()
    }

    fn command_available(program: &str, args: &[&str]) -> bool {
        Command::new(program)
            .args(args)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn status_summary_text(status: ui::WelcomeServiceState) -> &'static str {
        match status {
            ui::WelcomeServiceState::BuiltIn => "当前状态：内置支持（无需安装）",
            ui::WelcomeServiceState::Running => "当前状态：已检测到本机数据库服务",
            ui::WelcomeServiceState::InstalledNotRunning => "当前状态：已安装，但本机服务未启动",
            ui::WelcomeServiceState::NotDetected => "当前状态：未检测到本机安装",
        }
    }

    fn status_summary_color(status: ui::WelcomeServiceState) -> egui::Color32 {
        match status {
            ui::WelcomeServiceState::BuiltIn | ui::WelcomeServiceState::Running => {
                egui::Color32::from_rgb(120, 220, 170)
            }
            ui::WelcomeServiceState::InstalledNotRunning => egui::Color32::from_rgb(240, 190, 110),
            ui::WelcomeServiceState::NotDetected => egui::Color32::from_rgb(235, 130, 130),
        }
    }
}

fn default_sqlite_path() -> PathBuf {
    AppConfig::config_dir()
        .map(|dir| dir.join("gridix-local.sqlite3"))
        .unwrap_or_else(|| PathBuf::from("gridix-local.sqlite3"))
}
