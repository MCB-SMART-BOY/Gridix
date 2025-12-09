//! 数据导出对话框

use crate::core::ExportFormat;
use crate::ui::styles::{DANGER, GRAY, MUTED, SUCCESS, SPACING_SM, SPACING_MD, SPACING_LG};
use egui::{self, Color32, RichText, Rounding};

pub struct ExportDialog;

impl ExportDialog {
    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        format: &mut ExportFormat,
        table_name: &str,
        on_export: &mut Option<ExportFormat>,
        status_message: &Option<Result<String, String>>,
    ) {
        if !*show {
            return;
        }

        egui::Window::new("导出数据")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.add_space(SPACING_MD);

                // 表名信息
                ui.horizontal(|ui| {
                    ui.add_space(SPACING_SM);
                    ui.label(RichText::new("数据表：").color(GRAY));
                    
                    egui::Frame::none()
                        .fill(Color32::from_rgba_unmultiplied(100, 150, 200, 30))
                        .rounding(Rounding::same(4.0))
                        .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new(table_name).strong());
                        });
                });

                ui.add_space(SPACING_LG);

                // 格式选择
                ui.label(RichText::new("选择导出格式：").color(GRAY));
                ui.add_space(SPACING_SM);

                Self::show_format_options(ui, format);

                ui.add_space(SPACING_LG);

                // 状态消息
                if let Some(result) = status_message {
                    Self::show_status_message(ui, result);
                    ui.add_space(SPACING_MD);
                }

                ui.separator();
                ui.add_space(SPACING_MD);

                // 按钮
                ui.horizontal(|ui| {
                    // 取消按钮
                    if ui.add(
                        egui::Button::new("取消 [Esc]")
                            .rounding(Rounding::same(6.0))
                    ).clicked() {
                        *show = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // 导出按钮
                        let export_btn = egui::Button::new(
                            RichText::new("导出文件 [Enter]")
                                .color(Color32::WHITE)
                        )
                        .fill(SUCCESS)
                        .rounding(Rounding::same(6.0));

                        if ui.add(export_btn).clicked() {
                            *on_export = Some(*format);
                        }
                    });
                });

                ui.add_space(SPACING_SM);
            });
    }

    /// 显示格式选项
    fn show_format_options(ui: &mut egui::Ui, format: &mut ExportFormat) {
        ui.horizontal(|ui| {
            ui.add_space(SPACING_SM);

            for (fmt, icon, name, desc) in [
                (ExportFormat::Csv, "📊", "CSV", "兼容 Excel"),
                (ExportFormat::Sql, "📝", "SQL", "INSERT 语句"),
                (ExportFormat::Json, "🔧", "JSON", "Web 应用"),
            ] {
                let is_selected = *format == fmt;
                let accent = Color32::from_rgb(100, 160, 220);

                let fill = if is_selected {
                    Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 35)
                } else {
                    Color32::TRANSPARENT
                };

                let stroke = if is_selected {
                    egui::Stroke::new(2.0, accent)
                } else {
                    egui::Stroke::new(1.0, Color32::from_rgba_unmultiplied(150, 150, 160, 40))
                };

                let response = egui::Frame::none()
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(icon).size(20.0));
                            ui.add_space(2.0);
                            ui.label(RichText::new(name).strong().color(
                                if is_selected { accent } else { GRAY }
                            ));
                            ui.label(RichText::new(desc).small().color(MUTED));
                        });
                    })
                    .response
                    .interact(egui::Sense::click());

                if response.clicked() {
                    *format = fmt;
                }

                ui.add_space(SPACING_SM);
            }
        });
    }

    /// 显示状态消息
    fn show_status_message(ui: &mut egui::Ui, result: &Result<String, String>) {
        let (icon, message, color, bg_color) = match result {
            Ok(msg) => ("✓", msg.as_str(), SUCCESS, Color32::from_rgba_unmultiplied(82, 196, 106, 25)),
            Err(msg) => ("✗", msg.as_str(), DANGER, Color32::from_rgba_unmultiplied(235, 87, 87, 25)),
        };

        egui::Frame::none()
            .fill(bg_color)
            .rounding(Rounding::same(6.0))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).color(color));
                    ui.add_space(SPACING_SM);
                    ui.label(RichText::new(message).color(color));
                });
            });
    }
}
