//! 表列表渲染

use super::{SidebarActions, SidebarDeleteTarget, SidebarSelectionState};
use crate::database::ConnectionManager;
use crate::ui::SidebarSection;
use crate::ui::styles::{GRAY, MUTED, SPACING_LG, SPACING_SM};
use egui::{self, Color32, CornerRadius, RichText};

/// 表列表
pub struct TableList;

impl TableList {
    /// 显示表列表（SQLite 模式，直接在连接下）
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        conn_name: &str,
        tables: &[String],
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
        is_focused: bool,
        focused_section: SidebarSection,
        selection: &SidebarSelectionState,
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
            let is_nav_selected = highlight_tables && idx == selection.tables;

            let row_response = ui
                .horizontal(|ui| {
                    ui.add_space(SPACING_LG + 4.0);

                    // 表项
                    let table_bg = if is_nav_selected {
                        Color32::from_rgba_unmultiplied(100, 150, 255, 35) // 键盘导航选中（降低透明度）
                    } else if is_selected {
                        Color32::from_rgba_unmultiplied(100, 150, 200, 25)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let response = egui::Frame::NONE
                        .fill(table_bg)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            let (icon, color) = if is_nav_selected {
                                (">", Color32::from_rgb(100, 180, 255))
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
                        actions.section_change = Some(SidebarSection::Tables);
                        *selected_table = Some(table.clone());
                        connection_manager.active = Some(conn_name.to_string());
                        actions.query_table = Some(table.clone());
                    }

                    // 右键菜单
                    response.context_menu(|ui| {
                        if ui.button("📊 查询前 100 行").clicked() {
                            actions.query_table = Some(table.clone());
                            ui.close();
                        }
                        if ui.button("🔍 查看表结构").clicked() {
                            actions.show_table_schema = Some(table.clone());
                            ui.close();
                        }
                        if ui.button("🗑 删除表").clicked() {
                            actions.delete = Some(SidebarDeleteTarget::Table {
                                connection_name: conn_name.to_string(),
                                table_name: table.clone(),
                            });
                            ui.close();
                        }
                    });
                })
                .response;

            // 如果是选中项且有焦点，滚动到可见
            if is_nav_selected {
                row_response.scroll_to_me(Some(egui::Align::Center));
            }
        }
    }

    /// 显示嵌套的表列表（在数据库下方）
    #[allow(clippy::too_many_arguments)]
    pub fn show_nested(
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
            let row_response = ui
                .horizontal(|ui| {
                    ui.add_space(SPACING_LG);

                    let table_bg = if is_nav_selected {
                        Color32::from_rgba_unmultiplied(100, 150, 255, 35) // 键盘导航选中（降低透明度）
                    } else if is_selected {
                        Color32::from_rgba_unmultiplied(80, 120, 180, 30)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let response = egui::Frame::NONE
                        .fill(table_bg)
                        .corner_radius(CornerRadius::same(4))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            let text_color = if is_nav_selected {
                                Color32::from_rgb(100, 180, 255)
                            } else if is_selected {
                                Color32::from_rgb(150, 200, 255)
                            } else {
                                Color32::from_rgb(170, 170, 180)
                            };
                            let prefix = if is_nav_selected { "> " } else { "" };
                            ui.label(
                                RichText::new(format!("{}{}", prefix, table)).color(text_color),
                            );
                        })
                        .response
                        .interact(egui::Sense::click());

                    // 左键点击 - 查询表数据
                    if response.clicked() {
                        actions.section_change = Some(SidebarSection::Tables);
                        *selected_table = Some(table.clone());
                        connection_manager.active = Some(conn_name.to_string());
                        actions.query_table = Some(table.clone());
                    }

                    // 右键菜单
                    response.context_menu(|ui| {
                        if ui.button("查询前 100 行").clicked() {
                            actions.query_table = Some(table.clone());
                            ui.close();
                        }
                        if ui.button("查看表结构").clicked() {
                            actions.show_table_schema = Some(table.clone());
                            ui.close();
                        }
                        if ui.button("删除表").clicked() {
                            actions.delete = Some(SidebarDeleteTarget::Table {
                                connection_name: conn_name.to_string(),
                                table_name: table.clone(),
                            });
                            ui.close();
                        }
                    });
                })
                .response;

            // 如果是选中项且有焦点，滚动到可见
            if is_nav_selected {
                row_response.scroll_to_me(Some(egui::Align::Center));
            }
        }
    }
}
