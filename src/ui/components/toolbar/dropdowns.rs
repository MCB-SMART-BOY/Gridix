use egui::{Color32, CornerRadius, Id, Key, RichText, Vec2};

use super::actions::{DropdownState, ToolbarActions};
use super::utils::render_menu_item;

/// 操作下拉菜单
pub fn show_actions_dropdown(ui: &mut egui::Ui, has_result: bool, actions: &mut ToolbarActions) {
    let id = Id::new("actions_dropdown");
    let popup_id = id.with("popup");
    
    let mut state = ui.ctx().data_mut(|d| d.get_temp::<DropdownState>(id).unwrap_or_default());
    
    let response = ui.add(
        egui::Button::new(RichText::new("⚡").size(15.0).color(Color32::LIGHT_GRAY))
            .frame(false)
            .min_size(Vec2::new(24.0, 24.0)),
    ).on_hover_text("操作菜单");
    
    if response.clicked() {
        state.is_open = !state.is_open;
        state.selected_index = 0;
    }
    
    if state.is_open {
        let menu_items = [
            ("导出", "Ctrl+E", has_result),
            ("导入", "Ctrl+I", true),
            ("ER图", "Ctrl+R", true),
            ("历史", "Ctrl+H", true),
        ];
        
        egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(response.rect.left_bottom() + Vec2::new(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(CornerRadius::same(8))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(140.0);
                        
                        // 键盘导航
                        let input_result = ui.input(|i| {
                            let mut close = false;
                            let mut confirm = false;
                            let mut new_idx: Option<usize> = None;
                            
                            if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                                let next = state.selected_index.saturating_add(1);
                                if next < menu_items.len() {
                                    new_idx = Some(next);
                                }
                            }
                            
                            if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp)) && state.selected_index > 0 {
                                new_idx = Some(state.selected_index - 1);
                            }
                            
                            if i.key_pressed(Key::Enter) {
                                confirm = true;
                                close = true;
                            }
                            
                            if i.key_pressed(Key::Escape) {
                                close = true;
                            }
                            
                            (close, confirm, new_idx)
                        });
                        
                        if let Some(new_idx) = input_result.2 {
                            state.selected_index = new_idx;
                        }
                        
                        if input_result.0 {
                            state.is_open = false;
                        }
                        
                        ui.add_space(4.0);
                        
                        for (idx, (label, shortcut, enabled)) in menu_items.iter().enumerate() {
                            let is_selected = idx == state.selected_index;
                            let item_response = render_menu_item(ui, label, shortcut, is_selected, *enabled);
                            
                            if is_selected {
                                item_response.scroll_to_me(Some(egui::Align::Center));
                            }
                            
                            if item_response.clicked() && *enabled {
                                match idx {
                                    0 => actions.export = true,
                                    1 => actions.import = true,
                                    2 => actions.toggle_er_diagram = true,
                                    3 => actions.show_history = true,
                                    _ => {}
                                }
                                state.is_open = false;
                            }
                            
                            if item_response.hovered() {
                                state.selected_index = idx;
                            }
                            
                            // 键盘确认
                            if input_result.1 && is_selected && *enabled {
                                match idx {
                                    0 => actions.export = true,
                                    1 => actions.import = true,
                                    2 => actions.toggle_er_diagram = true,
                                    3 => actions.show_history = true,
                                    _ => {}
                                }
                            }
                        }
                        
                        ui.add_space(4.0);
                    });
            });
        
        // 点击外部关闭
        let click_outside = ui.input(|i| {
            i.pointer.any_click()
                && !response.rect.contains(i.pointer.interact_pos().unwrap_or_default())
        });
        if click_outside {
            state.is_open = false;
        }
    }
    
    ui.ctx().data_mut(|d| d.insert_temp(id, state));
}

/// 新建下拉菜单
pub fn show_create_dropdown(ui: &mut egui::Ui, actions: &mut ToolbarActions) {
    let id = Id::new("create_dropdown");
    let popup_id = id.with("popup");
    
    let mut state = ui.ctx().data_mut(|d| d.get_temp::<DropdownState>(id).unwrap_or_default());
    
    let response = ui.add(
        egui::Button::new(RichText::new("+").size(15.0).color(Color32::LIGHT_GRAY))
            .frame(false)
            .min_size(Vec2::new(24.0, 24.0)),
    ).on_hover_text("新建菜单");
    
    if response.clicked() {
        state.is_open = !state.is_open;
        state.selected_index = 0;
    }
    
    if state.is_open {
        let menu_items = [
            ("新建表", "Ctrl+Shift+N"),
            ("新建库", "Ctrl+Shift+D"),
            ("新建用户", "Ctrl+Shift+U"),
        ];
        
        egui::Area::new(popup_id)
            .order(egui::Order::Foreground)
            .fixed_pos(response.rect.left_bottom() + Vec2::new(0.0, 4.0))
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(CornerRadius::same(8))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 12,
                        spread: 0,
                        color: Color32::from_black_alpha(60),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(160.0);
                        
                        // 键盘导航
                        let input_result = ui.input(|i| {
                            let mut close = false;
                            let mut confirm = false;
                            let mut new_idx: Option<usize> = None;
                            
                            if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                                let next = state.selected_index.saturating_add(1);
                                if next < menu_items.len() {
                                    new_idx = Some(next);
                                }
                            }
                            
                            if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp)) && state.selected_index > 0 {
                                new_idx = Some(state.selected_index - 1);
                            }
                            
                            if i.key_pressed(Key::Enter) {
                                confirm = true;
                                close = true;
                            }
                            
                            if i.key_pressed(Key::Escape) {
                                close = true;
                            }
                            
                            (close, confirm, new_idx)
                        });
                        
                        if let Some(new_idx) = input_result.2 {
                            state.selected_index = new_idx;
                        }
                        
                        if input_result.0 {
                            state.is_open = false;
                        }
                        
                        ui.add_space(4.0);
                        
                        for (idx, (label, shortcut)) in menu_items.iter().enumerate() {
                            let is_selected = idx == state.selected_index;
                            let item_response = render_menu_item(ui, label, shortcut, is_selected, true);
                            
                            if is_selected {
                                item_response.scroll_to_me(Some(egui::Align::Center));
                            }
                            
                            if item_response.clicked() {
                                match idx {
                                    0 => actions.create_table = true,
                                    1 => actions.create_database = true,
                                    2 => actions.create_user = true,
                                    _ => {}
                                }
                                state.is_open = false;
                            }
                            
                            if item_response.hovered() {
                                state.selected_index = idx;
                            }
                            
                            // 键盘确认
                            if input_result.1 && is_selected {
                                match idx {
                                    0 => actions.create_table = true,
                                    1 => actions.create_database = true,
                                    2 => actions.create_user = true,
                                    _ => {}
                                }
                            }
                        }
                        
                        ui.add_space(4.0);
                    });
            });
        
        // 点击外部关闭
        let click_outside = ui.input(|i| {
            i.pointer.any_click()
                && !response.rect.contains(i.pointer.interact_pos().unwrap_or_default())
        });
        if click_outside {
            state.is_open = false;
        }
    }
    
    ui.ctx().data_mut(|d| d.insert_temp(id, state));
}
