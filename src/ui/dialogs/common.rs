//! 对话框公共样式和组件
//!
//! 提供统一的对话框样式和可复用的 UI 组件。

#![allow(dead_code)] // 公开 API，供未来使用

use crate::ui::styles::{DANGER, GRAY, MUTED, SUCCESS, SPACING_SM, SPACING_MD};
use egui::{self, Color32, RichText, CornerRadius, Vec2};

/// 对话框样式预设
#[derive(Debug, Clone, Copy)]
pub struct DialogStyle {
    /// 对话框宽度
    pub width: f32,
    /// 对话框最大高度
    pub max_height: f32,
    /// 标题字体大小
    pub title_size: f32,
    /// 内边距
    pub padding: f32,
    /// 按钮高度
    pub button_height: f32,
    /// 按钮圆角
    pub button_radius: u8,
}

impl DialogStyle {
    /// 小型对话框（确认框、简单提示）
    pub const SMALL: Self = Self {
        width: 320.0,
        max_height: 200.0,
        title_size: 16.0,
        padding: 12.0,
        button_height: 32.0,
        button_radius: 6,
    };

    /// 中型对话框（普通表单）
    pub const MEDIUM: Self = Self {
        width: 420.0,
        max_height: 400.0,
        title_size: 18.0,
        padding: 16.0,
        button_height: 36.0,
        button_radius: 6,
    };

    /// 大型对话框（复杂表单、预览）
    pub const LARGE: Self = Self {
        width: 520.0,
        max_height: 600.0,
        title_size: 20.0,
        padding: 20.0,
        button_height: 40.0,
        button_radius: 8,
    };

    /// 获取宽度
    pub fn width(&self) -> f32 {
        self.width
    }

    /// 获取最大高度
    pub fn max_height(&self) -> f32 {
        self.max_height
    }
}

impl Default for DialogStyle {
    fn default() -> Self {
        Self::MEDIUM
    }
}

/// 对话框头部渲染器
pub struct DialogHeader;

impl DialogHeader {
    /// 渲染对话框标题
    pub fn show(ui: &mut egui::Ui, title: &str, style: &DialogStyle) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(style.title_size).strong());
        });
        ui.add_space(SPACING_SM);
        ui.separator();
        ui.add_space(SPACING_MD);
    }

    /// 渲染带图标的对话框标题
    pub fn show_with_icon(ui: &mut egui::Ui, icon: &str, title: &str, style: &DialogStyle) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(icon).size(style.title_size));
            ui.add_space(SPACING_SM);
            ui.label(RichText::new(title).size(style.title_size).strong());
        });
        ui.add_space(SPACING_SM);
        ui.separator();
        ui.add_space(SPACING_MD);
    }

    /// 渲染带关闭按钮的标题栏
    pub fn show_with_close(ui: &mut egui::Ui, title: &str, style: &DialogStyle) -> bool {
        let mut close_clicked = false;
        
        ui.horizontal(|ui| {
            ui.label(RichText::new(title).size(style.title_size).strong());
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("✕").clicked() {
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
    pub const NONE: Self = Self { confirmed: false, cancelled: false };
    
    /// 已确认
    pub const CONFIRMED: Self = Self { confirmed: true, cancelled: false };
    
    /// 已取消
    pub const CANCELLED: Self = Self { confirmed: false, cancelled: true };
    
    /// 是否有任何操作
    pub fn has_action(&self) -> bool {
        self.confirmed || self.cancelled
    }
}

impl DialogFooter {
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
            // 取消按钮
            if ui.add(
                egui::Button::new(cancel_text)
                    .corner_radius(CornerRadius::same(style.button_radius))
                    .min_size(Vec2::new(80.0, style.button_height))
            ).clicked() {
                result.cancelled = true;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 确认按钮
                let btn = egui::Button::new(
                    RichText::new(confirm_text)
                        .color(if confirm_enabled { Color32::WHITE } else { GRAY })
                )
                .fill(if confirm_enabled { SUCCESS } else { Color32::from_rgb(80, 80, 90) })
                .corner_radius(CornerRadius::same(style.button_radius))
                .min_size(Vec2::new(100.0, style.button_height));
                
                if ui.add_enabled(confirm_enabled, btn).clicked() {
                    result.confirmed = true;
                }
            });
        });
        
        result
    }

    /// 渲染危险操作的按钮（红色确认按钮）
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
            // 取消按钮
            if ui.add(
                egui::Button::new(cancel_text)
                    .corner_radius(CornerRadius::same(style.button_radius))
                    .min_size(Vec2::new(80.0, style.button_height))
            ).clicked() {
                result.cancelled = true;
            }
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 危险确认按钮
                let btn = egui::Button::new(
                    RichText::new(confirm_text).color(Color32::WHITE)
                )
                .fill(DANGER)
                .corner_radius(CornerRadius::same(style.button_radius))
                .min_size(Vec2::new(100.0, style.button_height));
                
                if ui.add(btn).clicked() {
                    result.confirmed = true;
                }
            });
        });
        
        result
    }

    /// 渲染只有关闭按钮的底部
    pub fn show_close_only(
        ui: &mut egui::Ui,
        close_text: &str,
        style: &DialogStyle,
    ) -> bool {
        ui.add_space(SPACING_MD);
        ui.separator();
        ui.add_space(SPACING_SM);
        
        let mut clicked = false;
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(
                    egui::Button::new(close_text)
                        .corner_radius(CornerRadius::same(style.button_radius))
                        .min_size(Vec2::new(100.0, style.button_height))
                ).clicked() {
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
            ui.label(RichText::new("ℹ").color(Color32::from_rgb(100, 150, 255)));
            ui.label(RichText::new(text).small().color(MUTED));
        });
    }

    /// 渲染警告提示
    pub fn warning_text(ui: &mut egui::Ui, text: &str) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚠").color(Color32::from_rgb(255, 193, 7)));
            ui.label(RichText::new(text).small().color(Color32::from_rgb(255, 193, 7)));
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
        
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(60, 60, 70, 30))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::same(8))
            .show(ui, |ui| {
                content(ui);
            });
        
        ui.add_space(SPACING_MD);
    }

    /// 渲染快捷键提示
    pub fn shortcut_hint(ui: &mut egui::Ui, hints: &[(&str, &str)]) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
            
            for (i, (key, action)) in hints.iter().enumerate() {
                if i > 0 {
                    ui.label(RichText::new("·").small().color(MUTED));
                }
                ui.label(RichText::new(*key).small().color(GRAY));
                ui.label(RichText::new(*action).small().color(MUTED));
            }
        });
    }
}

/// 对话框状态消息组件
pub struct DialogStatus;

impl DialogStatus {
    /// 渲染状态消息
    pub fn show(ui: &mut egui::Ui, result: &Result<String, String>) {
        let (icon, message, color, bg_color) = match result {
            Ok(msg) => ("[OK]", msg.as_str(), SUCCESS, Color32::from_rgba_unmultiplied(82, 196, 106, 25)),
            Err(msg) => ("[X]", msg.as_str(), DANGER, Color32::from_rgba_unmultiplied(235, 87, 87, 25)),
        };

        egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
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
    /// 创建标准对话框窗口
    pub fn new<'a>(title: &'a str, style: &DialogStyle) -> egui::Window<'a> {
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .min_width(style.width)
            .max_width(style.width)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    }

    /// 创建可调整大小的对话框窗口
    pub fn resizable<'a>(title: &'a str, style: &DialogStyle) -> egui::Window<'a> {
        egui::Window::new(title)
            .collapsible(false)
            .resizable(true)
            .min_width(style.width)
            .default_width(style.width)
            .max_height(style.max_height)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    }

    /// 创建固定大小的对话框窗口
    pub fn fixed<'a>(title: &'a str, width: f32, height: f32) -> egui::Window<'a> {
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(width, height))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
    }
}

