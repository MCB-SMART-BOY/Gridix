//! Scoped command metadata shared by keymap, tooltips, and shortcut settings.
//!
//! This is the compatibility registry for the legacy `LocalShortcut` enum.  UI
//! code can still name local shortcuts while the keymap model moves toward
//! stable scoped command ids.

use super::{KeyBinding, KeyCode, KeyModifiers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedCommandBinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
}

impl ScopedCommandBinding {
    pub const fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    pub fn key_binding(self) -> KeyBinding {
        KeyBinding::new(self.key, self.modifiers)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScopedCommand {
    pub id: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub default_bindings: &'static [ScopedCommandBinding],
}

const fn bind(key: KeyCode, modifiers: KeyModifiers) -> ScopedCommandBinding {
    ScopedCommandBinding::new(key, modifiers)
}

pub const SCOPED_COMMANDS: &[ScopedCommand] = &[
    ScopedCommand {
        id: "dialog.common.confirm",
        description: "确认",
        category: "通用对话框",
        default_bindings: &[bind(KeyCode::Enter, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.common.cancel",
        description: "取消",
        category: "通用对话框",
        default_bindings: &[bind(KeyCode::Escape, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.common.dismiss",
        description: "关闭当前对话框",
        category: "通用对话框",
        default_bindings: &[
            bind(KeyCode::Escape, KeyModifiers::NONE),
            bind(KeyCode::Q, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.confirm.confirm",
        description: "危险确认",
        category: "危险确认",
        default_bindings: &[
            bind(KeyCode::Enter, KeyModifiers::NONE),
            bind(KeyCode::Y, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.confirm.cancel",
        description: "危险取消",
        category: "危险确认",
        default_bindings: &[
            bind(KeyCode::Escape, KeyModifiers::NONE),
            bind(KeyCode::N, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.help.scroll_up",
        description: "帮助页向上滚动",
        category: "帮助",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.help.scroll_down",
        description: "帮助页向下滚动",
        category: "帮助",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.help.page_up",
        description: "帮助页上翻",
        category: "帮助",
        default_bindings: &[
            bind(KeyCode::PageUp, KeyModifiers::NONE),
            bind(KeyCode::U, KeyModifiers::CTRL),
        ],
    },
    ScopedCommand {
        id: "dialog.help.page_down",
        description: "帮助页下翻",
        category: "帮助",
        default_bindings: &[
            bind(KeyCode::PageDown, KeyModifiers::NONE),
            bind(KeyCode::D, KeyModifiers::CTRL),
        ],
    },
    ScopedCommand {
        id: "dialog.picker.move_prev",
        description: "分级选择器上一项",
        category: "分级选择器",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.picker.move_next",
        description: "分级选择器下一项",
        category: "分级选择器",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.picker.open",
        description: "分级选择器打开当前项",
        category: "分级选择器",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
            bind(KeyCode::Enter, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.picker.back",
        description: "分级选择器返回上一层",
        category: "分级选择器",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.picker.focus_next",
        description: "分级选择器切到下一列",
        category: "分级选择器",
        default_bindings: &[bind(KeyCode::Tab, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.picker.focus_prev",
        description: "分级选择器切到上一列",
        category: "分级选择器",
        default_bindings: &[bind(KeyCode::Tab, KeyModifiers::SHIFT)],
    },
    ScopedCommand {
        id: "dialog.command_palette.prev",
        description: "命令面板上一项",
        category: "命令面板",
        default_bindings: &[bind(KeyCode::ArrowUp, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.command_palette.next",
        description: "命令面板下一项",
        category: "命令面板",
        default_bindings: &[bind(KeyCode::ArrowDown, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.command_palette.confirm",
        description: "命令面板执行选中命令",
        category: "命令面板",
        default_bindings: &[bind(KeyCode::Enter, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.command_palette.dismiss",
        description: "命令面板关闭",
        category: "命令面板",
        default_bindings: &[bind(KeyCode::Escape, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "toolbar.nav.prev",
        description: "工具栏上一项",
        category: "工具栏",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.nav.next",
        description: "工具栏下一项",
        category: "工具栏",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.nav.to_query_tabs",
        description: "工具栏切到查询标签栏",
        category: "工具栏",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.nav.activate",
        description: "工具栏激活当前项",
        category: "工具栏",
        default_bindings: &[bind(KeyCode::Enter, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "toolbar.nav.dismiss",
        description: "工具栏退出到查询标签栏",
        category: "工具栏",
        default_bindings: &[bind(KeyCode::Escape, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "toolbar.menu.prev",
        description: "工具栏菜单上一项",
        category: "工具栏菜单",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.menu.next",
        description: "工具栏菜单下一项",
        category: "工具栏菜单",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.menu.confirm",
        description: "工具栏菜单确认",
        category: "工具栏菜单",
        default_bindings: &[bind(KeyCode::Enter, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "toolbar.menu.dismiss",
        description: "工具栏菜单关闭",
        category: "工具栏菜单",
        default_bindings: &[bind(KeyCode::Escape, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "toolbar.theme.prev",
        description: "主题列表上一项",
        category: "主题选择",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.theme.next",
        description: "主题列表下一项",
        category: "主题选择",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.theme.confirm",
        description: "主题列表确认",
        category: "主题选择",
        default_bindings: &[
            bind(KeyCode::Enter, KeyModifiers::NONE),
            bind(KeyCode::L, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.theme.dismiss",
        description: "主题列表关闭",
        category: "主题选择",
        default_bindings: &[
            bind(KeyCode::Escape, KeyModifiers::NONE),
            bind(KeyCode::H, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "toolbar.theme.start",
        description: "主题列表跳到开头",
        category: "主题选择",
        default_bindings: &[bind(KeyCode::G, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "toolbar.theme.end",
        description: "主题列表跳到结尾",
        category: "主题选择",
        default_bindings: &[bind(KeyCode::G, KeyModifiers::SHIFT)],
    },
    ScopedCommand {
        id: "query_tabs.prev",
        description: "查询标签切到前一个",
        category: "查询标签",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "query_tabs.next",
        description: "查询标签切到后一个",
        category: "查询标签",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "query_tabs.to_data_grid",
        description: "查询标签切到结果表格",
        category: "查询标签",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "query_tabs.to_toolbar",
        description: "查询标签切到工具栏",
        category: "查询标签",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "query_tabs.activate",
        description: "查询标签打开当前标签内容",
        category: "查询标签",
        default_bindings: &[bind(KeyCode::Enter, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "query_tabs.close",
        description: "查询标签关闭当前标签",
        category: "查询标签",
        default_bindings: &[bind(KeyCode::D, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "query_tabs.dismiss",
        description: "查询标签退出到结果表格",
        category: "查询标签",
        default_bindings: &[bind(KeyCode::Escape, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "er_diagram.refresh",
        description: "ER 图刷新数据",
        category: "ER 图",
        default_bindings: &[bind(KeyCode::R, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "er_diagram.layout",
        description: "ER 图重新布局",
        category: "ER 图",
        default_bindings: &[bind(KeyCode::L, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "er_diagram.fit_view",
        description: "ER 图适应视图",
        category: "ER 图",
        default_bindings: &[bind(KeyCode::F, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "er_diagram.zoom_in",
        description: "ER 图放大",
        category: "ER 图",
        default_bindings: &[
            bind(KeyCode::Plus, KeyModifiers::NONE),
            bind(KeyCode::Equals, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "er_diagram.zoom_out",
        description: "ER 图缩小",
        category: "ER 图",
        default_bindings: &[bind(KeyCode::Minus, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.list.prev",
        description: "侧边栏上一项",
        category: "侧边栏",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.list.next",
        description: "侧边栏下一项",
        category: "侧边栏",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.list.start",
        description: "侧边栏跳到开头",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::Home, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.list.end",
        description: "侧边栏跳到结尾",
        category: "侧边栏",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::SHIFT),
            bind(KeyCode::End, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.list.move_left",
        description: "侧边栏向左返回",
        category: "侧边栏",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.list.move_right",
        description: "侧边栏向右进入",
        category: "侧边栏",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.list.toggle",
        description: "侧边栏切换/启用",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::Space, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.list.delete",
        description: "侧边栏删除",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::D, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.list.edit",
        description: "侧边栏编辑连接",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::E, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.list.rename",
        description: "侧边栏重命名",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::R, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.list.refresh",
        description: "侧边栏刷新",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::R, KeyModifiers::SHIFT)],
    },
    ScopedCommand {
        id: "sidebar.list.activate",
        description: "侧边栏激活",
        category: "侧边栏",
        default_bindings: &[bind(KeyCode::Enter, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.filters.add",
        description: "新增筛选条件",
        category: "筛选",
        default_bindings: &[bind(KeyCode::A, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.filters.delete",
        description: "删除筛选条件",
        category: "筛选",
        default_bindings: &[bind(KeyCode::X, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.filters.clear_all",
        description: "清空全部筛选",
        category: "筛选",
        default_bindings: &[bind(KeyCode::C, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.filters.column_next",
        description: "筛选列切到下一项",
        category: "筛选",
        default_bindings: &[
            bind(KeyCode::RightBracket, KeyModifiers::NONE),
            bind(KeyCode::W, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.filters.column_prev",
        description: "筛选列切到上一项",
        category: "筛选",
        default_bindings: &[
            bind(KeyCode::LeftBracket, KeyModifiers::NONE),
            bind(KeyCode::B, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.filters.operator_next",
        description: "筛选运算符下一项",
        category: "筛选",
        default_bindings: &[
            bind(KeyCode::Equals, KeyModifiers::NONE),
            bind(KeyCode::N, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.filters.operator_prev",
        description: "筛选运算符上一项",
        category: "筛选",
        default_bindings: &[
            bind(KeyCode::Minus, KeyModifiers::NONE),
            bind(KeyCode::N, KeyModifiers::SHIFT),
        ],
    },
    ScopedCommand {
        id: "sidebar.filters.logic_toggle",
        description: "切换 AND/OR",
        category: "筛选",
        default_bindings: &[
            bind(KeyCode::O, KeyModifiers::NONE),
            bind(KeyCode::T, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "sidebar.filters.focus_input",
        description: "聚焦筛选输入框",
        category: "筛选",
        default_bindings: &[bind(KeyCode::I, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.filters.case_toggle",
        description: "切换大小写敏感",
        category: "筛选",
        default_bindings: &[bind(KeyCode::S, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "sidebar.filters.input.dismiss",
        description: "筛选输入返回列表",
        category: "筛选",
        default_bindings: &[bind(KeyCode::Escape, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.export.format_csv",
        description: "导出切到 CSV",
        category: "导出",
        default_bindings: &[bind(KeyCode::Num1, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.export.format_tsv",
        description: "导出切到 TSV",
        category: "导出",
        default_bindings: &[bind(KeyCode::Num2, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.export.format_sql",
        description: "导出切到 SQL",
        category: "导出",
        default_bindings: &[bind(KeyCode::Num3, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.export.format_json",
        description: "导出切到 JSON",
        category: "导出",
        default_bindings: &[bind(KeyCode::Num4, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.export.cycle_prev",
        description: "导出格式向前切换",
        category: "导出",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.export.cycle_next",
        description: "导出格式向后切换",
        category: "导出",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.export.column_prev",
        description: "导出列选择上一项",
        category: "导出",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.export.column_next",
        description: "导出列选择下一项",
        category: "导出",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.export.column_start",
        description: "导出列跳到开头",
        category: "导出",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::NONE),
            bind(KeyCode::Home, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.export.column_end",
        description: "导出列跳到结尾",
        category: "导出",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::SHIFT),
            bind(KeyCode::End, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.export.column_toggle",
        description: "切换导出列选中",
        category: "导出",
        default_bindings: &[bind(KeyCode::Space, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.export.columns_toggle_all",
        description: "导出列全选/全不选",
        category: "导出",
        default_bindings: &[bind(KeyCode::A, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "editor.insert.execute",
        description: "执行 SQL",
        category: "SQL 编辑器",
        default_bindings: &[
            bind(KeyCode::Enter, KeyModifiers::CTRL),
            bind(KeyCode::F5, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "editor.insert.explain",
        description: "执行 EXPLAIN",
        category: "SQL 编辑器",
        default_bindings: &[bind(KeyCode::F6, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "editor.insert.clear",
        description: "清空 SQL 编辑器",
        category: "SQL 编辑器",
        default_bindings: &[bind(KeyCode::D, KeyModifiers::SHIFT)],
    },
    ScopedCommand {
        id: "editor.insert.trigger_completion",
        description: "手动触发补全",
        category: "SQL 编辑器",
        default_bindings: &[
            bind(KeyCode::Space, KeyModifiers::CTRL),
            bind(KeyCode::L, KeyModifiers::ALT),
        ],
    },
    ScopedCommand {
        id: "editor.insert.confirm_completion",
        description: "确认补全",
        category: "SQL 编辑器",
        default_bindings: &[
            bind(KeyCode::Tab, KeyModifiers::NONE),
            bind(KeyCode::Enter, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "editor.insert.history_prev",
        description: "SQL 历史上一条",
        category: "SQL 编辑器",
        default_bindings: &[
            bind(KeyCode::ArrowUp, KeyModifiers::SHIFT),
            bind(KeyCode::K, KeyModifiers::SHIFT),
        ],
    },
    ScopedCommand {
        id: "editor.insert.history_next",
        description: "SQL 历史下一条",
        category: "SQL 编辑器",
        default_bindings: &[
            bind(KeyCode::ArrowDown, KeyModifiers::SHIFT),
            bind(KeyCode::J, KeyModifiers::SHIFT),
        ],
    },
    ScopedCommand {
        id: "editor.insert.history_browse",
        description: "打开 SQL 历史",
        category: "SQL 编辑器",
        default_bindings: &[
            bind(KeyCode::ArrowUp, KeyModifiers::SHIFT),
            bind(KeyCode::ArrowDown, KeyModifiers::SHIFT),
            bind(KeyCode::K, KeyModifiers::SHIFT),
            bind(KeyCode::J, KeyModifiers::SHIFT),
        ],
    },
    ScopedCommand {
        id: "grid.insert.finish_edit",
        description: "表格结束单元格编辑",
        category: "表格编辑",
        default_bindings: &[
            bind(KeyCode::Enter, KeyModifiers::NONE),
            bind(KeyCode::Escape, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.import.refresh",
        description: "刷新导入预览",
        category: "导入",
        default_bindings: &[bind(KeyCode::R, KeyModifiers::CTRL)],
    },
    ScopedCommand {
        id: "dialog.import.format_sql",
        description: "导入切到 SQL",
        category: "导入",
        default_bindings: &[bind(KeyCode::Num1, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.import.format_csv",
        description: "导入切到 CSV",
        category: "导入",
        default_bindings: &[bind(KeyCode::Num2, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.import.format_tsv",
        description: "导入切到 TSV",
        category: "导入",
        default_bindings: &[bind(KeyCode::Num3, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.import.format_json",
        description: "导入切到 JSON",
        category: "导入",
        default_bindings: &[bind(KeyCode::Num4, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.import.cycle_prev",
        description: "导入格式向前切换",
        category: "导入",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.import.cycle_next",
        description: "导入格式向后切换",
        category: "导入",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.connection.type_sqlite",
        description: "连接切到 SQLite",
        category: "连接",
        default_bindings: &[bind(KeyCode::Num1, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.connection.type_postgres",
        description: "连接切到 PostgreSQL",
        category: "连接",
        default_bindings: &[bind(KeyCode::Num2, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.connection.type_mysql",
        description: "连接切到 MySQL",
        category: "连接",
        default_bindings: &[bind(KeyCode::Num3, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.connection.type_prev",
        description: "连接类型向前切换",
        category: "连接",
        default_bindings: &[
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.connection.type_next",
        description: "连接类型向后切换",
        category: "连接",
        default_bindings: &[
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.ddl.column_prev",
        description: "DDL 列上一项",
        category: "DDL",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.ddl.column_next",
        description: "DDL 列下一项",
        category: "DDL",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.ddl.column_start",
        description: "DDL 列跳到开头",
        category: "DDL",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::NONE),
            bind(KeyCode::Home, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.ddl.column_end",
        description: "DDL 列跳到结尾",
        category: "DDL",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::SHIFT),
            bind(KeyCode::End, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.ddl.column_delete",
        description: "DDL 删除列",
        category: "DDL",
        default_bindings: &[
            bind(KeyCode::D, KeyModifiers::NONE),
            bind(KeyCode::Delete, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.ddl.column_add_below",
        description: "DDL 在下方新增列",
        category: "DDL",
        default_bindings: &[bind(KeyCode::O, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.ddl.column_add_above",
        description: "DDL 在上方新增列",
        category: "DDL",
        default_bindings: &[bind(KeyCode::O, KeyModifiers::SHIFT)],
    },
    ScopedCommand {
        id: "dialog.ddl.column_toggle_primary_key",
        description: "DDL 切换主键",
        category: "DDL",
        default_bindings: &[bind(KeyCode::Space, KeyModifiers::NONE)],
    },
    ScopedCommand {
        id: "dialog.connection.sqlite_browse_file",
        description: "浏览 SQLite 文件",
        category: "连接",
        default_bindings: &[bind(KeyCode::O, KeyModifiers::CTRL)],
    },
    ScopedCommand {
        id: "dialog.common.format_selection_cycle",
        description: "循环切换格式选项",
        category: "通用对话框",
        default_bindings: &[
            bind(KeyCode::Num1, KeyModifiers::NONE),
            bind(KeyCode::Num2, KeyModifiers::NONE),
            bind(KeyCode::Num3, KeyModifiers::NONE),
            bind(KeyCode::Num4, KeyModifiers::NONE),
            bind(KeyCode::H, KeyModifiers::NONE),
            bind(KeyCode::L, KeyModifiers::NONE),
            bind(KeyCode::ArrowLeft, KeyModifiers::NONE),
            bind(KeyCode::ArrowRight, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.history.clear",
        description: "清空查询历史",
        category: "历史",
        default_bindings: &[bind(KeyCode::Delete, KeyModifiers::CTRL)],
    },
    ScopedCommand {
        id: "dialog.history.prev",
        description: "历史面板上一项",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::K, KeyModifiers::NONE),
            bind(KeyCode::ArrowUp, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.history.next",
        description: "历史面板下一项",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::J, KeyModifiers::NONE),
            bind(KeyCode::ArrowDown, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.history.start",
        description: "历史面板跳到开头",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::NONE),
            bind(KeyCode::Home, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.history.end",
        description: "历史面板跳到结尾",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::G, KeyModifiers::SHIFT),
            bind(KeyCode::End, KeyModifiers::NONE),
        ],
    },
    ScopedCommand {
        id: "dialog.history.page_up",
        description: "历史面板上翻",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::PageUp, KeyModifiers::NONE),
            bind(KeyCode::U, KeyModifiers::CTRL),
        ],
    },
    ScopedCommand {
        id: "dialog.history.page_down",
        description: "历史面板下翻",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::PageDown, KeyModifiers::NONE),
            bind(KeyCode::D, KeyModifiers::CTRL),
        ],
    },
    ScopedCommand {
        id: "dialog.history.use",
        description: "使用选中历史 SQL",
        category: "历史",
        default_bindings: &[
            bind(KeyCode::Enter, KeyModifiers::NONE),
            bind(KeyCode::L, KeyModifiers::NONE),
        ],
    },
];

pub fn scoped_commands() -> &'static [ScopedCommand] {
    SCOPED_COMMANDS
}

pub fn scoped_command(id: &str) -> Option<&'static ScopedCommand> {
    SCOPED_COMMANDS.iter().find(|command| command.id == id)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::{scoped_command, scoped_commands};

    #[test]
    fn scoped_command_ids_are_unique() {
        let mut seen = HashSet::new();

        for command in scoped_commands() {
            assert!(
                seen.insert(command.id),
                "duplicate command id {}",
                command.id
            );
        }
    }

    #[test]
    fn scoped_command_lookup_exposes_default_bindings() {
        let command = scoped_command("editor.insert.execute").expect("SQL execute command");

        assert_eq!(command.description, "执行 SQL");
        assert_eq!(command.category, "SQL 编辑器");
        assert_eq!(command.default_bindings.len(), 2);
        assert_eq!(
            command.default_bindings[0].key_binding().display(),
            "Ctrl+Enter"
        );
    }
}
