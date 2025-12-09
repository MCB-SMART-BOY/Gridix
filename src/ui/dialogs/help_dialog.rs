//! 帮助对话框 - 显示快捷键说明
//!
//! 支持 Helix 风格的键盘导航

use egui::{self, Color32, Key, RichText, ScrollArea, Vec2};

pub struct HelpDialog;

impl HelpDialog {
    /// 显示帮助对话框（带 Helix 键位支持）
    /// scroll_offset: 用于持久化滚动位置的可变引用
    pub fn show_with_scroll(ctx: &egui::Context, open: &mut bool, scroll_offset: &mut f32) {
        if !*open {
            return;
        }

        // 在窗口外处理键盘，这样即使窗口没有焦点也能响应
        let (should_close, scroll_delta) = ctx.input(|i| {
            let close = i.key_pressed(Key::Q) || i.key_pressed(Key::Escape);

            let mut delta = 0.0f32;
            // j/k 滚动
            if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                delta += 50.0;
            }
            if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
                delta -= 50.0;
            }
            // Ctrl+d/u 翻页
            if i.modifiers.ctrl && i.key_pressed(Key::D) {
                delta += 300.0;
            }
            if i.modifiers.ctrl && i.key_pressed(Key::U) {
                delta -= 300.0;
            }
            // G 跳到底部, gg 跳到顶部 (这里简化为 g 跳顶部)
            if i.key_pressed(Key::G) {
                if i.modifiers.shift {
                    delta = 10000.0; // 跳到底部
                } else {
                    *scroll_offset = 0.0; // 跳到顶部
                }
            }

            (close, delta)
        });

        if should_close {
            *open = false;
            return;
        }

        // 更新滚动位置
        *scroll_offset = (*scroll_offset + scroll_delta).max(0.0);

        egui::Window::new("⌨ 快捷键帮助")
            .open(open)
            .resizable(true)
            .default_width(720.0)
            .default_height(600.0)
            .min_width(500.0)
            .min_height(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // 显示滚动提示
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                    Self::show_hint_key(ui, "j/k", "滚动");
                    ui.label(RichText::new("•").small().color(Color32::DARK_GRAY));
                    Self::show_hint_key(ui, "Ctrl+d/u", "翻页");
                    ui.label(RichText::new("•").small().color(Color32::DARK_GRAY));
                    Self::show_hint_key(ui, "g/G", "顶部/底部");
                    ui.label(RichText::new("•").small().color(Color32::DARK_GRAY));
                    Self::show_hint_key(ui, "q/Esc", "关闭");
                });
                ui.add_space(6.0);
                ui.separator();

                ScrollArea::vertical()
                    .id_source("help_scroll")
                    .auto_shrink([false, false])
                    .vertical_scroll_offset(*scroll_offset)
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        Self::show_content(ui);
                        ui.add_space(16.0);
                    });
            });
    }

    /// 显示提示按键
    fn show_hint_key(ui: &mut egui::Ui, key: &str, desc: &str) {
        ui.label(
            RichText::new(key)
                .monospace()
                .small()
                .color(Color32::from_rgb(255, 200, 100)),
        );
        ui.label(
            RichText::new(desc)
                .small()
                .color(Color32::from_rgb(150, 150, 150)),
        );
    }

    fn show_content(ui: &mut egui::Ui) {
        let title_color = Color32::from_rgb(130, 180, 255);
        let section_color = Color32::from_rgb(150, 220, 150);
        let key_color = Color32::from_rgb(255, 200, 100);
        let desc_color = Color32::from_rgb(210, 210, 210);
        let muted_color = Color32::from_rgb(120, 120, 120);

        // =====================================================================
        // 全局快捷键
        // =====================================================================
        Self::show_title(ui, "🌐 全局快捷键", title_color);

        Self::show_section(ui, "窗口与面板", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+B", "切换侧边栏"),
                ("Ctrl+J", "切换 SQL 编辑器"),
                ("Ctrl+H", "打开查询历史"),
                ("F1", "打开/关闭帮助"),
                ("Esc", "关闭对话框"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "连接与数据", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+N", "新建数据库连接"),
                ("Ctrl+1", "快速切换连接"),
                ("Ctrl+2", "快速切换表"),
                ("F5", "刷新表列表"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "导入导出", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+E", "导出查询结果"),
                ("Ctrl+I", "导入 SQL 文件"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "主题与外观", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+D", "切换日间/夜间模式"),
                ("Ctrl+T", "打开主题选择器"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "其他", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+G", "跳转到行"),
                ("Ctrl+K", "清空搜索"),
                ("Ctrl+L", "清空 SQL 编辑器"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 侧边栏操作
        // =====================================================================
        Self::show_title(ui, "📂 侧边栏操作", title_color);

        Self::show_section(ui, "连接管理", section_color);
        Self::show_keys(
            ui,
            &[
                ("点击连接", "展开/折叠连接详情"),
                ("右键连接", "显示上下文菜单"),
                ("连接按钮", "连接到数据库"),
                ("断开按钮", "断开当前连接"),
                ("删除按钮", "删除连接配置"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "表操作", section_color);
        Self::show_keys(
            ui,
            &[
                ("点击表名", "查询表的前 100 行数据"),
                ("右键表名", "显示上下文菜单"),
                ("查询前100行", "执行 SELECT * LIMIT 100"),
                ("查看表结构", "显示表的列定义"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // SQL 编辑器
        // =====================================================================
        Self::show_title(ui, "📝 SQL 编辑器", title_color);

        Self::show_keys(
            ui,
            &[
                ("Ctrl+Enter / F5", "执行 SQL"),
                ("↑ / ↓", "浏览历史命令"),
                ("Ctrl+Space", "触发自动补全"),
                ("Tab / Enter", "选择补全项"),
                ("Ctrl+L", "清空编辑器"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 数据表格 - Normal 模式
        // =====================================================================
        Self::show_title(ui, "📊 数据表格 - Normal 模式", title_color);
        ui.label(
            RichText::new("采用 Helix/Vim 风格的模态编辑")
                .small()
                .italics()
                .color(muted_color),
        );
        ui.add_space(6.0);

        Self::show_section(ui, "基本移动", section_color);
        Self::show_keys(
            ui,
            &[
                ("h / ←", "向左"),
                ("j / ↓", "向下"),
                ("k / ↑", "向上"),
                ("l / →", "向右"),
                ("w", "下一列 (word)"),
                ("b", "上一列 (back)"),
                ("Home", "行首"),
                ("End", "行尾"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "快速移动", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+u", "向上翻半页"),
                ("Ctrl+d", "向下翻半页"),
                ("gg", "跳转到第一行"),
                ("G / ge", "跳转到最后一行"),
                ("gh", "跳转到行首"),
                ("gl", "跳转到行尾"),
                ("[n]+移动", "移动 n 次 (如 5j)"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "编辑模式", section_color);
        Self::show_keys(
            ui,
            &[
                ("i", "插入模式 (在当前位置编辑)"),
                ("a", "追加模式 (append)"),
                ("c", "修改 (清空并编辑)"),
                ("r", "替换模式"),
                ("d", "删除当前单元格内容"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "选择模式", section_color);
        Self::show_keys(
            ui,
            &[
                ("v", "进入选择模式"),
                ("x", "选择整行 (extend line)"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "复制粘贴", section_color);
        Self::show_keys(
            ui,
            &[
                ("y", "复制 (yank)"),
                ("p", "粘贴 (paste)"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "撤销与恢复", section_color);
        Self::show_keys(
            ui,
            &[
                ("u", "撤销单元格修改"),
                ("U", "取消行删除标记"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "行操作", section_color);
        Self::show_keys(
            ui,
            &[
                ("o", "在下方添加新行"),
                ("O", "在上方添加新行"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "Space 模式 (空格前缀)", section_color);
        Self::show_keys(
            ui,
            &[
                ("Space d", "标记删除当前行"),
                ("Space w", "保存修改"),
                ("Space q", "放弃所有修改"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "搜索与筛选", section_color);
        Self::show_keys(
            ui,
            &[
                ("/", "添加筛选条件"),
                ("f", "为当前列添加筛选"),
                ("Ctrl+F", "添加筛选条件"),
                ("Ctrl+Shift+F", "清空所有筛选"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "保存修改", section_color);
        Self::show_keys(
            ui,
            &[
                ("Ctrl+S", "保存表格修改到数据库"),
                ("Ctrl+Shift+Z", "放弃所有修改"),
            ],
            key_color,
            desc_color,
        );

        Self::show_section(ui, "右键菜单", section_color);
        Self::show_keys(
            ui,
            &[
                ("右键单元格", "显示单元格操作菜单"),
                ("编辑 [i]", "进入编辑模式"),
                ("复制 [y]", "复制单元格内容"),
                ("粘贴 [p]", "粘贴内容到单元格"),
                ("还原 [u]", "撤销单元格修改"),
                ("右键行号", "显示行操作菜单"),
                ("标记删除", "标记整行为待删除"),
                ("取消删除", "取消行的删除标记"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 数据表格 - Select 模式
        // =====================================================================
        Self::show_title(ui, "📊 数据表格 - Select 模式", title_color);

        Self::show_keys(
            ui,
            &[
                ("h/j/k/l", "扩展选择范围"),
                ("w/b", "按列扩展选择"),
                ("x", "扩展选择到整行"),
                ("d", "删除选中内容"),
                ("c", "清空选中并编辑"),
                ("y", "复制选中内容"),
                ("Esc", "退出选择模式"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 数据表格 - Insert 模式
        // =====================================================================
        Self::show_title(ui, "📊 数据表格 - Insert 模式", title_color);

        Self::show_keys(
            ui,
            &[
                ("输入文字", "编辑单元格内容"),
                ("Esc", "退出编辑并保存"),
                ("Enter", "确认编辑并退出"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 筛选操作符
        // =====================================================================
        Self::show_title(ui, "🔍 筛选操作符说明", title_color);

        Self::show_keys(
            ui,
            &[
                ("~ (包含)", "模糊匹配，如 ~john"),
                ("= (等于)", "精确匹配，如 =admin"),
                ("≠ (不等于)", "排除匹配"),
                ("> (大于)", "数值比较"),
                ("< (小于)", "数值比较"),
                ("∅ (为空)", "NULL 或空字符串"),
                ("≠∅ (不为空)", "有值的行"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 搜索与筛选栏
        // =====================================================================
        Self::show_title(ui, "🔎 搜索与筛选栏", title_color);

        Self::show_keys(
            ui,
            &[
                ("搜索框", "在所有列中模糊搜索"),
                ("列选择器", "限定搜索特定列"),
                ("✕ [Ctrl+K]", "清空搜索内容"),
                ("筛选条件", "添加精确筛选规则"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 导入导出
        // =====================================================================
        Self::show_title(ui, "📦 导入导出", title_color);

        Self::show_keys(
            ui,
            &[
                ("CSV 导出", "逗号分隔值格式"),
                ("JSON 导出", "JSON 数组格式"),
                ("SQL 导出", "INSERT 语句格式"),
                ("SQL 导入", "执行 .sql 文件内容"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 下拉选择器
        // =====================================================================
        Self::show_title(ui, "下拉选择器导航", title_color);
        ui.label(
            RichText::new("连接选择器和表选择器中的快捷键")
                .small()
                .italics()
                .color(muted_color),
        );
        ui.add_space(6.0);

        Self::show_keys(
            ui,
            &[
                ("j / ↓", "下一项"),
                ("k / ↑", "上一项"),
                ("Enter / l", "选择当前项"),
                ("Esc / h", "关闭选择器"),
                ("g", "跳转到第一项"),
                ("G", "跳转到最后一项"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 支持的数据库
        // =====================================================================
        Self::show_title(ui, "🗄 支持的数据库", title_color);

        Self::show_keys(
            ui,
            &[
                ("MySQL", "默认端口 3306"),
                ("PostgreSQL", "默认端口 5432"),
                ("SQLite", "本地文件数据库"),
            ],
            key_color,
            desc_color,
        );

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        // =====================================================================
        // 关于
        // =====================================================================
        Self::show_title(ui, "ℹ 关于", title_color);

        ui.label(
            RichText::new("Rust DB Manager v0.1.0")
                .color(desc_color),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("一个采用 Helix 风格键位的数据库管理工具")
                .small()
                .color(muted_color),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new("使用 Rust + egui 构建")
                .small()
                .color(muted_color),
        );
    }

    /// 显示标题
    fn show_title(ui: &mut egui::Ui, title: &str, color: Color32) {
        ui.add_space(4.0);
        ui.label(RichText::new(title).strong().size(17.0).color(color));
        ui.add_space(6.0);
    }

    /// 显示小节标题
    fn show_section(ui: &mut egui::Ui, title: &str, color: Color32) {
        ui.add_space(6.0);
        ui.label(RichText::new(title).strong().size(13.0).color(color));
        ui.add_space(2.0);
    }

    /// 显示快捷键列表
    fn show_keys(ui: &mut egui::Ui, items: &[(&str, &str)], key_color: Color32, desc_color: Color32) {
        egui::Grid::new(ui.next_auto_id())
            .num_columns(2)
            .spacing([24.0, 3.0])
            .min_col_width(120.0)
            .show(ui, |ui| {
                for (key, desc) in items {
                    ui.label(RichText::new(*key).monospace().color(key_color));
                    ui.label(RichText::new(*desc).color(desc_color));
                    ui.end_row();
                }
            });
    }
}
