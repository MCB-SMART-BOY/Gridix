//! 数据库连接对话框

use super::keyboard::{self, DialogAction};
use crate::database::{
    ConnectionConfig, DatabaseType, MySqlSslMode, PostgresSslMode, SshAuthMethod,
};
use crate::ui::styles::{DANGER, GRAY, MUTED, SPACING_LG, SPACING_MD, SPACING_SM, SUCCESS};
use crate::ui::{LocalShortcut, local_shortcut_text, local_shortcut_tooltip};
use egui::{self, Color32, CornerRadius, Key, Modifiers, RichText, TextEdit};
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

impl ConnectionDialog {
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

        // 键盘快捷键处理（仅在没有文本框焦点时）
        if !keyboard::has_text_focus(ctx) {
            // Esc/q 关闭
            if keyboard::handle_close_keys(ctx) {
                *open = false;
                return;
            }

            // Enter 保存（如果验证通过）
            let validation = validate_config(config);
            if validation.is_valid
                && let DialogAction::Confirm = keyboard::handle_dialog_keys(ctx)
            {
                *on_save = true;
                *open = false;
                return;
            }

            // 数据库类型快捷键
            let db_types = DatabaseType::all();
            ctx.input(|i| {
                // 数字键 1/2/3 选择数据库类型
                for (idx, key) in [Key::Num1, Key::Num2, Key::Num3].iter().enumerate() {
                    if i.key_pressed(*key)
                        && i.modifiers.is_none()
                        && let Some(db_type) = db_types.get(idx)
                    {
                        config.db_type = *db_type;
                        config.port = db_type.default_port();
                        if config.host.is_empty() && !matches!(db_type, DatabaseType::SQLite) {
                            config.host = "localhost".to_string();
                        }
                    }
                }

                // h/l 切换数据库类型
                if i.key_pressed(Key::H) && i.modifiers.is_none() {
                    let current_idx = db_types
                        .iter()
                        .position(|t| *t == config.db_type)
                        .unwrap_or(0);
                    if current_idx > 0 {
                        let new_type = db_types[current_idx - 1];
                        config.db_type = new_type;
                        config.port = new_type.default_port();
                        if config.host.is_empty() && !matches!(new_type, DatabaseType::SQLite) {
                            config.host = "localhost".to_string();
                        }
                    }
                }
                if i.key_pressed(Key::L) && i.modifiers.is_none() {
                    let current_idx = db_types
                        .iter()
                        .position(|t| *t == config.db_type)
                        .unwrap_or(0);
                    if current_idx < db_types.len() - 1 {
                        let new_type = db_types[current_idx + 1];
                        config.db_type = new_type;
                        config.port = new_type.default_port();
                        if config.host.is_empty() && !matches!(new_type, DatabaseType::SQLite) {
                            config.host = "localhost".to_string();
                        }
                    }
                }

                // Ctrl+O 打开文件（仅 SQLite）
                if matches!(config.db_type, DatabaseType::SQLite)
                    && i.key_pressed(Key::O)
                    && i.modifiers == Modifiers::CTRL
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("SQLite 数据库", &["db", "sqlite", "sqlite3"])
                        .add_filter("所有文件", &["*"])
                        .pick_file()
                {
                    config.database = path.display().to_string();
                }
            });
        }

        let dialog_title = if is_edit_mode {
            "🔗 编辑数据库连接"
        } else {
            "🔗 新建数据库连接"
        };

        egui::Window::new(dialog_title)
            .open(&mut is_open)
            .resizable(false)
            .collapsible(false)
            .min_width(480.0)
            .show(ctx, |ui| {
                ui.add_space(SPACING_MD);

                // 数据库类型选择卡片
                Self::show_db_type_selector(ui, config);

                ui.add_space(SPACING_LG);

                // 连接表单
                Self::show_connection_form(ui, config);

                ui.add_space(SPACING_MD);

                // 实时检查清单
                Self::show_realtime_checklist(ui, config);

                ui.add_space(SPACING_LG);

                Self::show_advanced_toggle(ui, show_advanced);
                ui.add_space(SPACING_MD);

                if *show_advanced {
                    // SSL/TLS 配置
                    match config.db_type {
                        DatabaseType::MySQL => {
                            Self::show_mysql_ssl_config(ui, config);
                            ui.add_space(SPACING_LG);
                        }
                        DatabaseType::PostgreSQL => {
                            Self::show_postgres_ssl_config(ui, config);
                            ui.add_space(SPACING_LG);
                        }
                        DatabaseType::SQLite => {}
                    }

                    // SSH 隧道配置（仅对非 SQLite 显示）
                    if !matches!(config.db_type, DatabaseType::SQLite) {
                        Self::show_ssh_tunnel_config(ui, config);
                        ui.add_space(SPACING_LG);
                    }

                    // 连接字符串预览
                    Self::show_connection_preview(ui, config);
                } else {
                    ui.horizontal(|ui| {
                        ui.add_space(SPACING_MD);
                        ui.label(
                            RichText::new("已隐藏 SSL/SSH/连接字符串等高级配置")
                                .small()
                                .color(MUTED),
                        );
                    });
                }

                ui.add_space(SPACING_LG);
                ui.separator();
                ui.add_space(SPACING_MD);

                // 底部按钮
                Self::show_buttons(ui, config, on_save, &mut should_close, is_edit_mode);

                ui.add_space(SPACING_SM);
            });

        if should_close {
            is_open = false;
        }
        *open = is_open;
    }

    fn show_advanced_toggle(ui: &mut egui::Ui, show_advanced: &mut bool) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(120, 120, 130, 10))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .button(if *show_advanced {
                            "收起高级配置"
                        } else {
                            "显示高级配置（SSL/SSH/连接串）"
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
        // 快捷键提示
        ui.horizontal(|ui| {
            ui.add_space(SPACING_SM);
            ui.label(
                RichText::new(format!(
                    "数据库类型 [{} 切换]",
                    local_shortcut_text(LocalShortcut::FormatSelectionCycle)
                ))
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
                        LocalShortcut::FormatSelectionCycle,
                    ));

                if response.clicked() {
                    config.db_type = *db_type;
                    config.port = db_type.default_port();
                    if config.host.is_empty() && !matches!(db_type, DatabaseType::SQLite) {
                        config.host = "localhost".to_string();
                    }
                }

                ui.add_space(SPACING_SM);
            }
        });
    }

    /// 连接表单
    fn show_connection_form(ui: &mut egui::Ui, config: &mut ConnectionConfig) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(100, 100, 110, 10))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(16, 12))
            .show(ui, |ui| {
                egui::Grid::new("connection_form")
                    .num_columns(2)
                    .spacing([16.0, 10.0])
                    .show(ui, |ui| {
                        // 连接名称
                        ui.label(RichText::new("连接名称").color(GRAY));
                        ui.add(
                            TextEdit::singleline(&mut config.name)
                                .hint_text("我的数据库")
                                .char_limit(64)
                                .desired_width(280.0),
                        );
                        ui.end_row();

                        if !matches!(config.db_type, DatabaseType::SQLite) {
                            // 主机地址
                            ui.label(RichText::new("主机地址").color(GRAY));
                            ui.add(
                                TextEdit::singleline(&mut config.host)
                                    .hint_text("localhost")
                                    .char_limit(255)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            // 端口
                            ui.label(RichText::new("端口").color(GRAY));
                            let mut port_string = config.port.to_string();
                            ui.add(
                                TextEdit::singleline(&mut port_string)
                                    .char_limit(5)
                                    .desired_width(80.0),
                            );
                            if let Ok(port) = port_string.parse::<u16>() {
                                config.port = port;
                            }
                            ui.end_row();

                            // 用户名
                            ui.label(RichText::new("用户名").color(GRAY));
                            ui.add(
                                TextEdit::singleline(&mut config.username)
                                    .hint_text("root")
                                    .char_limit(128)
                                    .desired_width(280.0),
                            );
                            ui.end_row();

                            // 密码
                            ui.label(RichText::new("密码").color(GRAY));
                            ui.add(
                                TextEdit::singleline(&mut config.password)
                                    .password(true)
                                    .char_limit(256)
                                    .desired_width(280.0),
                            );
                            ui.end_row();
                        }

                        // SQLite 文件路径（必填）
                        if matches!(config.db_type, DatabaseType::SQLite) {
                            ui.label(RichText::new("文件路径").color(GRAY));

                            ui.horizontal(|ui| {
                                ui.add(
                                    TextEdit::singleline(&mut config.database)
                                        .hint_text("/path/to/database.db")
                                        .desired_width(200.0),
                                );

                                if ui
                                    .add(
                                        egui::Button::new(format!(
                                            "浏览 [{}]",
                                            local_shortcut_text(LocalShortcut::SqliteBrowseFile)
                                        ))
                                        .corner_radius(CornerRadius::same(4)),
                                    )
                                    .on_hover_text(local_shortcut_tooltip(
                                        "浏览并选择 SQLite 数据库文件",
                                        LocalShortcut::SqliteBrowseFile,
                                    ))
                                    .clicked()
                                    && let Some(path) = rfd::FileDialog::new()
                                        .add_filter("SQLite 数据库", &["db", "sqlite", "sqlite3"])
                                        .add_filter("所有文件", &["*"])
                                        .pick_file()
                                {
                                    config.database = path.display().to_string();
                                }
                            });
                            ui.end_row();
                        }
                    });
            });

        // 提示信息
        ui.add_space(SPACING_SM);
        ui.horizontal(|ui| {
            ui.add_space(SPACING_MD);
            ui.add_space(4.0);
            let tip = match config.db_type {
                DatabaseType::SQLite => "输入 SQLite 数据库文件路径，文件不存在时将自动创建",
                DatabaseType::PostgreSQL => "默认端口 5432，连接后可选择数据库",
                DatabaseType::MySQL => "默认端口 3306，连接后可选择数据库",
            };
            ui.label(RichText::new(tip).small().color(MUTED));
            ui.add_space(SPACING_SM);
            ui.label(
                RichText::new("需要 SSL/SSH 时，点击“显示高级配置”即可。")
                    .small()
                    .color(MUTED),
            );
        });
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

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(90, 130, 210, 12))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(110, 150, 230, 32),
            ))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("实时检查")
                            .small()
                            .strong()
                            .color(Color32::from_rgb(125, 182, 246)),
                    );
                    let passed = items.iter().filter(|(_, ok)| *ok).count();
                    ui.label(
                        RichText::new(format!("{}/{} 通过", passed, items.len()))
                            .small()
                            .color(MUTED),
                    );
                });
                ui.add_space(4.0);

                for (label, ok) in items {
                    let color = if ok { SUCCESS } else { DANGER };
                    let icon = if ok { "✓" } else { "•" };
                    ui.label(
                        RichText::new(format!("{} {}", icon, label))
                            .small()
                            .color(color),
                    );
                }
            });
    }

    /// MySQL SSL 配置
    fn show_mysql_ssl_config(ui: &mut egui::Ui, config: &mut ConnectionConfig) {
        ui.collapsing("🔐 SSL/TLS 加密", |ui| {
            ui.add_space(SPACING_SM);

            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(100, 100, 110, 10))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    egui::Grid::new("mysql_ssl_form")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            // SSL 模式选择
                            ui.label(RichText::new("SSL 模式").color(GRAY));
                            egui::ComboBox::new("ssl_mode_combo", "")
                                .selected_text(config.mysql_ssl_mode.display_name())
                                .show_ui(ui, |ui| {
                                    for mode in MySqlSslMode::all() {
                                        let label = format!(
                                            "{} - {}",
                                            mode.display_name(),
                                            mode.description()
                                        );
                                        ui.selectable_value(
                                            &mut config.mysql_ssl_mode,
                                            *mode,
                                            label,
                                        );
                                    }
                                });
                            ui.end_row();

                            // CA 证书路径（仅在 VerifyCa 或 VerifyIdentity 模式下显示）
                            if matches!(
                                config.mysql_ssl_mode,
                                MySqlSslMode::VerifyCa | MySqlSslMode::VerifyIdentity
                            ) {
                                ui.label(RichText::new("CA 证书").color(GRAY));
                                ui.horizontal(|ui| {
                                    ui.add(
                                        TextEdit::singleline(&mut config.ssl_ca_cert)
                                            .hint_text("/path/to/ca-cert.pem")
                                            .desired_width(160.0),
                                    );
                                    if ui.button("浏览").clicked()
                                        && let Some(path) = rfd::FileDialog::new()
                                            .add_filter("证书文件", &["pem", "crt", "cer"])
                                            .add_filter("所有文件", &["*"])
                                            .pick_file()
                                    {
                                        config.ssl_ca_cert = path.display().to_string();
                                    }
                                });
                                ui.end_row();
                            }
                        });

                    ui.add_space(SPACING_SM);

                    // SSL 模式说明
                    let tip = match config.mysql_ssl_mode {
                        MySqlSslMode::Disabled => "不使用加密，数据以明文传输",
                        MySqlSslMode::Preferred => "优先使用 SSL，如果服务器不支持则回退到明文",
                        MySqlSslMode::Required => "必须使用 SSL 加密，不验证服务器证书",
                        MySqlSslMode::VerifyCa => "验证服务器 CA 证书，不检查主机名",
                        MySqlSslMode::VerifyIdentity => "完整验证：检查 CA 证书和服务器主机名",
                    };
                    ui.label(RichText::new(tip).small().color(MUTED));
                });
        });
    }

    /// PostgreSQL SSL 配置
    fn show_postgres_ssl_config(ui: &mut egui::Ui, config: &mut ConnectionConfig) {
        ui.collapsing("🔐 SSL/TLS 加密", |ui| {
            ui.add_space(SPACING_SM);

            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(100, 100, 110, 10))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    egui::Grid::new("postgres_ssl_form")
                        .num_columns(2)
                        .spacing([16.0, 8.0])
                        .show(ui, |ui| {
                            // SSL 模式选择
                            ui.label(RichText::new("SSL 模式").color(GRAY));
                            egui::ComboBox::new("pg_ssl_mode_combo", "")
                                .selected_text(config.postgres_ssl_mode.display_name())
                                .show_ui(ui, |ui| {
                                    for mode in PostgresSslMode::all() {
                                        let label = format!(
                                            "{} - {}",
                                            mode.display_name(),
                                            mode.description()
                                        );
                                        ui.selectable_value(
                                            &mut config.postgres_ssl_mode,
                                            *mode,
                                            label,
                                        );
                                    }
                                });
                            ui.end_row();

                            // CA 证书路径（仅在 VerifyCa 或 VerifyFull 模式下显示）
                            if matches!(
                                config.postgres_ssl_mode,
                                PostgresSslMode::VerifyCa | PostgresSslMode::VerifyFull
                            ) {
                                ui.label(RichText::new("CA 证书").color(GRAY));
                                ui.horizontal(|ui| {
                                    ui.add(
                                        TextEdit::singleline(&mut config.ssl_ca_cert)
                                            .hint_text("/path/to/ca-cert.pem")
                                            .desired_width(160.0),
                                    );
                                    if ui.button("浏览").clicked()
                                        && let Some(path) = rfd::FileDialog::new()
                                            .add_filter("证书文件", &["pem", "crt", "cer"])
                                            .add_filter("所有文件", &["*"])
                                            .pick_file()
                                    {
                                        config.ssl_ca_cert = path.display().to_string();
                                    }
                                });
                                ui.end_row();
                            }
                        });

                    ui.add_space(SPACING_SM);

                    // SSL 模式说明
                    let tip = match config.postgres_ssl_mode {
                        PostgresSslMode::Disable => "不使用加密，数据以明文传输",
                        PostgresSslMode::Prefer => "优先使用 SSL，如果服务器不支持则回退到明文",
                        PostgresSslMode::Require => "必须使用 SSL 加密，不验证服务器证书",
                        PostgresSslMode::VerifyCa => "验证服务器 CA 证书，不检查主机名",
                        PostgresSslMode::VerifyFull => "完整验证：检查 CA 证书和服务器主机名",
                    };
                    ui.label(RichText::new(tip).small().color(MUTED));
                });
        });
    }

    /// SSH 隧道配置
    fn show_ssh_tunnel_config(ui: &mut egui::Ui, config: &mut ConnectionConfig) {
        ui.collapsing("🔒 SSH 隧道（可选）", |ui| {
            ui.add_space(SPACING_SM);

            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(100, 100, 110, 10))
                .corner_radius(CornerRadius::same(8))
                .inner_margin(egui::Margin::symmetric(16, 12))
                .show(ui, |ui| {
                    // 启用 SSH 隧道
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut config.ssh_config.enabled, "");
                        ui.label(RichText::new("启用 SSH 隧道").color(GRAY));
                    });

                    if config.ssh_config.enabled {
                        ui.add_space(SPACING_SM);

                        egui::Grid::new("ssh_tunnel_form")
                            .num_columns(2)
                            .spacing([16.0, 8.0])
                            .show(ui, |ui| {
                                // SSH 主机
                                ui.label(RichText::new("SSH 主机").color(GRAY));
                                ui.add(
                                    TextEdit::singleline(&mut config.ssh_config.ssh_host)
                                        .hint_text("跳板机地址")
                                        .desired_width(200.0),
                                );
                                ui.end_row();

                                // SSH 端口
                                ui.label(RichText::new("SSH 端口").color(GRAY));
                                let mut port_str = config.ssh_config.ssh_port.to_string();
                                if ui
                                    .add(TextEdit::singleline(&mut port_str).desired_width(80.0))
                                    .changed()
                                    && let Ok(port) = port_str.parse::<u16>()
                                {
                                    config.ssh_config.ssh_port = port;
                                }
                                ui.end_row();

                                // SSH 用户名
                                ui.label(RichText::new("SSH 用户名").color(GRAY));
                                ui.add(
                                    TextEdit::singleline(&mut config.ssh_config.ssh_username)
                                        .hint_text("用户名")
                                        .desired_width(200.0),
                                );
                                ui.end_row();

                                // 认证方式
                                ui.label(RichText::new("认证方式").color(GRAY));
                                ui.horizontal(|ui| {
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
                                ui.end_row();

                                // 密码或私钥
                                match config.ssh_config.auth_method {
                                    SshAuthMethod::Password => {
                                        ui.label(RichText::new("SSH 密码").color(GRAY));
                                        ui.add(
                                            TextEdit::singleline(
                                                &mut config.ssh_config.ssh_password,
                                            )
                                            .password(true)
                                            .desired_width(200.0),
                                        );
                                        ui.end_row();
                                    }
                                    SshAuthMethod::PrivateKey => {
                                        ui.label(RichText::new("私钥路径").color(GRAY));
                                        ui.horizontal(|ui| {
                                            ui.add(
                                                TextEdit::singleline(
                                                    &mut config.ssh_config.private_key_path,
                                                )
                                                .hint_text("~/.ssh/id_rsa")
                                                .desired_width(160.0),
                                            );
                                            if ui.button("浏览").clicked()
                                                && let Some(path) = rfd::FileDialog::new()
                                                    .add_filter("私钥文件", &["pem", "key", "*"])
                                                    .pick_file()
                                            {
                                                config.ssh_config.private_key_path =
                                                    path.display().to_string();
                                            }
                                        });
                                        ui.end_row();

                                        ui.label(RichText::new("私钥密码").color(GRAY));
                                        ui.add(
                                            TextEdit::singleline(
                                                &mut config.ssh_config.private_key_passphrase,
                                            )
                                            .password(true)
                                            .hint_text("（可选）")
                                            .desired_width(200.0),
                                        );
                                        ui.end_row();
                                    }
                                }

                                // 远程数据库地址（从 SSH 服务器视角）
                                ui.label(RichText::new("远程主机").color(GRAY));
                                ui.add(
                                    TextEdit::singleline(&mut config.ssh_config.remote_host)
                                        .hint_text("数据库主机（如 127.0.0.1）")
                                        .desired_width(200.0),
                                );
                                ui.end_row();

                                // 远程端口
                                ui.label(RichText::new("远程端口").color(GRAY));
                                let mut remote_port_str = config.ssh_config.remote_port.to_string();
                                if ui
                                    .add(
                                        TextEdit::singleline(&mut remote_port_str)
                                            .hint_text("数据库端口")
                                            .desired_width(80.0),
                                    )
                                    .changed()
                                    && let Ok(port) = remote_port_str.parse::<u16>()
                                {
                                    config.ssh_config.remote_port = port;
                                }
                                ui.end_row();
                            });

                        ui.add_space(SPACING_SM);
                        ui.label(
                            RichText::new(
                                "提示：启用 SSH 隧道后，连接将通过跳板机转发到远程数据库",
                            )
                            .small()
                            .color(MUTED),
                        );
                    }
                });
        });
    }

    /// 连接字符串预览
    fn show_connection_preview(ui: &mut egui::Ui, config: &ConnectionConfig) {
        ui.collapsing("🔍 连接字符串预览", |ui| {
            ui.add_space(SPACING_SM);

            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(60, 60, 70, 40))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    let conn_str = config.connection_string();
                    let display_str = if !config.password.is_empty() {
                        conn_str.replace(&config.password, "****")
                    } else {
                        conn_str
                    };
                    ui.label(RichText::new(&display_str).monospace().small());
                });
        });
    }

    /// 底部按钮
    fn show_buttons(
        ui: &mut egui::Ui,
        config: &ConnectionConfig,
        on_save: &mut bool,
        should_close: &mut bool,
        is_edit_mode: bool,
    ) {
        // 执行验证
        let validation = validate_config(config);

        // 快捷键提示
        ui.horizontal(|ui| {
            ui.add_space(SPACING_SM);
            ui.label(
                RichText::new(format!(
                    "快捷键: {} 关闭 | {} 保存",
                    local_shortcut_text(LocalShortcut::Dismiss),
                    local_shortcut_text(LocalShortcut::Confirm)
                ))
                .small()
                .color(MUTED),
            );
        });
        ui.add_space(SPACING_SM);

        ui.horizontal(|ui| {
            // 取消按钮
            if ui
                .add(
                    egui::Button::new(format!(
                        "取消 [{}]",
                        local_shortcut_text(LocalShortcut::Dismiss)
                    ))
                    .corner_radius(CornerRadius::same(6)),
                )
                .on_hover_text(local_shortcut_tooltip(
                    "关闭连接对话框",
                    LocalShortcut::Dismiss,
                ))
                .clicked()
            {
                *should_close = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 保存按钮
                let confirm_shortcut = local_shortcut_text(LocalShortcut::Confirm);
                let save_label = if is_edit_mode {
                    format!("保存并重连 [{confirm_shortcut}]")
                } else {
                    format!("保存并连接 [{confirm_shortcut}]")
                };
                let save_btn =
                    egui::Button::new(RichText::new(save_label).color(if validation.is_valid {
                        Color32::WHITE
                    } else {
                        GRAY
                    }))
                    .fill(if validation.is_valid {
                        SUCCESS
                    } else {
                        Color32::from_rgb(80, 80, 90)
                    })
                    .corner_radius(CornerRadius::same(6));

                if ui
                    .add_enabled(validation.is_valid, save_btn)
                    .on_hover_text(local_shortcut_tooltip(
                        "保存并创建连接",
                        LocalShortcut::Confirm,
                    ))
                    .clicked()
                {
                    *on_save = true;
                    *should_close = true;
                }

                // 显示验证错误
                if !validation.is_valid {
                    ui.add_space(SPACING_MD);
                    // 只显示第一个错误
                    if let Some(error) = validation.errors.first() {
                        ui.label(RichText::new(error).small().color(DANGER));
                    }
                }
            });
        });
    }
}
