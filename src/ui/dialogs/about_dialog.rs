//! 关于对话框 - 显示项目信息
//!
//! 支持的快捷键：
//! - `Esc` / `q` / `Enter` - 关闭对话框

use super::keyboard::{self, DialogAction};
use egui::{self, Color32, RichText, Vec2};

pub struct AboutDialog;

impl AboutDialog {
    pub fn show(ctx: &egui::Context, show: &mut bool) {
        if !*show {
            return;
        }

        // 使用统一的键盘模块处理：Esc/q 关闭
        if keyboard::handle_close_keys(ctx) {
            *show = false;
            return;
        }

        // Enter 也关闭（确认动作）
        if let DialogAction::Confirm = keyboard::handle_dialog_keys(ctx) {
            *show = false;
            return;
        }

        egui::Window::new("关于")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(420.0, 340.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    // 大笑脸
                    ui.label(RichText::new("😄").size(42.0));

                    ui.add_space(12.0);

                    // 主标题
                    ui.label(
                        RichText::new("不是吧哥们")
                            .size(24.0)
                            .strong()
                            .color(Color32::from_rgb(255, 193, 7)),
                    );

                    ui.add_space(6.0);

                    // 副标题
                    ui.label(
                        RichText::new("真当我们 Navicat 了？")
                            .size(18.0)
                            .color(Color32::from_rgb(100, 149, 237)),
                    );

                    ui.add_space(12.0);

                    // 说明文字
                    ui.label(RichText::new("我们可是开源项目嘿嘿，不收费哈！").size(16.0));

                    ui.add_space(16.0);

                    ui.separator();

                    ui.add_space(12.0);

                    // GitHub 信息
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("👤").size(14.0));
                        ui.label(RichText::new("作者: MCB-SMART-BOY").size(14.0).strong());
                    });

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔗").size(14.0));
                        ui.label(
                            RichText::new("github.com/MCB-SMART-BOY/gridix")
                                .size(13.0)
                                .color(Color32::from_rgb(100, 149, 237)),
                        );
                    });

                    ui.add_space(12.0);

                    // GitHub 链接提示
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🌟").size(14.0));
                        ui.label(
                            RichText::new("欢迎 Star & 贡献代码")
                                .size(14.0)
                                .color(Color32::GRAY),
                        );
                        ui.label(RichText::new("🌟").size(14.0));
                    });

                    ui.add_space(16.0);

                    // 快捷键提示
                    ui.label(
                        RichText::new("[Esc/q/Enter 关闭]")
                            .small()
                            .color(Color32::GRAY),
                    );

                    ui.add_space(6.0);

                    // 关闭按钮
                    if ui
                        .button(RichText::new("知道啦~ [Enter]").size(14.0))
                        .clicked()
                    {
                        *show = false;
                    }

                    ui.add_space(10.0);
                });
            });
    }
}
