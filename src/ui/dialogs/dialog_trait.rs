//! 对话框统一接口
//!
//! 提供对话框的公共 trait 定义和结果类型，
//! 用于统一对话框的交互模式。

#![allow(dead_code)] // 公开 API，供未来使用

/// 对话框操作结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialogResult<T> {
    /// 无操作（对话框保持打开）
    None,
    /// 用户确认，返回结果数据
    Confirm(T),
    /// 用户取消
    Cancel,
}

impl<T> DialogResult<T> {
    /// 检查是否有确认结果
    #[inline]
    pub fn is_confirm(&self) -> bool {
        matches!(self, DialogResult::Confirm(_))
    }

    /// 检查是否取消
    #[inline]
    pub fn is_cancel(&self) -> bool {
        matches!(self, DialogResult::Cancel)
    }

    /// 检查是否无操作
    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, DialogResult::None)
    }

    /// 获取确认的值（如果有）
    pub fn confirmed(self) -> Option<T> {
        match self {
            DialogResult::Confirm(value) => Some(value),
            _ => None,
        }
    }

    /// 转换为 Option（Confirm -> Some, 其他 -> None）
    pub fn into_option(self) -> Option<T> {
        self.confirmed()
    }

    /// 映射确认值
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> DialogResult<U> {
        match self {
            DialogResult::None => DialogResult::None,
            DialogResult::Confirm(value) => DialogResult::Confirm(f(value)),
            DialogResult::Cancel => DialogResult::Cancel,
        }
    }
}

impl<T> Default for DialogResult<T> {
    fn default() -> Self {
        DialogResult::None
    }
}

/// 对话框尺寸预设
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DialogSize {
    /// 小型对话框（确认框、简单输入）
    /// 宽度约 320px
    Small,
    /// 中型对话框（普通表单）
    /// 宽度约 420px
    Medium,
    /// 大型对话框（复杂表单、预览）
    /// 宽度约 520px
    Large,
    /// 自定义尺寸
    Custom { width: f32, max_height: f32 },
}

impl DialogSize {
    /// 获取对话框宽度
    pub fn width(&self) -> f32 {
        match self {
            DialogSize::Small => 320.0,
            DialogSize::Medium => 420.0,
            DialogSize::Large => 520.0,
            DialogSize::Custom { width, .. } => *width,
        }
    }

    /// 获取对话框最大高度
    pub fn max_height(&self) -> f32 {
        match self {
            DialogSize::Small => 200.0,
            DialogSize::Medium => 400.0,
            DialogSize::Large => 600.0,
            DialogSize::Custom { max_height, .. } => *max_height,
        }
    }
}

impl Default for DialogSize {
    fn default() -> Self {
        DialogSize::Medium
    }
}

/// 对话框按钮配置
#[derive(Debug, Clone)]
pub struct DialogButtons {
    /// 确认按钮文本
    pub confirm_text: String,
    /// 取消按钮文本
    pub cancel_text: String,
    /// 确认按钮快捷键提示
    pub confirm_shortcut: String,
    /// 取消按钮快捷键提示
    pub cancel_shortcut: String,
    /// 是否显示取消按钮
    pub show_cancel: bool,
    /// 确认按钮是否使用危险样式
    pub confirm_danger: bool,
}

impl Default for DialogButtons {
    fn default() -> Self {
        Self {
            confirm_text: "确认".to_string(),
            cancel_text: "取消".to_string(),
            confirm_shortcut: "Enter".to_string(),
            cancel_shortcut: "Esc".to_string(),
            show_cancel: true,
            confirm_danger: false,
        }
    }
}

impl DialogButtons {
    /// 创建只有关闭按钮的配置（信息类对话框）
    pub fn close_only() -> Self {
        Self {
            confirm_text: "关闭".to_string(),
            cancel_text: String::new(),
            confirm_shortcut: "Enter/Esc".to_string(),
            cancel_shortcut: String::new(),
            show_cancel: false,
            confirm_danger: false,
        }
    }

    /// 创建危险操作的按钮配置
    pub fn danger(confirm_text: impl Into<String>) -> Self {
        Self {
            confirm_text: confirm_text.into(),
            cancel_text: "取消".to_string(),
            confirm_shortcut: "y".to_string(),
            cancel_shortcut: "n".to_string(),
            show_cancel: true,
            confirm_danger: true,
        }
    }

    /// 创建保存/取消按钮配置
    pub fn save_cancel() -> Self {
        Self {
            confirm_text: "保存".to_string(),
            cancel_text: "取消".to_string(),
            confirm_shortcut: "Ctrl+S".to_string(),
            cancel_shortcut: "Esc".to_string(),
            show_cancel: true,
            confirm_danger: false,
        }
    }

    /// 创建导出/取消按钮配置
    pub fn export_cancel() -> Self {
        Self {
            confirm_text: "导出".to_string(),
            cancel_text: "取消".to_string(),
            confirm_shortcut: "Enter".to_string(),
            cancel_shortcut: "Esc".to_string(),
            show_cancel: true,
            confirm_danger: false,
        }
    }

    /// 格式化确认按钮文本（含快捷键）
    pub fn confirm_label(&self) -> String {
        if self.confirm_shortcut.is_empty() {
            self.confirm_text.clone()
        } else {
            format!("{} [{}]", self.confirm_text, self.confirm_shortcut)
        }
    }

    /// 格式化取消按钮文本（含快捷键）
    pub fn cancel_label(&self) -> String {
        if self.cancel_shortcut.is_empty() {
            self.cancel_text.clone()
        } else {
            format!("{} [{}]", self.cancel_text, self.cancel_shortcut)
        }
    }
}

/// 对话框状态 trait
/// 
/// 实现此 trait 的类型可以用作对话框的状态容器。
/// 提供打开/关闭和重置功能。
pub trait DialogState: Default {
    /// 是否显示对话框
    fn is_open(&self) -> bool;
    
    /// 打开对话框
    fn open(&mut self);
    
    /// 关闭对话框
    fn close(&mut self);
    
    /// 重置对话框状态（关闭并清空数据）
    fn reset(&mut self) {
        *self = Self::default();
    }
}

/// 简单对话框状态包装器
/// 
/// 用于包装只需要一个 `bool` 控制显示的简单对话框。
#[derive(Debug, Clone, Default)]
pub struct SimpleDialogState {
    pub show: bool,
}

impl DialogState for SimpleDialogState {
    fn is_open(&self) -> bool {
        self.show
    }

    fn open(&mut self) {
        self.show = true;
    }

    fn close(&mut self) {
        self.show = false;
    }
}

/// 带数据的对话框状态包装器
/// 
/// 用于包装需要额外数据的对话框。
#[derive(Debug, Clone)]
pub struct DataDialogState<T: Default> {
    pub show: bool,
    pub data: T,
}

impl<T: Default> Default for DataDialogState<T> {
    fn default() -> Self {
        Self {
            show: false,
            data: T::default(),
        }
    }
}

impl<T: Default> DialogState for DataDialogState<T> {
    fn is_open(&self) -> bool {
        self.show
    }

    fn open(&mut self) {
        self.show = true;
    }

    fn close(&mut self) {
        self.show = false;
    }
}

impl<T: Default> DataDialogState<T> {
    /// 打开对话框并设置初始数据
    pub fn open_with(&mut self, data: T) {
        self.show = true;
        self.data = data;
    }

    /// 获取数据的可变引用
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    /// 获取数据的引用
    pub fn data(&self) -> &T {
        &self.data
    }
}

