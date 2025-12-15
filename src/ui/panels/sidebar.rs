//! 侧边栏组件 - 连接管理和表列表

use crate::core::constants;
use crate::database::ConnectionManager;
use crate::ui::styles::{DANGER, GRAY, MUTED, SUCCESS, SPACING_MD, SPACING_SM, SPACING_LG};
use crate::ui::SidebarSection;
use egui::{self, Color32, RichText, Rounding, Vec2};

pub struct Sidebar;

/// 连接项数据（用于避免借用冲突）
struct ConnectionItemData {
    is_active: bool,
    is_connected: bool,
    db_type: String,
    host: String,
    databases: Vec<String>,
    selected_database: Option<String>,
    tables: Vec<String>,
    error: Option<String>,
}

/// 焦点转移方向（从侧边栏转出）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarFocusTransfer {
    /// 转移到数据表格
    ToDataGrid,
}

/// 侧边栏操作
#[derive(Default)]
pub struct SidebarActions {
    pub connect: Option<String>,
    pub disconnect: Option<String>,
    pub delete: Option<String>,
    pub select_database: Option<String>,
    pub show_table_schema: Option<String>,
    pub query_table: Option<String>,
    /// 焦点转移请求
    pub focus_transfer: Option<SidebarFocusTransfer>,
}

impl SidebarActions {
    /// 检查是否有任何操作
    #[inline]
    #[allow(dead_code)]
    pub fn has_action(&self) -> bool {
        self.connect.is_some()
            || self.disconnect.is_some()
            || self.delete.is_some()
            || self.select_database.is_some()
            || self.show_table_schema.is_some()
            || self.query_table.is_some()
    }
}

impl Sidebar {
    pub fn show(
        ctx: &egui::Context,
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        show_connection_dialog: &mut bool,
        is_focused: bool,
        focused_section: SidebarSection,
        selected_index: &mut usize,
    ) -> SidebarActions {
        let mut actions = SidebarActions::default();
        
        // 获取当前区域的项目数量
        let item_count = match focused_section {
            SidebarSection::Connections => connection_manager.connections.len(),
            SidebarSection::Databases => connection_manager
                .get_active()
                .map(|c| c.databases.len())
                .unwrap_or(0),
            SidebarSection::Tables => connection_manager
                .get_active()
                .map(|c| c.tables.len())
                .unwrap_or(0),
        };
        
        // 处理侧边栏键盘导航（仅在聚焦时响应）
        if is_focused && item_count > 0 {
            ctx.input(|i| {
                // j 或下箭头：向下导航
                if i.key_pressed(egui::Key::J) || i.key_pressed(egui::Key::ArrowDown) {
                    *selected_index = (*selected_index + 1).min(item_count.saturating_sub(1));
                }
                // k 或上箭头：向上导航
                if i.key_pressed(egui::Key::K) || i.key_pressed(egui::Key::ArrowUp) {
                    *selected_index = selected_index.saturating_sub(1);
                }
                // g：跳到第一个
                if i.key_pressed(egui::Key::G) && !i.modifiers.shift {
                    *selected_index = 0;
                }
                // G (Shift+g)：跳到最后一个
                if i.key_pressed(egui::Key::G) && i.modifiers.shift {
                    *selected_index = item_count.saturating_sub(1);
                }
                // Enter：选择/激活当前项
                if i.key_pressed(egui::Key::Enter) {
                    match focused_section {
                        SidebarSection::Connections => {
                            let names: Vec<_> = connection_manager.connections.keys().cloned().collect();
                            if let Some(name) = names.get(*selected_index) {
                                actions.connect = Some(name.clone());
                            }
                        }
                        SidebarSection::Databases => {
                            if let Some(conn) = connection_manager.get_active() {
                                if let Some(db) = conn.databases.get(*selected_index) {
                                    actions.select_database = Some(db.clone());
                                }
                            }
                        }
                        SidebarSection::Tables => {
                            if let Some(conn) = connection_manager.get_active() {
                                if let Some(table) = conn.tables.get(*selected_index) {
                                    actions.query_table = Some(table.clone());
                                    *selected_table = Some(table.clone());
                                }
                            }
                        }
                    }
                }
                // l 或右箭头：转移焦点到数据表格
                if i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight) {
                    actions.focus_transfer = Some(SidebarFocusTransfer::ToDataGrid);
                }
            });
            
            // 确保索引在有效范围内
            if *selected_index >= item_count {
                *selected_index = item_count.saturating_sub(1);
            }
        }

        // 根据屏幕宽度按比例设置侧边栏宽度
        let screen_width = ctx.screen_rect().width();
        let default_width = (screen_width * constants::ui::SIDEBAR_DEFAULT_WIDTH_RATIO).clamp(200.0, 300.0);
        let min_width = (screen_width * constants::ui::SIDEBAR_MIN_WIDTH_RATIO).clamp(constants::ui::SIDEBAR_MIN_WIDTH_PX, 220.0);
        let max_width = (screen_width * constants::ui::SIDEBAR_MAX_WIDTH_RATIO).clamp(250.0, constants::ui::SIDEBAR_MAX_WIDTH_PX);

        egui::SidePanel::left("sidebar")
            .default_width(default_width)
            .min_width(min_width)
            .max_width(max_width)
            .resizable(true)
            .frame(egui::Frame::central_panel(&ctx.style()))
            .show(ctx, |ui| {
                // 标题栏（显示当前焦点区域）
                Self::show_header(ui, show_connection_dialog, is_focused, focused_section);

                ui.add_space(SPACING_SM);

                // 连接列表区域
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(SPACING_SM);

                        let connection_names: Vec<String> =
                            connection_manager.connections.keys().cloned().collect();

                        if connection_names.is_empty() {
                            Self::show_empty_state(ui, show_connection_dialog);
                        } else {
                            // 快捷键提示（在第一个连接上方）
                            Self::show_shortcuts_hint(ui);
                            
                            for (idx, name) in connection_names.iter().enumerate() {
                                // 判断是否为键盘导航选中项
                                let is_nav_selected = is_focused 
                                    && focused_section == SidebarSection::Connections 
                                    && idx == *selected_index;
                                Self::show_connection_item(
                                    ui,
                                    name,
                                    connection_manager,
                                    selected_table,
                                    &mut actions,
                                    is_focused,
                                    focused_section,
                                    is_nav_selected,
                                    selected_index,
                                );
                            }
                        }

                        ui.add_space(SPACING_LG);
                    });
            });

        actions
    }

    /// 显示标题栏
    fn show_header(ui: &mut egui::Ui, show_connection_dialog: &mut bool, is_focused: bool, focused_section: SidebarSection) {
        // 使用与工具栏完全相同的 Frame 包裹
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(SPACING_MD, SPACING_SM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::new(6.0, 0.0);

                    // 标题
                    ui.label(RichText::new("🔗 连接").strong());
                    
                    // 显示当前焦点区域提示
                    if is_focused {
                        let section_text = match focused_section {
                            SidebarSection::Connections => "连接",
                            SidebarSection::Databases => "数据库",
                            SidebarSection::Tables => "表",
                        };
                        ui.label(RichText::new(format!("→ {}", section_text)).small().color(SUCCESS));
                    }

                    // 把按钮推到右边
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 新建按钮 - 使用与工具栏一致的按钮样式
                        if ui
                            .add(
                                egui::Button::new(RichText::new("＋ 新建 [Ctrl+N]").size(13.0))
                                    .rounding(Rounding::same(6.0))
                                    .min_size(Vec2::new(0.0, 28.0)),
                            )
                            .clicked()
                        {
                            *show_connection_dialog = true;
                        }
                    });
                });
            });

        // 分隔线
        ui.separator();
    }

    /// 显示空状态
    fn show_empty_state(ui: &mut egui::Ui, show_connection_dialog: &mut bool) {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            // 图标
            ui.label(RichText::new("📭").size(48.0));

            ui.add_space(SPACING_LG);

            ui.label(
                RichText::new("暂无连接")
                    .size(16.0)
                    .color(GRAY),
            );

            ui.add_space(SPACING_SM);

            ui.label(
                RichText::new("创建一个数据库连接开始使用")
                    .small()
                    .color(MUTED),
            );

            ui.add_space(SPACING_LG);

            if ui
                .add(
                    egui::Button::new(RichText::new("＋ 新建连接 [Ctrl+N]").size(14.0))
                        .rounding(Rounding::same(8.0))
                        .min_size(Vec2::new(120.0, 36.0)),
                )
                .clicked()
            {
                *show_connection_dialog = true;
            }
        });
    }

    /// 显示连接项
    #[allow(clippy::too_many_arguments)]
    fn show_connection_item(
        ui: &mut egui::Ui,
        name: &str,
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
        is_focused: bool,
        focused_section: SidebarSection,
        is_nav_selected: bool,
        nav_index: &usize,
    ) {
        // 先提取需要的数据，避免借用冲突
        let conn_data = {
            let Some(conn) = connection_manager.connections.get(name) else {
                return;
            };
            ConnectionItemData {
                is_active: connection_manager.active.as_deref() == Some(name),
                is_connected: conn.connected,
                db_type: conn.config.db_type.display_name().to_string(),
                host: conn.config.host.clone(),
                databases: conn.databases.clone(),
                selected_database: conn.selected_database.clone(),
                tables: conn.tables.clone(),
                error: conn.error.clone(),
            }
        };

        // 连接项容器 - 键盘导航选中时高亮
        let frame_bg = if is_nav_selected {
            Color32::from_rgba_unmultiplied(100, 150, 255, 40)
        } else {
            Color32::TRANSPARENT
        };
        egui::Frame::none()
            .fill(frame_bg)
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(SPACING_SM, 2.0))
            .show(ui, |ui| {
                // 连接头部
                let header_response = egui::collapsing_header::CollapsingHeader::new(
                    Self::connection_header_text(name, conn_data.is_active, conn_data.is_connected, is_nav_selected),
                )
                .default_open(conn_data.is_active || is_nav_selected)
                .show(ui, |ui| {
                    ui.add_space(SPACING_SM);

                    // 连接信息
                    Self::show_connection_info(ui, &conn_data.db_type, &conn_data.host);

                    ui.add_space(SPACING_SM);

                    // 操作按钮
                    Self::show_connection_buttons(
                        ui,
                        name,
                        conn_data.is_active,
                        selected_table,
                        actions,
                    );

                    ui.add_space(SPACING_MD);

                    // 如果有数据库列表（MySQL/PostgreSQL），显示数据库列表
                    if conn_data.is_connected && !conn_data.databases.is_empty() {
                        Self::show_database_list(
                            ui,
                            name,
                            &conn_data.databases,
                            conn_data.selected_database.as_deref(),
                            &conn_data.tables,
                            connection_manager,
                            selected_table,
                            actions,
                            is_focused,
                            focused_section,
                            *nav_index,
                        );
                    } else if conn_data.is_connected {
                        // SQLite 模式：直接显示表列表
                        Self::show_table_list(
                            ui,
                            name,
                            &conn_data.tables,
                            connection_manager,
                            selected_table,
                            actions,
                            is_focused,
                            focused_section,
                            *nav_index,
                        );
                    }

                    // 错误显示
                    if let Some(error) = &conn_data.error {
                        ui.add_space(SPACING_SM);
                        Self::show_error(ui, error);
                    }
                });

                // 右键菜单
                let is_active_for_menu = conn_data.is_active;
                header_response.header_response.context_menu(|ui| {
                    if is_active_for_menu {
                        if ui.button("断开连接").clicked() {
                            actions.disconnect = Some(name.to_string());
                            ui.close_menu();
                        }
                    } else if ui.button("🔗 连接").clicked() {
                        actions.connect = Some(name.to_string());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .button(RichText::new("🗑 删除").color(DANGER))
                        .clicked()
                    {
                        actions.delete = Some(name.to_string());
                        ui.close_menu();
                    }
                });
            });
    }

    /// 连接头部文本
    /// 使用图标+颜色双重指示，对色盲友好
    fn connection_header_text(name: &str, is_active: bool, is_connected: bool, is_nav_selected: bool) -> RichText {
        // 使用不同形状的图标来区分状态，而不仅依赖颜色
        let (icon, color) = if is_nav_selected {
            ("▶", Color32::from_rgb(100, 180, 255))  // 键盘导航选中
        } else if is_active && is_connected {
            ("◆", SUCCESS)  // 实心菱形表示活跃连接
        } else if is_connected {
            ("◇", Color32::from_rgb(100, 180, 100))  // 空心菱形表示已连接但非活跃
        } else {
            ("○", GRAY)  // 空心圆表示未连接
        };

        RichText::new(format!("{} {}", icon, name))
            .strong()
            .color(color)
    }

    /// 显示连接信息
    fn show_connection_info(ui: &mut egui::Ui, db_type: &str, host: &str) {
        ui.horizontal(|ui| {
            ui.add_space(SPACING_LG);

            // 数据库类型标签
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(100, 150, 200, 30))
                .rounding(Rounding::same(4.0))
                .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                .show(ui, |ui| {
                    ui.label(RichText::new(db_type).small().strong());
                });

            if !host.is_empty() {
                ui.label(RichText::new("@").small().color(MUTED));
                ui.label(RichText::new(host).small().color(GRAY));
            }
        });
    }

    /// 显示连接操作按钮
    fn show_connection_buttons(
        ui: &mut egui::Ui,
        name: &str,
        is_active: bool,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(SPACING_LG);

            if is_active {
                if ui
                    .add(
                        egui::Button::new(RichText::new("断开").small())
                            .rounding(Rounding::same(4.0)),
                    )
                    .clicked()
                {
                    actions.disconnect = Some(name.to_string());
                    *selected_table = None;
                }
            } else if ui
                .add(
                    egui::Button::new(RichText::new("连接").small())
                        .rounding(Rounding::same(4.0)),
                )
                .clicked()
            {
                actions.connect = Some(name.to_string());
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("删除").small().color(DANGER))
                        .rounding(Rounding::same(4.0)),
                )
                .clicked()
            {
                actions.delete = Some(name.to_string());
            }
        });
    }

    /// 显示数据库列表（MySQL/PostgreSQL）
    #[allow(clippy::too_many_arguments)]
    fn show_database_list(
        ui: &mut egui::Ui,
        conn_name: &str,
        databases: &[String],
        selected_database: Option<&str>,
        tables: &[String],
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
        is_focused: bool,
        focused_section: SidebarSection,
        nav_index: usize,
    ) {
        // 数据库区域是否高亮
        let highlight_databases = is_focused && focused_section == SidebarSection::Databases;
        // 表区域是否高亮
        let highlight_tables = is_focused && focused_section == SidebarSection::Tables;
        // 数据库列表
        for (idx, database) in databases.iter().enumerate() {
            let is_selected = selected_database == Some(database.as_str());
            let is_nav_selected = highlight_databases && idx == nav_index;

            // 数据库项 - 整行可点击
            let db_bg = if is_nav_selected {
                Color32::from_rgba_unmultiplied(100, 150, 255, 60)  // 键盘导航选中
            } else if is_selected {
                Color32::from_rgba_unmultiplied(80, 140, 80, 50)
            } else {
                Color32::TRANSPARENT
            };
            let db_response = egui::Frame::none()
                .fill(db_bg)
                .rounding(Rounding::same(4.0))
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // 数据库名称
                        let db_color = if is_nav_selected {
                            Color32::from_rgb(100, 180, 255)
                        } else if is_selected {
                            Color32::from_rgb(140, 220, 140)
                        } else {
                            Color32::from_rgb(180, 180, 190)
                        };
                        let prefix = if is_nav_selected { "▶ " } else { "" };
                        ui.label(RichText::new(format!("{}{}", prefix, database)).color(db_color));
                        
                        // 表数量提示（选中时显示）
                        if is_selected {
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                ui.label(RichText::new(format!("{} 表", tables.len())).small().color(MUTED));
                            });
                        }
                    });
                })
                .response
                .interact(egui::Sense::click());

            // 左键点击 - 选择数据库
            if db_response.clicked() {
                connection_manager.active = Some(conn_name.to_string());
                actions.select_database = Some(database.clone());
            }

            // 如果此数据库被选中，显示其下的表列表
            if is_selected && !tables.is_empty() {
                Self::show_table_list_nested(
                    ui,
                    conn_name,
                    tables,
                    connection_manager,
                    selected_table,
                    actions,
                    highlight_tables,
                    nav_index,
                );
            }
        }
    }

    /// 显示嵌套的表列表（在数据库下方）
    fn show_table_list_nested(
        ui: &mut egui::Ui,
        conn_name: &str,
        tables: &[String],
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
        highlight_tables: bool,
        nav_index: usize,
    ) {
        // 表列表
        for (idx, table) in tables.iter().enumerate() {
            let is_nav_selected = highlight_tables && idx == nav_index;
            let is_selected = selected_table.as_deref() == Some(table);

            // 表项 - 带缩进
            ui.horizontal(|ui| {
                ui.add_space(SPACING_LG);

                let table_bg = if is_nav_selected {
                    Color32::from_rgba_unmultiplied(100, 150, 255, 60)  // 键盘导航选中
                } else if is_selected {
                    Color32::from_rgba_unmultiplied(80, 120, 180, 50)
                } else {
                    Color32::TRANSPARENT
                };
                let response = egui::Frame::none()
                    .fill(table_bg)
                    .rounding(Rounding::same(4.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width() - 8.0);
                        let text_color = if is_nav_selected {
                            Color32::from_rgb(100, 180, 255)
                        } else if is_selected {
                            Color32::from_rgb(150, 200, 255)
                        } else {
                            Color32::from_rgb(170, 170, 180)
                        };
                        let prefix = if is_nav_selected { "▶ " } else { "" };
                        ui.label(RichText::new(format!("{}{}", prefix, table)).color(text_color));
                    })
                    .response
                    .interact(egui::Sense::click());

                // 左键点击 - 查询表数据
                if response.clicked() {
                    *selected_table = Some(table.clone());
                    connection_manager.active = Some(conn_name.to_string());
                    actions.query_table = Some(table.clone());
                }

                // 右键菜单
                response.context_menu(|ui| {
                    if ui.button("查询前 100 行").clicked() {
                        actions.query_table = Some(table.clone());
                        ui.close_menu();
                    }
                    if ui.button("查看表结构").clicked() {
                        actions.show_table_schema = Some(table.clone());
                        ui.close_menu();
                    }
                });
            });
        }
    }

    /// 显示表列表（SQLite 模式，直接在连接下）
    fn show_table_list(
        ui: &mut egui::Ui,
        conn_name: &str,
        tables: &[String],
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
        is_focused: bool,
        focused_section: SidebarSection,
        nav_index: usize,
    ) {
        let highlight_tables = is_focused && focused_section == SidebarSection::Tables;
        if tables.is_empty() {
            ui.horizontal(|ui| {
                ui.add_space(SPACING_LG);
                ui.label(RichText::new("暂无数据表").italics().small().color(MUTED));
            });
            return;
        }

        // 表列表标题
        ui.horizontal(|ui| {
            ui.add_space(SPACING_LG);
            ui.label(
                RichText::new(format!("数据表 ({})", tables.len()))
                    .small()
                    .strong()
                    .color(GRAY),
            );
        });

        ui.add_space(SPACING_SM);

        // 表列表
        for (idx, table) in tables.iter().enumerate() {
            let is_selected = selected_table.as_deref() == Some(table);
            let is_nav_selected = highlight_tables && idx == nav_index;

            ui.horizontal(|ui| {
                ui.add_space(SPACING_LG + 4.0);

                // 表项
                let table_bg = if is_nav_selected {
                    Color32::from_rgba_unmultiplied(100, 150, 255, 60)  // 键盘导航选中
                } else if is_selected {
                    Color32::from_rgba_unmultiplied(100, 150, 200, 40)
                } else {
                    Color32::TRANSPARENT
                };
                let response = egui::Frame::none()
                    .fill(table_bg)
                    .rounding(Rounding::same(4.0))
                    .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                    .show(ui, |ui| {
                        ui.set_min_width(ui.available_width() - 8.0);
                        let (icon, color) = if is_nav_selected {
                            ("▶", Color32::from_rgb(100, 180, 255))
                        } else if is_selected {
                            (">", Color32::from_rgb(150, 200, 255))
                        } else {
                            (" ", Color32::from_rgb(180, 180, 190))
                        };
                        ui.label(RichText::new(format!("{} {}", icon, table)).color(color));
                    })
                    .response
                    .interact(egui::Sense::click());

                // 左键点击 - 查询表数据
                if response.clicked() {
                    *selected_table = Some(table.clone());
                    connection_manager.active = Some(conn_name.to_string());
                    actions.query_table = Some(table.clone());
                }

                // 右键菜单
                response.context_menu(|ui| {
                    if ui.button("📊 查询前 100 行").clicked() {
                        actions.query_table = Some(table.clone());
                        ui.close_menu();
                    }
                    if ui.button("🔍 查看表结构").clicked() {
                        actions.show_table_schema = Some(table.clone());
                        ui.close_menu();
                    }
                });
            });
        }
    }

    /// 显示错误信息
    fn show_error(ui: &mut egui::Ui, error: &str) {
        ui.horizontal(|ui| {
            ui.add_space(SPACING_LG);
            egui::Frame::none()
                .fill(Color32::from_rgba_unmultiplied(200, 80, 80, 30))
                .rounding(Rounding::same(4.0))
                .inner_margin(egui::Margin::symmetric(8.0, 4.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("⚠ {}", truncate_error(error)))
                            .small()
                            .color(DANGER),
                    );
                });
        });
    }

    /// 显示快捷键提示（在连接列表上方）
    fn show_shortcuts_hint(ui: &mut egui::Ui) {
        egui::Frame::none()
            .inner_margin(egui::Margin::symmetric(SPACING_SM, 2.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::new(4.0, 0.0);
                    ui.label(RichText::new("j/k").small().color(GRAY));
                    ui.label(RichText::new("导航").small().color(MUTED));
                    ui.label(RichText::new("·").small().color(MUTED));
                    ui.label(RichText::new("Enter").small().color(GRAY));
                    ui.label(RichText::new("选择").small().color(MUTED));
                    ui.label(RichText::new("·").small().color(MUTED));
                    ui.label(RichText::new("g/G").small().color(GRAY));
                    ui.label(RichText::new("首/尾").small().color(MUTED));
                });
            });
    }
}

/// 截断错误信息
fn truncate_error(error: &str) -> String {
    if error.len() > 50 {
        format!("{}...", &error[..47])
    } else {
        error.to_string()
    }
}
