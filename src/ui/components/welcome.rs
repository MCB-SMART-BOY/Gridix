//! 欢迎页面组件 - 应用启动时的欢迎界面

use crate::ui::styles::{GRAY, MUTED, SUCCESS, SPACING_SM, SPACING_MD, SPACING_LG};
use egui::{self, Color32, RichText, Rounding, Vec2};

pub struct Welcome;

impl Welcome {
    pub fn show(ui: &mut egui::Ui) {
        let available_rect = ui.available_rect_before_wrap();
        
        // 内容尺寸
        let content_width = 500.0;
        let content_height = 500.0;
        
        // 计算居中位置
        let x = available_rect.min.x + (available_rect.width() - content_width) / 2.0;
        let y = available_rect.min.y + (available_rect.height() - content_height) / 2.0;
        
        // 使用 Area 实现居中
        egui::Area::new(egui::Id::new("welcome_center"))
            .fixed_pos(egui::pos2(x.max(available_rect.min.x), y.max(available_rect.min.y)))
            .show(ui.ctx(), |ui| {
                ui.set_min_width(content_width);
                ui.set_max_width(content_width);
                
                ui.vertical_centered(|ui| {
                    // Logo 和标题区域
                    Self::show_header(ui);

                    ui.add_space(SPACING_LG * 2.0);

                    // 数据库卡片
                    Self::show_database_cards(ui);

                    ui.add_space(SPACING_LG * 2.0);

                    // 快速开始提示
                    Self::show_quick_start(ui);

                    ui.add_space(SPACING_LG * 2.0);

                    // 快捷键
                    Self::show_shortcuts(ui);
                });
            });
    }

    /// 显示头部标题
    fn show_header(ui: &mut egui::Ui) {
        // 应用标题
        ui.label(
            RichText::new("Rust DB Manager")
                .size(28.0)
                .strong()
                .color(Color32::from_rgb(100, 160, 220))
        );

        ui.add_space(SPACING_SM);

        // 主标题
        ui.label(
            RichText::new("简洁、快速、安全的数据库管理工具")
                .size(16.0)
                .color(GRAY)
        );

        ui.add_space(SPACING_SM);

        // 版本号
        ui.label(
            RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                .small()
                .color(MUTED)
        );
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
        egui::Frame::none()
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
            .rounding(Rounding::same(12.0))
            .inner_margin(egui::Margin::symmetric(16.0, 20.0))
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
                    ui.label(
                        RichText::new(name)
                            .size(15.0)
                            .strong()
                            .color(accent_color)
                    );

                    ui.add_space(4.0);

                    // 描述
                    ui.label(
                        RichText::new(desc)
                            .small()
                            .color(GRAY)
                    );
                });
            });
    }

    /// 显示快速开始提示
    fn show_quick_start(ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(100, 180, 100, 20))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(100, 180, 100, 40),
            ))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(24.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("\u{2139}").size(16.0).color(SUCCESS));  // info 符号
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("点击侧边栏的")
                            .color(GRAY)
                    );
                    ui.label(
                        RichText::new("「+ 新建」")
                            .strong()
                            .color(SUCCESS)
                    );
                    ui.label(
                        RichText::new("创建数据库连接，或按")
                            .color(GRAY)
                    );
                    ui.label(
                        RichText::new("Ctrl+N")
                            .monospace()
                            .strong()
                    );
                });
            });
    }

    /// 显示快捷键列表
    fn show_shortcuts(ui: &mut egui::Ui) {
        // 标题
        ui.label(
            RichText::new("\u{2328} 常用快捷键")  // 键盘符号
                .size(14.0)
                .strong()
                .color(GRAY)
        );

        ui.add_space(SPACING_MD);

        // 快捷键网格
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(120, 120, 130, 10))
            .rounding(Rounding::same(8.0))
            .inner_margin(egui::Margin::symmetric(24.0, 16.0))
            .show(ui, |ui| {
                egui::Grid::new("shortcuts_grid")
                    .num_columns(4)
                    .spacing([48.0, 8.0])
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
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(150, 150, 160, 30))
            .rounding(Rounding::same(4.0))
            .inner_margin(egui::Margin::symmetric(8.0, 3.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(key)
                        .monospace()
                        .size(12.0)
                );
            });

        // 描述
        ui.label(
            RichText::new(desc)
                .size(13.0)
                .color(Color32::from_rgb(180, 180, 190))
        );
    }
}
