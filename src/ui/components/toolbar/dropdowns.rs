use crate::core::{Action, KeyBindings};
use crate::ui::styles::theme_text;
use crate::ui::{LocalShortcut, action_tooltip, consume_local_shortcut, local_shortcut_tooltip};
use egui::{Color32, CornerRadius, Id, RichText, Vec2};

use super::actions::{DropdownState, ToolbarActions};
use super::utils::render_menu_item;

/// 操作下拉菜单
pub fn show_actions_dropdown(
    ui: &mut egui::Ui,
    keybindings: &KeyBindings,
    has_result: bool,
    force_open: bool,
    actions: &mut ToolbarActions,
) {
    let id = Id::new("actions_dropdown");
    let popup_id = id.with("popup");

    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<DropdownState>(id).unwrap_or_default());

    let response = ui
        .add(
            egui::Button::new(
                RichText::new("⚡")
                    .size(15.0)
                    .color(theme_text(ui.visuals())),
            )
            .frame(false)
            .min_size(Vec2::new(24.0, 24.0)),
        )
        .on_hover_text(local_shortcut_tooltip(
            "打开操作菜单（工具栏焦点在当前项时）",
            LocalShortcut::ToolbarActivate,
        ));

    if force_open {
        state.is_open = true;
        state.selected_index = 0;
    }

    if response.clicked() {
        state.is_open = !state.is_open;
        state.selected_index = 0;
    }

    if state.is_open {
        let menu_items = [
            ("导出", Action::Export, has_result),
            ("导入", Action::Import, true),
            ("ER图", Action::ToggleErDiagram, true),
            ("历史", Action::ShowHistory, true),
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
                        let input_result = ui.input_mut(|i| {
                            let mut close = false;
                            let mut confirm = false;
                            let mut new_idx: Option<usize> = None;

                            if consume_local_shortcut(i, LocalShortcut::ToolbarMenuNext) {
                                let next = state.selected_index.saturating_add(1);
                                if next < menu_items.len() {
                                    new_idx = Some(next);
                                }
                            } else if consume_local_shortcut(i, LocalShortcut::ToolbarMenuPrev)
                                && state.selected_index > 0
                            {
                                new_idx = Some(state.selected_index - 1);
                            } else if consume_local_shortcut(i, LocalShortcut::ToolbarMenuConfirm) {
                                confirm = true;
                                close = true;
                            } else if consume_local_shortcut(i, LocalShortcut::ToolbarMenuDismiss) {
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

                        for (idx, (label, action, enabled)) in menu_items.iter().enumerate() {
                            let is_selected = idx == state.selected_index;
                            let item_response = render_menu_item(
                                ui,
                                label,
                                &keybindings.display(*action),
                                is_selected,
                                *enabled,
                            )
                            .on_hover_text(action_tooltip(keybindings, *action));

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
                && !response
                    .rect
                    .contains(i.pointer.interact_pos().unwrap_or_default())
        });
        if click_outside {
            state.is_open = false;
        }
    }

    ui.ctx().data_mut(|d| d.insert_temp(id, state));
}

/// 新建下拉菜单
pub fn show_create_dropdown(
    ui: &mut egui::Ui,
    keybindings: &KeyBindings,
    force_open: bool,
    actions: &mut ToolbarActions,
) {
    let id = Id::new("create_dropdown");
    let popup_id = id.with("popup");

    let mut state = ui
        .ctx()
        .data_mut(|d| d.get_temp::<DropdownState>(id).unwrap_or_default());

    let response = ui
        .add(
            egui::Button::new(
                RichText::new("+")
                    .size(15.0)
                    .color(theme_text(ui.visuals())),
            )
            .frame(false)
            .min_size(Vec2::new(24.0, 24.0)),
        )
        .on_hover_text(local_shortcut_tooltip(
            "打开新建菜单（工具栏焦点在当前项时）",
            LocalShortcut::ToolbarActivate,
        ));

    if force_open {
        state.is_open = true;
        state.selected_index = 0;
    }

    if response.clicked() {
        state.is_open = !state.is_open;
        state.selected_index = 0;
    }

    if state.is_open {
        let menu_items = [
            ("新建表", Action::NewTable),
            ("新建库", Action::NewDatabase),
            ("新建用户", Action::NewUser),
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
                        let input_result = ui.input_mut(|i| {
                            let mut close = false;
                            let mut confirm = false;
                            let mut new_idx: Option<usize> = None;

                            if consume_local_shortcut(i, LocalShortcut::ToolbarMenuNext) {
                                let next = state.selected_index.saturating_add(1);
                                if next < menu_items.len() {
                                    new_idx = Some(next);
                                }
                            } else if consume_local_shortcut(i, LocalShortcut::ToolbarMenuPrev)
                                && state.selected_index > 0
                            {
                                new_idx = Some(state.selected_index - 1);
                            } else if consume_local_shortcut(i, LocalShortcut::ToolbarMenuConfirm) {
                                confirm = true;
                                close = true;
                            } else if consume_local_shortcut(i, LocalShortcut::ToolbarMenuDismiss) {
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

                        for (idx, (label, action)) in menu_items.iter().enumerate() {
                            let is_selected = idx == state.selected_index;
                            let item_response = render_menu_item(
                                ui,
                                label,
                                &keybindings.display(*action),
                                is_selected,
                                true,
                            )
                            .on_hover_text(action_tooltip(keybindings, *action));

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
                && !response
                    .rect
                    .contains(i.pointer.interact_pos().unwrap_or_default())
        });
        if click_outside {
            state.is_open = false;
        }
    }

    ui.ctx().data_mut(|d| d.insert_temp(id, state));
}
