//! 确认对话框组件
//!
//! 支持的快捷键：
//! - `Enter` / `y` - 确认操作
//! - `Esc` / `n` - 取消操作

use super::keyboard;
use crate::ui::styles::{DANGER, GRAY, SPACING_MD, SPACING_LG};
use egui::{self, Color32, RichText, CornerRadius};

pub struct ConfirmDialog;

impl ConfirmDialog {
    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        title: &str,
        message: &str,
        confirm_text: &str,
        on_confirm: &mut bool,
    ) {
        if !*show {
            return;
        }

        // 处理键盘快捷键
        match keyboard::handle_confirm_keys(ctx) {
            keyboard::DialogAction::Confirm => {
                *on_confirm = true;
                *show = false;
                return;
            }
            keyboard::DialogAction::Cancel => {
                *show = false;
                return;
            }
            keyboard::DialogAction::None => {}
        }

        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(320.0)
            .show(ctx, |ui| {
                ui.add_space(SPACING_MD);

                // 警告图标和消息
                ui.horizontal(|ui| {
                    ui.add_space(SPACING_MD);
                    
                    // 警告图标
                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(235, 87, 87, 25))
                        .corner_radius(CornerRadius::same(20))
                        .inner_margin(egui::Margin::same(8))
                        .show(ui, |ui| {
                            ui.label(RichText::new("⚠").size(20.0).color(DANGER));
                        });
                    
                    ui.add_space(SPACING_MD);
                    
                    // 消息文本
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(RichText::new(message).size(14.0));
                    });
                });

                ui.add_space(SPACING_LG);

                // 快捷键提示
                ui.horizontal(|ui| {
                    ui.add_space(SPACING_MD);
                    ui.label(RichText::new("按 y 确认，n 取消").small().color(GRAY));
                });

                ui.add_space(SPACING_MD);

                // 按钮区域
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 确认按钮（危险样式）
                        let confirm_btn = egui::Button::new(
                            RichText::new(format!("{} [y]", confirm_text))
                                .color(Color32::WHITE)
                        )
                        .fill(DANGER)
                        .corner_radius(CornerRadius::same(6));

                        if ui.add(confirm_btn).clicked() {
                            *on_confirm = true;
                            *show = false;
                        }

                        ui.add_space(SPACING_MD);

                        // 取消按钮
                        let cancel_btn = egui::Button::new("取消 [n]")
                            .corner_radius(CornerRadius::same(6));

                        if ui.add(cancel_btn).clicked() {
                            *show = false;
                        }
                    });
                });

                ui.add_space(SPACING_MD);
            });
    }
}
