//! 统一键盘导航模块
//!
//! 本模块提供 Gridix 全局统一的键盘操作处理。
//! 所有 UI 组件都应该使用这些函数，确保操作逻辑一致。

use egui::{Context, Key};

// ============================================================================
// 对话框操作
// ============================================================================

/// 对话框操作结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DialogAction {
    #[default]
    None,
    /// 确认操作
    Confirm,
    /// 取消操作
    Cancel,
}

/// 列表导航结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListNavigation {
    #[default]
    None,
    Up,
    Down,
    Start,
    End,
    PageUp,
    PageDown,
    /// 切换选中状态 (Space)
    Toggle,
    /// 删除 (d/dd)
    Delete,
    /// 在下方添加 (o)
    AddBelow,
    /// 在上方添加 (O)
    AddAbove,
}

/// 水平导航结果
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HorizontalNavigation {
    #[default]
    None,
    Left,
    Right,
}

// ============================================================================
// 核心处理函数
// ============================================================================

/// 检查是否有文本输入焦点
///
/// 当返回 true 时，大多数快捷键应该被跳过，让文本框处理输入
pub fn has_text_focus(ctx: &Context) -> bool {
    ctx.memory(|m| m.focused().is_some())
}

/// 处理关闭快捷键 (Esc 或 q)
///
/// 用于关闭对话框、面板等
pub fn handle_close_keys(ctx: &Context) -> bool {
    ctx.input(|i| i.key_pressed(Key::Escape) || (i.key_pressed(Key::Q) && i.modifiers.is_none()))
}

/// 处理对话框基本快捷键
///
/// - Enter: 确认
/// - Esc/q: 取消
pub fn handle_dialog_keys(ctx: &Context) -> DialogAction {
    ctx.input(|i| {
        // Enter: 确认
        if i.key_pressed(Key::Enter) && i.modifiers.is_none() {
            return DialogAction::Confirm;
        }

        // Esc: 取消
        if i.key_pressed(Key::Escape) {
            return DialogAction::Cancel;
        }

        // q: 取消 (无修饰键，非文本输入状态)
        if i.key_pressed(Key::Q) && i.modifiers.is_none() {
            return DialogAction::Cancel;
        }

        // y: 确认 (用于确认对话框)
        if i.key_pressed(Key::Y) && i.modifiers.is_none() {
            return DialogAction::Confirm;
        }

        // n: 取消 (用于确认对话框)
        if i.key_pressed(Key::N) && i.modifiers.is_none() {
            return DialogAction::Cancel;
        }

        DialogAction::None
    })
}

/// 处理确认对话框快捷键 (别名)
pub fn handle_confirm_keys(ctx: &Context) -> DialogAction {
    handle_dialog_keys(ctx)
}

/// 处理列表导航快捷键
///
/// - j/↓: 向下
/// - k/↑: 向上
/// - gg: 跳到开头
/// - G: 跳到结尾
/// - PageUp/PageDown: 翻页
/// - Space: 切换选中
/// - d/dd: 删除
/// - o/O: 添加
pub fn handle_list_navigation(ctx: &Context) -> ListNavigation {
    ctx.input(|i| {
        // j / 下箭头: 向下
        if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
            return ListNavigation::Down;
        }

        // k / 上箭头: 向上
        if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
            return ListNavigation::Up;
        }

        // G (Shift+g): 跳到结尾
        if i.key_pressed(Key::G) && i.modifiers.shift {
            return ListNavigation::End;
        }

        // gg: 跳到开头 (简化处理，只检测单个 g)
        // 注意：完整的 gg 需要命令缓冲区，这里简化为 g
        if i.key_pressed(Key::G) && !i.modifiers.shift {
            return ListNavigation::Start;
        }

        // PageUp
        if i.key_pressed(Key::PageUp) {
            return ListNavigation::PageUp;
        }

        // PageDown
        if i.key_pressed(Key::PageDown) {
            return ListNavigation::PageDown;
        }

        // Space: 切换
        if i.key_pressed(Key::Space) {
            return ListNavigation::Toggle;
        }

        // d: 删除
        if i.key_pressed(Key::D) && i.modifiers.is_none() {
            return ListNavigation::Delete;
        }

        // o: 在下方添加
        if i.key_pressed(Key::O) && !i.modifiers.shift {
            return ListNavigation::AddBelow;
        }

        // O: 在上方添加
        if i.key_pressed(Key::O) && i.modifiers.shift {
            return ListNavigation::AddAbove;
        }

        // Home: 开头
        if i.key_pressed(Key::Home) {
            return ListNavigation::Start;
        }

        // End: 结尾
        if i.key_pressed(Key::End) {
            return ListNavigation::End;
        }

        ListNavigation::None
    })
}

/// 处理水平导航快捷键
///
/// - h/←: 向左
/// - l/→: 向右
pub fn handle_horizontal_navigation(ctx: &Context) -> HorizontalNavigation {
    ctx.input(|i| {
        // h / 左箭头: 向左
        if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
            return HorizontalNavigation::Left;
        }

        // l / 右箭头: 向右
        if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
            return HorizontalNavigation::Right;
        }

        HorizontalNavigation::None
    })
}
