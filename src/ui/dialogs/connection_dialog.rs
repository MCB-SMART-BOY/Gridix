//! 数据库连接对话框
use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStyle, DialogWindow, FormDialogShell,
};
use crate::database::{
    ConnectionConfig, DatabaseType, MySqlSslMode, PostgresSslMode, SshAuthMethod,
};
use crate::ui::styles::{DANGER, GRAY, MUTED, SPACING_MD, SPACING_SM, SUCCESS};
use crate::ui::{LocalShortcut, local_shortcut_text, local_shortcut_tooltip, local_shortcuts_text};
use egui::{self, Color32, CornerRadius, RichText, TextEdit};
use std::path::Path;

/// 输入验证结果
struct ValidationResult {
    is_valid: bool,
    errors: Vec<String>,
}

impl ValidationResult {
    fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
        }
    }

    fn add_error(&mut self, error: impl Into<String>) {
        self.is_valid = false;
        self.errors.push(error.into());
    }
}

/// 验证连接配置
fn validate_config(config: &ConnectionConfig) -> ValidationResult {
    let mut result = ValidationResult::new();

    // 验证连接名称
    if config.name.is_empty() {
        result.add_error("连接名称不能为空");
    } else if config.name.len() > 64 {
        result.add_error("连接名称不能超过 64 个字符");
    }

    match config.db_type {
        DatabaseType::SQLite => {
            // SQLite 验证
            if config.database.is_empty() {
                result.add_error("数据库文件路径不能为空");
            } else {
                let path = Path::new(&config.database);
                // 检查父目录是否存在
                if let Some(parent) = path.parent()
                    && !parent.as_os_str().is_empty()
                    && !parent.exists()
                {
                    result.add_error(format!("目录不存在: {}", parent.display()));
                }
                // 检查文件扩展名
                if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if !["db", "sqlite", "sqlite3", "s3db"].contains(&ext_lower.as_str()) {
                        // 只是警告，不阻止保存
                    }
                }
            }
        }
        DatabaseType::PostgreSQL | DatabaseType::MySQL => {
            // 主机验证
            if config.host.is_empty() {
                result.add_error("主机地址不能为空");
            } else if config.host.contains(' ') {
                result.add_error("主机地址不能包含空格");
            } else if config.host.len() > 255 {
                result.add_error("主机地址过长");
            }

            // 端口验证（u16 类型范围已确保 0-65535）
            if config.port == 0 {
                result.add_error("端口号不能为 0");
            }
            // 注: 小于 1024 的端口是系统保留端口，但某些数据库可能使用

            // 用户名验证（可选但推荐）
            if config.username.len() > 128 {
                result.add_error("用户名过长");
            }
        }
    }

    result
}

pub struct ConnectionDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionDialogAction {
    Close,
    Confirm,
    SetDatabaseType(DatabaseType),
    CycleDatabaseTypePrev,
    CycleDatabaseTypeNext,
    Browse(ConnectionBrowseTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConnectionBrowseTarget {
    SqliteFile,
    CaCertificate,
    SshPrivateKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponsiveRowClass {
    Wide,
    Medium,
    Narrow,
}

struct ResponsivePathRowSpec<'a> {
    label: &'a str,
    hint_text: &'a str,
    button_label: &'a str,
    hover_text: Option<&'a str>,
}

impl ConnectionDialog {
    const WIDE_ROW_THRESHOLD: f32 = 720.0;
    const MEDIUM_ROW_THRESHOLD: f32 = 560.0;

    #[inline]
    fn apply_database_type(config: &mut ConnectionConfig, db_type: DatabaseType) {
        config.db_type = db_type;
        config.port = db_type.default_port();
        if config.host.is_empty() && !matches!(db_type, DatabaseType::SQLite) {
            config.host = "localhost".to_string();
        }
    }

    fn detect_key_action(
        ctx: &egui::Context,
        config: &ConnectionConfig,
        can_confirm: bool,
    ) -> Option<ConnectionDialogAction> {
        let shortcuts = DialogShortcutContext::new(ctx);

        if shortcuts.consume_command(LocalShortcut::Dismiss.config_key()) {
            return Some(ConnectionDialogAction::Close);
        }
        if can_confirm && shortcuts.consume_command(LocalShortcut::Confirm.config_key()) {
            return Some(ConnectionDialogAction::Confirm);
        }

        if let Some(action) = shortcuts.resolve_commands(&[
            (
                LocalShortcut::ConnectionTypeSqlite.config_key(),
                ConnectionDialogAction::SetDatabaseType(DatabaseType::SQLite),
            ),
            (
                LocalShortcut::ConnectionTypePostgres.config_key(),
                ConnectionDialogAction::SetDatabaseType(DatabaseType::PostgreSQL),
            ),
            (
                LocalShortcut::ConnectionTypeMySql.config_key(),
                ConnectionDialogAction::SetDatabaseType(DatabaseType::MySQL),
            ),
            (
                LocalShortcut::ConnectionTypePrev.config_key(),
                ConnectionDialogAction::CycleDatabaseTypePrev,
            ),
            (
                LocalShortcut::ConnectionTypeNext.config_key(),
                ConnectionDialogAction::CycleDatabaseTypeNext,
            ),
        ]) {
            return Some(action);
        }

        if matches!(config.db_type, DatabaseType::SQLite)
            && shortcuts.consume_command(LocalShortcut::SqliteBrowseFile.config_key())
        {
            return Some(ConnectionDialogAction::Browse(
                ConnectionBrowseTarget::SqliteFile,
            ));
        }

        None
    }

    fn cycle_database_type(config: &mut ConnectionConfig, direction: isize) {
        let db_types = DatabaseType::all();
        let current_idx = db_types
            .iter()
            .position(|db_type| *db_type == config.db_type)
            .unwrap_or(0);
        let next_idx = current_idx as isize + direction;
        if next_idx >= 0 && next_idx < db_types.len() as isize {
            Self::apply_database_type(config, db_types[next_idx as usize]);
        }
    }

    fn browse_target_path(target: ConnectionBrowseTarget) -> Option<String> {
        let path = match target {
            ConnectionBrowseTarget::SqliteFile => rfd::FileDialog::new()
                .add_filter("SQLite 数据库", &["db", "sqlite", "sqlite3"])
                .add_filter("所有文件", &["*"])
                .pick_file(),
            ConnectionBrowseTarget::CaCertificate => rfd::FileDialog::new()
                .add_filter("证书文件", &["pem", "crt", "cer"])
                .add_filter("所有文件", &["*"])
                .pick_file(),
            ConnectionBrowseTarget::SshPrivateKey => rfd::FileDialog::new()
                .add_filter("私钥文件", &["pem", "key", "*"])
                .pick_file(),
        }?;

        Some(path.display().to_string())
    }

    fn apply_browse_target(
        config: &mut ConnectionConfig,
        target: ConnectionBrowseTarget,
        selected_path: String,
    ) {
        match target {
            ConnectionBrowseTarget::SqliteFile => config.database = selected_path,
            ConnectionBrowseTarget::CaCertificate => config.ssl_ca_cert = selected_path,
            ConnectionBrowseTarget::SshPrivateKey => {
                config.ssh_config.private_key_path = selected_path
            }
        }
    }

    fn apply_dialog_action(
        action: ConnectionDialogAction,
        config: &mut ConnectionConfig,
        on_save: &mut bool,
        should_close: &mut bool,
    ) {
        match action {
            ConnectionDialogAction::Close => {
                *should_close = true;
            }
            ConnectionDialogAction::Confirm => {
                *on_save = true;
                *should_close = true;
            }
            ConnectionDialogAction::SetDatabaseType(db_type) => {
                Self::apply_database_type(config, db_type);
            }
            ConnectionDialogAction::CycleDatabaseTypePrev => {
                Self::cycle_database_type(config, -1);
            }
            ConnectionDialogAction::CycleDatabaseTypeNext => {
                Self::cycle_database_type(config, 1);
            }
            ConnectionDialogAction::Browse(target) => {
                if let Some(path) = Self::browse_target_path(target) {
                    Self::apply_browse_target(config, target, path);
                }
            }
        }
    }

    pub fn show(
        ctx: &egui::Context,
        open: &mut bool,
        show_advanced: &mut bool,
        config: &mut ConnectionConfig,
        on_save: &mut bool,
        is_edit_mode: bool,
    ) {
        let mut is_open = *open;
        let mut should_close = false;

        // 键盘快捷键处理（文本输入优先于普通命令键）
        let validation = validate_config(config);
        if let Some(key_action) = Self::detect_key_action(ctx, config, validation.is_valid) {
            Self::apply_dialog_action(key_action, config, on_save, &mut should_close);
            if should_close {
                *open = false;
                return;
            }
        }

        let dialog_title = if is_edit_mode {
            "🔗 编辑数据库连接"
        } else {
            "🔗 新建数据库连接"
        };

        let style = DialogStyle::LARGE;
        let footer_validation = validate_config(config);
        let mut click_action = None;
        DialogWindow::standard(ctx, dialog_title, &style)
            .open(&mut is_open)
            .show(ctx, |ui| {
                FormDialogShell::show(
                    ui,
                    "connection_dialog_form_shell",
                    |ui| {
                        DialogContent::shortcut_hint(
                            ui,
                            &[
                                (local_shortcut_text(LocalShortcut::Dismiss).as_str(), "关闭"),
                                (local_shortcut_text(LocalShortcut::Confirm).as_str(), "保存"),
                            ],
                        );
                    },
                    |ui, _body_ctx| {
                        DialogContent::section_with_description(
                            ui,
                            "数据库类型",
                            "先确定连接类型，再补齐对应的主机、文件或认证字段。",
                            |ui| Self::show_db_type_selector(ui, config),
                        );

                        DialogContent::section_with_description(
                            ui,
                            "连接信息",
                            "核心字段始终固定在前，高级能力单独放在后面。",
                            |ui| {
                                if click_action.is_none() {
                                    click_action = Self::show_connection_form(ui, config);
                                } else {
                                    let _ = Self::show_connection_form(ui, config);
                                }
                            },
                        );

                        DialogContent::section_with_description(
                            ui,
                            "实时检查",
                            "保存前先给出最小可用性检查，避免无效配置直接落盘。",
                            |ui| Self::show_realtime_checklist(ui, config),
                        );

                        Self::show_advanced_toggle(ui, show_advanced);
                        ui.add_space(SPACING_MD);

                        if *show_advanced {
                            match config.db_type {
                                DatabaseType::MySQL => {
                                    DialogContent::section_with_description(
                                        ui,
                                        "SSL / TLS",
                                        "加密链路配置与连接核心参数分离，避免信息堆叠。",
                                        |ui| {
                                            if click_action.is_none() {
                                                click_action =
                                                    Self::show_mysql_ssl_config(ui, config);
                                            } else {
                                                let _ = Self::show_mysql_ssl_config(ui, config);
                                            }
                                        },
                                    );
                                }
                                DatabaseType::PostgreSQL => {
                                    DialogContent::section_with_description(
                                        ui,
                                        "SSL / TLS",
                                        "按 PostgreSQL 语义展示验证级别和证书字段。",
                                        |ui| {
                                            if click_action.is_none() {
                                                click_action =
                                                    Self::show_postgres_ssl_config(ui, config);
                                            } else {
                                                let _ = Self::show_postgres_ssl_config(ui, config);
                                            }
                                        },
                                    );
                                }
                                DatabaseType::SQLite => {}
                            }

                            if !matches!(config.db_type, DatabaseType::SQLite) {
                                DialogContent::section_with_description(
                                    ui,
                                    "SSH 隧道",
                                    "SSH 只在需要时展开，避免常规直连场景被额外字段干扰。",
                                    |ui| {
                                        if click_action.is_none() {
                                            click_action = Self::show_ssh_tunnel_config(ui, config);
                                        } else {
                                            let _ = Self::show_ssh_tunnel_config(ui, config);
                                        }
                                    },
                                );
                            }

                            DialogContent::section_with_description(
                                ui,
                                "连接字符串预览",
                                "用于快速校验目标地址和方言，敏感字段会自动脱敏。",
                                |ui| Self::show_connection_preview(ui, config),
                            );
                        } else {
                            DialogContent::info_text(
                                ui,
                                "已隐藏 SSL、SSH 和连接字符串预览，聚焦核心连接字段。",
                            );
                            ui.add_space(SPACING_SM);
                        }
                    },
                    |ui| {
                        Self::show_buttons(
                            ui,
                            &footer_validation,
                            on_save,
                            &mut should_close,
                            is_edit_mode,
                            &style,
                        );
                    },
                );
            });

        if let Some(action) = click_action {
            Self::apply_dialog_action(action, config, on_save, &mut should_close);
        }

        if should_close {
            is_open = false;
        }
        *open = is_open;
    }

    fn show_advanced_toggle(ui: &mut egui::Ui, show_advanced: &mut bool) {
        DialogContent::toolbar(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .button(if *show_advanced {
                        "收起高级配置"
                    } else {
                        "显示高级配置（SSL / SSH / 连接串）"
                    })
                    .clicked()
                {
                    *show_advanced = !*show_advanced;
                }
                ui.label(RichText::new("默认只保留核心连接字段").small().color(MUTED));
            });
        });
    }

    /// 数据库类型选择器
    fn show_db_type_selector(ui: &mut egui::Ui, config: &mut ConnectionConfig) {
        let db_type_shortcuts = local_shortcuts_text(&[
            LocalShortcut::ConnectionTypeSqlite,
            LocalShortcut::ConnectionTypePostgres,
            LocalShortcut::ConnectionTypeMySql,
            LocalShortcut::ConnectionTypePrev,
            LocalShortcut::ConnectionTypeNext,
        ]);

        // 快捷键提示
        ui.horizontal(|ui| {
            ui.add_space(SPACING_SM);
            ui.label(
                RichText::new(format!("数据库类型 [{}]", db_type_shortcuts))
                    .small()
                    .color(MUTED),
            );
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.add_space(SPACING_SM);

            for (idx, db_type) in DatabaseType::all().iter().enumerate() {
                let is_selected = config.db_type == *db_type;
                let (icon, name, color, key) = match db_type {
                    DatabaseType::SQLite => ("🗃️", "SQLite", Color32::from_rgb(80, 160, 220), "1"),
                    DatabaseType::PostgreSQL => {
                        ("🐘", "PostgreSQL", Color32::from_rgb(80, 130, 180), "2")
                    }
                    DatabaseType::MySQL => ("🐬", "MySQL", Color32::from_rgb(240, 150, 80), "3"),
                };
                let _ = idx; // 用于后续扩展

                let fill = if is_selected {
                    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 40)
                } else {
                    Color32::TRANSPARENT
                };

                let stroke = if is_selected {
                    egui::Stroke::new(2.0, color)
                } else {
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(150, 150, 160, 50))
                };

                let shortcut = match db_type {
                    DatabaseType::SQLite => LocalShortcut::ConnectionTypeSqlite,
                    DatabaseType::PostgreSQL => LocalShortcut::ConnectionTypePostgres,
                    DatabaseType::MySQL => LocalShortcut::ConnectionTypeMySql,
                };

                let response = egui::Frame::NONE
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(16, 10))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon).size(18.0));
                            ui.add_space(4.0);
                            let text_color = if is_selected { color } else { GRAY };
                            ui.label(
                                RichText::new(format!("[{}] {}", key, name))
                                    .strong()
                                    .color(text_color),
                            );
                        });
                    })
                    .response
                    .interact(egui::Sense::click())
                    .on_hover_text(local_shortcut_tooltip(
                        &format!("切换到 {} 连接类型", name),
                        shortcut,
                    ));

                if response.clicked() {
                    Self::apply_database_type(config, *db_type);
                }

                ui.add_space(SPACING_SM);
            }
        });
    }

    /// 连接表单
    fn show_connection_form(
        ui: &mut egui::Ui,
        config: &mut ConnectionConfig,
    ) -> Option<ConnectionDialogAction> {
        let mut action = None;
        Self::show_responsive_labeled_row(ui, "连接名称", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 320.0);
            ui.add_sized(
                [control_width, 0.0],
                TextEdit::singleline(&mut config.name)
                    .hint_text("我的数据库")
                    .char_limit(64),
            );
        });

        if !matches!(config.db_type, DatabaseType::SQLite) {
            Self::show_responsive_labeled_row(ui, "主机地址", |ui, row_class| {
                let control_width = Self::control_width(ui, row_class, 320.0);
                ui.add_sized(
                    [control_width, 0.0],
                    TextEdit::singleline(&mut config.host)
                        .hint_text("localhost")
                        .char_limit(255),
                );
            });

            Self::show_responsive_labeled_row(ui, "端口", |ui, row_class| {
                let mut port_string = config.port.to_string();
                let control_width = Self::control_width(ui, row_class, 120.0);
                if ui
                    .add_sized(
                        [control_width, 0.0],
                        TextEdit::singleline(&mut port_string).char_limit(5),
                    )
                    .changed()
                    && let Ok(port) = port_string.parse::<u16>()
                {
                    config.port = port;
                }
            });

            Self::show_responsive_labeled_row(ui, "用户名", |ui, row_class| {
                let control_width = Self::control_width(ui, row_class, 320.0);
                ui.add_sized(
                    [control_width, 0.0],
                    TextEdit::singleline(&mut config.username)
                        .hint_text("root")
                        .char_limit(128),
                );
            });

            Self::show_responsive_labeled_row(ui, "密码", |ui, row_class| {
                let control_width = Self::control_width(ui, row_class, 320.0);
                ui.add_sized(
                    [control_width, 0.0],
                    TextEdit::singleline(&mut config.password)
                        .password(true)
                        .char_limit(256),
                );
            });
        }

        if matches!(config.db_type, DatabaseType::SQLite) {
            let sqlite_browse_label = format!(
                "浏览 [{}]",
                local_shortcut_text(LocalShortcut::SqliteBrowseFile)
            );
            let sqlite_browse_tooltip = local_shortcut_tooltip(
                "浏览并选择 SQLite 数据库文件",
                LocalShortcut::SqliteBrowseFile,
            );
            Self::show_responsive_path_row(
                ui,
                ResponsivePathRowSpec {
                    label: "文件路径",
                    hint_text: "/path/to/database.db",
                    button_label: &sqlite_browse_label,
                    hover_text: Some(&sqlite_browse_tooltip),
                },
                &mut config.database,
                |action_slot| {
                    *action_slot = Some(ConnectionDialogAction::Browse(
                        ConnectionBrowseTarget::SqliteFile,
                    ));
                },
                &mut action,
            );
        }

        ui.add_space(SPACING_SM);
        DialogContent::info_text(
            ui,
            match config.db_type {
                DatabaseType::SQLite => "输入 SQLite 文件路径，文件不存在时会按路径创建。",
                DatabaseType::PostgreSQL => "默认端口 5432，保存后会进入数据库选择与连接流程。",
                DatabaseType::MySQL => "默认端口 3306，高级 SSL / SSH 配置可稍后展开。",
            },
        );

        action
    }

    fn show_realtime_checklist(ui: &mut egui::Ui, config: &ConnectionConfig) {
        let mut items: Vec<(&str, bool)> = Vec::new();
        items.push(("连接名称已填写", !config.name.trim().is_empty()));

        match config.db_type {
            DatabaseType::SQLite => {
                items.push(("SQLite 文件路径已填写", !config.database.trim().is_empty()));
            }
            DatabaseType::PostgreSQL | DatabaseType::MySQL => {
                items.push(("主机地址已填写", !config.host.trim().is_empty()));
                items.push(("端口有效（1-65535）", config.port != 0));
                items.push(("用户名已填写", !config.username.trim().is_empty()));
            }
        }

        let passed = items.iter().filter(|(_, ok)| *ok).count();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(format!("已通过 {}/{} 项", passed, items.len()))
                    .small()
                    .color(MUTED),
            );
        });
        ui.add_space(SPACING_SM);

        for (label, ok) in items {
            let color = if ok { SUCCESS } else { DANGER };
            let icon = if ok { "✓" } else { "•" };
            ui.label(
                RichText::new(format!("{} {}", icon, label))
                    .small()
                    .color(color),
            );
        }
    }

    /// MySQL SSL 配置
    fn show_mysql_ssl_config(
        ui: &mut egui::Ui,
        config: &mut ConnectionConfig,
    ) -> Option<ConnectionDialogAction> {
        let mut action = None;
        Self::show_responsive_labeled_row(ui, "SSL 模式", |ui, row_class| {
            let combo_width = Self::control_width(ui, row_class, 320.0);
            egui::ComboBox::new("ssl_mode_combo", "")
                .selected_text(config.mysql_ssl_mode.display_name())
                .width(combo_width)
                .show_ui(ui, |ui| {
                    for mode in MySqlSslMode::all() {
                        let label = format!("{} - {}", mode.display_name(), mode.description());
                        ui.selectable_value(&mut config.mysql_ssl_mode, *mode, label);
                    }
                });
        });

        if matches!(
            config.mysql_ssl_mode,
            MySqlSslMode::VerifyCa | MySqlSslMode::VerifyIdentity
        ) {
            Self::show_responsive_path_row(
                ui,
                ResponsivePathRowSpec {
                    label: "CA 证书",
                    hint_text: "/path/to/ca-cert.pem",
                    button_label: "浏览",
                    hover_text: None,
                },
                &mut config.ssl_ca_cert,
                |action_slot| {
                    *action_slot = Some(ConnectionDialogAction::Browse(
                        ConnectionBrowseTarget::CaCertificate,
                    ));
                },
                &mut action,
            );
        }

        ui.add_space(SPACING_SM);
        DialogContent::info_text(
            ui,
            match config.mysql_ssl_mode {
                MySqlSslMode::Disabled => "不使用加密，数据以明文传输。",
                MySqlSslMode::Preferred => "优先使用 SSL，服务端不支持时回退为明文。",
                MySqlSslMode::Required => "必须使用 SSL，但不验证服务器证书。",
                MySqlSslMode::VerifyCa => "验证服务器 CA 证书，不检查主机名。",
                MySqlSslMode::VerifyIdentity => "同时验证 CA 证书与服务器主机名。",
            },
        );

        action
    }

    /// PostgreSQL SSL 配置
    fn show_postgres_ssl_config(
        ui: &mut egui::Ui,
        config: &mut ConnectionConfig,
    ) -> Option<ConnectionDialogAction> {
        let mut action = None;
        Self::show_responsive_labeled_row(ui, "SSL 模式", |ui, row_class| {
            let combo_width = Self::control_width(ui, row_class, 320.0);
            egui::ComboBox::new("pg_ssl_mode_combo", "")
                .selected_text(config.postgres_ssl_mode.display_name())
                .width(combo_width)
                .show_ui(ui, |ui| {
                    for mode in PostgresSslMode::all() {
                        let label = format!("{} - {}", mode.display_name(), mode.description());
                        ui.selectable_value(&mut config.postgres_ssl_mode, *mode, label);
                    }
                });
        });

        if matches!(
            config.postgres_ssl_mode,
            PostgresSslMode::VerifyCa | PostgresSslMode::VerifyFull
        ) {
            Self::show_responsive_path_row(
                ui,
                ResponsivePathRowSpec {
                    label: "CA 证书",
                    hint_text: "/path/to/ca-cert.pem",
                    button_label: "浏览",
                    hover_text: None,
                },
                &mut config.ssl_ca_cert,
                |action_slot| {
                    *action_slot = Some(ConnectionDialogAction::Browse(
                        ConnectionBrowseTarget::CaCertificate,
                    ));
                },
                &mut action,
            );
        }

        ui.add_space(SPACING_SM);
        DialogContent::info_text(
            ui,
            match config.postgres_ssl_mode {
                PostgresSslMode::Disable => "不使用加密，数据以明文传输。",
                PostgresSslMode::Prefer => "优先使用 SSL，服务端不支持时回退为明文。",
                PostgresSslMode::Require => "必须使用 SSL，但不验证服务器证书。",
                PostgresSslMode::VerifyCa => "验证服务器 CA 证书，不检查主机名。",
                PostgresSslMode::VerifyFull => "同时验证 CA 证书与服务器主机名。",
            },
        );

        action
    }

    /// SSH 隧道配置
    fn show_ssh_tunnel_config(
        ui: &mut egui::Ui,
        config: &mut ConnectionConfig,
    ) -> Option<ConnectionDialogAction> {
        ui.checkbox(&mut config.ssh_config.enabled, "启用 SSH 隧道");

        if !config.ssh_config.enabled {
            DialogContent::info_text(ui, "关闭时将直接连接数据库地址，不经过跳板机。");
            return None;
        }

        let mut action = None;
        ui.add_space(SPACING_SM);
        Self::show_responsive_labeled_row(ui, "SSH 主机", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 260.0);
            ui.add_sized(
                [control_width, 0.0],
                TextEdit::singleline(&mut config.ssh_config.ssh_host).hint_text("跳板机地址"),
            );
        });

        Self::show_responsive_labeled_row(ui, "SSH 端口", |ui, row_class| {
            let mut port_str = config.ssh_config.ssh_port.to_string();
            let control_width = Self::control_width(ui, row_class, 120.0);
            if ui
                .add_sized([control_width, 0.0], TextEdit::singleline(&mut port_str))
                .changed()
                && let Ok(port) = port_str.parse::<u16>()
            {
                config.ssh_config.ssh_port = port;
            }
        });

        Self::show_responsive_labeled_row(ui, "SSH 用户名", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 260.0);
            ui.add_sized(
                [control_width, 0.0],
                TextEdit::singleline(&mut config.ssh_config.ssh_username).hint_text("用户名"),
            );
        });

        Self::show_responsive_labeled_row(ui, "认证方式", |ui, row_class| match row_class {
            ResponsiveRowClass::Narrow => {
                ui.vertical(|ui| {
                    ui.selectable_value(
                        &mut config.ssh_config.auth_method,
                        SshAuthMethod::Password,
                        SshAuthMethod::Password.display_name(),
                    );
                    ui.selectable_value(
                        &mut config.ssh_config.auth_method,
                        SshAuthMethod::PrivateKey,
                        SshAuthMethod::PrivateKey.display_name(),
                    );
                });
            }
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(
                        &mut config.ssh_config.auth_method,
                        SshAuthMethod::Password,
                        SshAuthMethod::Password.display_name(),
                    );
                    ui.selectable_value(
                        &mut config.ssh_config.auth_method,
                        SshAuthMethod::PrivateKey,
                        SshAuthMethod::PrivateKey.display_name(),
                    );
                });
            }
        });

        match config.ssh_config.auth_method {
            SshAuthMethod::Password => {
                Self::show_responsive_labeled_row(ui, "SSH 密码", |ui, row_class| {
                    let control_width = Self::control_width(ui, row_class, 260.0);
                    ui.add_sized(
                        [control_width, 0.0],
                        TextEdit::singleline(&mut config.ssh_config.ssh_password).password(true),
                    );
                });
            }
            SshAuthMethod::PrivateKey => {
                Self::show_responsive_path_row(
                    ui,
                    ResponsivePathRowSpec {
                        label: "私钥路径",
                        hint_text: "~/.ssh/id_rsa",
                        button_label: "浏览",
                        hover_text: None,
                    },
                    &mut config.ssh_config.private_key_path,
                    |action_slot| {
                        *action_slot = Some(ConnectionDialogAction::Browse(
                            ConnectionBrowseTarget::SshPrivateKey,
                        ));
                    },
                    &mut action,
                );

                Self::show_responsive_labeled_row(ui, "私钥密码", |ui, row_class| {
                    let control_width = Self::control_width(ui, row_class, 260.0);
                    ui.add_sized(
                        [control_width, 0.0],
                        TextEdit::singleline(&mut config.ssh_config.private_key_passphrase)
                            .password(true)
                            .hint_text("（可选）"),
                    );
                });
            }
        }

        Self::show_responsive_labeled_row(ui, "远程主机", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 260.0);
            ui.add_sized(
                [control_width, 0.0],
                TextEdit::singleline(&mut config.ssh_config.remote_host)
                    .hint_text("数据库主机（如 127.0.0.1）"),
            );
        });

        Self::show_responsive_labeled_row(ui, "远程端口", |ui, row_class| {
            let mut remote_port_str = config.ssh_config.remote_port.to_string();
            let control_width = Self::control_width(ui, row_class, 120.0);
            if ui
                .add_sized(
                    [control_width, 0.0],
                    TextEdit::singleline(&mut remote_port_str).hint_text("数据库端口"),
                )
                .changed()
                && let Ok(port) = remote_port_str.parse::<u16>()
            {
                config.ssh_config.remote_port = port;
            }
        });

        ui.add_space(SPACING_SM);
        DialogContent::info_text(
            ui,
            "启用后，数据库连接会先连到 SSH 跳板机，再转发到远程数据库地址。",
        );

        action
    }

    /// 连接字符串预览
    fn show_connection_preview(ui: &mut egui::Ui, config: &ConnectionConfig) {
        let conn_str = config.connection_string();
        let display_str = if !config.password.is_empty() {
            conn_str.replace(&config.password, "****")
        } else {
            conn_str
        };
        DialogContent::code_block(ui, &display_str, 120.0);
    }

    /// 底部按钮
    fn show_buttons(
        ui: &mut egui::Ui,
        validation: &ValidationResult,
        on_save: &mut bool,
        should_close: &mut bool,
        is_edit_mode: bool,
        style: &DialogStyle,
    ) {
        if let Some(error) = validation.errors.first() {
            DialogContent::warning_text(ui, error);
            ui.add_space(SPACING_SM);
        }

        let footer = DialogFooter::show(
            ui,
            &if is_edit_mode {
                format!(
                    "保存并重连 [{}]",
                    local_shortcut_text(LocalShortcut::Confirm)
                )
            } else {
                format!(
                    "保存并连接 [{}]",
                    local_shortcut_text(LocalShortcut::Confirm)
                )
            },
            &format!("取消 [{}]", local_shortcut_text(LocalShortcut::Dismiss)),
            validation.is_valid,
            style,
        );

        if footer.cancelled {
            *should_close = true;
        }
        if footer.confirmed {
            *on_save = true;
            *should_close = true;
        }
    }

    fn row_width_class(available_width: f32) -> ResponsiveRowClass {
        if available_width >= Self::WIDE_ROW_THRESHOLD {
            ResponsiveRowClass::Wide
        } else if available_width >= Self::MEDIUM_ROW_THRESHOLD {
            ResponsiveRowClass::Medium
        } else {
            ResponsiveRowClass::Narrow
        }
    }

    fn label_width(row_class: ResponsiveRowClass) -> f32 {
        match row_class {
            ResponsiveRowClass::Wide => 96.0,
            ResponsiveRowClass::Medium => 88.0,
            ResponsiveRowClass::Narrow => 0.0,
        }
    }

    fn control_width(ui: &egui::Ui, row_class: ResponsiveRowClass, preferred_width: f32) -> f32 {
        match row_class {
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                ui.available_width().min(preferred_width)
            }
            ResponsiveRowClass::Narrow => ui.available_width(),
        }
    }

    fn show_responsive_labeled_row(
        ui: &mut egui::Ui,
        label: &str,
        body: impl FnOnce(&mut egui::Ui, ResponsiveRowClass),
    ) {
        let row_class = Self::row_width_class(ui.available_width());

        match row_class {
            ResponsiveRowClass::Narrow => {
                ui.label(RichText::new(label).color(GRAY));
                ui.add_space(4.0);
                body(ui, row_class);
            }
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                let label_width = Self::label_width(row_class);
                ui.horizontal_top(|ui| {
                    ui.add_sized(
                        [label_width, 0.0],
                        egui::Label::new(RichText::new(label).color(GRAY)),
                    );
                    ui.add_space(SPACING_SM);
                    ui.vertical(|ui| {
                        body(ui, row_class);
                    });
                });
            }
        }

        ui.add_space(SPACING_SM);
    }

    fn show_responsive_path_row(
        ui: &mut egui::Ui,
        spec: ResponsivePathRowSpec<'_>,
        value: &mut String,
        on_button_clicked: impl FnOnce(&mut Option<ConnectionDialogAction>),
        action_slot: &mut Option<ConnectionDialogAction>,
    ) {
        let mut on_button_clicked = Some(on_button_clicked);
        Self::show_responsive_labeled_row(ui, spec.label, |ui, row_class| match row_class {
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                ui.horizontal(|ui| {
                    let button_width = 112.0;
                    let field_width = (ui.available_width() - button_width - SPACING_SM).max(120.0);
                    ui.add_sized(
                        [field_width, 0.0],
                        TextEdit::singleline(value).hint_text(spec.hint_text),
                    );

                    let mut response = ui.add_sized(
                        [button_width, 0.0],
                        egui::Button::new(spec.button_label).corner_radius(CornerRadius::same(4)),
                    );
                    if let Some(hover_text) = spec.hover_text {
                        response = response.on_hover_text(hover_text);
                    }
                    if response.clicked()
                        && let Some(handler) = on_button_clicked.take()
                    {
                        handler(action_slot);
                    }
                });
            }
            ResponsiveRowClass::Narrow => {
                ui.add_sized(
                    [ui.available_width(), 0.0],
                    TextEdit::singleline(value).hint_text(spec.hint_text),
                );
                ui.add_space(4.0);

                let button_width = ui.available_width().min(140.0);
                let mut response = ui.add_sized(
                    [button_width, 0.0],
                    egui::Button::new(spec.button_label).corner_radius(CornerRadius::same(4)),
                );
                if let Some(hover_text) = spec.hover_text {
                    response = response.on_hover_text(hover_text);
                }
                if response.clicked()
                    && let Some(handler) = on_button_clicked.take()
                {
                    handler(action_slot);
                }
            }
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

    fn begin_key_pass_with_modifiers(ctx: &egui::Context, key: Key, modifiers: Modifiers) {
        ctx.begin_pass(RawInput {
            events: vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            modifiers,
            ..Default::default()
        });
    }

    fn ctrl_modifiers() -> Modifiers {
        Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        }
    }

    fn focus_text_input(ctx: &egui::Context) {
        let mut text = String::new();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("connection dialog shortcut test input").show(ctx, |ui| {
            let response = ui.add(
                egui::TextEdit::singleline(&mut text).id_salt("connection_shortcut_text_input"),
            );
            response.request_focus();
        });
        let _ = ctx.end_pass();
    }

    fn valid_server_config() -> ConnectionConfig {
        ConnectionConfig {
            name: "pg".to_string(),
            db_type: DatabaseType::PostgreSQL,
            host: "localhost".to_string(),
            port: 5432,
            username: "postgres".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn connection_dialog_detects_database_type_shortcut_through_scoped_command_id() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Num2);

        let action = ConnectionDialog::detect_key_action(&ctx, &ConnectionConfig::default(), false);

        assert_eq!(
            action,
            Some(ConnectionDialogAction::SetDatabaseType(
                DatabaseType::PostgreSQL,
            ))
        );

        let _ = ctx.end_pass();
    }

    #[test]
    fn connection_dialog_confirm_requires_valid_config() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Enter);

        let action = ConnectionDialog::detect_key_action(&ctx, &ConnectionConfig::default(), false);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn connection_dialog_sqlite_browse_shortcut_only_enabled_for_sqlite() {
        let ctx = egui::Context::default();
        begin_key_pass_with_modifiers(&ctx, Key::O, ctrl_modifiers());
        let sqlite_config = ConnectionConfig::new("", DatabaseType::SQLite);

        let action = ConnectionDialog::detect_key_action(&ctx, &sqlite_config, false);

        assert_eq!(
            action,
            Some(ConnectionDialogAction::Browse(
                ConnectionBrowseTarget::SqliteFile,
            ))
        );

        let _ = ctx.end_pass();

        let ctx = egui::Context::default();
        begin_key_pass_with_modifiers(&ctx, Key::O, ctrl_modifiers());

        let action = ConnectionDialog::detect_key_action(&ctx, &valid_server_config(), true);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn connection_dialog_blocks_type_cycle_text_conflicts_when_text_input_is_focused() {
        let ctx = egui::Context::default();
        focus_text_input(&ctx);
        begin_key_pass(&ctx, Key::H);

        let action = ConnectionDialog::detect_key_action(&ctx, &valid_server_config(), true);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn responsive_row_width_classes_follow_design_thresholds() {
        assert_eq!(
            ConnectionDialog::row_width_class(ConnectionDialog::WIDE_ROW_THRESHOLD),
            ResponsiveRowClass::Wide
        );
        assert_eq!(
            ConnectionDialog::row_width_class(680.0),
            ResponsiveRowClass::Medium
        );
        assert_eq!(
            ConnectionDialog::row_width_class(ConnectionDialog::MEDIUM_ROW_THRESHOLD - 1.0),
            ResponsiveRowClass::Narrow
        );
    }
}
