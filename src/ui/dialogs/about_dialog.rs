//! 关于对话框 - 显示项目信息

use super::common::{DialogContent, DialogFooter, DialogStyle, DialogWindow};
use crate::ui::styles::{theme_muted_text, theme_text};
use crate::ui::{LocalShortcut, local_shortcuts_text};
use egui::{self, Color32, RichText};

pub struct AboutDialog;

impl AboutDialog {
    pub fn show(ctx: &egui::Context, show: &mut bool) {
        if !*show {
            return;
        }

        let close_shortcuts = [LocalShortcut::Dismiss, LocalShortcut::Confirm];

        let style = DialogStyle::MEDIUM;
        DialogWindow::standard(ctx, "关于 Gridix", &style).show(ctx, |ui| {
            let body_text = theme_text(ui.visuals());
            let muted_text = theme_muted_text(ui.visuals());
            ui.vertical(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("GRIDIX")
                            .size(28.0)
                            .strong()
                            .color(Color32::from_rgb(122, 162, 247)),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new("Grid-first database manager")
                            .size(16.0)
                            .color(body_text),
                    );
                    ui.label(
                        RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                            .small()
                            .color(muted_text),
                    );
                });

                DialogContent::section(ui, "产品定位", |ui| {
                    ui.label(RichText::new("开源、键盘优先、面向数据库学习与日常使用").strong());
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "支持 SQLite / PostgreSQL / MySQL(MariaDB)，并提供帮助与学习体系、导入导出、ER 图与筛选工作流。",
                        )
                        .color(body_text),
                    );
                });

                DialogContent::section(ui, "项目信息", |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("仓库").strong());
                        ui.label(
                            RichText::new("github.com/MCB-SMART-BOY/Gridix")
                                .monospace()
                                .color(Color32::from_rgb(125, 207, 255)),
                        );
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("作者").strong());
                        ui.label(RichText::new("MCB-SMART-BOY").color(body_text));
                    });
                });

                DialogContent::shortcut_hint(
                    ui,
                    &[(local_shortcuts_text(&close_shortcuts).as_str(), "关闭")],
                );

                if DialogFooter::show_close_only(
                    ui,
                    &format!("关闭 [{}]", local_shortcuts_text(&close_shortcuts)),
                    &style,
                ) {
                    *show = false;
                }
            });
        });
    }
}
