//! 关于对话框 - 显示项目信息

use crate::ui::{
    LocalShortcut, consume_local_shortcut, local_shortcuts_text, local_shortcuts_tooltip,
};
use egui::{self, Color32, RichText, Vec2};

pub struct AboutDialog;

impl AboutDialog {
    pub fn show(ctx: &egui::Context, show: &mut bool) {
        if !*show {
            return;
        }

        let close_shortcuts = [LocalShortcut::Dismiss, LocalShortcut::Confirm];
        if ctx.input_mut(|i| {
            consume_local_shortcut(i, LocalShortcut::Dismiss)
                || consume_local_shortcut(i, LocalShortcut::Confirm)
        }) {
            *show = false;
            return;
        }

        egui::Window::new("关于 Gridix")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(460.0, 360.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(18.0);

                    ui.label(
                        RichText::new("GRIDIX")
                            .size(30.0)
                            .strong()
                            .color(Color32::from_rgb(122, 162, 247)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Grid-first database manager")
                            .size(17.0)
                            .color(Color32::from_rgb(192, 202, 245)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(Color32::from_rgb(146, 156, 197)),
                    );

                    ui.add_space(16.0);
                    ui.separator();
                    ui.add_space(14.0);

                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(90, 130, 210, 14))
                        .stroke(egui::Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(110, 150, 230, 28),
                        ))
                        .corner_radius(egui::CornerRadius::same(8))
                        .inner_margin(egui::Margin::symmetric(14, 12))
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("开源、键盘优先、面向数据库学习与日常使用")
                                        .strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new(
                                        "支持 SQLite / PostgreSQL / MySQL(MariaDB)，\
                                         并提供帮助与学习体系、导入导出、ER 图与筛选工作流。",
                                    )
                                    .color(Color32::LIGHT_GRAY),
                                );
                            });
                        });

                    ui.add_space(14.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("仓库").strong());
                        ui.label(
                            RichText::new("github.com/MCB-SMART-BOY/Gridix")
                                .monospace()
                                .color(Color32::from_rgb(125, 207, 255)),
                        );
                    });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("作者").strong());
                        ui.label(RichText::new("MCB-SMART-BOY").color(Color32::LIGHT_GRAY));
                    });

                    ui.add_space(18.0);

                    ui.label(
                        RichText::new(format!(
                            "快捷键: {} 关闭",
                            local_shortcuts_text(&close_shortcuts)
                        ))
                        .small()
                        .color(Color32::from_rgb(120, 120, 120)),
                    );

                    ui.add_space(8.0);

                    if ui
                        .button(format!("关闭 [{}]", local_shortcuts_text(&close_shortcuts)))
                        .on_hover_text(local_shortcuts_tooltip("关闭关于对话框", &close_shortcuts))
                        .clicked()
                    {
                        *show = false;
                    }
                });
            });
    }
}
