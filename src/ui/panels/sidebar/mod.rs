//! 侧边栏组件 - 连接管理和表列表
//! 
//! 侧边栏分为上下两部分：
//! - 上部：连接/数据库/表列表
//! - 下部：触发器列表

mod state;
mod actions;
mod connection_list;
mod database_list;
mod table_list;
mod trigger_panel;

pub use state::{SidebarPanelState, SidebarSelectionState};
pub use actions::{SidebarActions, SidebarFocusTransfer};

use connection_list::ConnectionList;
use database_list::DatabaseList;
use table_list::TableList;
use trigger_panel::TriggerPanel;

use crate::database::ConnectionManager;
use crate::ui::SidebarSection;
use egui::{self, Color32, CornerRadius, Vec2};

pub struct Sidebar;

impl Sidebar {
    /// 在给定的 UI 区域内显示侧边栏内容
    /// 
    /// 这个函数不再使用 SidePanel，而是直接在传入的 ui 区域内渲染，
    /// 以便与 CentralPanel 内的其他内容（如数据表格）紧密贴合。
    pub fn show_in_ui(
        ui: &mut egui::Ui,
        connection_manager: &mut ConnectionManager,
        selected_table: &mut Option<String>,
        show_connection_dialog: &mut bool,
        is_focused: bool,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        width: f32,
    ) -> SidebarActions {
        let mut actions = SidebarActions::default();
        let ctx = ui.ctx().clone();
        
        // 获取当前区域的项目数量和对应的选中索引
        let (item_count, selected_index) = match focused_section {
            SidebarSection::Connections => (
                connection_manager.connections.len(),
                &mut panel_state.selection.connections,
            ),
            SidebarSection::Databases => (
                connection_manager.get_active().map(|c| c.databases.len()).unwrap_or(0),
                &mut panel_state.selection.databases,
            ),
            SidebarSection::Tables => (
                connection_manager.get_active().map(|c| c.tables.len()).unwrap_or(0),
                &mut panel_state.selection.tables,
            ),
            SidebarSection::Triggers => (
                panel_state.triggers.len(),
                &mut panel_state.selection.triggers,
            ),
        };
        
        // 确保索引在有效范围内
        if item_count > 0 && *selected_index >= item_count {
            *selected_index = item_count.saturating_sub(1);
        }
        
        // 处理侧边栏键盘导航（仅在聚焦时响应）
        if is_focused && item_count > 0 {
            Self::handle_keyboard_navigation(
                &ctx,
                focused_section,
                panel_state,
                item_count,
                connection_manager,
                selected_table,
                &mut actions,
            );
        }

        // 固定侧边栏宽度，防止内容导致自动扩展
        ui.set_max_width(width);
        ui.set_min_width(width);
        let available_height = ui.available_height();
        
        // 计算上下面板的高度
        let divider_height = 8.0;
        let header_height = 40.0; // 预估标题栏高度
        let content_height = available_height - divider_height;
        
        let (upper_height, lower_height) = if panel_state.upper_collapsed && panel_state.lower_collapsed {
            // 都折叠时平分
            (content_height / 2.0, content_height / 2.0)
        } else if panel_state.upper_collapsed {
            // 上部折叠
            (header_height, content_height - header_height)
        } else if panel_state.lower_collapsed {
            // 下部折叠
            (content_height - header_height, header_height)
        } else {
            // 都展开，按比例分配
            let upper = content_height * panel_state.divider_ratio;
            let lower = content_height * (1.0 - panel_state.divider_ratio);
            (upper.max(60.0), lower.max(60.0))
        };
        
        // ====== 上部面板：连接/数据库/表 ======
        ConnectionList::show(
            ui,
            connection_manager,
            selected_table,
            show_connection_dialog,
            is_focused,
            focused_section,
            panel_state,
            &mut actions,
            upper_height,
        );
        
        // ====== 分割条 ======
        Self::show_divider(ui, panel_state, divider_height);
                
        // ====== 下部面板：触发器 ======
        TriggerPanel::show(
            ui,
            is_focused,
            focused_section,
            panel_state,
            lower_height,
        );

        actions
    }
    
    /// 处理键盘导航
    #[allow(clippy::too_many_arguments)]
    fn handle_keyboard_navigation(
        ctx: &egui::Context,
        focused_section: SidebarSection,
        panel_state: &mut SidebarPanelState,
        item_count: usize,
        connection_manager: &ConnectionManager,
        selected_table: &mut Option<String>,
        actions: &mut SidebarActions,
    ) {
        // 获取当前 section 对应的选中索引
        let selected_index = match focused_section {
            SidebarSection::Connections => &mut panel_state.selection.connections,
            SidebarSection::Databases => &mut panel_state.selection.databases,
            SidebarSection::Tables => &mut panel_state.selection.tables,
            SidebarSection::Triggers => &mut panel_state.selection.triggers,
        };
        
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
                    SidebarSection::Triggers => {
                        // 触发器选中时可以显示详情（暂时只是选中）
                    }
                }
            }
            
            // h 或左箭头：向上层级导航（表→数据库→连接）
            if i.key_pressed(egui::Key::H) || i.key_pressed(egui::Key::ArrowLeft) {
                let new_section = match focused_section {
                    SidebarSection::Triggers => Some(SidebarSection::Tables),
                    SidebarSection::Tables => {
                        // 如果有数据库列表（MySQL/PostgreSQL），进入数据库；否则进入连接
                        if connection_manager.get_active().map(|c| !c.databases.is_empty()).unwrap_or(false) {
                            Some(SidebarSection::Databases)
                        } else {
                            Some(SidebarSection::Connections)
                        }
                    }
                    SidebarSection::Databases => Some(SidebarSection::Connections),
                    SidebarSection::Connections => None, // 已经在最顶层
                };
                if let Some(section) = new_section {
                    actions.section_change = Some(section);
                }
            }
            
            // l 或右箭头：向下层级导航（连接→数据库→表→数据表格）
            if i.key_pressed(egui::Key::L) || i.key_pressed(egui::Key::ArrowRight) {
                let conn = connection_manager.get_active();
                let has_databases = conn.map(|c| !c.databases.is_empty()).unwrap_or(false);
                let has_tables = conn.map(|c| !c.tables.is_empty()).unwrap_or(false);
                let has_triggers = !panel_state.triggers.is_empty();
                
                let new_section = match focused_section {
                    SidebarSection::Connections => {
                        // 如果有数据库列表，进入数据库；否则如果有表，进入表
                        if has_databases {
                            Some(SidebarSection::Databases)
                        } else if has_tables {
                            Some(SidebarSection::Tables)
                        } else {
                            None // 转出到数据表格
                        }
                    }
                    SidebarSection::Databases => {
                        if has_tables {
                            Some(SidebarSection::Tables)
                        } else {
                            None // 转出到数据表格
                        }
                    }
                    SidebarSection::Tables => {
                        if has_triggers {
                            Some(SidebarSection::Triggers)
                        } else {
                            None // 转出到数据表格
                        }
                    }
                    SidebarSection::Triggers => None, // 转出到数据表格
                };
                
                if let Some(section) = new_section {
                    actions.section_change = Some(section);
                } else {
                    actions.focus_transfer = Some(SidebarFocusTransfer::ToDataGrid);
                }
            }
        });
        
        // 同步到旧的 trigger_selected_index 字段（保持向后兼容）
        if focused_section == SidebarSection::Triggers {
            panel_state.trigger_selected_index = panel_state.selection.triggers;
        }
    }
    
    /// 显示分割条
    fn show_divider(ui: &mut egui::Ui, panel_state: &mut SidebarPanelState, height: f32) {
        let (rect, response) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), height),
            egui::Sense::drag(),
        );
        
        // 绘制分割条
        let visuals = ui.style().interact(&response);
        let color = if response.dragged() || response.hovered() {
            visuals.bg_fill
        } else {
            Color32::from_rgba_unmultiplied(128, 128, 128, 60)
        };
        
        ui.painter().rect_filled(
            rect.shrink2(Vec2::new(4.0, 2.0)),
            CornerRadius::same(2),
            color,
        );
        
        // 中间的拖动指示器
        let center = rect.center();
        ui.painter().circle_filled(center, 3.0, Color32::from_gray(160));
        
        // 处理拖动
        if response.dragged() {
            panel_state.dragging_divider = true;
            let delta = response.drag_delta().y;
            let total_height = ui.available_height() + rect.height();
            let delta_ratio = delta / total_height;
            panel_state.divider_ratio = (panel_state.divider_ratio + delta_ratio).clamp(0.2, 0.8);
        } else {
            panel_state.dragging_divider = false;
        }
        
        // 鼠标光标
        if response.hovered() || response.dragged() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
    }
}
