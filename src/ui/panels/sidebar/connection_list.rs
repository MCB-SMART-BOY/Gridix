//! 连接列表渲染

use crate::database::ConnectionManager;
use crate::ui::styles::{DANGER, GRAY, MUTED, SUCCESS, MARGIN_MD, MARGIN_SM, SPACING_SM, SPACING_MD, SPACING_LG};
use crate::ui::SidebarSection;
use super::{SidebarActions, SidebarPanelState, SidebarSelectionState, DatabaseList, TableList};
use egui::{self, Color32, RichText, CornerRadius, Vec2};

/// 连接项数据（用于避免借用冲突）
pub(crate) struct ConnectionItemData {
    pub is_active: bool,
    pub is_connected: bool,
    pub db_type: String,
    pub host: String,
    pub databases: Vec<String>,
    pub selected_database: Option<String>,
    pub tables: Vec<String>,
    pub error: Option<String>,
}

/// 连接列表
pub struct ConnectionList;

impl ConnectionList {
    /// 显示上部面板（连接/数据库/表）
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        show_connection_dialog: &mut bool,
        is_focused: bool,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        actions: &mut SidebarActions,
        height: f32,
    ) {
        // 上部标题栏
        ui.horizontal(|ui| {
            // 折叠按钮
            let collapse_icon = if panel_state.upper_collapsed { ">" } else { "v" };
            if ui.small_button(collapse_icon).clicked() {
                panel_state.upper_collapsed = !panel_state.upper_collapsed;
            }
            
            Self::show_header(ui, show_connection_dialog, is_focused, focused_section);
        });
        
        if panel_state.upper_collapsed {
            return;
        }

        // 连接列表区域 - 使用固定宽度防止内容扩展面板
        let scroll_width = ui.available_width();
        egui::ScrollArea::vertical()
            .id_salt("upper_scroll")
            .max_height(height - 40.0)
            .auto_shrink([false, false])  // 不自动收缩，保持固定宽度
            .show(ui, |ui| {
                ui.set_max_width(scroll_width);  // 限制内容最大宽度
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
                            && idx == panel_state.selection.connections;
                        Self::show_connection_item(
                            ui,
                            name,
                            connection_manager,
                            selected_table,
                            actions,
                            is_focused,
                            focused_section,
                            is_nav_selected,
                            &panel_state.selection,
                        );
                    }
                }

                ui.add_space(SPACING_LG);
            });
    }

    /// 显示标题栏
    fn show_header(ui: &mut egui::Ui, show_connection_dialog: &mut bool, is_focused: bool, focused_section: SidebarSection) {
        // 使用与工具栏完全相同的 Frame 包裹
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(MARGIN_MD, MARGIN_SM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::new(6.0, 0.0);

                    // 标题
                    ui.label(RichText::new("🔗 连接").strong());
                    
                    // 显示当前焦点区域提示
                    if is_focused && focused_section != SidebarSection::Triggers {
                        let section_text = match focused_section {
                            SidebarSection::Connections => "连接",
                            SidebarSection::Databases => "数据库",
                            SidebarSection::Tables => "表",
                            SidebarSection::Triggers => "触发器",
                        };
                        ui.label(RichText::new(format!("→ {}", section_text)).small().color(SUCCESS));
                    }

                    // 把按钮推到右边
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 新建按钮 - 使用与工具栏一致的按钮样式
                        if ui
                            .add(
                                egui::Button::new(RichText::new("＋ 新建 [Ctrl+N]").size(13.0))
                                    .corner_radius(CornerRadius::same(6))
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
                        .corner_radius(CornerRadius::same(8))
                        .min_size(Vec2::new(120.0, 36.0)),
                )
                .clicked()
            {
                *show_connection_dialog = true;
            }
        });
    }

    /// 显示快捷键提示（在连接列表上方）
    fn show_shortcuts_hint(ui: &mut egui::Ui) {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(MARGIN_SM, 2))
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
        selection: &SidebarSelectionState,
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

        // 连接项容器 - 不再使用整体背景高亮，改为只高亮头部文字
        egui::Frame::NONE
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(MARGIN_SM, 2))
            .show(ui, |ui| {
                // 连接头部
                let header_response = egui::collapsing_header::CollapsingHeader::new(
                    Self::connection_header_text(name, conn_data.is_active, conn_data.is_connected, is_nav_selected),
                )
                .default_open(conn_data.is_active)
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
                        DatabaseList::show(
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
                            selection,
                        );
                    } else if conn_data.is_connected {
                        // SQLite 模式：直接显示表列表
                        TableList::show(
                            ui,
                            name,
                            &conn_data.tables,
                            connection_manager,
                            selected_table,
                            actions,
                            is_focused,
                            focused_section,
                            selection,
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
                            ui.close();
                        }
                    } else if ui.button("🔗 连接").clicked() {
                        actions.connect = Some(name.to_string());
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .button(RichText::new("🗑 删除").color(DANGER))
                        .clicked()
                    {
                        actions.delete = Some(name.to_string());
                        ui.close();
                    }
                });
            });
    }

    /// 连接头部文本
    /// 使用图标+颜色双重指示，对色盲友好
    fn connection_header_text(name: &str, is_active: bool, is_connected: bool, is_nav_selected: bool) -> RichText {
        // 使用不同形状的图标来区分状态，而不仅依赖颜色
        let (icon, color) = if is_nav_selected {
            (">", Color32::from_rgb(100, 180, 255))  // 键盘导航选中
        } else if is_active && is_connected {
            ("*", SUCCESS)  // 星号表示活跃连接
        } else if is_connected {
            ("+", Color32::from_rgb(100, 180, 100))  // 加号表示已连接但非活跃
        } else {
            ("-", GRAY)  // 减号表示未连接
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
            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(100, 150, 200, 30))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(6, 2))
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
                            .corner_radius(CornerRadius::same(4)),
                    )
                    .clicked()
                {
                    actions.disconnect = Some(name.to_string());
                    *selected_table = None;
                }
            } else if ui
                .add(
                    egui::Button::new(RichText::new("连接").small())
                        .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                actions.connect = Some(name.to_string());
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("删除").small().color(DANGER))
                        .corner_radius(CornerRadius::same(4)),
                )
                .clicked()
            {
                actions.delete = Some(name.to_string());
            }
        });
    }

    /// 显示错误信息
    fn show_error(ui: &mut egui::Ui, error: &str) {
        ui.horizontal(|ui| {
            ui.add_space(SPACING_LG);
            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(200, 80, 80, 30))
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("⚠ {}", truncate_error(error)))
                            .small()
                            .color(DANGER),
                    );
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
