//! 确认对话框组件
//!
//! 支持的快捷键：
//! - `Enter` / `y` - 确认操作
//! - `Esc` / `n` - 取消操作

use crate::ui::{
    LocalShortcut, consume_local_shortcut, local_shortcut_text, local_shortcut_tooltip,
    local_shortcuts_text,
    styles::{DANGER, GRAY, SPACING_LG, SPACING_MD},
};
use egui::{self, Color32, CornerRadius, RichText};

pub struct ConfirmDialog;

impl ConfirmDialog {
    fn handle_shortcuts(ctx: &egui::Context, show: &mut bool, on_confirm: &mut bool) -> bool {
        ctx.input_mut(|i| {
            if consume_local_shortcut(i, LocalShortcut::DangerConfirm) {
                *on_confirm = true;
                *show = false;
                return true;
            }
            if consume_local_shortcut(i, LocalShortcut::DangerCancel) {
                *show = false;
                return true;
            }
            false
        })
    }

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

        if Self::handle_shortcuts(ctx, show, on_confirm) {
            return;
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
                    ui.label(
                        RichText::new(format!(
                            "快捷键: {} 确认 | {} 取消",
                            local_shortcuts_text(&[LocalShortcut::DangerConfirm]),
                            local_shortcuts_text(&[LocalShortcut::DangerCancel]),
                        ))
                        .small()
                        .color(GRAY),
                    );
                });

                ui.add_space(SPACING_MD);

                // 按钮区域
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 确认按钮（危险样式）
                        let confirm_btn = egui::Button::new(
                            RichText::new(format!(
                                "{} [{}]",
                                confirm_text,
                                local_shortcut_text(LocalShortcut::DangerConfirm)
                            ))
                            .color(Color32::WHITE),
                        )
                        .fill(DANGER)
                        .corner_radius(CornerRadius::same(6));

                        if ui
                            .add(confirm_btn)
                            .on_hover_text(local_shortcut_tooltip(
                                "确认当前危险操作",
                                LocalShortcut::DangerConfirm,
                            ))
                            .clicked()
                        {
                            *on_confirm = true;
                            *show = false;
                        }

                        ui.add_space(SPACING_MD);

                        // 取消按钮
                        let cancel_btn = egui::Button::new(format!(
                            "取消 [{}]",
                            local_shortcut_text(LocalShortcut::DangerCancel)
                        ))
                        .corner_radius(CornerRadius::same(6));

                        if ui
                            .add(cancel_btn)
                            .on_hover_text(local_shortcut_tooltip(
                                "取消并关闭确认对话框",
                                LocalShortcut::DangerCancel,
                            ))
                            .clicked()
                        {
                            *show = false;
                        }
                    });
                });

                ui.add_space(SPACING_MD);
            });
    }
}
