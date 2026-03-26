//! 欢迎页面组件 - 应用启动时的欢迎界面

use crate::ui::styles::{GRAY, MUTED, SPACING_LG, SPACING_MD, SPACING_SM, SUCCESS};
use egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

pub struct Welcome;

impl Welcome {
    pub fn show(ui: &mut egui::Ui) {
        let max_width = (ui.available_width() - SPACING_LG * 2.0).clamp(360.0, 860.0);

        ui.vertical_centered(|ui| {
            ui.set_max_width(max_width);
            ui.add_space((ui.available_height() * 0.08).max(SPACING_MD));

            Self::show_hero(ui, max_width);
            ui.add_space(SPACING_LG);

            Self::show_database_cards(ui);
            ui.add_space(SPACING_LG);

            Self::show_quick_start(ui);
            ui.add_space(SPACING_MD);

            Self::show_shortcuts(ui);
        });
    }

    fn show_hero(ui: &mut egui::Ui, width: f32) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(90, 140, 210, 18))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(120, 170, 230, 48),
            ))
            .corner_radius(CornerRadius::same(14))
            .inner_margin(egui::Margin::symmetric(24, 20))
            .show(ui, |ui| {
                ui.set_width(width.min(820.0));
                Self::show_header(ui);
            });
    }

    /// 显示头部标题
    fn show_header(ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("GRIDIX")
                    .size(30.0)
                    .strong()
                    .color(Color32::from_rgb(105, 168, 236)),
            );

            ui.add_space(6.0);

            ui.label(
                RichText::new("简洁、快速、安全的数据库管理工具")
                    .size(16.0)
                    .color(GRAY),
            );

            ui.add_space(4.0);

            ui.label(
                RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                    .small()
                    .color(MUTED),
            );
        });
    }

    /// 显示数据库类型卡片
    fn show_database_cards(ui: &mut egui::Ui) {
        let card_width = 130.0;
        let card_spacing = 16.0;
        let total_width = card_width * 3.0 + card_spacing * 2.0;

        // 手动居中
        let available = ui.available_width();
        let offset = ((available - total_width) / 2.0).max(0.0);

        ui.horizontal(|ui| {
            ui.add_space(offset);
            ui.spacing_mut().item_spacing.x = card_spacing;

            // SQLite 卡片
            Self::database_card(
                ui,
                "S",
                "SQLite",
                "本地文件数据库",
                Color32::from_rgb(80, 160, 220),
                card_width,
            );

            // PostgreSQL 卡片
            Self::database_card(
                ui,
                "P",
                "PostgreSQL",
                "企业级关系数据库",
                Color32::from_rgb(80, 130, 180),
                card_width,
            );

            // MySQL 卡片
            Self::database_card(
                ui,
                "M",
                "MySQL",
                "流行的开源数据库",
                Color32::from_rgb(200, 120, 60),
                card_width,
            );
        });
    }

    /// 单个数据库卡片
    fn database_card(
        ui: &mut egui::Ui,
        icon: &str,
        name: &str,
        desc: &str,
        accent_color: Color32,
        width: f32,
    ) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(
                accent_color.r(),
                accent_color.g(),
                accent_color.b(),
                15,
            ))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(
                    accent_color.r(),
                    accent_color.g(),
                    accent_color.b(),
                    40,
                ),
            ))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(16, 20))
            .show(ui, |ui| {
                ui.set_min_width(width - 32.0);
                ui.set_max_width(width - 32.0);

                ui.vertical_centered(|ui| {
                    // 图标 - 使用圆形背景的字母
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(48.0, 48.0), egui::Sense::hover());
                    let painter = ui.painter();

                    // 绘制圆形背景
                    painter.circle_filled(
                        rect.center(),
                        24.0,
                        Color32::from_rgba_unmultiplied(
                            accent_color.r(),
                            accent_color.g(),
                            accent_color.b(),
                            40,
                        ),
                    );

                    // 绘制字母
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        icon,
                        egui::FontId::proportional(24.0),
                        accent_color,
                    );

                    ui.add_space(SPACING_SM);

                    // 名称
                    ui.label(RichText::new(name).size(15.0).strong().color(accent_color));

                    ui.add_space(4.0);

                    // 描述
                    ui.label(RichText::new(desc).small().color(GRAY));
                });
            });
    }

    /// 显示快速开始提示
    fn show_quick_start(ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(92, 180, 118, 22))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(100, 190, 126, 52),
            ))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(20, 12))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("\u{2139}").size(16.0).color(SUCCESS));
                        ui.label(
                            RichText::new("快速开始")
                                .size(14.0)
                                .strong()
                                .color(Color32::from_rgb(190, 230, 200)),
                        );
                    });

                    ui.add_space(2.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("点击侧边栏的").color(GRAY));
                        ui.label(RichText::new("「+ 新建」").strong().color(SUCCESS));
                        ui.label(RichText::new("创建数据库连接，或按").color(GRAY));
                        ui.label(RichText::new("Ctrl+N").monospace().strong());
                    });

                    ui.add_space(8.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("连接后可直接使用").color(GRAY));
                        ui.label(RichText::new("Ctrl+Enter").monospace().strong());
                        ui.label(RichText::new("执行 SQL 查询").color(GRAY));
                    });
                });
            });
    }

    /// 显示快捷键列表
    fn show_shortcuts(ui: &mut egui::Ui) {
        // 标题
        ui.label(
            RichText::new("\u{2328} 常用快捷键") // 键盘符号
                .size(14.0)
                .strong()
                .color(GRAY),
        );

        ui.add_space(SPACING_SM);

        // 快捷键网格
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(120, 120, 130, 12))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(155, 155, 170, 26),
            ))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::symmetric(20, 14))
            .show(ui, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(4)
                    .spacing([36.0, 10.0])
                    .show(ui, |ui| {
                        let shortcuts = [
                            ("Ctrl+N", "新建连接"),
                            ("Ctrl+Enter", "执行查询"),
                            ("Ctrl+J", "切换编辑器"),
                            ("Ctrl+H", "查询历史"),
                            ("Ctrl+E", "导出结果"),
                            ("Ctrl+I", "导入 SQL"),
                            ("F5", "刷新表"),
                            ("F1", "帮助"),
                        ];

                        for (i, (key, desc)) in shortcuts.iter().enumerate() {
                            Self::shortcut_item(ui, key, desc);

                            // 每两个换行
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
            });
    }

    /// 单个快捷键项
    fn shortcut_item(ui: &mut egui::Ui, key: &str, desc: &str) {
        // 按键
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(150, 150, 160, 36))
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 3))
            .show(ui, |ui| {
                ui.label(RichText::new(key).monospace().size(11.5));
            });

        // 描述
        ui.label(
            RichText::new(desc)
                .size(12.5)
                .color(Color32::from_rgb(180, 180, 190)),
        );
    }
}
