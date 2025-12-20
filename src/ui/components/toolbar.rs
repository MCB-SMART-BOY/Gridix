#![allow(clippy::too_many_arguments)]

use crate::core::{ProgressManager, ThemeManager, ThemePreset};
use crate::ui::styles::{MUTED, MARGIN_MD, MARGIN_SM};
use egui::{self, Color32, Id, Key, RichText, CornerRadius, Vec2};

use super::ProgressIndicator;

/// 主题下拉框状态
#[derive(Default, Clone)]
struct ThemeComboState {
    selected_index: usize,
    is_open: bool,
}

/// 下拉菜单状态
#[derive(Default, Clone)]
struct DropdownState {
    is_open: bool,
    selected_index: usize,
}

pub struct Toolbar;

#[derive(Default)]
pub struct ToolbarActions {
    pub refresh_tables: bool,
    pub export: bool,
    pub import: bool,
    pub show_history: bool,
    pub show_help: bool,
    pub toggle_sidebar: bool,
    pub toggle_editor: bool,
    pub show_editor: bool,
    pub theme_changed: Option<ThemePreset>,
    pub toggle_dark_mode: bool,
    pub switch_connection: Option<String>,
    pub switch_database: Option<String>,
    pub switch_table: Option<String>,
    // 快捷键触发的下拉框打开
    pub open_theme_selector: bool,
    // 缩放操作
    pub zoom_in: bool,
    pub zoom_out: bool,
    pub zoom_reset: bool,
    // DDL 操作
    pub create_table: bool,
    pub create_database: bool,
    pub create_user: bool,
    // ER 图
    pub toggle_er_diagram: bool,
    // 关于对话框
    pub show_about: bool,
    // 快捷键设置
    pub show_keybindings: bool,
}

// 暗色主题列表
const DARK_THEMES: &[ThemePreset] = &[
    ThemePreset::TokyoNight,
    ThemePreset::TokyoNightStorm,
    ThemePreset::CatppuccinMocha,
    ThemePreset::CatppuccinMacchiato,
    ThemePreset::CatppuccinFrappe,
    ThemePreset::OneDark,
    ThemePreset::OneDarkVivid,
    ThemePreset::GruvboxDark,
    ThemePreset::Dracula,
    ThemePreset::Nord,
    ThemePreset::SolarizedDark,
    ThemePreset::MonokaiPro,
    ThemePreset::GithubDark,
];

// 亮色主题列表
const LIGHT_THEMES: &[ThemePreset] = &[
    ThemePreset::TokyoNightLight,
    ThemePreset::CatppuccinLatte,
    ThemePreset::OneLight,
    ThemePreset::GruvboxLight,
    ThemePreset::SolarizedLight,
    ThemePreset::GithubLight,
];

impl Toolbar {
    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ui: &mut egui::Ui,
        theme_manager: &ThemeManager,
        has_result: bool,
        show_sidebar: bool,
        show_editor: bool,
        is_dark_mode: bool,
        actions: &mut ToolbarActions,
        connections: &[String],
        active_connection: Option<&str>,
        databases: &[String],
        selected_database: Option<&str>,
        tables: &[String],
        selected_table: Option<&str>,
        ui_scale: f32,
        progress: &ProgressManager,
    ) -> Option<u64> {
        let mut cancel_task_id = None;
        actions.show_editor = show_editor;

        // 工具栏容器
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(MARGIN_MD, MARGIN_SM))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

                    // 左侧按钮组
                    Self::show_left_buttons(ui, show_sidebar, show_editor, actions);

                    ui.add_space(8.0);
                    Self::separator(ui);
                    ui.add_space(8.0);

                    // 操作按钮（移除了连接/库/表选择器，这些在左侧栏中已有）
                    Self::show_action_buttons(ui, has_result, actions);
                    
                    // 保留快捷键功能但不显示选择器
                    // 快捷键 Ctrl+1/2/3 仍可在 app 中触发侧边栏操作
                    let _ = (connections, active_connection, databases, selected_database, tables, selected_table);

                    // 右侧区域
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 圆形头像按钮
                        let avatar_size = 24.0;
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::splat(avatar_size),
                            egui::Sense::click(),
                        );
                        
                        // 绘制圆形背景
                        let center = rect.center();
                        let radius = avatar_size / 2.0;
                        let bg_color = if response.hovered() {
                            Color32::from_rgb(100, 149, 237)  // 悬停时更亮
                        } else {
                            Color32::from_rgb(70, 130, 180)   // 钢蓝色
                        };
                        
                        ui.painter().circle_filled(center, radius, bg_color);
                        
                        // 绘制笑脸图标
                        let text = "😊";
                        let font_id = egui::FontId::proportional(14.0);
                        let text_color = Color32::WHITE;
                        ui.painter().text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            text,
                            font_id,
                            text_color,
                        );
                        
                        if response.clicked() {
                            actions.show_about = true;
                        }
                        
                        response.on_hover_text("关于我们");
                        
                        ui.add_space(8.0);
                        Self::separator(ui);
                        ui.add_space(8.0);

                        // 缩放控制
                        Self::show_zoom_controls(ui, ui_scale, actions);

                        ui.add_space(4.0);
                        Self::separator(ui);
                        ui.add_space(4.0);

                        // 主题选择器 - 根据当前模式显示对应主题列表
                        let themes = if is_dark_mode { DARK_THEMES } else { LIGHT_THEMES };
                        let current_theme_idx = themes
                            .iter()
                            .position(|&t| t == theme_manager.current)
                            .unwrap_or(0);

                        if let Some(new_idx) = Self::helix_theme_combo_simple(
                            ui,
                            "theme_selector",
                            theme_manager.current,
                            current_theme_idx,
                            themes,
                            200.0,
                            actions.open_theme_selector,
                        )
                            && let Some(&preset) = themes.get(new_idx) {
                                actions.theme_changed = Some(preset);
                            }
                        actions.open_theme_selector = false;

                        ui.add_space(4.0);

                        // 日/夜模式切换按钮
                        let mode_icon = if is_dark_mode {
                            "夜 [Ctrl+D]"
                        } else {
                            "日 [Ctrl+D]"
                        };
                        let mode_tooltip = if is_dark_mode {
                            "切换到日间模式 [Ctrl+D]"
                        } else {
                            "切换到夜间模式 [Ctrl+D]"
                        };

                        let btn = Self::styled_button(mode_icon);
                        if ui.add(btn).on_hover_text(mode_tooltip).clicked() {
                            actions.toggle_dark_mode = true;
                        }

                        // 进度指示器（如果有活跃任务）
                        if progress.has_active_tasks() {
                            ui.add_space(8.0);
                            Self::separator(ui);
                            ui.add_space(4.0);
                            
                            if let Some(id) = ProgressIndicator::show_in_toolbar(ui, progress) {
                                cancel_task_id = Some(id);
                            }
                        }
                    });
                });
            });
        
        cancel_task_id
    }

    /// 显示左侧按钮
    fn show_left_buttons(
        ui: &mut egui::Ui,
        show_sidebar: bool,
        show_editor: bool,
        actions: &mut ToolbarActions,
    ) {
        // 侧边栏切换
        let sidebar_icon = if show_sidebar { "<" } else { ">" };
        if ui.add(Self::styled_button(&format!("{} 侧栏 [Ctrl+B]", sidebar_icon))).clicked() {
            actions.toggle_sidebar = true;
        }

        // 编辑器切换
        let editor_icon = if show_editor { "v" } else { "^" };
        if ui.add(Self::styled_button(&format!("{} 编辑器 [Ctrl+J]", editor_icon))).clicked() {
            actions.toggle_editor = true;
        }
    }

    /// 显示缩放控制
    fn show_zoom_controls(ui: &mut egui::Ui, ui_scale: f32, actions: &mut ToolbarActions) {
        // 放大按钮
        if ui
            .add(Self::small_button("+"))
            .on_hover_text("放大 [Ctrl++]")
            .clicked()
        {
            actions.zoom_in = true;
        }

        // 缩放比例显示（可点击重置）
        let scale_text = format!("{}%", (ui_scale * 100.0).round() as i32);
        if ui
            .add(Self::styled_button(&scale_text))
            .on_hover_text("重置缩放 [Ctrl+0]")
            .clicked()
        {
            actions.zoom_reset = true;
        }

        // 缩小按钮
        if ui
            .add(Self::small_button("-"))
            .on_hover_text("缩小 [Ctrl+-]")
            .clicked()
        {
            actions.zoom_out = true;
        }
    }

    /// 小按钮样式
    fn small_button(text: &str) -> egui::Button<'_> {
        egui::Button::new(RichText::new(text).size(13.0))
            .corner_radius(CornerRadius::same(6))
            .min_size(Vec2::new(28.0, 28.0))
    }

    /// 显示操作按钮
    fn show_action_buttons(ui: &mut egui::Ui, has_result: bool, actions: &mut ToolbarActions) {
        // 刷新
        if ui.add(Self::styled_button("刷新 [F5]")).clicked() {
            actions.refresh_tables = true;
        }

        ui.add_space(4.0);
        Self::separator(ui);
        ui.add_space(4.0);

        // 操作下拉菜单
        Self::show_actions_dropdown(ui, has_result, actions);

        ui.add_space(4.0);

        // 新建下拉菜单
        Self::show_create_dropdown(ui, actions);

        ui.add_space(4.0);
        Self::separator(ui);
        ui.add_space(4.0);

        // 快捷键设置
        if ui.add(Self::styled_button("⌨")).on_hover_text("快捷键设置").clicked() {
            actions.show_keybindings = true;
        }

        ui.add_space(4.0);

        // 帮助
        if ui.add(Self::styled_button("? [F1]")).on_hover_text("帮助 [F1]").clicked() {
            actions.show_help = true;
        }
    }

    /// 操作下拉菜单
    fn show_actions_dropdown(ui: &mut egui::Ui, has_result: bool, actions: &mut ToolbarActions) {
        let id = Id::new("actions_dropdown");
        let popup_id = id.with("popup");
        
        let mut state = ui.ctx().data_mut(|d| d.get_temp::<DropdownState>(id).unwrap_or_default());
        
        let response = ui.add(Self::styled_button("操作 v"));
        
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
                                let item_response = Self::render_menu_item(ui, label, shortcut, is_selected, *enabled);
                                
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
    fn show_create_dropdown(ui: &mut egui::Ui, actions: &mut ToolbarActions) {
        let id = Id::new("create_dropdown");
        let popup_id = id.with("popup");
        
        let mut state = ui.ctx().data_mut(|d| d.get_temp::<DropdownState>(id).unwrap_or_default());
        
        let response = ui.add(Self::styled_button("+ 新建 v"));
        
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
                                let item_response = Self::render_menu_item(ui, label, shortcut, is_selected, true);
                                
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

    /// 渲染菜单项
    fn render_menu_item(
        ui: &mut egui::Ui,
        label: &str,
        shortcut: &str,
        is_selected: bool,
        enabled: bool,
    ) -> egui::Response {
        let bg_color = if is_selected {
            Color32::from_rgba_unmultiplied(100, 140, 200, 40)
        } else {
            Color32::TRANSPARENT
        };
        
        let text_color = if !enabled {
            Color32::from_gray(100)
        } else if is_selected {
            Color32::from_rgb(200, 220, 255)
        } else {
            Color32::from_rgb(180, 180, 190)
        };
        
        let frame_response = egui::Frame::NONE
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    // 选中指示器
                    let indicator = if is_selected { ">" } else { " " };
                    ui.label(RichText::new(indicator).color(Color32::from_rgb(130, 180, 255)).monospace());
                    
                    // 标签
                    ui.label(RichText::new(label).color(text_color));
                    
                    // 快捷键（右对齐）
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(shortcut).small().color(MUTED));
                    });
                });
            });
        
        frame_response.response.interact(egui::Sense::click())
    }

    /// 样式化按钮
    fn styled_button(text: &str) -> egui::Button<'_> {
        egui::Button::new(RichText::new(text).size(13.0))
            .corner_radius(CornerRadius::same(6))
            .min_size(Vec2::new(0.0, 28.0))
    }

    /// 分隔符
    fn separator(ui: &mut egui::Ui) {
        ui.add_space(2.0);
        let rect = ui.available_rect_before_wrap();
        let height = 20.0;
        let y_center = rect.center().y;
        ui.painter().vline(
            rect.left(),
            (y_center - height / 2.0)..=(y_center + height / 2.0),
            egui::Stroke::new(1.0, Color32::from_white_alpha(30)),
        );
        ui.add_space(2.0);
    }

    /// 简化版主题选择下拉框（只显示指定的主题列表）
    fn helix_theme_combo_simple(
        ui: &mut egui::Ui,
        id_source: &str,
        current_theme: ThemePreset,
        selected_index: usize,
        themes: &[ThemePreset],
        width: f32,
        force_open: bool,
    ) -> Option<usize> {
        let id = Id::new(id_source);
        let popup_id = id.with("popup");
        let mut result = None;

        // 获取状态
        let mut state = ui
            .ctx()
            .data_mut(|d| d.get_temp::<ThemeComboState>(id).unwrap_or_default());

        // 快捷键触发打开
        if force_open {
            state.is_open = true;
        }

        // 同步选中索引
        if !state.is_open {
            state.selected_index = selected_index;
        }

        // 按钮
        let display_text = format!("{} [Ctrl+Shift+T]", current_theme.display_name());
        let response = ui
            .add(Self::styled_button(&display_text))
            .on_hover_text("选择主题 [Ctrl+Shift+T]");

        if response.clicked() {
            state.is_open = !state.is_open;
        }

        // 弹出菜单
        if state.is_open {
            egui::Area::new(popup_id)
                .order(egui::Order::Foreground)
                .fixed_pos(
                    response.rect.left_bottom()
                        - Vec2::new(width - response.rect.width(), -4.0),
                )
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
                            ui.set_min_width(width.min(200.0));
                            ui.set_max_width(width);

                            let themes_len = themes.len();

                            // 键盘处理
                            let input_result = ui.input(|i| {
                                let mut close = false;
                                let mut confirm = false;
                                let mut new_idx: Option<usize> = None;

                                if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                                    let next = state.selected_index.saturating_add(1);
                                    if next < themes_len {
                                        new_idx = Some(next);
                                    }
                                }

                                if (i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp))
                                    && state.selected_index > 0
                                {
                                    new_idx = Some(state.selected_index - 1);
                                }

                                if i.key_pressed(Key::Enter) || i.key_pressed(Key::L) {
                                    confirm = true;
                                    close = true;
                                }

                                if i.key_pressed(Key::Escape) || i.key_pressed(Key::H) {
                                    close = true;
                                }

                                if i.key_pressed(Key::G) && !i.modifiers.shift {
                                    new_idx = Some(0);
                                }

                                if i.key_pressed(Key::G) && i.modifiers.shift {
                                    new_idx = Some(themes_len.saturating_sub(1));
                                }

                                (close, confirm, new_idx)
                            });

                            if let Some(new_idx) = input_result.2 {
                                state.selected_index = new_idx;
                            }

                            if input_result.0 {
                                if input_result.1 {
                                    result = Some(state.selected_index);
                                }
                                state.is_open = false;
                            }

                            // 主题列表
                            egui::ScrollArea::vertical()
                                .max_height(300.0)
                                .show(ui, |ui| {
                                    ui.add_space(4.0);
                                    for (idx, theme) in themes.iter().enumerate() {
                                        let is_hover = idx == state.selected_index;
                                        let item_response =
                                            Self::render_combo_item(ui, theme.display_name(), is_hover, false);

                                        // 键盘选中时自动滚动到该项
                                        if is_hover {
                                            item_response.scroll_to_me(Some(egui::Align::Center));
                                        }

                                        if item_response.clicked() {
                                            result = Some(idx);
                                            state.is_open = false;
                                        }

                                        if item_response.hovered() {
                                            state.selected_index = idx;
                                        }
                                    }
                                    ui.add_space(4.0);
                                });

                            // 提示
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.add_space(8.0);
                                ui.label(
                                    RichText::new("j/k 选择  Enter 确认  Esc 取消")
                                        .small()
                                        .color(MUTED),
                                );
                            });
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

        // 保存状态
        ui.ctx().data_mut(|d| {
            d.insert_temp(id, state);
        });

        result
    }

    /// 渲染下拉选项
    fn render_combo_item(
        ui: &mut egui::Ui,
        text: &str,
        is_hover: bool,
        is_light_theme: bool,
    ) -> egui::Response {
        let bg_color = if is_hover {
            Color32::from_rgba_unmultiplied(100, 140, 200, 40)
        } else {
            Color32::TRANSPARENT
        };

        let frame_response = egui::Frame::NONE
            .fill(bg_color)
            .inner_margin(egui::Margin::symmetric(12, 6))
            .corner_radius(4.0)
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal(|ui| {
                    // 选中指示器
                    let indicator = if is_hover { ">" } else { " " };
                    ui.label(
                        RichText::new(indicator)
                            .color(Color32::from_rgb(130, 180, 255))
                            .monospace(),
                    );

                    // 主题名称
                    let text_color = if is_hover {
                        Color32::from_rgb(200, 220, 255)
                    } else {
                        Color32::from_rgb(180, 180, 190)
                    };
                    ui.label(RichText::new(text).color(text_color));

                    // 浅色主题标识
                    if is_light_theme {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new("日").small().color(Color32::from_rgb(255, 200, 100)));
                        });
                    }
                });
            });

        frame_response.response.interact(egui::Sense::click())
    }
}
