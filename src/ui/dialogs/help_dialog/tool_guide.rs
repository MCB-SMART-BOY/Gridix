use super::*;

impl HelpDialog {
    pub(super) fn show_tool_guide(ui: &mut egui::Ui) {
        let accent = Color32::from_rgb(130, 180, 255);
        let highlight = Color32::from_rgb(180, 230, 140);
        let key_color = Color32::from_rgb(255, 200, 100);
        let text = Color32::from_rgb(220, 220, 220);
        let muted = Color32::from_rgb(150, 150, 160);

        ui.label(
            RichText::new("Gridix 工具快速使用指南")
                .size(20.0)
                .strong()
                .color(accent),
        );
        ui.add_space(6.0);
        Self::wrapped_text(
            ui,
            "这一页只讲 Gridix 怎么用，不教数据库概念。它面向已经理解表、行、列、查询这些基础概念，只想快速上手工具的人。",
            text,
        );
        ui.add_space(12.0);
        Self::guide_card(
            ui,
            "先明确这页的目标",
            "Gridix 是键盘优先的数据库工具。你不需要一开始记住全部功能，只需要先掌握焦点、模式和最常用的几个动作。",
            &[
                "如果你还不熟悉数据库基础概念，请切到“数据库相关知识点学习指南”。",
                "如果某个按键表现和你预期不同，先看当前焦点在哪个区域。",
                "写操作前先用 SELECT 确认范围，这是最重要的安全习惯。",
            ],
            accent,
            text,
            muted,
        );

        ui.add_space(20.0);

        Self::section(ui, "如果只记住 6 个动作", accent);
        Self::keys(
            ui,
            &[
                ("Ctrl+N", "新建连接"),
                (
                    "Tab / Shift+Tab",
                    "在侧边栏、结果表格、SQL 编辑器之间切换焦点",
                ),
                ("Ctrl+J", "显示 / 隐藏 SQL 编辑器"),
                ("Ctrl+Enter / F5", "在 SQL 编辑器中执行当前 SQL"),
                ("i", "在表格或编辑器里进入编辑模式"),
                ("Ctrl+S", "保存表格里的修改"),
            ],
            key_color,
            text,
        );

        ui.add_space(20.0);

        Self::section(ui, "5 分钟上手", accent);
        Self::workflow_step(
            ui,
            "1",
            "先建立连接",
            "按 Ctrl+N 创建连接。SQLite 最适合快速验证，PostgreSQL / MySQL 适合连真实数据库。",
            accent,
            text,
            muted,
        );
        Self::workflow_step(
            ui,
            "2",
            "在侧边栏打开表",
            "用 j / k 选中对象，Enter 或 l 展开。选中表后直接查看结果，先确认你操作的是哪张表。",
            accent,
            text,
            muted,
        );
        Self::workflow_step(
            ui,
            "3",
            "切到 SQL 编辑器执行第一条查询",
            "按 Ctrl+J 打开编辑器，按 i 或双击进入输入模式，输入 SQL 后用 Ctrl+Enter 或 F5 执行。",
            accent,
            text,
            muted,
        );
        Self::workflow_step(
            ui,
            "4",
            "需要改数据时再进入表格编辑",
            "先用 SELECT 确认范围，再回到表格按 i 编辑，最后用 Ctrl+S 保存。不要一上来直接改。",
            accent,
            text,
            muted,
        );
        Self::workflow_step(
            ui,
            "5",
            "卡住时先看焦点和模式",
            "同一个键在不同区域的意义可能不同。先确认当前是在侧边栏、结果表格还是 SQL 编辑器。",
            accent,
            text,
            muted,
        );

        ui.add_space(20.0);

        Self::section(ui, "先理解焦点和模式", accent);
        ui.label(
            RichText::new(
                "Gridix 是键盘优先工具。先理解“焦点在哪个区域”，再去记快捷键，效率会高很多。",
            )
            .color(text),
        );
        ui.add_space(8.0);
        ui.label(
            RichText::new("同一个键在不同焦点区域的意义不同：例如 F5 在编辑器里执行 SQL，在全局则更接近刷新。")
                .color(muted)
                .italics(),
        );
        ui.add_space(12.0);
        Self::keys(
            ui,
            &[
                (
                    "Tab / Shift+Tab",
                    "在侧边栏、结果表格、SQL 编辑器之间循环切换焦点",
                ),
                ("h / j / k / l", "在当前区域内导航，或在区域之间转移"),
                ("Ctrl+B", "显示 / 隐藏侧边栏"),
                ("Ctrl+J", "显示 / 隐藏 SQL 编辑器"),
                ("Ctrl+R", "显示 / 隐藏 ER 关系图"),
            ],
            key_color,
            text,
        );

        ui.add_space(20.0);

        Self::section(ui, "按区域使用", accent);
        Self::subsection(ui, "侧边栏", highlight);
        Self::keys(
            ui,
            &[
                ("j / k", "上下移动选择"),
                ("Enter / l", "展开 / 连接 / 打开表"),
                ("h", "折叠 / 返回上级"),
                ("Ctrl+1", "连接面板"),
                ("Ctrl+2 / Ctrl+3", "快速转到数据库 / 表区域"),
                ("Ctrl+4 / Ctrl+5 / Ctrl+6", "筛选 / 触发器 / 存储过程面板"),
            ],
            key_color,
            text,
        );

        ui.add_space(8.0);

        Self::subsection(ui, "结果表格", highlight);
        Self::keys(
            ui,
            &[
                ("hjkl / 方向键", "移动光标"),
                ("i / a / c", "进入编辑模式 / 追加 / 清空后编辑"),
                ("v / x", "进入选择模式 / 选择整行"),
                ("/ / f", "打开筛选 / 为当前列添加筛选"),
                ("o / O", "在下方 / 上方插入新行"),
                ("dd / yy / p", "删除标记当前行 / 复制整行 / 粘贴"),
                ("u / U", "撤销修改 / 取消删除标记"),
                ("Ctrl+S", "保存修改"),
            ],
            key_color,
            text,
        );

        ui.add_space(8.0);

        Self::subsection(ui, "SQL 编辑器", highlight);
        Self::keys(
            ui,
            &[
                ("i / 双击", "进入输入模式"),
                ("Esc", "退出输入模式"),
                ("Ctrl+Enter / F5", "执行 SQL"),
                ("F6", "分析执行计划 (EXPLAIN)"),
                ("Ctrl+Space / Alt+L", "触发自动补全"),
                ("Shift+J / Shift+K", "浏览历史命令"),
            ],
            key_color,
            text,
        );

        ui.add_space(8.0);

        Self::subsection(ui, "查询标签与辅助面板", highlight);
        Self::keys(
            ui,
            &[
                ("Ctrl+T", "新建查询标签页"),
                ("Ctrl+W", "关闭当前标签页"),
                ("Ctrl+Tab", "下一个查询标签"),
                ("Ctrl+Shift+Tab", "上一个查询标签"),
                ("Ctrl+H", "显示 / 隐藏查询历史"),
                ("F1", "打开帮助与学习"),
            ],
            key_color,
            text,
        );

        ui.add_space(20.0);

        Self::section(ui, "最常用的全局快捷键", accent);
        Self::keys(
            ui,
            &[
                ("Ctrl+N", "新建连接"),
                ("Ctrl+B", "切换侧边栏"),
                ("Ctrl+J", "切换 SQL 编辑器"),
                ("Ctrl+R", "切换 ER 关系图"),
                ("Ctrl+H", "切换查询历史"),
                ("F1", "帮助与学习"),
                ("Ctrl+D", "切换日间 / 夜间模式"),
                ("Ctrl++ / Ctrl+- / Ctrl+0", "放大 / 缩小 / 重置缩放"),
            ],
            key_color,
            text,
        );

        ui.add_space(16.0);
        Self::section(ui, "筛选、导入导出与清理", accent);
        Self::keys(
            ui,
            &[
                ("Ctrl+F", "添加筛选条件"),
                ("Ctrl+Shift+F", "清空筛选条件"),
                ("Ctrl+E", "导出结果"),
                ("Ctrl+I", "导入数据"),
                ("Ctrl+L", "清空 SQL 命令行"),
                ("Ctrl+K", "清空搜索"),
                ("Ctrl+G", "跳转到指定行"),
            ],
            key_color,
            text,
        );

        ui.add_space(16.0);
        Self::section(ui, "筛选输入怎么写", accent);
        ui.label(
            RichText::new("在表格里按 / 或 f 进入筛选后，可以直接输入条件表达式。")
                .color(muted)
                .italics(),
        );
        ui.add_space(8.0);
        Self::keys(
            ui,
            &[
                ("~john", "包含 john"),
                ("=admin", "精确等于 admin"),
                ("!=guest", "排除 guest"),
                (">100 / <100", "数值比较"),
                ("为空 / 不为空", "判断 NULL"),
            ],
            key_color,
            text,
        );

        ui.add_space(16.0);
        Self::section(ui, "连接和平台支持", accent);
        Self::keys(
            ui,
            &[
                ("SQLite", "本地文件数据库，最适合快速验证与学习"),
                ("PostgreSQL", "默认端口 5432，支持真实服务端连接"),
                ("MySQL", "默认端口 3306，支持真实服务端连接"),
            ],
            key_color,
            text,
        );

        ui.add_space(20.0);

        ui.separator();
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Gridix").strong().color(accent));
            ui.label(RichText::new(format!("v{}", env!("CARGO_PKG_VERSION"))).color(muted));
        });
        ui.add_space(4.0);
        ui.label(
            RichText::new("一款采用 Helix 风格键位的现代数据库管理工具")
                .small()
                .color(muted),
        );
        ui.label(
            RichText::new("使用 Rust + egui 构建 | 开源免费")
                .small()
                .color(muted),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("GitHub:").small().color(muted));
            ui.hyperlink_to(
                RichText::new("github.com/MCB-SMART-BOY/Gridix")
                    .small()
                    .color(accent),
                "https://github.com/MCB-SMART-BOY/Gridix",
            );
        });
    }

    fn guide_card(
        ui: &mut egui::Ui,
        title: &str,
        summary: &str,
        items: &[&str],
        accent: Color32,
        text: Color32,
        muted: Color32,
    ) {
        Self::feature_card(
            ui,
            Color32::from_rgba_unmultiplied(88, 108, 150, 14),
            Color32::from_rgba_unmultiplied(130, 170, 230, 28),
            |ui, content_width| {
                ui.allocate_ui_with_layout(
                    Vec2::new(content_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_min_width(content_width);
                        ui.set_max_width(content_width);
                        ui.label(RichText::new(title).strong().color(accent));
                        ui.add_space(6.0);
                        Self::wrapped_text(ui, summary, text);
                        ui.add_space(10.0);

                        for item in items {
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(8.0, 4.0);
                                ui.label(RichText::new("•").strong().color(accent));
                                ui.add(egui::Label::new(RichText::new(*item).color(muted)).wrap());
                            });
                            ui.add_space(4.0);
                        }
                    },
                );
            },
        );
    }

    fn feature_card(
        ui: &mut egui::Ui,
        fill: Color32,
        stroke: Color32,
        add_contents: impl FnOnce(&mut egui::Ui, f32),
    ) {
        egui::Frame::NONE
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                let content_width = ui.available_width();
                ui.set_min_width(content_width);
                ui.set_max_width(content_width);
                add_contents(ui, content_width);
            });
    }

    fn workflow_step(
        ui: &mut egui::Ui,
        step: &str,
        title: &str,
        desc: &str,
        accent: Color32,
        text: Color32,
        muted: Color32,
    ) {
        let width = ui.available_width();
        let step_width = 40.0;
        let gap = 12.0;
        let text_width = (width - step_width - gap).max(180.0);

        ui.allocate_ui_with_layout(
            Vec2::new(width, 0.0),
            egui::Layout::left_to_right(egui::Align::TOP),
            |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(gap, 8.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(130, 180, 255, 20))
                    .stroke(egui::Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(130, 180, 255, 42),
                    ))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.set_width(step_width);
                        ui.vertical_centered(|ui| {
                            ui.label(RichText::new(step).monospace().strong().color(accent));
                        });
                    });

                ui.allocate_ui_with_layout(
                    Vec2::new(text_width, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.label(RichText::new(title).strong().color(text));
                        ui.add_space(2.0);
                        Self::wrapped_text(ui, desc, muted);
                    },
                );
            },
        );
        ui.add_space(8.0);
    }

    fn section(ui: &mut egui::Ui, title: &str, color: Color32) {
        ui.add_space(4.0);
        ui.label(RichText::new(title).size(16.0).strong().color(color));
        ui.add_space(8.0);
    }

    fn subsection(ui: &mut egui::Ui, title: &str, color: Color32) {
        ui.label(RichText::new(format!("  {}", title)).strong().color(color));
        ui.add_space(2.0);
    }

    fn keys(ui: &mut egui::Ui, items: &[(&str, &str)], key_color: Color32, desc_color: Color32) {
        let width = ui.available_width();
        let key_width = 220.0f32.min(width * 0.34).max(140.0);
        let desc_width = (width - key_width - 16.0).max(160.0);

        for (key, desc) in items {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(16.0, 4.0);
                ui.add_sized(
                    [key_width, 0.0],
                    egui::Label::new(RichText::new(*key).monospace().color(key_color)),
                );
                ui.add_sized(
                    [desc_width, 0.0],
                    egui::Label::new(RichText::new(*desc).color(desc_color)).wrap(),
                );
            });
            ui.add_space(4.0);
        }
    }

    fn wrapped_text(ui: &mut egui::Ui, text: &str, color: Color32) {
        ui.add(egui::Label::new(RichText::new(text).color(color)).wrap());
    }
}
