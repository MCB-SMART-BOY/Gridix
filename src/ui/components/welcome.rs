//! 欢迎页面组件 - 应用启动时的欢迎界面

use crate::ui::styles::{GRAY, MUTED, SPACING_LG, SPACING_MD, SPACING_SM};
use egui::{self, Color32, CornerRadius, RichText, Stroke, Vec2};

pub struct Welcome;

impl Welcome {
    pub fn show(ui: &mut egui::Ui) {
        let content_width = (ui.available_width() - SPACING_LG * 2.0).clamp(460.0, 760.0);

        ui.add_space(SPACING_LG);
        ui.horizontal(|ui| {
            let offset = ((ui.available_width() - content_width) / 2.0).max(0.0);
            ui.add_space(offset);

            ui.allocate_ui_with_layout(
                Vec2::new(content_width, 0.0),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                Self::show_hero(ui, content_width);
                ui.add_space(SPACING_LG);

                Self::show_database_cards(ui, content_width);
                ui.add_space(SPACING_LG);

                Self::show_quick_start(ui);
                ui.add_space(SPACING_MD);

                Self::show_shortcuts(ui, content_width);
                },
            );
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
                ui.set_min_width((width - 48.0).max(320.0));
                ui.set_max_width((width - 48.0).max(320.0));
                Self::show_header(ui);
            });
    }

    /// 显示头部标题
    fn show_header(ui: &mut egui::Ui) {
        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
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
    fn show_database_cards(ui: &mut egui::Ui, width: f32) {
        let card_spacing = 16.0;
        let card_width = (width - card_spacing * 2.0) / 3.0;

        ui.horizontal(|ui| {
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
                ui.set_min_height(190.0);

                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    // 图标 - 使用圆形背景的字母
                    let (rect, _) = ui.allocate_exact_size(Vec2::new(56.0, 56.0), egui::Sense::hover());
                    let painter = ui.painter();

                    // 绘制圆形背景
                    painter.circle_filled(
                        rect.center(),
                        28.0,
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
                        egui::FontId::proportional(32.0),
                        accent_color,
                    );

                    ui.add_space(SPACING_SM + 2.0);

                    // 名称
                    ui.label(RichText::new(name).size(18.0).strong().color(accent_color));

                    ui.add_space(8.0);

                    // 描述
                    ui.label(RichText::new(desc).size(13.5).strong().color(MUTED));
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
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new("\u{2139} 快速开始")
                            .size(14.0)
                            .strong()
                            .color(Color32::from_rgb(190, 230, 200)),
                    );

                    ui.add_space(2.0);

                    ui.label(
                        RichText::new("点击侧边栏的 「+ 新建」 创建数据库连接，或按 Ctrl+N")
                            .color(GRAY),
                    );

                    ui.add_space(8.0);

                    ui.label(RichText::new("连接后可直接使用 Ctrl+Enter 执行 SQL 查询").color(GRAY));
                });
            });
    }

    /// 显示快捷键列表
    fn show_shortcuts(ui: &mut egui::Ui, width: f32) {
        ui.set_width(width);

        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("\u{2328} 常用快捷键")
                    .size(14.0)
                    .strong()
                    .color(GRAY),
            );
        });

        ui.add_space(SPACING_SM * 0.8);

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
                // 用当前卡片内容区宽度做计算，避免拿到外层更宽的 available_width 造成右侧溢出
                let inner_width = ui.available_width();
                let column_gap = 18.0;
                let row_gap = 10.0;
                let pair_width = (inner_width - column_gap).max(0.0) / 2.0;
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

                let total_rows = shortcuts.chunks(2).len();
                for (row_idx, row) in shortcuts.chunks(2).enumerate() {
                    ui.horizontal(|ui| {
                        Self::shortcut_item(ui, row[0].0, row[0].1, pair_width);
                        if let Some((key, desc)) = row.get(1) {
                            ui.add_space(column_gap);
                            Self::shortcut_item(ui, key, desc, pair_width);
                        }
                    });

                    if row_idx + 1 < total_rows {
                        ui.add_space(row_gap);
                    }
                }
            });
    }

    /// 单个快捷键项
    fn shortcut_item(ui: &mut egui::Ui, key: &str, desc: &str, width: f32) {
        // Frame 有左右 inner_margin(10, 4)，所以按键胶囊外宽 = 内宽 + 20
        let key_inner_width = 96.0;
        let key_outer_width = key_inner_width + 20.0;
        let label_width = (width - key_outer_width - 12.0).max(40.0);

        ui.allocate_ui_with_layout(
            Vec2::new(width, 0.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(150, 150, 160, 36))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(10, 4))
                    .show(ui, |ui| {
                        ui.set_min_width(key_inner_width);
                        ui.set_max_width(key_inner_width);
                        ui.with_layout(
                            egui::Layout::top_down_justified(egui::Align::Center),
                            |ui| {
                                ui.label(RichText::new(key).monospace().size(14.0).strong());
                            },
                        );
                    });

                ui.add_space(12.0);
                ui.add_sized(
                    [label_width, 22.0],
                    egui::Label::new(
                        RichText::new(desc)
                            .size(14.0)
                            .color(Color32::from_rgb(180, 180, 190)),
                    ),
                );
            },
        );
    }
}
