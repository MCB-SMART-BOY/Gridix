//! 对话框公共样式和组件
//!
//! 提供统一的对话框样式和可复用的 UI 组件。

#![allow(dead_code)] // 公开 API，供未来使用

use crate::ui::styles::{
    DANGER, GRAY, MUTED, SPACING_LG, SPACING_MD, SPACING_SM, SUCCESS, contrasting_text,
    theme_accent, theme_muted_text, theme_selection_fill, theme_subtle_stroke, theme_text,
    theme_warn,
};
use crate::ui::{
    LocalShortcut, consume_scoped_command_with_text_priority, local_shortcut_tooltip,
    text_entry_has_priority,
};
use egui::{self, Color32, CornerRadius, RichText, ScrollArea, Stroke, Vec2};
use std::collections::HashMap;
use std::hash::Hash;

/// 对话框样式预设
#[derive(Debug, Clone, Copy)]
pub struct DialogStyle {
    /// 最小宽度
    pub min_width: f32,
    /// 默认宽度
    pub default_width: f32,
    /// 最大宽度
    pub max_width: f32,
    /// 最小高度
    pub min_height: f32,
    /// 默认高度
    pub default_height: f32,
    /// 最大高度
    pub max_height: f32,
    /// 窗口内边距
    pub padding: i8,
    /// 窗口圆角
    pub radius: u8,
    /// 按钮高度
    pub button_height: f32,
    /// 按钮圆角
    pub button_radius: u8,
}

impl DialogStyle {
    /// 小型对话框（确认框、简单提示）
    pub const SMALL: Self = Self {
        min_width: 300.0,
        default_width: 360.0,
        max_width: 460.0,
        min_height: 160.0,
        default_height: 220.0,
        max_height: 360.0,
        padding: 14,
        radius: 12,
        button_height: 34.0,
        button_radius: 10,
    };

    /// 中型对话框（普通表单）
    pub const MEDIUM: Self = Self {
        min_width: 400.0,
        default_width: 520.0,
        max_width: 760.0,
        min_height: 240.0,
        default_height: 420.0,
        max_height: 720.0,
        padding: 16,
        radius: 14,
        button_height: 36.0,
        button_radius: 10,
    };

    /// 大型对话框（复杂表单、预览）
    pub const LARGE: Self = Self {
        min_width: 540.0,
        default_width: 700.0,
        max_width: 1040.0,
        min_height: 360.0,
        default_height: 540.0,
        max_height: 860.0,
        padding: 18,
        radius: 14,
        button_height: 38.0,
        button_radius: 11,
    };

    /// 工作台型对话框（帮助、快捷键、大型编辑器）
    pub const WORKSPACE: Self = Self {
        min_width: 560.0,
        default_width: 980.0,
        max_width: 1480.0,
        min_height: 480.0,
        default_height: 720.0,
        max_height: 980.0,
        padding: 18,
        radius: 16,
        button_height: 38.0,
        button_radius: 11,
    };

    fn responsive_widths(&self, ctx: &egui::Context) -> (f32, f32, f32) {
        Self::clamp_dimension(
            ctx.input(|i| i.content_rect().width()),
            self.min_width,
            self.default_width,
            self.max_width,
        )
    }

    fn responsive_heights(&self, ctx: &egui::Context) -> (f32, f32, f32) {
        Self::clamp_dimension(
            ctx.input(|i| i.content_rect().height()),
            self.min_height,
            self.default_height,
            self.max_height,
        )
    }

    fn clamp_dimension(available: f32, min: f32, default: f32, max: f32) -> (f32, f32, f32) {
        let usable = (available - 36.0).max(220.0);
        let max_value = usable.min(max);
        let min_value = min.min(max_value);
        let default_value = default.clamp(min_value, max_value);
        (min_value, default_value, max_value)
    }
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// 对话框局部快捷键适配器。
///
/// Dialog 是输入路由中的最高优先级区域；这里统一使用 scoped local shortcut，
/// 让文本输入控件优先于普通命令键，同时保留 Esc 等非文本控制键。
pub struct DialogShortcutContext<'a> {
    ctx: &'a egui::Context,
}

impl<'a> DialogShortcutContext<'a> {
    pub const fn new(ctx: &'a egui::Context) -> Self {
        Self { ctx }
    }

    pub fn consume(&self, shortcut: LocalShortcut) -> bool {
        self.consume_command(shortcut.config_key())
    }

    pub fn consume_command(&self, command_id: &'static str) -> bool {
        let text_entry_active = text_entry_has_priority(self.ctx);
        self.ctx.input_mut(|input| {
            consume_scoped_command_with_text_priority(input, command_id, text_entry_active)
        })
    }

    pub fn consume_any(&self, shortcuts: &[LocalShortcut]) -> bool {
        let command_ids: Vec<&'static str> = shortcuts
            .iter()
            .copied()
            .map(LocalShortcut::config_key)
            .collect();
        self.consume_any_commands(&command_ids)
    }

    pub fn consume_any_commands(&self, command_ids: &[&'static str]) -> bool {
        let text_entry_active = text_entry_has_priority(self.ctx);
        self.ctx.input_mut(|input| {
            command_ids.iter().copied().any(|command_id| {
                consume_scoped_command_with_text_priority(input, command_id, text_entry_active)
            })
        })
    }

    pub fn resolve<T: Copy>(&self, shortcuts: &[(LocalShortcut, T)]) -> Option<T> {
        let command_ids: Vec<(&'static str, T)> = shortcuts
            .iter()
            .copied()
            .map(|(shortcut, action)| (shortcut.config_key(), action))
            .collect();
        self.resolve_commands(&command_ids)
    }

    pub fn resolve_commands<T: Copy>(&self, command_ids: &[(&'static str, T)]) -> Option<T> {
        let text_entry_active = text_entry_has_priority(self.ctx);
        self.ctx.input_mut(|input| {
            command_ids.iter().find_map(|(command_id, action)| {
                consume_scoped_command_with_text_priority(input, command_id, text_entry_active)
                    .then_some(*action)
            })
        })
    }
}

/// 对话框头部渲染器
pub struct DialogHeader;

impl DialogHeader {
    /// 渲染对话框标题
    pub fn show(ui: &mut egui::Ui, title: &str, style: &DialogStyle) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(18.0).strong());
        });
        ui.add_space(SPACING_SM);
        ui.separator();
        ui.add_space(SPACING_MD);
        let _ = style;
    }

    /// 渲染带图标的对话框标题
    pub fn show_with_icon(ui: &mut egui::Ui, icon: &str, title: &str, _style: &DialogStyle) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(20.0));
            ui.add_space(SPACING_SM);
            ui.label(RichText::new(title).size(18.0).strong());
        });
        ui.add_space(SPACING_SM);
        ui.separator();
        ui.add_space(SPACING_MD);
    }

    /// 渲染带关闭按钮的标题栏
    pub fn show_with_close(ui: &mut egui::Ui, title: &str, _style: &DialogStyle) -> bool {
        let mut close_clicked = false;

        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(18.0).strong());

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("✕")
                                .size(14.0)
                                .color(theme_text(ui.visuals())),
                        )
                        .frame(false)
                        .min_size(Vec2::new(24.0, 24.0)),
                    )
                    .on_hover_text(local_shortcut_tooltip("关闭", LocalShortcut::Cancel))
                    .clicked()
                {
                    close_clicked = true;
                }
            });
        });
        ui.add_space(SPACING_SM);
        ui.separator();
        ui.add_space(SPACING_MD);

        close_clicked
    }
}

/// 对话框底部按钮渲染器
pub struct DialogFooter;

/// 底部按钮点击结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FooterResult {
    /// 确认按钮是否被点击
    pub confirmed: bool,
    /// 取消按钮是否被点击
    pub cancelled: bool,
}

impl FooterResult {
    /// 无操作
    pub const NONE: Self = Self {
        confirmed: false,
        cancelled: false,
    };

    /// 已确认
    pub const CONFIRMED: Self = Self {
        confirmed: true,
        cancelled: false,
    };

    /// 已取消
    pub const CANCELLED: Self = Self {
        confirmed: false,
        cancelled: true,
    };

    /// 是否有任何操作
    pub fn has_action(&self) -> bool {
        self.confirmed || self.cancelled
    }
}

impl DialogFooter {
    fn secondary_button(ui: &egui::Ui, text: &str, style: &DialogStyle) -> egui::Button<'static> {
        egui::Button::new(RichText::new(text.to_owned()).color(ui.visuals().text_color()))
            .fill(ui.visuals().faint_bg_color)
            .stroke(Stroke::new(
                1.0,
                ui.visuals().window_stroke.color.gamma_multiply(0.7),
            ))
            .corner_radius(CornerRadius::same(style.button_radius))
            .min_size(Vec2::new(88.0, style.button_height))
    }

    fn primary_button(
        text: &str,
        style: &DialogStyle,
        fill: Color32,
        enabled: bool,
    ) -> egui::Button<'static> {
        egui::Button::new(
            RichText::new(text.to_owned())
                .strong()
                .color(contrasting_text(fill)),
        )
        .fill(if enabled {
            fill
        } else {
            fill.gamma_multiply(0.35)
        })
        .corner_radius(CornerRadius::same(style.button_radius))
        .min_size(Vec2::new(112.0, style.button_height))
    }

    /// 渲染标准的确认/取消按钮
    pub fn show(
        ui: &mut egui::Ui,
        confirm_text: &str,
        cancel_text: &str,
        confirm_enabled: bool,
        style: &DialogStyle,
    ) -> FooterResult {
        let mut result = FooterResult::NONE;

        ui.add_space(SPACING_MD);
        ui.separator();
        ui.add_space(SPACING_SM);

        ui.horizontal(|ui| {
            if ui
                .add(Self::secondary_button(ui, cancel_text, style))
                .on_hover_text(local_shortcut_tooltip("取消", LocalShortcut::Cancel))
                .clicked()
            {
                result.cancelled = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(
                        confirm_enabled,
                        Self::primary_button(confirm_text, style, SUCCESS, confirm_enabled),
                    )
                    .on_hover_text(local_shortcut_tooltip("确认", LocalShortcut::Confirm))
                    .clicked()
                {
                    result.confirmed = true;
                }
            });
        });

        result
    }

    /// 渲染危险操作的按钮
    pub fn show_danger(
        ui: &mut egui::Ui,
        confirm_text: &str,
        cancel_text: &str,
        style: &DialogStyle,
    ) -> FooterResult {
        let mut result = FooterResult::NONE;

        ui.add_space(SPACING_MD);
        ui.separator();
        ui.add_space(SPACING_SM);

        ui.horizontal(|ui| {
            if ui
                .add(Self::secondary_button(ui, cancel_text, style))
                .on_hover_text(local_shortcut_tooltip("取消", LocalShortcut::Cancel))
                .clicked()
            {
                result.cancelled = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(Self::primary_button(confirm_text, style, DANGER, true))
                    .on_hover_text(local_shortcut_tooltip("确认操作", LocalShortcut::Confirm))
                    .clicked()
                {
                    result.confirmed = true;
                }
            });
        });

        result
    }

    /// 渲染只有关闭按钮的底部
    pub fn show_close_only(ui: &mut egui::Ui, close_text: &str, style: &DialogStyle) -> bool {
        ui.add_space(SPACING_MD);
        ui.separator();
        ui.add_space(SPACING_SM);

        let mut clicked = false;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(Self::primary_button(
                        close_text,
                        style,
                        ui.visuals().selection.bg_fill,
                        true,
                    ))
                    .on_hover_text(local_shortcut_tooltip("关闭", LocalShortcut::Cancel))
                    .clicked()
                {
                    clicked = true;
                }
            });
        });

        clicked
    }
}

/// 对话框内容区域组件
pub struct DialogContent;

impl DialogContent {
    const WORKSPACE_MIN_SIDEBAR_WIDTH: f32 = 180.0;
    const WORKSPACE_MIN_CONTENT_WIDTH: f32 = 280.0;
    const WORKSPACE_MIN_LEFT_WIDTH: f32 = 180.0;
    const WORKSPACE_MIN_MIDDLE_WIDTH: f32 = 260.0;
    const WORKSPACE_MIN_RIGHT_WIDTH: f32 = 280.0;

    fn blend(from: Color32, to: Color32, to_weight: f32) -> Color32 {
        let to_weight = to_weight.clamp(0.0, 1.0);
        let from_weight = 1.0 - to_weight;
        let mix = |a: u8, b: u8| ((a as f32 * from_weight) + (b as f32 * to_weight)).round() as u8;

        Color32::from_rgba_unmultiplied(
            mix(from.r(), to.r()),
            mix(from.g(), to.g()),
            mix(from.b(), to.b()),
            mix(from.a(), to.a()),
        )
    }

    fn with_alpha(color: Color32, alpha: u8) -> Color32 {
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha)
    }

    fn surface_fill(ui: &egui::Ui) -> Color32 {
        let visuals = ui.visuals();
        Self::blend(
            visuals.window_fill,
            visuals.faint_bg_color,
            if visuals.dark_mode { 0.18 } else { 0.10 },
        )
    }

    fn surface_fill_elevated(ui: &egui::Ui) -> Color32 {
        let visuals = ui.visuals();
        Self::blend(
            visuals.window_fill,
            visuals.extreme_bg_color,
            if visuals.dark_mode { 0.12 } else { 0.04 },
        )
    }

    fn surface_stroke(ui: &egui::Ui, alpha: u8) -> Color32 {
        let stroke = theme_subtle_stroke(ui.visuals());
        Self::with_alpha(stroke, alpha)
    }

    fn card_frame(ui: &egui::Ui, tint: Option<Color32>) -> egui::Frame {
        let visuals = ui.visuals();
        let base = Self::surface_fill(ui);
        let fill = tint
            .map(|color| Self::blend(base, color, if visuals.dark_mode { 0.18 } else { 0.12 }))
            .unwrap_or(base);
        let stroke_color = tint
            .map(|color| Self::blend(Self::surface_stroke(ui, 128), color, 0.34))
            .unwrap_or_else(|| Self::surface_stroke(ui, if visuals.dark_mode { 112 } else { 96 }));

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke_color))
            .corner_radius(CornerRadius::same(9))
            .inner_margin(egui::Margin::same(10))
    }

    pub fn card(ui: &mut egui::Ui, tint: Option<Color32>, content: impl FnOnce(&mut egui::Ui)) {
        Self::card_frame(ui, tint).show(ui, content);
    }

    /// 根据当前可用高度返回更稳健的滚动区域高度。
    pub fn adaptive_height(ui: &egui::Ui, fraction: f32, min: f32, max: f32) -> f32 {
        let usable = ui.available_height().max(min);
        (usable * fraction).clamp(min, max)
    }

    /// 渲染用于摘要、筛选、模式切换等轻量控制区的工具条。
    pub fn toolbar(ui: &mut egui::Ui, content: impl FnOnce(&mut egui::Ui)) {
        let fill = Self::surface_fill_elevated(ui);
        let stroke = Self::surface_stroke(ui, if ui.visuals().dark_mode { 84 } else { 70 });

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, content);
    }

    /// 渲染表单字段
    pub fn form_field(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).color(GRAY));
            content(ui);
        });
        ui.add_space(SPACING_SM);
    }

    /// 渲染必填表单字段
    pub fn required_field(ui: &mut egui::Ui, label: &str, content: impl FnOnce(&mut egui::Ui)) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(label).color(GRAY));
            ui.label(RichText::new("*").color(DANGER).small());
            content(ui);
        });
        ui.add_space(SPACING_SM);
    }

    /// 渲染信息提示
    pub fn info_text(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ℹ").color(theme_accent(ui.visuals())));
            ui.label(RichText::new(text).small().color(MUTED));
        });
    }

    /// 渲染警告提示
    pub fn warning_text(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            let warn = theme_warn(ui.visuals());
            ui.label(RichText::new("⚠").color(warn));
            ui.label(RichText::new(text).small().color(warn));
        });
    }

    /// 渲染错误提示
    pub fn error_text(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("✕").color(DANGER));
            ui.label(RichText::new(text).small().color(DANGER));
        });
    }

    /// 渲染成功提示
    pub fn success_text(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("[OK]").color(SUCCESS));
            ui.label(RichText::new(text).small().color(SUCCESS));
        });
    }

    /// 渲染分隔的区块
    pub fn section(ui: &mut egui::Ui, title: &str, content: impl FnOnce(&mut egui::Ui)) {
        ui.add_space(SPACING_SM);
        ui.label(RichText::new(title).strong().color(GRAY));
        ui.add_space(SPACING_SM);

        Self::card(ui, None, content);

        ui.add_space(SPACING_MD);
    }

    /// 渲染带描述的区块，适合复杂表单和工作台面板。
    pub fn section_with_description(
        ui: &mut egui::Ui,
        title: &str,
        description: &str,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        ui.add_space(SPACING_SM);
        ui.label(RichText::new(title).strong().color(GRAY));
        if !description.is_empty() {
            ui.add_space(2.0);
            ui.label(RichText::new(description).small().color(MUTED));
        }
        ui.add_space(SPACING_SM);
        Self::card(ui, None, content);
        ui.add_space(SPACING_MD);
    }

    /// 渲染统一的代码/预览容器。
    pub fn code_surface(ui: &mut egui::Ui, max_height: f32, content: impl FnOnce(&mut egui::Ui)) {
        let tint = if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(36, 39, 45, 220)
        } else {
            Color32::from_rgba_unmultiplied(245, 247, 250, 255)
        };

        Self::card(ui, Some(tint), |ui| {
            ScrollArea::both()
                .auto_shrink([false, false])
                .max_height(max_height)
                .show(ui, content);
        });
    }

    /// 渲染统一的代码/预览容器，并允许显式指定滚动区 ID。
    pub fn code_surface_with_id(
        ui: &mut egui::Ui,
        id_salt: impl Hash,
        max_height: f32,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        let tint = if ui.visuals().dark_mode {
            Color32::from_rgba_unmultiplied(36, 39, 45, 220)
        } else {
            Color32::from_rgba_unmultiplied(245, 247, 250, 255)
        };

        Self::card(ui, Some(tint), |ui| {
            ScrollArea::both()
                .id_salt(id_salt)
                .auto_shrink([false, false])
                .max_height(max_height)
                .show(ui, content);
        });
    }

    /// 渲染统一的代码文本块。
    pub fn code_block(ui: &mut egui::Ui, text: &str, max_height: f32) {
        Self::code_surface(ui, max_height, |ui| {
            ui.label(
                RichText::new(text)
                    .monospace()
                    .small()
                    .color(ui.visuals().text_color().gamma_multiply(0.9)),
            );
        });
    }

    /// 渲染带显式 ID 的统一代码文本块。
    pub fn code_block_with_id(ui: &mut egui::Ui, id_salt: impl Hash, text: &str, max_height: f32) {
        Self::code_surface_with_id(ui, id_salt, max_height, |ui| {
            ui.label(
                RichText::new(text)
                    .monospace()
                    .small()
                    .color(ui.visuals().text_color().gamma_multiply(0.9)),
            );
        });
    }

    /// 渲染快捷键提示
    pub fn shortcut_hint(ui: &mut egui::Ui, hints: &[(&str, &str)]) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);

            for (key, action) in hints {
                ui.horizontal(|ui| {
                    egui::Frame::NONE
                        .fill(ui.visuals().extreme_bg_color.gamma_multiply(0.72))
                        .stroke(Stroke::new(1.0, Self::surface_stroke(ui, 72)))
                        .corner_radius(CornerRadius::same(5))
                        .inner_margin(egui::Margin::symmetric(6, 2))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(*key)
                                    .small()
                                    .monospace()
                                    .color(theme_text(ui.visuals())),
                            );
                        });
                    ui.label(
                        RichText::new(*action)
                            .small()
                            .color(theme_muted_text(ui.visuals())),
                    );
                });
            }
        });
        ui.add_space(SPACING_SM);
    }

    /// 用于工作台型对话框的左右分栏。
    pub fn split_workspace(
        ui: &mut egui::Ui,
        sidebar_width: f32,
        sidebar: impl FnOnce(&mut egui::Ui),
        content: impl FnOnce(&mut egui::Ui),
    ) {
        match Self::workspace_two_pane_widths(ui.available_width(), sidebar_width) {
            Some((sidebar_width, content_width)) => {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(sidebar_width, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_width(sidebar_width);
                            ui.set_max_width(sidebar_width);
                            sidebar(ui);
                        },
                    );

                    ui.add_space(SPACING_LG);

                    ui.allocate_ui_with_layout(
                        Vec2::new(content_width, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_width(content_width);
                            ui.set_max_width(content_width);
                            content(ui);
                        },
                    );
                });
            }
            None => {
                ui.vertical(|ui| {
                    sidebar(ui);
                    ui.add_space(SPACING_LG);
                    content(ui);
                });
            }
        }
    }

    /// 三栏工作台布局，适合树/列表/详情式复杂对话框。
    pub fn split_workspace_three(
        ui: &mut egui::Ui,
        left_width: f32,
        middle_width: f32,
        left: impl FnOnce(&mut egui::Ui),
        middle: impl FnOnce(&mut egui::Ui),
        right: impl FnOnce(&mut egui::Ui),
    ) {
        match Self::workspace_three_pane_widths(ui.available_width(), left_width, middle_width) {
            Some((left_width, middle_width, right_width)) => {
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(left_width, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_width(left_width);
                            ui.set_max_width(left_width);
                            left(ui);
                        },
                    );

                    ui.add_space(SPACING_LG);

                    ui.allocate_ui_with_layout(
                        Vec2::new(middle_width, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_width(middle_width);
                            ui.set_max_width(middle_width);
                            middle(ui);
                        },
                    );

                    ui.add_space(SPACING_LG);

                    ui.allocate_ui_with_layout(
                        Vec2::new(right_width, ui.available_height()),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_min_width(right_width);
                            ui.set_max_width(right_width);
                            right(ui);
                        },
                    );
                });
            }
            None => {
                ui.vertical(|ui| {
                    left(ui);
                    ui.add_space(SPACING_LG);
                    middle(ui);
                    ui.add_space(SPACING_LG);
                    right(ui);
                });
            }
        }
    }

    fn workspace_two_pane_widths(
        available_width: f32,
        preferred_sidebar_width: f32,
    ) -> Option<(f32, f32)> {
        let min_total =
            Self::WORKSPACE_MIN_SIDEBAR_WIDTH + SPACING_LG + Self::WORKSPACE_MIN_CONTENT_WIDTH;
        if available_width < min_total {
            return None;
        }

        let max_sidebar_width = (available_width - SPACING_LG - Self::WORKSPACE_MIN_CONTENT_WIDTH)
            .max(Self::WORKSPACE_MIN_SIDEBAR_WIDTH);
        let sidebar_width =
            preferred_sidebar_width.clamp(Self::WORKSPACE_MIN_SIDEBAR_WIDTH, max_sidebar_width);
        let content_width = available_width - sidebar_width - SPACING_LG;

        Some((sidebar_width, content_width))
    }

    fn workspace_three_pane_widths(
        available_width: f32,
        preferred_left_width: f32,
        preferred_middle_width: f32,
    ) -> Option<(f32, f32, f32)> {
        let spacing_total = SPACING_LG * 2.0;
        let min_total = Self::WORKSPACE_MIN_LEFT_WIDTH
            + Self::WORKSPACE_MIN_MIDDLE_WIDTH
            + Self::WORKSPACE_MIN_RIGHT_WIDTH
            + spacing_total;
        if available_width < min_total {
            return None;
        }

        let max_left_width = (available_width
            - spacing_total
            - Self::WORKSPACE_MIN_MIDDLE_WIDTH
            - Self::WORKSPACE_MIN_RIGHT_WIDTH)
            .max(Self::WORKSPACE_MIN_LEFT_WIDTH);
        let left_width = preferred_left_width.clamp(Self::WORKSPACE_MIN_LEFT_WIDTH, max_left_width);

        let max_middle_width =
            (available_width - spacing_total - left_width - Self::WORKSPACE_MIN_RIGHT_WIDTH)
                .max(Self::WORKSPACE_MIN_MIDDLE_WIDTH);
        let middle_width =
            preferred_middle_width.clamp(Self::WORKSPACE_MIN_MIDDLE_WIDTH, max_middle_width);
        let right_width = available_width - spacing_total - left_width - middle_width;

        Some((left_width, middle_width, right_width))
    }

    /// 工作台 pane：统一标题、说明和内容容器。
    pub fn workspace_pane(
        ui: &mut egui::Ui,
        title: &str,
        description: &str,
        content: impl FnOnce(&mut egui::Ui),
    ) {
        Self::card_frame(ui, None)
            .fill(Self::surface_fill_elevated(ui))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(title)
                        .strong()
                        .color(theme_text(ui.visuals())),
                );
                if !description.is_empty() {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(description)
                            .small()
                            .color(theme_muted_text(ui.visuals())),
                    );
                }
                ui.add_space(SPACING_SM);
                let rect = egui::Rect::from_min_size(
                    ui.cursor().min,
                    Vec2::new(ui.available_width(), 1.0),
                );
                ui.painter().line_segment(
                    [rect.left_center(), rect.right_center()],
                    Stroke::new(1.0, Self::surface_stroke(ui, 70)),
                );
                ui.add_space(1.0);
                ui.add_space(SPACING_SM);
                content(ui);
            });
    }

    /// 统一的鼠标交互提示。
    pub fn mouse_hint(ui: &mut egui::Ui, hints: &[(&str, &str)]) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 2.0);
            for (index, (gesture, desc)) in hints.iter().enumerate() {
                if index > 0 {
                    ui.label(
                        RichText::new("·")
                            .small()
                            .color(theme_muted_text(ui.visuals())),
                    );
                }
                ui.label(
                    RichText::new(*gesture)
                        .small()
                        .color(theme_text(ui.visuals())),
                );
                ui.label(
                    RichText::new(*desc)
                        .small()
                        .color(theme_muted_text(ui.visuals())),
                );
            }
        });
    }

    /// 工作台导航项，统一单击选中语义。
    pub fn nav_item(
        ui: &mut egui::Ui,
        selected: bool,
        label: impl Into<String>,
        meta: Option<&str>,
    ) -> egui::Response {
        let label = label.into();
        let fill = if selected {
            theme_selection_fill(ui.visuals(), if ui.visuals().dark_mode { 42 } else { 32 })
        } else {
            Color32::TRANSPARENT
        };

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(
                1.0,
                if selected {
                    theme_accent(ui.visuals()).gamma_multiply(0.42)
                } else {
                    Color32::TRANSPARENT
                },
            ))
            .corner_radius(CornerRadius::same(7))
            .inner_margin(egui::Margin::symmetric(8, 7))
            .show(ui, |ui| {
                ui.set_min_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    if selected {
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                ui.cursor().min + Vec2::new(0.0, 2.0),
                                Vec2::new(2.0, 24.0),
                            ),
                            CornerRadius::same(255),
                            theme_accent(ui.visuals()),
                        );
                    }
                    ui.add_space(8.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(label).strong().color(if selected {
                            theme_accent(ui.visuals())
                        } else {
                            theme_text(ui.visuals())
                        }));
                        if let Some(meta) = meta
                            && !meta.is_empty()
                        {
                            ui.add_space(2.0);
                            ui.label(
                                RichText::new(meta)
                                    .small()
                                    .color(theme_muted_text(ui.visuals())),
                            );
                        }
                    });
                });
            })
            .response
            .interact(egui::Sense::click())
    }
}

/// 对话框状态消息组件
pub struct DialogStatus;

impl DialogStatus {
    /// 渲染状态消息
    pub fn show(ui: &mut egui::Ui, result: &Result<String, String>) {
        let (icon, message, color) = match result {
            Ok(msg) => ("[OK]", msg.as_str(), SUCCESS),
            Err(msg) => ("[X]", msg.as_str(), DANGER),
        };

        DialogContent::card(ui, Some(color), |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).color(color));
                ui.label(RichText::new(message).small().color(color));
            });
        });
    }

    /// 渲染加载状态
    pub fn show_loading(ui: &mut egui::Ui, message: &str) {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(RichText::new(message).small().color(MUTED));
        });
    }
}

/// 对话框窗口配置助手
pub struct DialogWindow;

impl DialogWindow {
    fn viewport_rect(ctx: &egui::Context) -> egui::Rect {
        ctx.input(|input| input.content_rect())
    }

    fn frame(ctx: &egui::Context, style: &DialogStyle) -> egui::Frame {
        let visuals = &ctx.global_style().visuals;
        let shadow_alpha = if visuals.dark_mode { 74 } else { 26 };
        let window_fill = visuals.window_fill;
        let stroke = theme_subtle_stroke(visuals).gamma_multiply(0.92);

        egui::Frame::NONE
            .fill(window_fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(CornerRadius::same(style.radius))
            .inner_margin(egui::Margin::same(style.padding))
            .shadow(egui::epaint::Shadow {
                offset: [0, 10],
                blur: 30,
                spread: 0,
                color: Color32::from_black_alpha(shadow_alpha),
            })
    }

    /// 创建阻塞式 modal 对话框壳层。
    pub fn blocking(ctx: &egui::Context, id: impl Hash, style: &DialogStyle) -> egui::Modal {
        let backdrop_alpha = if ctx.global_style().visuals.dark_mode {
            132
        } else {
            110
        };

        egui::Modal::new(egui::Id::new(id))
            .backdrop_color(Color32::from_black_alpha(backdrop_alpha))
            .frame(Self::frame(ctx, style))
    }

    /// 为 modal 内容应用与标准 dialog shell 相同的宽度约束。
    pub fn apply_modal_width(ui: &mut egui::Ui, ctx: &egui::Context, style: &DialogStyle) {
        let (min_width, default_width, max_width) = style.responsive_widths(ctx);
        ui.set_min_width(min_width);
        ui.set_width(default_width);
        ui.set_max_width(max_width);
    }

    /// 创建标准对话框窗口
    pub fn standard<'a>(
        ctx: &egui::Context,
        title: &'a str,
        style: &DialogStyle,
    ) -> egui::Window<'a> {
        let content_rect = Self::viewport_rect(ctx);
        let (min_width, default_width, max_width) = style.responsive_widths(ctx);
        let (_, _, max_height) = style.responsive_heights(ctx);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .default_width(default_width)
            .min_width(min_width)
            .max_width(max_width)
            .max_height(max_height)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(content_rect)
            .frame(Self::frame(ctx, style))
    }

    /// 创建可调整大小的对话框窗口
    pub fn resizable<'a>(
        ctx: &egui::Context,
        title: &'a str,
        style: &DialogStyle,
    ) -> egui::Window<'a> {
        let content_rect = Self::viewport_rect(ctx);
        let (min_width, default_width, max_width) = style.responsive_widths(ctx);
        let (min_height, default_height, max_height) = style.responsive_heights(ctx);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_width(default_width)
            .default_height(default_height)
            .min_width(min_width)
            .min_height(min_height)
            .max_width(max_width)
            .max_height(max_height)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(content_rect)
            .frame(Self::frame(ctx, style))
    }

    /// 创建可拖拽、可调整大小的工作台型对话框窗口。
    pub fn workspace<'a>(
        ctx: &egui::Context,
        title: &'a str,
        style: &DialogStyle,
        default_width: f32,
        default_height: f32,
    ) -> egui::Window<'a> {
        let content_rect = Self::viewport_rect(ctx);
        let (min_width, _, max_width) = style.responsive_widths(ctx);
        let (min_height, _, max_height) = style.responsive_heights(ctx);
        let default_size = Vec2::new(
            default_width.clamp(min_width, max_width),
            default_height.clamp(min_height, max_height),
        );
        let default_pos = content_rect.center() - default_size * 0.5;

        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_pos(default_pos)
            .default_size(default_size)
            .min_width(min_width)
            .min_height(min_height)
            .max_width(max_width)
            .max_height(max_height)
            .hscroll(false)
            .constrain_to(content_rect)
            .frame(Self::frame(ctx, style))
    }

    /// 创建固定大小的对话框窗口
    pub fn fixed<'a>(
        ctx: &egui::Context,
        title: &'a str,
        width: f32,
        height: f32,
    ) -> egui::Window<'a> {
        let content_rect = Self::viewport_rect(ctx);
        let style = DialogStyle::MEDIUM;
        let (min_width, _, max_width) = style.responsive_widths(ctx);
        let (_, _, max_height) = style.responsive_heights(ctx);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(
                width.clamp(min_width, max_width),
                height.min(max_height),
            ))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(content_rect)
            .frame(Self::frame(ctx, &style))
    }

    /// 创建固定尺寸且沿用指定样式范围的对话框窗口。
    pub fn fixed_style<'a>(
        ctx: &egui::Context,
        title: &'a str,
        style: &DialogStyle,
        width: f32,
        height: f32,
    ) -> egui::Window<'a> {
        let content_rect = Self::viewport_rect(ctx);
        let (min_width, _, max_width) = style.responsive_widths(ctx);
        let (min_height, _, max_height) = style.responsive_heights(ctx);

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(
                width.clamp(min_width, max_width),
                height.clamp(min_height, max_height),
            ))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(content_rect)
            .frame(Self::frame(ctx, style))
    }
}

/// 工作台型对话框布局壳层。
///
/// 负责固定 header/subheader/footer，让 body 始终占用剩余空间。
pub(crate) struct WorkspaceDialogShell;

impl WorkspaceDialogShell {
    pub(crate) fn show(
        ui: &mut egui::Ui,
        id_source: impl Hash,
        header: impl FnOnce(&mut egui::Ui),
        subheader: impl FnOnce(&mut egui::Ui),
        body: impl FnOnce(&mut egui::Ui),
        footer: impl FnOnce(&mut egui::Ui),
    ) {
        let shell_id = ui.id().with(id_source);
        let top_rule = Stroke::new(
            1.0,
            DialogContent::surface_stroke(ui, if ui.visuals().dark_mode { 70 } else { 58 }),
        );

        egui::Panel::bottom(shell_id.with("footer"))
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().window_fill)
                    .stroke(top_rule)
                    .inner_margin(egui::Margin::symmetric(0, 6)),
            )
            .resizable(false)
            .show_inside(ui, |ui| {
                footer(ui);
            });

        egui::Panel::top(shell_id.with("header"))
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::Margin::same(0)),
            )
            .resizable(false)
            .show_inside(ui, |ui| {
                header(ui);
            });

        egui::Panel::top(shell_id.with("subheader"))
            .frame(
                egui::Frame::NONE
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::Margin::same(0)),
            )
            .resizable(false)
            .show_inside(ui, |ui| {
                subheader(ui);
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show_inside(ui, |ui| {
                body(ui);
            });
    }
}

/// 长表单对话框布局壳层。
///
/// 负责固定 header/footer，并让 body 成为唯一主滚动区域。
pub(crate) type FormFieldId = &'static str;

#[derive(Default)]
pub(crate) struct FormDialogBodyContext {
    field_rects: HashMap<FormFieldId, egui::Rect>,
    requested_first_error: Option<FormFieldId>,
}

impl FormDialogBodyContext {
    pub(crate) fn register_field(&mut self, field_id: FormFieldId, response: &egui::Response) {
        self.register_rect(field_id, response.rect);
    }

    pub(crate) fn register_rect(&mut self, field_id: FormFieldId, rect: egui::Rect) {
        self.field_rects.insert(field_id, rect);
    }

    pub(crate) fn request_first_error(&mut self, field_id: FormFieldId) {
        self.requested_first_error.get_or_insert(field_id);
    }

    fn requested_error_rect(&self) -> Option<egui::Rect> {
        self.requested_first_error
            .and_then(|field_id| self.field_rects.get(field_id).copied())
    }
}

pub(crate) struct FormDialogShell;

impl FormDialogShell {
    pub(crate) fn show(
        ui: &mut egui::Ui,
        id_source: impl Hash,
        header: impl FnOnce(&mut egui::Ui),
        body: impl FnOnce(&mut egui::Ui, &mut FormDialogBodyContext),
        footer: impl FnOnce(&mut egui::Ui),
    ) {
        let shell_id = ui.id().with(id_source);

        egui::Panel::bottom(shell_id.with("footer"))
            .frame(egui::Frame::NONE)
            .resizable(false)
            .show_inside(ui, |ui| {
                footer(ui);
            });

        egui::Panel::top(shell_id.with("header"))
            .frame(egui::Frame::NONE)
            .resizable(false)
            .show_inside(ui, |ui| {
                header(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ScrollArea::vertical()
                    .id_salt(shell_id.with("body"))
                    .show(ui, |ui| {
                        let mut body_context = FormDialogBodyContext::default();
                        body(ui, &mut body_context);
                        if let Some(rect) = body_context.requested_error_rect() {
                            ui.scroll_to_rect(rect, Some(egui::Align::Center));
                        }
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::{DialogContent, DialogShortcutContext, FormDialogBodyContext, SPACING_LG};
    use crate::ui::{LocalShortcut, text_entry_has_priority};
    use egui::{Event, Key, Modifiers, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        ctx.begin_pass(RawInput {
            events: vec![key_event(key)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
    }

    fn focus_text_input(ctx: &egui::Context) {
        let mut text = String::new();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("dialog shortcut test input").show(ctx, |ui| {
            let response =
                ui.add(egui::TextEdit::singleline(&mut text).id_salt("dialog_shortcut_text_input"));
            response.request_focus();
        });
        let _ = ctx.end_pass();
    }

    #[test]
    fn dialog_shortcut_context_consumes_non_text_control_key() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Escape);

        assert!(DialogShortcutContext::new(&ctx).consume(LocalShortcut::Dismiss));

        let _ = ctx.end_pass();
    }

    #[test]
    fn dialog_shortcut_context_consumes_scoped_command_id() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Escape);

        assert!(DialogShortcutContext::new(&ctx).consume_command("dialog.common.dismiss"));

        let _ = ctx.end_pass();
    }

    #[test]
    fn dialog_shortcut_context_blocks_text_conflicting_binding() {
        let ctx = egui::Context::default();
        focus_text_input(&ctx);
        begin_key_pass(&ctx, Key::Q);

        assert!(text_entry_has_priority(&ctx));
        assert!(!DialogShortcutContext::new(&ctx).consume(LocalShortcut::Dismiss));

        let _ = ctx.end_pass();
    }

    #[test]
    fn dialog_shortcut_context_resolves_scoped_command_ids_in_order() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Y);

        let action = DialogShortcutContext::new(&ctx).resolve_commands(&[
            ("dialog.confirm.cancel", false),
            ("dialog.confirm.confirm", true),
        ]);

        assert_eq!(action, Some(true));

        let _ = ctx.end_pass();
    }

    #[test]
    fn dialog_shortcut_context_resolves_first_matching_action() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Y);

        let action = DialogShortcutContext::new(&ctx).resolve(&[
            (LocalShortcut::DangerCancel, false),
            (LocalShortcut::DangerConfirm, true),
        ]);

        assert_eq!(action, Some(true));

        let _ = ctx.end_pass();
    }

    #[test]
    fn workspace_two_pane_widths_stack_when_width_is_too_small() {
        let widths = DialogContent::workspace_two_pane_widths(430.0, 250.0);

        assert_eq!(widths, None);
    }

    #[test]
    fn workspace_two_pane_widths_fit_available_width_without_overflow() {
        let (sidebar_width, content_width) =
            DialogContent::workspace_two_pane_widths(640.0, 250.0).unwrap();

        assert!(sidebar_width >= 180.0);
        assert!(content_width >= 280.0);
        assert!(sidebar_width + content_width + SPACING_LG <= 640.0 + f32::EPSILON);
    }

    #[test]
    fn workspace_three_pane_widths_stack_when_width_is_too_small() {
        let widths = DialogContent::workspace_three_pane_widths(740.0, 240.0, 420.0);

        assert_eq!(widths, None);
    }

    #[test]
    fn workspace_three_pane_widths_fit_available_width_without_overflow() {
        let (left_width, middle_width, right_width) =
            DialogContent::workspace_three_pane_widths(980.0, 240.0, 420.0).unwrap();

        assert!(left_width >= 180.0);
        assert!(middle_width >= 260.0);
        assert!(right_width >= 280.0);
        assert!(left_width + middle_width + right_width + SPACING_LG * 2.0 <= 980.0 + f32::EPSILON);
    }

    #[test]
    fn form_dialog_body_context_resolves_requested_first_error_rect() {
        let mut ctx = FormDialogBodyContext::default();
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(30.0, 40.0));

        ctx.register_rect("field.username", rect);
        ctx.request_first_error("field.username");

        assert_eq!(ctx.requested_error_rect(), Some(rect));
    }

    #[test]
    fn form_dialog_body_context_keeps_first_requested_error_target() {
        let mut ctx = FormDialogBodyContext::default();
        let first_rect = egui::Rect::from_min_size(egui::pos2(1.0, 2.0), egui::vec2(3.0, 4.0));
        let second_rect = egui::Rect::from_min_size(egui::pos2(5.0, 6.0), egui::vec2(7.0, 8.0));

        ctx.register_rect("field.first", first_rect);
        ctx.register_rect("field.second", second_rect);
        ctx.request_first_error("field.first");
        ctx.request_first_error("field.second");

        assert_eq!(ctx.requested_error_rect(), Some(first_rect));
    }
}
