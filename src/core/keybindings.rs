//! 可配置快捷键系统
//!
//! 支持用户自定义快捷键绑定，并持久化到配置文件。

#![allow(dead_code)] // 公开 API，供未来使用

use egui::{Key, Modifiers};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// 快捷键绑定
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    /// 主键
    pub key: KeyCode,
    /// 修饰键
    pub modifiers: KeyModifiers,
}

/// 可序列化的按键代码
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyCode {
    // 字母键
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    // 数字键
    Num0, Num1, Num2, Num3, Num4, Num5, Num6, Num7, Num8, Num9,
    // 功能键
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,
    // 特殊键
    Escape, Tab, Space, Enter, Backspace, Delete, Insert, Home, End,
    PageUp, PageDown, ArrowUp, ArrowDown, ArrowLeft, ArrowRight,
    // 符号键
    Minus, Plus, Equals, LeftBracket, RightBracket,
    Semicolon, Quote, Comma, Period, Slash, Backslash, Grave,
}

impl KeyCode {
    /// 转换为 egui::Key
    pub fn to_egui_key(self) -> Key {
        match self {
            KeyCode::A => Key::A, KeyCode::B => Key::B, KeyCode::C => Key::C,
            KeyCode::D => Key::D, KeyCode::E => Key::E, KeyCode::F => Key::F,
            KeyCode::G => Key::G, KeyCode::H => Key::H, KeyCode::I => Key::I,
            KeyCode::J => Key::J, KeyCode::K => Key::K, KeyCode::L => Key::L,
            KeyCode::M => Key::M, KeyCode::N => Key::N, KeyCode::O => Key::O,
            KeyCode::P => Key::P, KeyCode::Q => Key::Q, KeyCode::R => Key::R,
            KeyCode::S => Key::S, KeyCode::T => Key::T, KeyCode::U => Key::U,
            KeyCode::V => Key::V, KeyCode::W => Key::W, KeyCode::X => Key::X,
            KeyCode::Y => Key::Y, KeyCode::Z => Key::Z,
            KeyCode::Num0 => Key::Num0, KeyCode::Num1 => Key::Num1,
            KeyCode::Num2 => Key::Num2, KeyCode::Num3 => Key::Num3,
            KeyCode::Num4 => Key::Num4, KeyCode::Num5 => Key::Num5,
            KeyCode::Num6 => Key::Num6, KeyCode::Num7 => Key::Num7,
            KeyCode::Num8 => Key::Num8, KeyCode::Num9 => Key::Num9,
            KeyCode::F1 => Key::F1, KeyCode::F2 => Key::F2, KeyCode::F3 => Key::F3,
            KeyCode::F4 => Key::F4, KeyCode::F5 => Key::F5, KeyCode::F6 => Key::F6,
            KeyCode::F7 => Key::F7, KeyCode::F8 => Key::F8, KeyCode::F9 => Key::F9,
            KeyCode::F10 => Key::F10, KeyCode::F11 => Key::F11, KeyCode::F12 => Key::F12,
            KeyCode::Escape => Key::Escape, KeyCode::Tab => Key::Tab,
            KeyCode::Space => Key::Space, KeyCode::Enter => Key::Enter,
            KeyCode::Backspace => Key::Backspace, KeyCode::Delete => Key::Delete,
            KeyCode::Insert => Key::Insert, KeyCode::Home => Key::Home,
            KeyCode::End => Key::End, KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown, KeyCode::ArrowUp => Key::ArrowUp,
            KeyCode::ArrowDown => Key::ArrowDown, KeyCode::ArrowLeft => Key::ArrowLeft,
            KeyCode::ArrowRight => Key::ArrowRight, KeyCode::Minus => Key::Minus,
            KeyCode::Plus => Key::Plus, KeyCode::Equals => Key::Equals,
            KeyCode::LeftBracket => Key::OpenBracket, KeyCode::RightBracket => Key::CloseBracket,
            KeyCode::Semicolon => Key::Semicolon, KeyCode::Quote => Key::Quote,
            KeyCode::Comma => Key::Comma, KeyCode::Period => Key::Period,
            KeyCode::Slash => Key::Slash, KeyCode::Backslash => Key::Backslash,
            KeyCode::Grave => Key::Backtick,
        }
    }

    /// 从 egui::Key 转换
    pub fn from_egui_key(key: Key) -> Option<Self> {
        Some(match key {
            Key::A => KeyCode::A, Key::B => KeyCode::B, Key::C => KeyCode::C,
            Key::D => KeyCode::D, Key::E => KeyCode::E, Key::F => KeyCode::F,
            Key::G => KeyCode::G, Key::H => KeyCode::H, Key::I => KeyCode::I,
            Key::J => KeyCode::J, Key::K => KeyCode::K, Key::L => KeyCode::L,
            Key::M => KeyCode::M, Key::N => KeyCode::N, Key::O => KeyCode::O,
            Key::P => KeyCode::P, Key::Q => KeyCode::Q, Key::R => KeyCode::R,
            Key::S => KeyCode::S, Key::T => KeyCode::T, Key::U => KeyCode::U,
            Key::V => KeyCode::V, Key::W => KeyCode::W, Key::X => KeyCode::X,
            Key::Y => KeyCode::Y, Key::Z => KeyCode::Z,
            Key::Num0 => KeyCode::Num0, Key::Num1 => KeyCode::Num1,
            Key::Num2 => KeyCode::Num2, Key::Num3 => KeyCode::Num3,
            Key::Num4 => KeyCode::Num4, Key::Num5 => KeyCode::Num5,
            Key::Num6 => KeyCode::Num6, Key::Num7 => KeyCode::Num7,
            Key::Num8 => KeyCode::Num8, Key::Num9 => KeyCode::Num9,
            Key::F1 => KeyCode::F1, Key::F2 => KeyCode::F2, Key::F3 => KeyCode::F3,
            Key::F4 => KeyCode::F4, Key::F5 => KeyCode::F5, Key::F6 => KeyCode::F6,
            Key::F7 => KeyCode::F7, Key::F8 => KeyCode::F8, Key::F9 => KeyCode::F9,
            Key::F10 => KeyCode::F10, Key::F11 => KeyCode::F11, Key::F12 => KeyCode::F12,
            Key::Escape => KeyCode::Escape, Key::Tab => KeyCode::Tab,
            Key::Space => KeyCode::Space, Key::Enter => KeyCode::Enter,
            Key::Backspace => KeyCode::Backspace, Key::Delete => KeyCode::Delete,
            Key::Insert => KeyCode::Insert, Key::Home => KeyCode::Home,
            Key::End => KeyCode::End, Key::PageUp => KeyCode::PageUp,
            Key::PageDown => KeyCode::PageDown, Key::ArrowUp => KeyCode::ArrowUp,
            Key::ArrowDown => KeyCode::ArrowDown, Key::ArrowLeft => KeyCode::ArrowLeft,
            Key::ArrowRight => KeyCode::ArrowRight, Key::Minus => KeyCode::Minus,
            Key::Plus => KeyCode::Plus, Key::Equals => KeyCode::Equals,
            Key::OpenBracket => KeyCode::LeftBracket, Key::CloseBracket => KeyCode::RightBracket,
            Key::Semicolon => KeyCode::Semicolon, Key::Quote => KeyCode::Quote,
            Key::Comma => KeyCode::Comma, Key::Period => KeyCode::Period,
            Key::Slash => KeyCode::Slash, Key::Backslash => KeyCode::Backslash,
            Key::Backtick => KeyCode::Grave,
            _ => return None,
        })
    }

    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            KeyCode::A => "A", KeyCode::B => "B", KeyCode::C => "C",
            KeyCode::D => "D", KeyCode::E => "E", KeyCode::F => "F",
            KeyCode::G => "G", KeyCode::H => "H", KeyCode::I => "I",
            KeyCode::J => "J", KeyCode::K => "K", KeyCode::L => "L",
            KeyCode::M => "M", KeyCode::N => "N", KeyCode::O => "O",
            KeyCode::P => "P", KeyCode::Q => "Q", KeyCode::R => "R",
            KeyCode::S => "S", KeyCode::T => "T", KeyCode::U => "U",
            KeyCode::V => "V", KeyCode::W => "W", KeyCode::X => "X",
            KeyCode::Y => "Y", KeyCode::Z => "Z",
            KeyCode::Num0 => "0", KeyCode::Num1 => "1", KeyCode::Num2 => "2",
            KeyCode::Num3 => "3", KeyCode::Num4 => "4", KeyCode::Num5 => "5",
            KeyCode::Num6 => "6", KeyCode::Num7 => "7", KeyCode::Num8 => "8",
            KeyCode::Num9 => "9",
            KeyCode::F1 => "F1", KeyCode::F2 => "F2", KeyCode::F3 => "F3",
            KeyCode::F4 => "F4", KeyCode::F5 => "F5", KeyCode::F6 => "F6",
            KeyCode::F7 => "F7", KeyCode::F8 => "F8", KeyCode::F9 => "F9",
            KeyCode::F10 => "F10", KeyCode::F11 => "F11", KeyCode::F12 => "F12",
            KeyCode::Escape => "Esc", KeyCode::Tab => "Tab",
            KeyCode::Space => "Space", KeyCode::Enter => "Enter",
            KeyCode::Backspace => "Backspace", KeyCode::Delete => "Delete",
            KeyCode::Insert => "Insert", KeyCode::Home => "Home",
            KeyCode::End => "End", KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown", KeyCode::ArrowUp => "Up",
            KeyCode::ArrowDown => "Down", KeyCode::ArrowLeft => "Left",
            KeyCode::ArrowRight => "Right", KeyCode::Minus => "-",
            KeyCode::Plus => "+", KeyCode::Equals => "=",
            KeyCode::LeftBracket => "[", KeyCode::RightBracket => "]",
            KeyCode::Semicolon => ";", KeyCode::Quote => "'",
            KeyCode::Comma => ",", KeyCode::Period => ".",
            KeyCode::Slash => "/", KeyCode::Backslash => "\\",
            KeyCode::Grave => "`",
        }
    }
}

/// 可序列化的修饰键
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyModifiers {
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    #[serde(default)]
    pub mac_cmd: bool,
}

impl KeyModifiers {
    pub const NONE: Self = Self { ctrl: false, shift: false, alt: false, mac_cmd: false };
    pub const CTRL: Self = Self { ctrl: true, shift: false, alt: false, mac_cmd: false };
    pub const SHIFT: Self = Self { ctrl: false, shift: true, alt: false, mac_cmd: false };
    pub const ALT: Self = Self { ctrl: false, shift: false, alt: true, mac_cmd: false };
    pub const CTRL_SHIFT: Self = Self { ctrl: true, shift: true, alt: false, mac_cmd: false };
    pub const CTRL_ALT: Self = Self { ctrl: true, shift: false, alt: true, mac_cmd: false };

    /// 从 egui::Modifiers 转换
    pub fn from_egui(mods: Modifiers) -> Self {
        Self {
            ctrl: mods.ctrl,
            shift: mods.shift,
            alt: mods.alt,
            mac_cmd: mods.mac_cmd,
        }
    }

    /// 转换为 egui::Modifiers
    pub fn to_egui(self) -> Modifiers {
        Modifiers {
            ctrl: self.ctrl,
            shift: self.shift,
            alt: self.alt,
            mac_cmd: self.mac_cmd,
            command: self.ctrl || self.mac_cmd,
        }
    }

    /// 检查是否匹配 egui 的修饰键状态
    pub fn matches(&self, mods: &Modifiers) -> bool {
        self.ctrl == mods.ctrl
            && self.shift == mods.shift
            && self.alt == mods.alt
    }
}

impl fmt::Display for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.shift { parts.push("Shift"); }
        if self.alt { parts.push("Alt"); }
        if self.mac_cmd { parts.push("Cmd"); }
        write!(f, "{}", parts.join("+"))
    }
}

impl KeyBinding {
    /// 创建新的快捷键绑定
    pub fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    /// 创建无修饰键的绑定
    pub fn key_only(key: KeyCode) -> Self {
        Self { key, modifiers: KeyModifiers::NONE }
    }

    /// 创建 Ctrl+Key 绑定
    pub fn ctrl(key: KeyCode) -> Self {
        Self { key, modifiers: KeyModifiers::CTRL }
    }

    /// 创建 Ctrl+Shift+Key 绑定
    pub fn ctrl_shift(key: KeyCode) -> Self {
        Self { key, modifiers: KeyModifiers::CTRL_SHIFT }
    }

    /// 从字符串解析快捷键 (如 "Ctrl+Shift+N")
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
        if parts.is_empty() {
            return None;
        }

        let mut modifiers = KeyModifiers::NONE;
        let mut key_str = "";

        for part in &parts {
            let part_lower = part.to_lowercase();
            match part_lower.as_str() {
                "ctrl" | "control" => modifiers.ctrl = true,
                "shift" => modifiers.shift = true,
                "alt" => modifiers.alt = true,
                "cmd" | "command" | "meta" => modifiers.mac_cmd = true,
                _ => key_str = part,
            }
        }

        let key = Self::parse_key(key_str)?;
        Some(Self { key, modifiers })
    }

    fn parse_key(s: &str) -> Option<KeyCode> {
        let s_upper = s.to_uppercase();
        Some(match s_upper.as_str() {
            "A" => KeyCode::A, "B" => KeyCode::B, "C" => KeyCode::C,
            "D" => KeyCode::D, "E" => KeyCode::E, "F" => KeyCode::F,
            "G" => KeyCode::G, "H" => KeyCode::H, "I" => KeyCode::I,
            "J" => KeyCode::J, "K" => KeyCode::K, "L" => KeyCode::L,
            "M" => KeyCode::M, "N" => KeyCode::N, "O" => KeyCode::O,
            "P" => KeyCode::P, "Q" => KeyCode::Q, "R" => KeyCode::R,
            "S" => KeyCode::S, "T" => KeyCode::T, "U" => KeyCode::U,
            "V" => KeyCode::V, "W" => KeyCode::W, "X" => KeyCode::X,
            "Y" => KeyCode::Y, "Z" => KeyCode::Z,
            "0" | "NUM0" => KeyCode::Num0, "1" | "NUM1" => KeyCode::Num1,
            "2" | "NUM2" => KeyCode::Num2, "3" | "NUM3" => KeyCode::Num3,
            "4" | "NUM4" => KeyCode::Num4, "5" | "NUM5" => KeyCode::Num5,
            "6" | "NUM6" => KeyCode::Num6, "7" | "NUM7" => KeyCode::Num7,
            "8" | "NUM8" => KeyCode::Num8, "9" | "NUM9" => KeyCode::Num9,
            "F1" => KeyCode::F1, "F2" => KeyCode::F2, "F3" => KeyCode::F3,
            "F4" => KeyCode::F4, "F5" => KeyCode::F5, "F6" => KeyCode::F6,
            "F7" => KeyCode::F7, "F8" => KeyCode::F8, "F9" => KeyCode::F9,
            "F10" => KeyCode::F10, "F11" => KeyCode::F11, "F12" => KeyCode::F12,
            "ESC" | "ESCAPE" => KeyCode::Escape, "TAB" => KeyCode::Tab,
            "SPACE" => KeyCode::Space, "ENTER" | "RETURN" => KeyCode::Enter,
            "BACKSPACE" => KeyCode::Backspace, "DELETE" | "DEL" => KeyCode::Delete,
            "INSERT" | "INS" => KeyCode::Insert, "HOME" => KeyCode::Home,
            "END" => KeyCode::End, "PAGEUP" | "PGUP" => KeyCode::PageUp,
            "PAGEDOWN" | "PGDN" => KeyCode::PageDown,
            "UP" | "ARROWUP" => KeyCode::ArrowUp,
            "DOWN" | "ARROWDOWN" => KeyCode::ArrowDown,
            "LEFT" | "ARROWLEFT" => KeyCode::ArrowLeft,
            "RIGHT" | "ARROWRIGHT" => KeyCode::ArrowRight,
            "-" | "MINUS" => KeyCode::Minus, "+" | "PLUS" => KeyCode::Plus,
            "=" | "EQUALS" => KeyCode::Equals,
            "[" => KeyCode::LeftBracket, "]" => KeyCode::RightBracket,
            ";" => KeyCode::Semicolon, "'" => KeyCode::Quote,
            "," => KeyCode::Comma, "." => KeyCode::Period,
            "/" => KeyCode::Slash, "\\" => KeyCode::Backslash,
            "`" => KeyCode::Grave,
            _ => return None,
        })
    }

    /// 检查快捷键是否在当前帧被按下
    pub fn is_pressed(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| {
            self.modifiers.matches(&i.modifiers) && i.key_pressed(self.key.to_egui_key())
        })
    }

    /// 显示快捷键字符串
    pub fn display(&self) -> String {
        let mods = self.modifiers.to_string();
        let key = self.key.display_name();
        if mods.is_empty() {
            key.to_string()
        } else {
            format!("{}+{}", mods, key)
        }
    }
}

impl fmt::Display for KeyBinding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// 可用的操作
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    // === 全局操作 ===
    /// 新建连接
    NewConnection,
    /// 切换侧边栏
    ToggleSidebar,
    /// 切换 SQL 编辑器
    ToggleEditor,
    /// 切换 ER 关系图
    ToggleErDiagram,
    /// 显示帮助
    ShowHelp,
    /// 显示历史记录
    ShowHistory,
    /// 导出数据
    Export,
    /// 导入数据
    Import,
    /// 刷新
    Refresh,
    /// 清空命令行
    ClearCommandLine,
    /// 清空搜索
    ClearSearch,

    // === 创建操作 ===
    /// 新建表
    NewTable,
    /// 新建数据库
    NewDatabase,
    /// 新建用户
    NewUser,

    // === Tab 操作 ===
    /// 新建 Tab
    NewTab,
    /// 关闭 Tab
    CloseTab,
    /// 下一个 Tab
    NextTab,
    /// 上一个 Tab
    PrevTab,

    // === 编辑操作 ===
    /// 保存
    Save,
    /// 添加筛选
    AddFilter,
    /// 清空筛选
    ClearFilters,
    /// 跳转到行
    GotoLine,

    // === 缩放 ===
    /// 放大
    ZoomIn,
    /// 缩小
    ZoomOut,
    /// 重置缩放
    ZoomReset,
}

impl Action {
    /// 获取所有操作
    pub fn all() -> &'static [Action] {
        &[
            Action::NewConnection,
            Action::ToggleSidebar,
            Action::ToggleEditor,
            Action::ToggleErDiagram,
            Action::ShowHelp,
            Action::ShowHistory,
            Action::Export,
            Action::Import,
            Action::Refresh,
            Action::ClearCommandLine,
            Action::ClearSearch,
            Action::NewTable,
            Action::NewDatabase,
            Action::NewUser,
            Action::NewTab,
            Action::CloseTab,
            Action::NextTab,
            Action::PrevTab,
            Action::Save,
            Action::AddFilter,
            Action::ClearFilters,
            Action::GotoLine,
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomReset,
        ]
    }

    /// 获取操作的描述
    pub fn description(&self) -> &'static str {
        match self {
            Action::NewConnection => "新建连接",
            Action::ToggleSidebar => "切换侧边栏",
            Action::ToggleEditor => "切换 SQL 编辑器",
            Action::ToggleErDiagram => "切换 ER 关系图",
            Action::ShowHelp => "显示帮助",
            Action::ShowHistory => "显示历史记录",
            Action::Export => "导出数据",
            Action::Import => "导入数据",
            Action::Refresh => "刷新",
            Action::ClearCommandLine => "清空命令行",
            Action::ClearSearch => "清空搜索",
            Action::NewTable => "新建表",
            Action::NewDatabase => "新建数据库",
            Action::NewUser => "新建用户",
            Action::NewTab => "新建 Tab",
            Action::CloseTab => "关闭 Tab",
            Action::NextTab => "下一个 Tab",
            Action::PrevTab => "上一个 Tab",
            Action::Save => "保存",
            Action::AddFilter => "添加筛选",
            Action::ClearFilters => "清空筛选",
            Action::GotoLine => "跳转到行",
            Action::ZoomIn => "放大",
            Action::ZoomOut => "缩小",
            Action::ZoomReset => "重置缩放",
        }
    }

    /// 获取操作的分类
    pub fn category(&self) -> &'static str {
        match self {
            Action::NewConnection | Action::ToggleSidebar | Action::ToggleEditor
            | Action::ToggleErDiagram | Action::ShowHelp | Action::ShowHistory
            | Action::Export | Action::Import | Action::Refresh
            | Action::ClearCommandLine | Action::ClearSearch => "全局",
            Action::NewTable | Action::NewDatabase | Action::NewUser => "创建",
            Action::NewTab | Action::CloseTab | Action::NextTab | Action::PrevTab => "Tab",
            Action::Save | Action::AddFilter | Action::ClearFilters | Action::GotoLine => "编辑",
            Action::ZoomIn | Action::ZoomOut | Action::ZoomReset => "缩放",
        }
    }
}

/// 快捷键绑定管理器
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    /// 操作到快捷键的映射
    bindings: HashMap<Action, KeyBinding>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // 全局操作
        bindings.insert(Action::NewConnection, KeyBinding::ctrl(KeyCode::N));
        bindings.insert(Action::ToggleSidebar, KeyBinding::ctrl(KeyCode::B));
        bindings.insert(Action::ToggleEditor, KeyBinding::ctrl(KeyCode::J));
        bindings.insert(Action::ToggleErDiagram, KeyBinding::ctrl(KeyCode::R));
        bindings.insert(Action::ShowHelp, KeyBinding::key_only(KeyCode::F1));
        bindings.insert(Action::ShowHistory, KeyBinding::ctrl(KeyCode::H));
        bindings.insert(Action::Export, KeyBinding::ctrl(KeyCode::E));
        bindings.insert(Action::Import, KeyBinding::ctrl(KeyCode::I));
        bindings.insert(Action::Refresh, KeyBinding::key_only(KeyCode::F5));
        bindings.insert(Action::ClearCommandLine, KeyBinding::ctrl(KeyCode::L));
        bindings.insert(Action::ClearSearch, KeyBinding::ctrl(KeyCode::K));

        // 创建操作
        bindings.insert(Action::NewTable, KeyBinding::ctrl_shift(KeyCode::N));
        bindings.insert(Action::NewDatabase, KeyBinding::ctrl_shift(KeyCode::D));
        bindings.insert(Action::NewUser, KeyBinding::ctrl_shift(KeyCode::U));

        // Tab 操作
        bindings.insert(Action::CloseTab, KeyBinding::ctrl(KeyCode::W));
        bindings.insert(Action::NextTab, KeyBinding::new(KeyCode::Tab, KeyModifiers::CTRL));
        bindings.insert(Action::PrevTab, KeyBinding::new(KeyCode::Tab, KeyModifiers::CTRL_SHIFT));

        // 编辑操作
        bindings.insert(Action::Save, KeyBinding::ctrl(KeyCode::S));
        bindings.insert(Action::AddFilter, KeyBinding::ctrl(KeyCode::F));
        bindings.insert(Action::ClearFilters, KeyBinding::ctrl_shift(KeyCode::F));
        bindings.insert(Action::GotoLine, KeyBinding::ctrl(KeyCode::G));

        // 缩放
        bindings.insert(Action::ZoomIn, KeyBinding::ctrl(KeyCode::Plus));
        bindings.insert(Action::ZoomOut, KeyBinding::ctrl(KeyCode::Minus));
        bindings.insert(Action::ZoomReset, KeyBinding::ctrl(KeyCode::Num0));

        Self { bindings }
    }
}

impl KeyBindings {
    /// 创建新的快捷键管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取操作的快捷键
    pub fn get(&self, action: Action) -> Option<&KeyBinding> {
        self.bindings.get(&action)
    }

    /// 设置操作的快捷键
    pub fn set(&mut self, action: Action, binding: KeyBinding) {
        self.bindings.insert(action, binding);
    }

    /// 移除操作的快捷键
    pub fn remove(&mut self, action: Action) {
        self.bindings.remove(&action);
    }

    /// 检查操作是否被触发
    pub fn is_triggered(&self, ctx: &egui::Context, action: Action) -> bool {
        self.bindings
            .get(&action)
            .map(|b| b.is_pressed(ctx))
            .unwrap_or(false)
    }

    /// 查找被触发的操作
    pub fn find_triggered(&self, ctx: &egui::Context) -> Option<Action> {
        for (&action, binding) in &self.bindings {
            if binding.is_pressed(ctx) {
                return Some(action);
            }
        }
        None
    }

    /// 获取操作的快捷键显示文本
    pub fn display(&self, action: Action) -> String {
        self.bindings
            .get(&action)
            .map(|b| b.display())
            .unwrap_or_default()
    }

    /// 获取所有绑定
    pub fn all_bindings(&self) -> &HashMap<Action, KeyBinding> {
        &self.bindings
    }

    /// 按分类获取所有绑定
    pub fn bindings_by_category(&self) -> Vec<(&'static str, Vec<(Action, &KeyBinding)>)> {
        let mut categories: HashMap<&'static str, Vec<(Action, &KeyBinding)>> = HashMap::new();

        for (&action, binding) in &self.bindings {
            categories
                .entry(action.category())
                .or_default()
                .push((action, binding));
        }

        let mut result: Vec<_> = categories.into_iter().collect();
        result.sort_by_key(|(cat, _)| *cat);
        
        // 对每个分类内的操作排序
        for (_, actions) in &mut result {
            actions.sort_by_key(|(action, _)| action.description());
        }

        result
    }

    /// 重置为默认值
    pub fn reset_to_defaults(&mut self) {
        *self = Self::default();
    }

    /// 检查是否有冲突的快捷键
    pub fn find_conflicts(&self) -> Vec<(Action, Action, KeyBinding)> {
        let mut conflicts = Vec::new();
        let actions: Vec<_> = self.bindings.iter().collect();

        for i in 0..actions.len() {
            for j in (i + 1)..actions.len() {
                let (&action1, binding1) = actions[i];
                let (&action2, binding2) = actions[j];
                if binding1 == binding2 {
                    conflicts.push((action1, action2, binding1.clone()));
                }
            }
        }

        conflicts
    }
}
