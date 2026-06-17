//! 进度指示器组件
//!
//! 在工具栏显示当前活跃的后台任务

#![allow(dead_code)] // 公开 API

use crate::core::ProgressManager;
use crate::ui::styles::ACCENT_BLUE;
use eframe::egui;

/// 进度指示器
pub struct ProgressIndicator;

impl ProgressIndicator {
    /// 在工具栏显示进度指示器
    pub fn show_in_toolbar(ui: &mut egui::Ui, progress: &ProgressManager) -> Option<u64> {
        let mut cancel_id = None;

        if !progress.has_active_tasks() {
            return None;
        }

        let tasks = progress.active_tasks();

        // 显示第一个任务的进度
        if let Some(task) = tasks.first() {
            ui.horizontal(|ui| {
                // 旋转动画
                let time = ui.input(|i| i.time);
                let angle = (time * 2.0) % std::f64::consts::TAU;
                let spinner_char = match ((angle / (std::f64::consts::TAU / 4.0)) as usize) % 4 {
                    0 => "|",
                    1 => "/",
                    2 => "-",
                    _ => "\\",
                };

                ui.label(
                    egui::RichText::new(spinner_char)
                        .color(ACCENT_BLUE)
                        .monospace(),
                );

                // 任务描述
                let desc = if tasks.len() > 1 {
                    format!("{} (+{})", task.description, tasks.len() - 1)
                } else {
                    task.description.clone()
                };

                ui.label(
                    egui::RichText::new(&desc)
                        .size(12.0)
                        .color(ui.visuals().weak_text_color()),
                );

                // 进度条（如果有确定进度）
                if let Some(progress_value) = task.progress {
                    let progress_width = 60.0;
                    let progress_height = 4.0;

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(progress_width, progress_height),
                        egui::Sense::hover(),
                    );

                    // 背景
                    ui.painter().rect_filled(
                        rect,
                        egui::CornerRadius::same(2),
                        ui.visuals().faint_bg_color,
                    );

                    // 进度
                    let progress_rect = egui::Rect::from_min_size(
                        rect.min,
                        egui::vec2(rect.width() * progress_value, rect.height()),
                    );
                    ui.painter().rect_filled(
                        progress_rect,
                        egui::CornerRadius::same(2),
                        ACCENT_BLUE,
                    );
                }

                // 耗时
                let elapsed = task.elapsed_ms();
                if elapsed > 1000 {
                    ui.label(
                        egui::RichText::new(format!("{:.1}s", elapsed as f32 / 1000.0))
                            .size(11.0)
                            .color(ui.visuals().weak_text_color()),
                    );
                }

                // 取消按钮
                if task.cancellable
                    && ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("x")
                                    .size(11.0)
                                    .color(ui.visuals().weak_text_color()),
                            )
                            .frame(false),
                        )
                        .on_hover_text("取消")
                        .clicked()
                {
                    cancel_id = Some(task.id);
                }
            });

            // 请求持续重绘以更新动画
            ui.ctx().request_repaint();
        }

        cancel_id
    }

    /// 在状态栏显示简洁的进度信息
    pub fn show_in_status_bar(ui: &mut egui::Ui, progress: &ProgressManager) {
        if !progress.has_active_tasks() {
            return;
        }

        let count = progress.active_count();
        let text = if count == 1 {
            "1 个任务运行中...".to_string()
        } else {
            format!("{} 个任务运行中...", count)
        };

        // 旋转动画
        let time = ui.input(|i| i.time);
        let angle = (time * 3.0) % std::f64::consts::TAU;
        let spinner_char = match ((angle / (std::f64::consts::TAU / 8.0)) as usize) % 8 {
            0 => "⠋",
            1 => "⠙",
            2 => "⠹",
            3 => "⠸",
            4 => "⠼",
            5 => "⠴",
            6 => "⠦",
            _ => "⠧",
        };

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(spinner_char).color(ACCENT_BLUE));
            ui.label(
                egui::RichText::new(&text)
                    .size(12.0)
                    .color(ui.visuals().weak_text_color()),
            );
        });

        ui.ctx().request_repaint();
    }
}
