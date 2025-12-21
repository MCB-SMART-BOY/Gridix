//! 帮助对话框 - 友好的使用指南
//!
//! 提供人性化的快速上手指南和功能说明

use egui::{self, Color32, Key, RichText, ScrollArea, Vec2};

pub struct HelpDialog;

impl HelpDialog {
    /// 显示帮助对话框
    pub fn show_with_scroll(ctx: &egui::Context, open: &mut bool, _scroll_offset: &mut f32) {
        if !*open {
            return;
        }

        // 处理键盘关闭
        let should_close = ctx.input(|i| {
            i.key_pressed(Key::Q) || i.key_pressed(Key::Escape)
        });

        if should_close {
            *open = false;
            return;
        }

        egui::Window::new("Gridix 使用指南")
            .open(open)
            .resizable(true)
            .default_width(680.0)
            .default_height(550.0)
            .min_width(480.0)
            .min_height(350.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // 顶部操作提示
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
                    Self::hint(ui, "j/k", "滚动");
                    Self::hint(ui, "q/Esc", "关闭");
                });
                ui.add_space(8.0);
                ui.separator();

                ScrollArea::vertical()
                    .id_salt("help_scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // 键盘滚动
                        let scroll_delta = ui.input(|i| {
                            let mut delta = 0.0f32;
                            if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                                delta += 50.0;
                            }
                            if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
                                delta -= 50.0;
                            }
                            if i.modifiers.ctrl && i.key_pressed(Key::D) {
                                delta += 300.0;
                            }
                            if i.modifiers.ctrl && i.key_pressed(Key::U) {
                                delta -= 300.0;
                            }
                            delta
                        });
                        
                        if scroll_delta != 0.0 {
                            ui.scroll_with_delta(Vec2::new(0.0, -scroll_delta));
                        }
                        
                        ui.add_space(12.0);
                        Self::show_content(ui);
                        ui.add_space(20.0);
                    });
            });
    }

    fn hint(ui: &mut egui::Ui, key: &str, desc: &str) {
        ui.label(RichText::new(key).monospace().small().color(Color32::from_rgb(255, 200, 100)));
        ui.label(RichText::new(desc).small().color(Color32::GRAY));
    }

    fn show_content(ui: &mut egui::Ui) {
        let accent = Color32::from_rgb(130, 180, 255);
        let highlight = Color32::from_rgb(180, 230, 140);
        let key_color = Color32::from_rgb(255, 200, 100);
        let text = Color32::from_rgb(220, 220, 220);
        let muted = Color32::from_rgb(140, 140, 140);

        // =====================================================================
        // 欢迎
        // =====================================================================
        ui.label(RichText::new("欢迎使用 Gridix!").size(20.0).strong().color(accent));
        ui.add_space(6.0);
        ui.label(RichText::new(
            "Gridix 是一款采用 Helix/Vim 风格键位的数据库管理工具。\n\
             无需频繁使用鼠标，双手始终保持在键盘上，高效管理你的数据。"
        ).color(text));

        ui.add_space(20.0);

        // =====================================================================
        // 快速上手
        // =====================================================================
        Self::section(ui, "快速上手", accent);
        
        ui.label(RichText::new("1. 连接数据库").color(highlight));
        ui.label(RichText::new("   按 Ctrl+N 创建新连接，支持 MySQL、PostgreSQL、SQLite").color(text));
        ui.add_space(4.0);
        
        ui.label(RichText::new("2. 浏览数据").color(highlight));
        ui.label(RichText::new("   在左侧边栏选择表，点击或按 Enter 查看数据").color(text));
        ui.add_space(4.0);
        
        ui.label(RichText::new("3. 编辑数据").color(highlight));
        ui.label(RichText::new("   在表格中按 i 进入编辑模式，修改后按 Ctrl+S 保存").color(text));
        ui.add_space(4.0);
        
        ui.label(RichText::new("4. 执行 SQL").color(highlight));
        ui.label(RichText::new("   按 Ctrl+J 打开编辑器，输入 SQL 后按 Ctrl+Enter 执行").color(text));

        ui.add_space(20.0);

        // =====================================================================
        // 界面导航
        // =====================================================================
        Self::section(ui, "界面导航 (hjkl 全局通用)", accent);
        
        ui.label(RichText::new(
            "Gridix 的各个区域之间可以用 hjkl 无缝切换："
        ).color(muted).italics());
        ui.add_space(8.0);

        Self::nav_diagram(ui, key_color, text);

        ui.add_space(12.0);
        
        Self::keys(ui, &[
            ("h", "向左移动 / 返回上级"),
            ("j", "向下移动 / 进入下级区域"),
            ("k", "向上移动 / 进入上级区域"),
            ("l", "向右移动 / 展开 / 确认"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // 常用快捷键
        // =====================================================================
        Self::section(ui, "常用快捷键", accent);

        Self::subsection(ui, "窗口控制", highlight);
        Self::keys(ui, &[
            ("Ctrl+B", "切换侧边栏"),
            ("Ctrl+J", "切换 SQL 编辑器"),
            ("Ctrl+R", "切换 ER 关系图"),
            ("Ctrl+T", "新建查询标签页"),
            ("Ctrl+W", "关闭当前标签页"),
            ("F1", "打开此帮助"),
        ], key_color, text);

        ui.add_space(8.0);

        Self::subsection(ui, "数据操作", highlight);
        Self::keys(ui, &[
            ("F5", "刷新数据"),
            ("Ctrl+S", "保存修改"),
            ("Ctrl+E", "导出数据"),
            ("Ctrl+I", "导入数据"),
            ("/", "添加筛选条件"),
        ], key_color, text);

        ui.add_space(8.0);

        Self::subsection(ui, "外观设置", highlight);
        Self::keys(ui, &[
            ("Ctrl+D", "切换日间/夜间模式"),
            ("Ctrl++/-", "放大/缩小界面"),
            ("Ctrl+0", "重置缩放"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // 表格编辑 (Helix 风格)
        // =====================================================================
        Self::section(ui, "表格编辑 (Helix 风格)", accent);

        ui.label(RichText::new(
            "表格采用模态编辑，类似 Helix/Vim 编辑器，分为三种模式："
        ).color(muted).italics());
        ui.add_space(8.0);

        Self::subsection(ui, "Normal 模式 - 浏览和导航", highlight);
        Self::keys(ui, &[
            ("hjkl / 方向键", "移动光标"),
            ("gg", "跳到第一行"),
            ("G", "跳到最后一行"),
            ("Ctrl+u/d", "上/下翻半页"),
            ("5j", "向下移动5行 (数字前缀)"),
        ], key_color, text);

        ui.add_space(8.0);

        Self::subsection(ui, "Insert 模式 - 编辑内容", highlight);
        Self::keys(ui, &[
            ("i", "进入编辑模式"),
            ("a", "追加模式"),
            ("c", "清空单元格并编辑"),
            ("Esc / Enter", "退出编辑"),
        ], key_color, text);

        ui.add_space(8.0);

        Self::subsection(ui, "Select 模式 - 批量操作", highlight);
        Self::keys(ui, &[
            ("v", "进入选择模式"),
            ("x", "选择整行"),
            ("y", "复制选中内容"),
            ("d", "删除选中内容"),
            ("Esc", "退出选择"),
        ], key_color, text);

        ui.add_space(8.0);

        Self::subsection(ui, "行操作", highlight);
        Self::keys(ui, &[
            ("o / O", "在下方/上方插入新行"),
            ("dd", "标记删除当前行"),
            ("yy", "复制整行"),
            ("p", "粘贴"),
            ("u", "撤销修改"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // SQL 编辑器
        // =====================================================================
        Self::section(ui, "SQL 编辑器", accent);

        ui.label(RichText::new(
            "编辑器同样支持 Normal/Insert 双模式，左下角显示当前模式。"
        ).color(muted).italics());
        ui.add_space(8.0);

        Self::keys(ui, &[
            ("i / 双击", "进入编辑模式"),
            ("Esc", "退出编辑模式"),
            ("Ctrl+Enter / F5", "执行 SQL"),
            ("F6", "分析执行计划 (EXPLAIN)"),
            ("Tab", "选择自动补全"),
            ("Shift+k/j", "浏览历史命令"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // 侧边栏
        // =====================================================================
        Self::section(ui, "侧边栏导航", accent);

        ui.label(RichText::new(
            "侧边栏分为多个面板：连接、数据库、表、筛选、触发器、存储过程。"
        ).color(muted).italics());
        ui.add_space(8.0);

        Self::keys(ui, &[
            ("j/k", "上下移动选择"),
            ("Enter / l", "展开 / 连接 / 查询表"),
            ("h", "折叠 / 返回上级面板"),
            ("d", "删除选中项"),
            ("Ctrl+1~6", "快速切换到对应面板"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // 筛选功能
        // =====================================================================
        Self::section(ui, "筛选功能", accent);

        ui.label(RichText::new(
            "在表格中按 / 或 f 快速添加筛选条件。支持以下操作符："
        ).color(muted).italics());
        ui.add_space(8.0);

        Self::keys(ui, &[
            ("~ 包含", "模糊匹配，如 ~john"),
            ("= 等于", "精确匹配，如 =admin"),
            ("!= 不等于", "排除匹配"),
            ("> / <", "数值比较"),
            ("为空 / 不为空", "NULL 判断"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // 支持的数据库
        // =====================================================================
        Self::section(ui, "支持的数据库", accent);

        Self::keys(ui, &[
            ("MySQL", "默认端口 3306，支持 SSH 隧道"),
            ("PostgreSQL", "默认端口 5432，支持 SSH 隧道"),
            ("SQLite", "本地文件数据库，无需网络"),
        ], key_color, text);

        ui.add_space(20.0);

        // =====================================================================
        // 关于
        // =====================================================================
        ui.separator();
        ui.add_space(12.0);
        
        ui.horizontal(|ui| {
            ui.label(RichText::new("Gridix").strong().color(accent));
            ui.label(RichText::new("v2.0.0").color(muted));
        });
        ui.add_space(4.0);
        ui.label(RichText::new(
            "一款采用 Helix 风格键位的现代数据库管理工具"
        ).small().color(muted));
        ui.label(RichText::new(
            "使用 Rust + egui 构建 | 开源免费"
        ).small().color(muted));
        
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("GitHub:").small().color(muted));
            ui.hyperlink_to(
                RichText::new("github.com/pzyyll/Gridix").small().color(accent),
                "https://github.com/pzyyll/Gridix"
            );
        });
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
        egui::Grid::new(ui.next_auto_id())
            .num_columns(2)
            .spacing([20.0, 4.0])
            .min_col_width(140.0)
            .show(ui, |ui| {
                for (key, desc) in items {
                    ui.label(RichText::new(format!("    {}", key)).monospace().color(key_color));
                    ui.label(RichText::new(*desc).color(desc_color));
                    ui.end_row();
                }
            });
    }

    /// 绘制导航示意图
    fn nav_diagram(ui: &mut egui::Ui, _key_color: Color32, text_color: Color32) {
        let box_color = Color32::from_rgb(60, 70, 90);
        let arrow_color = Color32::from_rgb(100, 130, 180);
        
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.vertical(|ui| {
                // 简化的布局示意
                ui.label(RichText::new("    ┌─────────────────────────────────────────┐").monospace().color(box_color));
                ui.label(RichText::new("    │              [工具栏]                   │").monospace().color(text_color));
                ui.label(RichText::new("    └─────────────────────────────────────────┘").monospace().color(box_color));
                ui.label(RichText::new("                    ↑k  ↓j").monospace().color(arrow_color));
                ui.label(RichText::new("    ┌─────────────────────────────────────────┐").monospace().color(box_color));
                ui.label(RichText::new("    │            [查询标签栏]                 │").monospace().color(text_color));
                ui.label(RichText::new("    └─────────────────────────────────────────┘").monospace().color(box_color));
                ui.label(RichText::new("                    ↑k  ↓j").monospace().color(arrow_color));
                ui.label(RichText::new("┌────────┐    ┌────────────────────────────────┐").monospace().color(box_color));
                ui.label(RichText::new("│        │ ←h │                                │").monospace().color(arrow_color));
                ui.label(RichText::new("│[侧边栏]│ l→ │          [数据表格]            │").monospace().color(text_color));
                ui.label(RichText::new("│        │    │                                │").monospace().color(box_color));
                ui.label(RichText::new("└────────┘    └────────────────────────────────┘").monospace().color(box_color));
                ui.label(RichText::new("                    ↑k  ↓j").monospace().color(arrow_color));
                ui.label(RichText::new("              ┌────────────────────────────────┐").monospace().color(box_color));
                ui.label(RichText::new("              │        [SQL 编辑器]            │").monospace().color(text_color));
                ui.label(RichText::new("              └────────────────────────────────┘").monospace().color(box_color));
            });
        });
    }
}
