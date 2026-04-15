//! 连接列表渲染

use super::{
    DatabaseList, SidebarActions, SidebarDeleteTarget, SidebarPanelState, SidebarSelectionState,
    TableList,
};
use crate::core::{Action, KeyBindings};
use crate::database::ConnectionManager;
use crate::ui::styles::{
    DANGER, GRAY, MARGIN_MD, MARGIN_SM, MUTED, SPACING_LG, SPACING_MD, SPACING_SM, SUCCESS,
    theme_text,
};
use crate::ui::{SidebarSection, action_tooltip};
use egui::{self, Color32, CornerRadius, Rect, RichText, Vec2};

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

#[derive(Debug)]
struct ConnectionHeaderActionGroup {
    rect: Rect,
    connection_delete_rect: Option<Rect>,
}

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
struct ConnectionHeaderRender {
    interaction_response: egui::Response,
    label_clicked: bool,
    toggle_clicked: bool,
    action_rect: Rect,
    toggle_rect: Rect,
    label_rect: Rect,
    connection_delete_rect: Option<Rect>,
}

/// 连接列表
pub struct ConnectionList;

impl ConnectionList {
    fn request_delete_target(actions: &mut SidebarActions, target: SidebarDeleteTarget) {
        actions.delete = Some(target);
    }

    pub(super) fn request_connection_delete(connection_name: &str, actions: &mut SidebarActions) {
        Self::request_delete_target(
            actions,
            SidebarDeleteTarget::connection(connection_name.to_string()),
        );
    }

    pub(super) fn request_database_delete(
        connection_name: &str,
        database_name: &str,
        actions: &mut SidebarActions,
    ) {
        Self::request_delete_target(
            actions,
            SidebarDeleteTarget::database(connection_name.to_string(), database_name.to_string()),
        );
    }

    pub(super) fn request_table_delete(
        connection_name: &str,
        table_name: &str,
        actions: &mut SidebarActions,
    ) {
        Self::request_delete_target(
            actions,
            SidebarDeleteTarget::table(connection_name.to_string(), table_name.to_string()),
        );
    }

    pub(super) fn delete_targets_for_context(
        connection_name: &str,
        selected_database: Option<&str>,
    ) -> Vec<SidebarDeleteTarget> {
        let mut targets = Vec::new();
        if let Some(database) = selected_database
            .map(str::trim)
            .filter(|database| !database.is_empty())
        {
            targets.push(SidebarDeleteTarget::database(
                connection_name.to_string(),
                database.to_string(),
            ));
        }
        targets.push(SidebarDeleteTarget::connection(connection_name.to_string()));
        targets
    }

    pub(super) fn show_delete_targets_menu(
        ui: &mut egui::Ui,
        connection_name: &str,
        selected_database: Option<&str>,
        actions: &mut SidebarActions,
    ) {
        for target in Self::delete_targets_for_context(connection_name, selected_database) {
            let (label, target) = match target {
                SidebarDeleteTarget::Database {
                    connection_name,
                    database_name,
                } => (
                    format!("🗑 删除数据库 {}", database_name),
                    SidebarDeleteTarget::Database {
                        connection_name,
                        database_name,
                    },
                ),
                SidebarDeleteTarget::Connection(connection) => (
                    format!("🗑 删除连接 {}", connection),
                    SidebarDeleteTarget::Connection(connection),
                ),
                SidebarDeleteTarget::Table {
                    connection_name,
                    table_name,
                } => (
                    format!("🗑 删除表 {}", table_name),
                    SidebarDeleteTarget::Table {
                        connection_name,
                        table_name,
                    },
                ),
            };

            if ui.button(RichText::new(label).color(DANGER)).clicked() {
                Self::request_delete_target(actions, target);
                ui.close();
            }
        }
    }

    /// 显示上部面板（连接/数据库/表）
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        show_connection_dialog: &mut bool,
        keybindings: &KeyBindings,
        is_focused: bool,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        actions: &mut SidebarActions,
        height: f32,
    ) {
        // 上部标题栏
        ui.horizontal(|ui| {
            Self::show_header(
                ui,
                show_connection_dialog,
                keybindings,
                is_focused,
                focused_section,
            );
        });

        // 连接列表区域 - 使用固定宽度防止内容扩展面板
        let scroll_width = ui.available_width();
        egui::ScrollArea::vertical()
            .id_salt("upper_scroll")
            .max_height(height - 40.0)
            .auto_shrink([false, false]) // 不自动收缩，保持固定宽度
            .show(ui, |ui| {
                ui.set_max_width(scroll_width); // 限制内容最大宽度
                ui.add_space(SPACING_SM);

                let mut connection_names: Vec<String> =
                    connection_manager.connections.keys().cloned().collect();
                connection_names.sort_unstable();

                if connection_names.is_empty() {
                    Self::show_empty_state(ui, show_connection_dialog, keybindings);
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
    fn show_header(
        ui: &mut egui::Ui,
        show_connection_dialog: &mut bool,
        keybindings: &KeyBindings,
        is_focused: bool,
        focused_section: SidebarSection,
    ) {
        // 使用与工具栏完全相同的 Frame 包裹
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(MARGIN_MD, MARGIN_SM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::new(6.0, 0.0);

                    // 标题
                    ui.label(RichText::new("🔗 连接").strong());

                    // 显示当前焦点区域提示
                    if is_focused
                        && !matches!(
                            focused_section,
                            SidebarSection::Triggers
                                | SidebarSection::Routines
                                | SidebarSection::Filters
                        )
                    {
                        let section_text = match focused_section {
                            SidebarSection::Connections => "连接",
                            SidebarSection::Databases => "数据库",
                            SidebarSection::Tables => "表",
                            SidebarSection::Triggers => "触发器",
                            SidebarSection::Routines => "存储过程",
                            SidebarSection::Filters => "筛选",
                        };
                        ui.label(
                            RichText::new(format!("→ {}", section_text))
                                .small()
                                .color(SUCCESS),
                        );
                    }

                    // 把按钮推到右边
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 新建按钮 - 无边框图标样式
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("+")
                                        .size(15.0)
                                        .color(theme_text(ui.visuals())),
                                )
                                .frame(false)
                                .min_size(Vec2::new(24.0, 24.0)),
                            )
                            .on_hover_text(action_tooltip(keybindings, Action::NewConnection))
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
    fn show_empty_state(
        ui: &mut egui::Ui,
        show_connection_dialog: &mut bool,
        keybindings: &KeyBindings,
    ) {
        ui.vertical_centered(|ui| {
            ui.add_space(60.0);

            // 图标
            ui.label(RichText::new("📭").size(48.0));

            ui.add_space(SPACING_LG);

            ui.label(RichText::new("暂无连接").size(16.0).color(GRAY));

            ui.add_space(SPACING_SM);

            ui.label(
                RichText::new("创建一个数据库连接开始使用")
                    .small()
                    .color(MUTED),
            );

            ui.add_space(SPACING_LG);

            if ui
                .add(
                    egui::Button::new(
                        RichText::new("+ 新建连接")
                            .size(14.0)
                            .color(theme_text(ui.visuals())),
                    )
                    .frame(false)
                    .min_size(Vec2::new(0.0, 24.0)),
                )
                .on_hover_text(action_tooltip(keybindings, Action::NewConnection))
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
                let header_id = ui.make_persistent_id(("sidebar_connection", name));
                let mut collapsing_state =
                    egui::collapsing_header::CollapsingState::load_with_default_open(
                        ui.ctx(),
                        header_id,
                        conn_data.is_active,
                    );
                let header = Self::show_connection_header(
                    ui,
                    name,
                    conn_data.is_active,
                    conn_data.is_connected,
                    conn_data.selected_database.as_deref(),
                    is_nav_selected,
                    selected_table,
                    actions,
                    &mut collapsing_state,
                );

                collapsing_state.show_body_unindented(ui, |ui| {
                    ui.add_space(SPACING_SM);

                    // 连接信息
                    Self::show_connection_info(ui, &conn_data.db_type, &conn_data.host);

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

                if header.label_clicked || header.toggle_clicked {
                    actions.section_change = Some(SidebarSection::Connections);
                }

                // 右键菜单
                let is_active_for_menu = conn_data.is_active;
                header.interaction_response.context_menu(|ui| {
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
                    Self::show_delete_targets_menu(
                        ui,
                        name,
                        conn_data.selected_database.as_deref(),
                        actions,
                    );
                });
            });
    }

    #[allow(clippy::too_many_arguments)]
    fn show_connection_header(
        ui: &mut egui::Ui,
        name: &str,
        is_active: bool,
        is_connected: bool,
        selected_database: Option<&str>,
        is_nav_selected: bool,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
        collapsing_state: &mut egui::collapsing_header::CollapsingState,
    ) -> ConnectionHeaderRender {
        let mut label_clicked = false;
        let mut toggle_clicked = false;
        let mut interaction_response = None;
        let mut action_group = None;
        let mut toggle_rect = Rect::NOTHING;
        let mut label_rect = Rect::NOTHING;

        egui::Frame::NONE.show(ui, |ui| {
            let combined_response = ui
                .horizontal(|ui| {
                    let toggle_response = collapsing_state
                        .show_toggle_button(ui, egui::collapsing_header::paint_default_icon);
                    toggle_rect = toggle_response.rect;
                    if toggle_response.clicked() {
                        toggle_clicked = true;
                    }

                    let label_response = ui.add(
                        egui::Label::new(Self::connection_header_text(
                            name,
                            is_active,
                            is_connected,
                            is_nav_selected,
                        ))
                        .sense(egui::Sense::click()),
                    );
                    label_rect = label_response.rect;
                    if label_response.clicked() {
                        collapsing_state.toggle(ui);
                        label_clicked = true;
                    }

                    ui.add_space(SPACING_SM);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        action_group = Some(Self::show_connection_header_actions(
                            ui,
                            name,
                            is_active,
                            selected_database,
                            selected_table,
                            actions,
                        ));
                    });

                    toggle_response.union(label_response)
                })
                .inner;
            interaction_response = Some(combined_response);
        });

        let action_group = action_group.expect("connection header action group");
        ConnectionHeaderRender {
            interaction_response: interaction_response.expect("connection header response"),
            label_clicked,
            toggle_clicked,
            action_rect: action_group.rect,
            toggle_rect,
            label_rect,
            connection_delete_rect: action_group.connection_delete_rect,
        }
    }

    /// 连接头部文本
    /// 使用图标+颜色双重指示，对色盲友好
    fn connection_header_text(
        name: &str,
        is_active: bool,
        is_connected: bool,
        is_nav_selected: bool,
    ) -> RichText {
        // 使用不同形状的图标来区分状态，而不仅依赖颜色
        let (icon, color) = if is_nav_selected {
            (">", Color32::from_rgb(100, 180, 255)) // 键盘导航选中
        } else if is_active && is_connected {
            ("*", SUCCESS) // 星号表示活跃连接
        } else if is_connected {
            ("+", Color32::from_rgb(100, 180, 100)) // 加号表示已连接但非活跃
        } else {
            ("-", GRAY) // 减号表示未连接
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
    fn show_connection_header_actions(
        ui: &mut egui::Ui,
        name: &str,
        is_active: bool,
        selected_database: Option<&str>,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
    ) -> ConnectionHeaderActionGroup {
        ui.horizontal(|ui| {
            let row_start = ui.cursor().min;
            ui.add_space(SPACING_LG);

            // 无边框图标按钮
            let icon_btn = |ui: &mut egui::Ui, icon: &str, tooltip: &str, color: Color32| -> bool {
                ui.add(
                    egui::Button::new(RichText::new(icon).size(14.0).color(color))
                        .frame(false)
                        .min_size(Vec2::new(22.0, 22.0)),
                )
                .on_hover_text(tooltip)
                .clicked()
            };

            if is_active {
                if icon_btn(ui, "⏏", "断开连接", theme_text(ui.visuals())) {
                    actions.disconnect = Some(name.to_string());
                    *selected_table = None;
                }
            } else if icon_btn(ui, "🔗", "连接", theme_text(ui.visuals())) {
                actions.connect = Some(name.to_string());
            }

            if let Some(database) = selected_database
                .map(str::trim)
                .filter(|database| !database.is_empty())
                && ui
                    .add(
                        egui::Button::new(RichText::new("删库").small().color(DANGER))
                            .min_size(Vec2::new(44.0, 22.0)),
                    )
                    .on_hover_text(format!("删除数据库 {}", database))
                    .clicked()
            {
                Self::request_database_delete(name, database, actions);
            }

            let connection_delete_response = ui
                .add(
                    egui::Button::new(RichText::new("删连").small().color(DANGER))
                        .min_size(Vec2::new(44.0, 22.0)),
                )
                .on_hover_text(format!("删除连接 {}", name));
            if connection_delete_response.clicked() {
                Self::request_connection_delete(name, actions);
            }
            let rect = Rect::from_min_max(row_start, ui.min_rect().max);
            ConnectionHeaderActionGroup {
                rect,
                connection_delete_rect: Some(connection_delete_response.rect),
            }
        })
        .inner
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

#[cfg(test)]
mod tests {
    use super::ConnectionList;
    use crate::ui::panels::sidebar::{SidebarActions, SidebarDeleteTarget};
    use egui::{Area, Context, Event, Id, RawInput};

    fn render_connection_header_pass(
        ctx: &Context,
        events: Vec<Event>,
    ) -> (super::ConnectionHeaderRender, SidebarActions) {
        ctx.begin_pass(RawInput {
            events,
            ..Default::default()
        });

        let mut actions = SidebarActions::default();
        let mut selected_table = None;
        let mut header = None;

        Area::new(Id::new("connection_header_test")).show(ctx, |ui| {
            let header_id = ui.make_persistent_id(("sidebar_connection_test", "prod"));
            let mut collapsing_state =
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    header_id,
                    true,
                );
            header = Some(ConnectionList::show_connection_header(
                ui,
                "prod",
                true,
                true,
                Some("analytics"),
                false,
                &mut selected_table,
                &mut actions,
                &mut collapsing_state,
            ));
        });

        let _ = ctx.end_pass();
        (header.expect("rendered connection header"), actions)
    }

    #[test]
    fn delete_targets_include_selected_database_and_connection() {
        let targets = ConnectionList::delete_targets_for_context("prod", Some("analytics"));
        assert_eq!(
            targets,
            vec![
                SidebarDeleteTarget::Database {
                    connection_name: "prod".to_string(),
                    database_name: "analytics".to_string(),
                },
                SidebarDeleteTarget::Connection("prod".to_string()),
            ]
        );
    }

    #[test]
    fn delete_targets_without_selected_database_only_include_connection() {
        let targets = ConnectionList::delete_targets_for_context("sqlite", None);
        assert_eq!(
            targets,
            vec![SidebarDeleteTarget::connection("sqlite".to_string())]
        );
    }

    #[test]
    fn request_connection_delete_matches_context_menu_connection_target() {
        let mut actions = SidebarActions::default();

        ConnectionList::request_connection_delete("prod", &mut actions);

        assert_eq!(
            actions.delete,
            ConnectionList::delete_targets_for_context("prod", Some("analytics"))
                .last()
                .cloned()
        );
    }

    #[test]
    fn request_database_delete_matches_context_menu_database_target() {
        let mut actions = SidebarActions::default();

        ConnectionList::request_database_delete("prod", "analytics", &mut actions);

        assert_eq!(
            actions.delete,
            ConnectionList::delete_targets_for_context("prod", Some("analytics"))
                .first()
                .cloned()
        );
    }

    #[test]
    fn request_table_delete_preserves_connection_context() {
        let mut actions = SidebarActions::default();

        ConnectionList::request_table_delete("prod", "users", &mut actions);

        assert_eq!(
            actions.delete,
            Some(SidebarDeleteTarget::table(
                "prod".to_string(),
                "users".to_string()
            ))
        );
    }

    #[test]
    fn connection_header_interaction_surface_excludes_action_buttons() {
        let ctx = Context::default();
        let (header, _) = render_connection_header_pass(&ctx, Vec::new());

        assert!(
            header
                .interaction_response
                .interact_rect
                .contains(header.label_rect.center())
        );
        assert!(
            header
                .interaction_response
                .interact_rect
                .contains(header.toggle_rect.center())
        );
        assert!(
            !header
                .interaction_response
                .interact_rect
                .contains(header.action_rect.center())
        );
    }

    #[test]
    fn connection_header_delete_button_has_independent_pointer_surface() {
        let ctx = Context::default();
        let (layout, _) = render_connection_header_pass(&ctx, Vec::new());
        let delete_rect = layout
            .connection_delete_rect
            .expect("connection delete button rect");

        assert!(layout.action_rect.contains(delete_rect.center()));
        assert!(
            !layout
                .interaction_response
                .interact_rect
                .contains(delete_rect.center())
        );
    }
}
