//! 通知渲染组件
//!
//! 在界面右下角显示通知消息

#![allow(dead_code)] // 公开 API

use crate::core::{Notification, NotificationLevel, NotificationManager};
use eframe::egui;

/// 通知组件
pub struct NotificationToast;

impl NotificationToast {
    /// 在屏幕右下角渲染所有活跃通知
    pub fn show(ctx: &egui::Context, notifications: &NotificationManager) {
        if notifications.is_empty() {
            return;
        }

        // 使用 Area 在右下角显示通知
        // interactable(false) 确保通知不会阻止其下方 UI 元素的交互
        egui::Area::new(egui::Id::new("notification_area"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -16.0))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    for notification in notifications.iter() {
                        Self::render_notification(ui, notification);
                        ui.add_space(8.0);
                    }
                });
            });
    }

    /// 渲染单条通知，返回是否被点击关闭
    fn render_notification(ui: &mut egui::Ui, notification: &Notification) -> bool {
        let level_color = notification.level.color();
        let remaining = notification.remaining_ratio();

        // 计算淡出效果
        let alpha = if remaining < 0.3 {
            (remaining / 0.3 * 255.0) as u8
        } else {
            255
        };

        let bg_color = egui::Color32::from_rgba_unmultiplied(40, 40, 40, alpha);
        let border_color = egui::Color32::from_rgba_unmultiplied(
            level_color.r(),
            level_color.g(),
            level_color.b(),
            alpha,
        );
        let text_alpha = alpha;

        let frame = egui::Frame::NONE
            .fill(bg_color)
            .stroke(egui::Stroke::new(2.0, border_color))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .corner_radius(egui::CornerRadius::same(6));

        let response = frame
            .show(ui, |ui| {
                ui.set_min_width(200.0);
                ui.set_max_width(400.0);

                ui.horizontal(|ui| {
                    // 图标
                    let icon_color = egui::Color32::from_rgba_unmultiplied(
                        level_color.r(),
                        level_color.g(),
                        level_color.b(),
                        text_alpha,
                    );
                    ui.label(
                        egui::RichText::new(notification.level.icon())
                            .color(icon_color)
                            .strong()
                            .size(14.0),
                    );

                    ui.add_space(8.0);

                    // 消息文本
                    let text_color = egui::Color32::from_rgba_unmultiplied(220, 220, 220, text_alpha);
                    ui.label(
                        egui::RichText::new(&notification.message)
                            .color(text_color)
                            .size(13.0),
                    );
                });

                // 进度条显示剩余时间
                let progress_height = 2.0;
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), progress_height),
                    egui::Sense::hover(),
                );

                let progress_color = egui::Color32::from_rgba_unmultiplied(
                    level_color.r(),
                    level_color.g(),
                    level_color.b(),
                    (alpha as f32 * 0.6) as u8,
                );

                let progress_rect = egui::Rect::from_min_size(
                    rect.min,
                    egui::vec2(rect.width() * remaining, progress_height),
                );

                ui.painter()
                    .rect_filled(progress_rect, egui::CornerRadius::same(1), progress_color);
            })
            .response;

        // 悬停时显示提示
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }

        response.clicked()
    }

    /// 在状态栏显示最新通知（单行模式）
    pub fn show_in_status_bar(ui: &mut egui::Ui, notifications: &NotificationManager) {
        if let Some(notification) = notifications.latest() {
            let level_color = notification.level.color();

            ui.horizontal(|ui| {
                // 图标
                ui.label(
                    egui::RichText::new(notification.level.icon())
                        .color(level_color)
                        .strong(),
                );

                ui.add_space(4.0);

                // 消息
                let text_color = match notification.level {
                    NotificationLevel::Error => level_color,
                    NotificationLevel::Warning => level_color,
                    _ => ui.style().visuals.text_color(),
                };

                ui.label(egui::RichText::new(&notification.message).color(text_color));
            });
        }
    }
}
