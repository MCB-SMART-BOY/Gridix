//! 欢迎页面组件 - 应用启动时的欢迎界面

use crate::core::{Action, KeyBindings};
use crate::database::DatabaseType;
use crate::ui::styles::{GRAY, MUTED, SPACING_LG, SPACING_MD, SPACING_SM};
use crate::ui::{LocalShortcut, local_shortcut_text};
use egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

/// 欢迎页数据库运行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeServiceState {
    /// SQLite 内置支持，无需外部安装
    BuiltIn,
    /// 检测到可用服务
    Running,
    /// 检测到已安装，但服务未启动
    InstalledNotRunning,
    /// 未检测到相关安装/服务
    NotDetected,
}

/// 欢迎页数据库状态汇总
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WelcomeStatusSummary {
    pub sqlite: WelcomeServiceState,
    pub postgres: WelcomeServiceState,
    pub mysql: WelcomeServiceState,
}

impl Default for WelcomeStatusSummary {
    fn default() -> Self {
        Self {
            sqlite: WelcomeServiceState::BuiltIn,
            postgres: WelcomeServiceState::NotDetected,
            mysql: WelcomeServiceState::NotDetected,
        }
    }
}

impl WelcomeStatusSummary {
    pub const fn state_for(&self, db_type: DatabaseType) -> WelcomeServiceState {
        match db_type {
            DatabaseType::SQLite => self.sqlite,
            DatabaseType::PostgreSQL => self.postgres,
            DatabaseType::MySQL => self.mysql,
        }
    }
}

/// 欢迎页动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeAction {
    OpenConnection(DatabaseType),
    OpenSetupGuide(DatabaseType),
    RecheckEnvironment,
    OpenLearningSample,
}

/// 首启引导步骤
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WelcomeOnboardingStep {
    EnvironmentCheck,
    CreateConnection,
    InitializeDatabase,
    CreateUser,
    RunFirstQuery,
}

/// 欢迎页首启引导状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct WelcomeOnboardingStatus {
    pub environment_checked: bool,
    pub connection_created: bool,
    pub database_initialized: bool,
    pub user_created: bool,
    pub first_query_executed: bool,
    pub require_user_step: bool,
}

impl WelcomeOnboardingStatus {
    pub fn completed_steps(&self) -> usize {
        self.steps()
            .iter()
            .filter(|step| self.is_step_done(**step))
            .count()
    }

    pub fn total_steps(&self) -> usize {
        self.steps().len()
    }

    pub fn next_step(&self) -> Option<WelcomeOnboardingStep> {
        self.steps()
            .into_iter()
            .find(|step| !self.is_step_done(*step))
    }

    pub fn steps(&self) -> Vec<WelcomeOnboardingStep> {
        let mut steps = vec![
            WelcomeOnboardingStep::EnvironmentCheck,
            WelcomeOnboardingStep::CreateConnection,
            WelcomeOnboardingStep::InitializeDatabase,
        ];
        if self.require_user_step {
            steps.push(WelcomeOnboardingStep::CreateUser);
        }
        steps.push(WelcomeOnboardingStep::RunFirstQuery);
        steps
    }

    pub fn is_complete(&self) -> bool {
        self.next_step().is_none()
    }

    pub fn is_step_done(&self, step: WelcomeOnboardingStep) -> bool {
        match step {
            WelcomeOnboardingStep::EnvironmentCheck => self.environment_checked,
            WelcomeOnboardingStep::CreateConnection => self.connection_created,
            WelcomeOnboardingStep::InitializeDatabase => self.database_initialized,
            WelcomeOnboardingStep::CreateUser => !self.require_user_step || self.user_created,
            WelcomeOnboardingStep::RunFirstQuery => self.first_query_executed,
        }
    }

    pub fn step_label(step: WelcomeOnboardingStep) -> &'static str {
        match step {
            WelcomeOnboardingStep::EnvironmentCheck => "1. 环境检测",
            WelcomeOnboardingStep::CreateConnection => "2. 新建连接",
            WelcomeOnboardingStep::InitializeDatabase => "3. 初始化数据库",
            WelcomeOnboardingStep::CreateUser => "4. 创建用户",
            WelcomeOnboardingStep::RunFirstQuery => "5. 执行首条查询",
        }
    }

    pub fn action_label(step: WelcomeOnboardingStep) -> &'static str {
        match step {
            WelcomeOnboardingStep::EnvironmentCheck => "继续：检测本机环境",
            WelcomeOnboardingStep::CreateConnection => "继续：新建连接",
            WelcomeOnboardingStep::InitializeDatabase => "继续：初始化数据库",
            WelcomeOnboardingStep::CreateUser => "继续：创建用户",
            WelcomeOnboardingStep::RunFirstQuery => "继续：执行首条查询",
        }
    }
}

pub struct Welcome;

struct DatabaseCardSpec<'a> {
    db_type: DatabaseType,
    icon: &'a str,
    name: &'a str,
    desc: &'a str,
    accent_color: Color32,
}

impl Welcome {
    pub fn show(
        ui: &mut egui::Ui,
        status: WelcomeStatusSummary,
        keybindings: &KeyBindings,
    ) -> Option<WelcomeAction> {
        let mut action = None;
        let available_height = ui.available_height();
        let compact = available_height < 900.0;
        let dense = available_height < 680.0;
        let section_gap = if dense {
            SPACING_SM
        } else if compact {
            SPACING_MD
        } else {
            SPACING_LG
        };
        let content_width = (ui.available_width() - SPACING_LG * 2.0).clamp(440.0, 760.0);

        ui.add_space(SPACING_MD);
        egui::ScrollArea::vertical()
            .id_salt("welcome_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let offset = ((ui.available_width() - content_width) / 2.0).max(0.0);
                    ui.add_space(offset);

                    ui.allocate_ui_with_layout(
                        Vec2::new(content_width, 0.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            Self::show_hero(ui, content_width, compact);
                            ui.add_space(section_gap);

                            if action.is_none() {
                                action =
                                    Self::show_database_cards(ui, content_width, status, compact);
                            } else {
                                Self::show_database_cards(ui, content_width, status, compact);
                            }
                            ui.add_space(section_gap);

                            if action.is_none() {
                                action = Self::show_quick_start(ui, compact, keybindings);
                            } else {
                                Self::show_quick_start(ui, compact, keybindings);
                            }

                            if dense {
                                ui.add_space(SPACING_SM);
                                Self::show_shortcuts_hint(ui, keybindings);
                            } else {
                                ui.add_space(if compact { SPACING_SM } else { SPACING_MD });
                                Self::show_shortcuts(ui, content_width, compact, keybindings);
                            }
                        },
                    );
                });
            });

        if dense {
            ui.add_space(SPACING_SM);
            ui.horizontal_centered(|ui| {
                ui.label(
                    RichText::new("内容较多时可上下滚动查看完整欢迎页")
                        .small()
                        .color(MUTED),
                );
            });
        }

        action
    }

    fn show_hero(ui: &mut egui::Ui, width: f32, compact: bool) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(90, 140, 210, 18))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(120, 170, 230, 48),
            ))
            .corner_radius(CornerRadius::same(14))
            .inner_margin(egui::Margin::symmetric(22, if compact { 12 } else { 18 }))
            .show(ui, |ui| {
                ui.set_min_width((width - 44.0).max(320.0));
                ui.set_max_width((width - 44.0).max(320.0));
                ui.set_min_height(if compact { 108.0 } else { 124.0 });
                Self::show_header(ui, compact);
            });
    }

    /// 显示头部标题
    fn show_header(ui: &mut egui::Ui, compact: bool) {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("GRIDIX")
                    .size(if compact { 25.5 } else { 30.0 })
                    .strong()
                    .color(Color32::from_rgb(105, 168, 236)),
            );

            ui.add_space(if compact { 4.0 } else { 6.0 });

            ui.label(
                RichText::new("简洁、快速、安全的数据库管理工具")
                    .size(if compact { 14.5 } else { 16.0 })
                    .color(GRAY),
            );

            ui.add_space(if compact { 2.0 } else { 4.0 });

            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .small()
                    .color(MUTED),
            );
        });
    }

    /// 显示数据库类型卡片
    fn show_database_cards(
        ui: &mut egui::Ui,
        width: f32,
        status: WelcomeStatusSummary,
        compact: bool,
    ) -> Option<WelcomeAction> {
        let mut action = None;
        let card_spacing = if compact { 12.0 } else { 16.0 };
        let card_width = (width - card_spacing * 2.0) / 3.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = card_spacing;

            let sqlite = Self::database_card(
                ui,
                DatabaseCardSpec {
                    db_type: DatabaseType::SQLite,
                    icon: "S",
                    name: "SQLite",
                    desc: "本地文件数据库",
                    accent_color: Color32::from_rgb(80, 160, 220),
                },
                card_width,
                status.state_for(DatabaseType::SQLite),
                compact,
            );
            if action.is_none() {
                action = sqlite;
            }

            let postgres = Self::database_card(
                ui,
                DatabaseCardSpec {
                    db_type: DatabaseType::PostgreSQL,
                    icon: "P",
                    name: "PostgreSQL",
                    desc: "企业级关系数据库",
                    accent_color: Color32::from_rgb(80, 130, 180),
                },
                card_width,
                status.state_for(DatabaseType::PostgreSQL),
                compact,
            );
            if action.is_none() {
                action = postgres;
            }

            let mysql = Self::database_card(
                ui,
                DatabaseCardSpec {
                    db_type: DatabaseType::MySQL,
                    icon: "M",
                    name: "MySQL/MariaDB",
                    desc: "流行的开源数据库",
                    accent_color: Color32::from_rgb(200, 120, 60),
                },
                card_width,
                status.state_for(DatabaseType::MySQL),
                compact,
            );
            if action.is_none() {
                action = mysql;
            }
        });

        action
    }

    /// 单个数据库卡片
    fn database_card(
        ui: &mut egui::Ui,
        spec: DatabaseCardSpec<'_>,
        width: f32,
        status: WelcomeServiceState,
        compact: bool,
    ) -> Option<WelcomeAction> {
        let DatabaseCardSpec {
            db_type,
            icon,
            name,
            desc,
            accent_color,
        } = spec;
        let mut setup_clicked = false;
        let frame_response = egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(
                accent_color.r(),
                accent_color.g(),
                accent_color.b(),
                15,
            ))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(
                    accent_color.r(),
                    accent_color.g(),
                    accent_color.b(),
                    40,
                ),
            ))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(12, if compact { 12 } else { 16 }))
            .show(ui, |ui| {
                ui.set_min_width(width - 24.0);
                ui.set_max_width(width - 24.0);
                ui.set_min_height(if compact { 176.0 } else { 210.0 });

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let icon_size = if compact { 44.0 } else { 54.0 };
                    let icon_radius = if compact { 22.0 } else { 27.0 };
                    let icon_font = if compact { 26.0 } else { 32.0 };
                    let (rect, _) = ui
                        .allocate_exact_size(Vec2::new(icon_size, icon_size), egui::Sense::hover());
                    let painter = ui.painter();

                    painter.circle_filled(
                        rect.center(),
                        icon_radius,
                        Color32::from_rgba_unmultiplied(
                            accent_color.r(),
                            accent_color.g(),
                            accent_color.b(),
                            40,
                        ),
                    );

                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::proportional(icon_font),
                        accent_color,
                    );

                    ui.add_space(if compact { 4.0 } else { SPACING_SM + 2.0 });
                    ui.label(
                        RichText::new(name)
                            .size(if compact { 16.5 } else { 18.0 })
                            .strong()
                            .color(accent_color),
                    );
                    ui.add_space(if compact { 4.0 } else { 7.0 });
                    ui.label(
                        RichText::new(desc)
                            .size(if compact { 12.0 } else { 13.5 })
                            .strong()
                            .color(MUTED),
                    );

                    ui.add_space(if compact { 6.0 } else { 9.0 });
                    let (status_label, status_color) = Self::status_text_color(status);
                    ui.label(
                        RichText::new(status_label)
                            .size(if compact { 11.3 } else { 12.5 })
                            .strong()
                            .color(status_color),
                    );

                    ui.add_space(if compact { 5.0 } else { 8.0 });
                    match status {
                        WelcomeServiceState::NotDetected => {
                            if ui
                                .add_sized(
                                    [if compact { 106.0 } else { 120.0 }, 25.0],
                                    egui::Button::new(
                                        RichText::new("安装与初始化").size(12.0).strong(),
                                    ),
                                )
                                .clicked()
                            {
                                setup_clicked = true;
                            }
                        }
                        WelcomeServiceState::InstalledNotRunning => {
                            if ui
                                .add_sized(
                                    [if compact { 106.0 } else { 120.0 }, 25.0],
                                    egui::Button::new(
                                        RichText::new("启动服务引导").size(12.0).strong(),
                                    ),
                                )
                                .clicked()
                            {
                                setup_clicked = true;
                            }
                        }
                        WelcomeServiceState::BuiltIn | WelcomeServiceState::Running => {
                            ui.label(
                                RichText::new("点击卡片创建连接")
                                    .size(if compact { 10.8 } else { 11.5 })
                                    .color(MUTED),
                            );
                        }
                    }
                });
            })
            .response;

        let click_response = ui.interact(
            frame_response.rect,
            ui.make_persistent_id(format!("welcome_db_card_{name}")),
            egui::Sense::click(),
        );

        if setup_clicked {
            return Some(WelcomeAction::OpenSetupGuide(db_type));
        }
        if click_response.clicked() {
            return Some(WelcomeAction::OpenConnection(db_type));
        }
        None
    }

    fn status_text_color(status: WelcomeServiceState) -> (&'static str, Color32) {
        match status {
            WelcomeServiceState::BuiltIn => {
                ("内置支持（无需安装）", Color32::from_rgb(120, 220, 170))
            }
            WelcomeServiceState::Running => ("已检测到本机服务", Color32::from_rgb(120, 220, 170)),
            WelcomeServiceState::InstalledNotRunning => {
                ("已安装，但服务未启动", Color32::from_rgb(240, 190, 110))
            }
            WelcomeServiceState::NotDetected => {
                ("未检测到本机安装", Color32::from_rgb(235, 130, 130))
            }
        }
    }

    /// 显示快速开始提示
    fn show_quick_start(
        ui: &mut egui::Ui,
        compact: bool,
        keybindings: &KeyBindings,
    ) -> Option<WelcomeAction> {
        let mut action = None;
        let new_connection = Self::binding_or(keybindings, Action::NewConnection, "Ctrl+N");
        let show_help = Self::binding_or(keybindings, Action::ShowHelp, "F1");
        let execute_sql = local_shortcut_text(LocalShortcut::SqlExecute);
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(92, 180, 118, 22))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(100, 190, 126, 52),
            ))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(16, if compact { 8 } else { 10 }))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("\u{2139} 快速开始")
                            .size(if compact { 13.0 } else { 14.0 })
                            .strong()
                            .color(Color32::from_rgb(190, 230, 200)),
                    );

                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(if compact {
                            format!("点击「+ 新建」创建连接，或按 {new_connection}")
                        } else {
                            format!(
                                "点击侧边栏的 「+ 新建」 创建数据库连接，或按 {new_connection}"
                            )
                        })
                        .color(GRAY),
                    );

                    ui.add_space(if compact { 4.0 } else { 6.0 });
                    ui.label(
                        RichText::new(format!("连接后可直接使用 {execute_sql} 执行 SQL 查询"))
                            .color(GRAY),
                    );

                    if !compact {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(
                                format!(
                                    "如果是想要入门操作和学习数据库相关概念，请点击左上角小问号或按 {show_help}"
                                ),
                            )
                            .color(GRAY),
                        );
                    }

                    ui.add_space(if compact { 6.0 } else { 8.0 });
                    let button_width = if compact { 170.0 } else { 190.0 };
                    let button_height = if compact { 27.0 } else { 28.0 };
                    let button_gap = if compact { 8.0 } else { 10.0 };
                    let total_button_width = button_width * 2.0 + button_gap;
                    let row_width = ui.available_width();
                    if row_width >= total_button_width {
                        ui.allocate_ui_with_layout(
                            Vec2::new(row_width, button_height),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                let left_pad = ((row_width - total_button_width) / 2.0).max(0.0);
                                ui.add_space(left_pad);

                                if ui
                                    .add_sized(
                                        [button_width, button_height],
                                        egui::Button::new("一键打开 SQLite 学习示例库"),
                                    )
                                    .clicked()
                                {
                                    action = Some(WelcomeAction::OpenLearningSample);
                                }

                                ui.add_space(button_gap);

                                if ui
                                    .add_sized(
                                        [button_width, button_height],
                                        egui::Button::new("重新检测本机数据库环境"),
                                    )
                                    .clicked()
                                {
                                    action = Some(WelcomeAction::RecheckEnvironment);
                                }
                            },
                        );
                    } else {
                        let stacked_width = button_width.min((row_width - 2.0).max(150.0));
                        ui.vertical_centered(|ui| {
                            if ui
                                .add_sized(
                                    [stacked_width, button_height],
                                    egui::Button::new("一键打开 SQLite 学习示例库"),
                                )
                                .clicked()
                            {
                                action = Some(WelcomeAction::OpenLearningSample);
                            }
                            ui.add_space(button_gap.max(6.0));
                            if ui
                                .add_sized(
                                    [stacked_width, button_height],
                                    egui::Button::new("重新检测本机数据库环境"),
                                )
                                .clicked()
                            {
                                action = Some(WelcomeAction::RecheckEnvironment);
                            }
                        });
                    }
                });
            });
        action
    }

    fn show_shortcuts_hint(ui: &mut egui::Ui, keybindings: &KeyBindings) {
        let new_connection = Self::binding_or(keybindings, Action::NewConnection, "Ctrl+N");
        let show_help = Self::binding_or(keybindings, Action::ShowHelp, "F1");
        let execute_sql = local_shortcut_text(LocalShortcut::SqlExecute);
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(120, 120, 130, 10))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(14, 8))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "\u{2328} 快捷键：{new_connection} 新建 | {execute_sql} 执行 | {show_help} 帮助"
                        ))
                            .small()
                            .color(MUTED),
                    );
                });
            });
    }

    /// 显示快捷键列表
    fn show_shortcuts(ui: &mut egui::Ui, width: f32, compact: bool, keybindings: &KeyBindings) {
        ui.set_width(width);
        let shortcuts = [
            (
                Self::binding_or(keybindings, Action::NewConnection, "Ctrl+N"),
                "新建连接",
            ),
            (local_shortcut_text(LocalShortcut::SqlExecute), "执行查询"),
            (
                Self::binding_or(keybindings, Action::ToggleEditor, "Ctrl+J"),
                "切换编辑器",
            ),
            (
                Self::binding_or(keybindings, Action::ShowHistory, "Ctrl+H"),
                "查询历史",
            ),
            (
                Self::binding_or(keybindings, Action::Export, "Ctrl+E"),
                "导出结果",
            ),
            (
                Self::binding_or(keybindings, Action::Import, "Ctrl+I"),
                "导入 SQL",
            ),
            (
                Self::binding_or(keybindings, Action::Refresh, "F5"),
                "刷新表",
            ),
            (
                Self::binding_or(keybindings, Action::ShowHelp, "F1"),
                "帮助",
            ),
        ];

        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("\u{2328} 常用快捷键")
                    .size(if compact { 13.5 } else { 14.0 })
                    .strong()
                    .color(GRAY),
            );
        });

        ui.add_space(if compact { 3.0 } else { SPACING_SM * 0.8 });

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(128, 132, 146, 16))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(170, 176, 195, 40),
            ))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(16, if compact { 12 } else { 14 }))
            .show(ui, |ui| {
                let inner_width = ui.available_width();
                let column_gap = if compact { 14.0 } else { 18.0 };
                let row_gap = if compact { 5.0 } else { 8.0 };
                let pair_width = (inner_width - column_gap).max(0.0) / 2.0;

                let total_rows = shortcuts.chunks(2).len();
                for (row_idx, row) in shortcuts.chunks(2).enumerate() {
                    ui.horizontal(|ui| {
                        Self::shortcut_item(ui, row[0].0.as_str(), row[0].1, pair_width, compact);
                        if let Some((key, desc)) = row.get(1) {
                            ui.add_space(column_gap);
                            Self::shortcut_item(ui, key.as_str(), desc, pair_width, compact);
                        }
                    });

                    if row_idx + 1 < total_rows {
                        ui.add_space(row_gap);
                    }
                }
            });
    }

    /// 单个快捷键项
    fn shortcut_item(ui: &mut egui::Ui, key: &str, desc: &str, width: f32, compact: bool) {
        let key_inner_width = if compact { 84.0 } else { 96.0 };
        let key_outer_width = key_inner_width + if compact { 18.0 } else { 20.0 };
        let label_width = (width - key_outer_width - 12.0).max(40.0);

        ui.allocate_ui_with_layout(
            Vec2::new(width, 0.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(150, 158, 182, 48))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(if compact { 9 } else { 10 }, 3))
                    .show(ui, |ui| {
                        ui.set_min_width(key_inner_width);
                        ui.set_max_width(key_inner_width);
                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                ui.label(
                                    RichText::new(key)
                                        .monospace()
                                        .size(if compact { 13.5 } else { 14.0 })
                                        .color(Color32::from_rgb(198, 208, 230))
                                        .strong(),
                                );
                            },
                        );
                    });

                ui.add_space(12.0);
                ui.add_sized(
                    [label_width, if compact { 21.0 } else { 22.0 }],
                    egui::Label::new(
                        RichText::new(desc)
                            .size(if compact { 13.5 } else { 14.0 })
                            .color(Color32::from_rgb(194, 200, 218)),
                    ),
                );
            },
        );
    }

    fn binding_or(keybindings: &KeyBindings, action: Action, fallback: &str) -> String {
        let binding = keybindings.display(action);
        if binding.is_empty() {
            fallback.to_owned()
        } else {
            binding
        }
    }
}
