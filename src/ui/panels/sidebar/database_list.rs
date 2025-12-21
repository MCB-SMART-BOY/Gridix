//! 数据库列表渲染

use crate::database::ConnectionManager;
use crate::ui::styles::{MUTED, SPACING_LG};
use crate::ui::SidebarSection;
use super::{SidebarActions, SidebarSelectionState, TableList};
use egui::{self, Color32, RichText, CornerRadius};

/// 数据库列表
pub struct DatabaseList;

impl DatabaseList {
    /// 显示数据库列表（MySQL/PostgreSQL）
    #[allow(clippy::too_many_arguments)]
    pub fn show(
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
        selection: &SidebarSelectionState,
    ) {
        // 数据库区域是否高亮
        let highlight_databases = is_focused && focused_section == SidebarSection::Databases;
        // 表区域是否高亮
        let highlight_tables = is_focused && focused_section == SidebarSection::Tables;
        
        // 数据库列表
        for (idx, database) in databases.iter().enumerate() {
            let is_selected = selected_database == Some(database.as_str());
            let is_nav_selected = highlight_databases && idx == selection.databases;

            // 数据库项 - 整行可点击
            let db_bg = if is_nav_selected {
                Color32::from_rgba_unmultiplied(100, 150, 255, 35)  // 键盘导航选中（降低透明度）
            } else if is_selected {
                Color32::from_rgba_unmultiplied(80, 140, 80, 30)
            } else {
                Color32::TRANSPARENT
            };
            let db_response = egui::Frame::NONE
                .fill(db_bg)
                .corner_radius(CornerRadius::same(4))
                .inner_margin(egui::Margin::symmetric(8, 4))
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
                        let prefix = if is_nav_selected { "> " } else { "" };
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
            
            // 如果是选中项且有焦点，滚动到可见
            if is_nav_selected {
                db_response.scroll_to_me(Some(egui::Align::Center));
            }

            // 如果此数据库被选中，显示其下的表列表
            if is_selected && !tables.is_empty() {
                ui.add_space(SPACING_LG / 2.0);
                TableList::show_nested(
                    ui,
                    conn_name,
                    tables,
                    connection_manager,
                    selected_table,
                    actions,
                    highlight_tables,
                    selection.tables,
                );
            }
        }
    }
}
