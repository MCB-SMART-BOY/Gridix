//! 可配置快捷键系统
//!
//! 支持用户自定义快捷键绑定，并持久化到配置文件。

#![allow(dead_code)] // 公开 API，供未来使用

use super::commands::scoped_commands;
use egui::{Key, Modifiers};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// 快捷键绑定
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyBinding {
    /// 主键
    pub key: KeyCode,
    /// 修饰键
    pub modifiers: KeyModifiers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapDiagnosticSeverity {
    Warning,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapDiagnosticCode {
    UnknownSection,
    UnknownAction,
    InvalidBinding,
    ExactScopeConflict,
    ParentShadowing,
    TextEntryPlainCharacterRejected,
    WorkspaceFallbackShadowingTextEntry,
    LegacyConfigMigrationPending,
    DeprecatedAlias,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeymapDiagnostic {
    pub severity: KeymapDiagnosticSeverity,
    pub code: KeymapDiagnosticCode,
    pub path: String,
    pub message: String,
}

/// 可序列化的按键代码
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyCode {
    // 字母键
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    // 数字键
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    // 功能键
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    // 特殊键
    Escape,
    Tab,
    Space,
    Enter,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    // 符号键
    Minus,
    Plus,
    Equals,
    LeftBracket,
    RightBracket,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Backslash,
    Grave,
}

impl KeyCode {
    /// 转换为 egui::Key
    pub fn to_egui_key(self) -> Key {
        match self {
            KeyCode::A => Key::A,
            KeyCode::B => Key::B,
            KeyCode::C => Key::C,
            KeyCode::D => Key::D,
            KeyCode::E => Key::E,
            KeyCode::F => Key::F,
            KeyCode::G => Key::G,
            KeyCode::H => Key::H,
            KeyCode::I => Key::I,
            KeyCode::J => Key::J,
            KeyCode::K => Key::K,
            KeyCode::L => Key::L,
            KeyCode::M => Key::M,
            KeyCode::N => Key::N,
            KeyCode::O => Key::O,
            KeyCode::P => Key::P,
            KeyCode::Q => Key::Q,
            KeyCode::R => Key::R,
            KeyCode::S => Key::S,
            KeyCode::T => Key::T,
            KeyCode::U => Key::U,
            KeyCode::V => Key::V,
            KeyCode::W => Key::W,
            KeyCode::X => Key::X,
            KeyCode::Y => Key::Y,
            KeyCode::Z => Key::Z,
            KeyCode::Num0 => Key::Num0,
            KeyCode::Num1 => Key::Num1,
            KeyCode::Num2 => Key::Num2,
            KeyCode::Num3 => Key::Num3,
            KeyCode::Num4 => Key::Num4,
            KeyCode::Num5 => Key::Num5,
            KeyCode::Num6 => Key::Num6,
            KeyCode::Num7 => Key::Num7,
            KeyCode::Num8 => Key::Num8,
            KeyCode::Num9 => Key::Num9,
            KeyCode::F1 => Key::F1,
            KeyCode::F2 => Key::F2,
            KeyCode::F3 => Key::F3,
            KeyCode::F4 => Key::F4,
            KeyCode::F5 => Key::F5,
            KeyCode::F6 => Key::F6,
            KeyCode::F7 => Key::F7,
            KeyCode::F8 => Key::F8,
            KeyCode::F9 => Key::F9,
            KeyCode::F10 => Key::F10,
            KeyCode::F11 => Key::F11,
            KeyCode::F12 => Key::F12,
            KeyCode::Escape => Key::Escape,
            KeyCode::Tab => Key::Tab,
            KeyCode::Space => Key::Space,
            KeyCode::Enter => Key::Enter,
            KeyCode::Backspace => Key::Backspace,
            KeyCode::Delete => Key::Delete,
            KeyCode::Insert => Key::Insert,
            KeyCode::Home => Key::Home,
            KeyCode::End => Key::End,
            KeyCode::PageUp => Key::PageUp,
            KeyCode::PageDown => Key::PageDown,
            KeyCode::ArrowUp => Key::ArrowUp,
            KeyCode::ArrowDown => Key::ArrowDown,
            KeyCode::ArrowLeft => Key::ArrowLeft,
            KeyCode::ArrowRight => Key::ArrowRight,
            KeyCode::Minus => Key::Minus,
            KeyCode::Plus => Key::Plus,
            KeyCode::Equals => Key::Equals,
            KeyCode::LeftBracket => Key::OpenBracket,
            KeyCode::RightBracket => Key::CloseBracket,
            KeyCode::Semicolon => Key::Semicolon,
            KeyCode::Quote => Key::Quote,
            KeyCode::Comma => Key::Comma,
            KeyCode::Period => Key::Period,
            KeyCode::Slash => Key::Slash,
            KeyCode::Backslash => Key::Backslash,
            KeyCode::Grave => Key::Backtick,
        }
    }

    /// 从 egui::Key 转换
    pub fn from_egui_key(key: Key) -> Option<Self> {
        Some(match key {
            Key::A => KeyCode::A,
            Key::B => KeyCode::B,
            Key::C => KeyCode::C,
            Key::D => KeyCode::D,
            Key::E => KeyCode::E,
            Key::F => KeyCode::F,
            Key::G => KeyCode::G,
            Key::H => KeyCode::H,
            Key::I => KeyCode::I,
            Key::J => KeyCode::J,
            Key::K => KeyCode::K,
            Key::L => KeyCode::L,
            Key::M => KeyCode::M,
            Key::N => KeyCode::N,
            Key::O => KeyCode::O,
            Key::P => KeyCode::P,
            Key::Q => KeyCode::Q,
            Key::R => KeyCode::R,
            Key::S => KeyCode::S,
            Key::T => KeyCode::T,
            Key::U => KeyCode::U,
            Key::V => KeyCode::V,
            Key::W => KeyCode::W,
            Key::X => KeyCode::X,
            Key::Y => KeyCode::Y,
            Key::Z => KeyCode::Z,
            Key::Num0 => KeyCode::Num0,
            Key::Num1 => KeyCode::Num1,
            Key::Num2 => KeyCode::Num2,
            Key::Num3 => KeyCode::Num3,
            Key::Num4 => KeyCode::Num4,
            Key::Num5 => KeyCode::Num5,
            Key::Num6 => KeyCode::Num6,
            Key::Num7 => KeyCode::Num7,
            Key::Num8 => KeyCode::Num8,
            Key::Num9 => KeyCode::Num9,
            Key::F1 => KeyCode::F1,
            Key::F2 => KeyCode::F2,
            Key::F3 => KeyCode::F3,
            Key::F4 => KeyCode::F4,
            Key::F5 => KeyCode::F5,
            Key::F6 => KeyCode::F6,
            Key::F7 => KeyCode::F7,
            Key::F8 => KeyCode::F8,
            Key::F9 => KeyCode::F9,
            Key::F10 => KeyCode::F10,
            Key::F11 => KeyCode::F11,
            Key::F12 => KeyCode::F12,
            Key::Escape => KeyCode::Escape,
            Key::Tab => KeyCode::Tab,
            Key::Space => KeyCode::Space,
            Key::Enter => KeyCode::Enter,
            Key::Backspace => KeyCode::Backspace,
            Key::Delete => KeyCode::Delete,
            Key::Insert => KeyCode::Insert,
            Key::Home => KeyCode::Home,
            Key::End => KeyCode::End,
            Key::PageUp => KeyCode::PageUp,
            Key::PageDown => KeyCode::PageDown,
            Key::ArrowUp => KeyCode::ArrowUp,
            Key::ArrowDown => KeyCode::ArrowDown,
            Key::ArrowLeft => KeyCode::ArrowLeft,
            Key::ArrowRight => KeyCode::ArrowRight,
            Key::Minus => KeyCode::Minus,
            Key::Plus => KeyCode::Plus,
            Key::Equals => KeyCode::Equals,
            Key::OpenBracket => KeyCode::LeftBracket,
            Key::CloseBracket => KeyCode::RightBracket,
            Key::Semicolon => KeyCode::Semicolon,
            Key::Quote => KeyCode::Quote,
            Key::Comma => KeyCode::Comma,
            Key::Period => KeyCode::Period,
            Key::Slash => KeyCode::Slash,
            Key::Backslash => KeyCode::Backslash,
            Key::Backtick => KeyCode::Grave,
            _ => return None,
        })
    }

    /// 显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            KeyCode::A => "A",
            KeyCode::B => "B",
            KeyCode::C => "C",
            KeyCode::D => "D",
            KeyCode::E => "E",
            KeyCode::F => "F",
            KeyCode::G => "G",
            KeyCode::H => "H",
            KeyCode::I => "I",
            KeyCode::J => "J",
            KeyCode::K => "K",
            KeyCode::L => "L",
            KeyCode::M => "M",
            KeyCode::N => "N",
            KeyCode::O => "O",
            KeyCode::P => "P",
            KeyCode::Q => "Q",
            KeyCode::R => "R",
            KeyCode::S => "S",
            KeyCode::T => "T",
            KeyCode::U => "U",
            KeyCode::V => "V",
            KeyCode::W => "W",
            KeyCode::X => "X",
            KeyCode::Y => "Y",
            KeyCode::Z => "Z",
            KeyCode::Num0 => "0",
            KeyCode::Num1 => "1",
            KeyCode::Num2 => "2",
            KeyCode::Num3 => "3",
            KeyCode::Num4 => "4",
            KeyCode::Num5 => "5",
            KeyCode::Num6 => "6",
            KeyCode::Num7 => "7",
            KeyCode::Num8 => "8",
            KeyCode::Num9 => "9",
            KeyCode::F1 => "F1",
            KeyCode::F2 => "F2",
            KeyCode::F3 => "F3",
            KeyCode::F4 => "F4",
            KeyCode::F5 => "F5",
            KeyCode::F6 => "F6",
            KeyCode::F7 => "F7",
            KeyCode::F8 => "F8",
            KeyCode::F9 => "F9",
            KeyCode::F10 => "F10",
            KeyCode::F11 => "F11",
            KeyCode::F12 => "F12",
            KeyCode::Escape => "Esc",
            KeyCode::Tab => "Tab",
            KeyCode::Space => "Space",
            KeyCode::Enter => "Enter",
            KeyCode::Backspace => "Backspace",
            KeyCode::Delete => "Delete",
            KeyCode::Insert => "Insert",
            KeyCode::Home => "Home",
            KeyCode::End => "End",
            KeyCode::PageUp => "PageUp",
            KeyCode::PageDown => "PageDown",
            KeyCode::ArrowUp => "Up",
            KeyCode::ArrowDown => "Down",
            KeyCode::ArrowLeft => "Left",
            KeyCode::ArrowRight => "Right",
            KeyCode::Minus => "-",
            KeyCode::Plus => "+",
            KeyCode::Equals => "=",
            KeyCode::LeftBracket => "[",
            KeyCode::RightBracket => "]",
            KeyCode::Semicolon => ";",
            KeyCode::Quote => "'",
            KeyCode::Comma => ",",
            KeyCode::Period => ".",
            KeyCode::Slash => "/",
            KeyCode::Backslash => "\\",
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
    pub const NONE: Self = Self {
        ctrl: false,
        shift: false,
        alt: false,
        mac_cmd: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        shift: false,
        alt: false,
        mac_cmd: false,
    };
    pub const SHIFT: Self = Self {
        ctrl: false,
        shift: true,
        alt: false,
        mac_cmd: false,
    };
    pub const ALT: Self = Self {
        ctrl: false,
        shift: false,
        alt: true,
        mac_cmd: false,
    };
    pub const CTRL_SHIFT: Self = Self {
        ctrl: true,
        shift: true,
        alt: false,
        mac_cmd: false,
    };
    pub const CTRL_ALT: Self = Self {
        ctrl: true,
        shift: false,
        alt: true,
        mac_cmd: false,
    };

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
        self.ctrl == mods.ctrl && self.shift == mods.shift && self.alt == mods.alt
    }
}

impl fmt::Display for KeyModifiers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.mac_cmd {
            parts.push("Cmd");
        }
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
        Self {
            key,
            modifiers: KeyModifiers::NONE,
        }
    }

    /// 创建 Ctrl+Key 绑定
    pub fn ctrl(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::CTRL,
        }
    }

    /// 创建 Ctrl+Shift+Key 绑定
    pub fn ctrl_shift(key: KeyCode) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::CTRL_SHIFT,
        }
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
            if part.is_empty() {
                // 支持 "Ctrl++" 这类写法，将最后的空片段视为 '+' 键。
                key_str = "+";
                continue;
            }

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
            "A" => KeyCode::A,
            "B" => KeyCode::B,
            "C" => KeyCode::C,
            "D" => KeyCode::D,
            "E" => KeyCode::E,
            "F" => KeyCode::F,
            "G" => KeyCode::G,
            "H" => KeyCode::H,
            "I" => KeyCode::I,
            "J" => KeyCode::J,
            "K" => KeyCode::K,
            "L" => KeyCode::L,
            "M" => KeyCode::M,
            "N" => KeyCode::N,
            "O" => KeyCode::O,
            "P" => KeyCode::P,
            "Q" => KeyCode::Q,
            "R" => KeyCode::R,
            "S" => KeyCode::S,
            "T" => KeyCode::T,
            "U" => KeyCode::U,
            "V" => KeyCode::V,
            "W" => KeyCode::W,
            "X" => KeyCode::X,
            "Y" => KeyCode::Y,
            "Z" => KeyCode::Z,
            "0" | "NUM0" => KeyCode::Num0,
            "1" | "NUM1" => KeyCode::Num1,
            "2" | "NUM2" => KeyCode::Num2,
            "3" | "NUM3" => KeyCode::Num3,
            "4" | "NUM4" => KeyCode::Num4,
            "5" | "NUM5" => KeyCode::Num5,
            "6" | "NUM6" => KeyCode::Num6,
            "7" | "NUM7" => KeyCode::Num7,
            "8" | "NUM8" => KeyCode::Num8,
            "9" | "NUM9" => KeyCode::Num9,
            "F1" => KeyCode::F1,
            "F2" => KeyCode::F2,
            "F3" => KeyCode::F3,
            "F4" => KeyCode::F4,
            "F5" => KeyCode::F5,
            "F6" => KeyCode::F6,
            "F7" => KeyCode::F7,
            "F8" => KeyCode::F8,
            "F9" => KeyCode::F9,
            "F10" => KeyCode::F10,
            "F11" => KeyCode::F11,
            "F12" => KeyCode::F12,
            "ESC" | "ESCAPE" => KeyCode::Escape,
            "TAB" => KeyCode::Tab,
            "SPACE" => KeyCode::Space,
            "ENTER" | "RETURN" => KeyCode::Enter,
            "BACKSPACE" => KeyCode::Backspace,
            "DELETE" | "DEL" => KeyCode::Delete,
            "INSERT" | "INS" => KeyCode::Insert,
            "HOME" => KeyCode::Home,
            "END" => KeyCode::End,
            "PAGEUP" | "PGUP" => KeyCode::PageUp,
            "PAGEDOWN" | "PGDN" => KeyCode::PageDown,
            "UP" | "ARROWUP" => KeyCode::ArrowUp,
            "DOWN" | "ARROWDOWN" => KeyCode::ArrowDown,
            "LEFT" | "ARROWLEFT" => KeyCode::ArrowLeft,
            "RIGHT" | "ARROWRIGHT" => KeyCode::ArrowRight,
            "-" | "MINUS" => KeyCode::Minus,
            "+" | "PLUS" => KeyCode::Plus,
            "=" | "EQUALS" => KeyCode::Equals,
            "[" => KeyCode::LeftBracket,
            "]" => KeyCode::RightBracket,
            ";" => KeyCode::Semicolon,
            "'" => KeyCode::Quote,
            "," => KeyCode::Comma,
            "." => KeyCode::Period,
            "/" => KeyCode::Slash,
            "\\" => KeyCode::Backslash,
            "`" => KeyCode::Grave,
            _ => return None,
        })
    }

    /// 检查快捷键是否在当前帧被按下
    pub fn is_pressed(&self, ctx: &egui::Context) -> bool {
        ctx.input(|i| self.modifiers.matches(&i.modifiers) && i.key_pressed(self.key.to_egui_key()))
    }

    /// 文本输入作用域必须拒绝会直接写入字符的普通按键命令。
    pub fn conflicts_with_text_entry(&self) -> bool {
        if self.modifiers.ctrl || self.modifiers.alt || self.modifiers.mac_cmd {
            return false;
        }

        matches!(
            self.key,
            KeyCode::A
                | KeyCode::B
                | KeyCode::C
                | KeyCode::D
                | KeyCode::E
                | KeyCode::F
                | KeyCode::G
                | KeyCode::H
                | KeyCode::I
                | KeyCode::J
                | KeyCode::K
                | KeyCode::L
                | KeyCode::M
                | KeyCode::N
                | KeyCode::O
                | KeyCode::P
                | KeyCode::Q
                | KeyCode::R
                | KeyCode::S
                | KeyCode::T
                | KeyCode::U
                | KeyCode::V
                | KeyCode::W
                | KeyCode::X
                | KeyCode::Y
                | KeyCode::Z
                | KeyCode::Num0
                | KeyCode::Num1
                | KeyCode::Num2
                | KeyCode::Num3
                | KeyCode::Num4
                | KeyCode::Num5
                | KeyCode::Num6
                | KeyCode::Num7
                | KeyCode::Num8
                | KeyCode::Num9
                | KeyCode::Space
                | KeyCode::Minus
                | KeyCode::Plus
                | KeyCode::Equals
                | KeyCode::LeftBracket
                | KeyCode::RightBracket
                | KeyCode::Semicolon
                | KeyCode::Quote
                | KeyCode::Comma
                | KeyCode::Period
                | KeyCode::Slash
                | KeyCode::Backslash
                | KeyCode::Grave
        )
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
    /// 切到下一个主区域
    NextFocusArea,
    /// 切到上一个主区域
    PrevFocusArea,
    /// 新建连接
    NewConnection,
    /// 打开命令面板
    CommandPalette,
    /// 打开快捷键设置
    OpenKeybindingsDialog,
    /// 打开主题选择器
    OpenThemeSelector,
    /// 切换侧边栏
    ToggleSidebar,
    /// 切换明暗主题
    ToggleDarkMode,
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
    /// 跳转到行
    GotoLine,

    // === 侧边栏焦点 ===
    /// 聚焦连接分区
    FocusSidebarConnections,
    /// 聚焦数据库分区
    FocusSidebarDatabases,
    /// 聚焦表分区
    FocusSidebarTables,
    /// 聚焦筛选分区
    FocusSidebarFilters,
    /// 聚焦触发器分区
    FocusSidebarTriggers,
    /// 聚焦存储过程分区
    FocusSidebarRoutines,

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
            Action::NextFocusArea,
            Action::PrevFocusArea,
            Action::NewConnection,
            Action::CommandPalette,
            Action::OpenKeybindingsDialog,
            Action::OpenThemeSelector,
            Action::ToggleSidebar,
            Action::ToggleDarkMode,
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
            Action::GotoLine,
            Action::FocusSidebarConnections,
            Action::FocusSidebarDatabases,
            Action::FocusSidebarTables,
            Action::FocusSidebarFilters,
            Action::FocusSidebarTriggers,
            Action::FocusSidebarRoutines,
            Action::ZoomIn,
            Action::ZoomOut,
            Action::ZoomReset,
        ]
    }

    /// 获取操作的描述
    pub fn description(&self) -> &'static str {
        match self {
            Action::NextFocusArea => "切到下一个主区域",
            Action::PrevFocusArea => "切到上一个主区域",
            Action::NewConnection => "新建连接",
            Action::CommandPalette => "打开命令面板",
            Action::OpenKeybindingsDialog => "打开快捷键设置",
            Action::OpenThemeSelector => "打开主题选择器",
            Action::ToggleSidebar => "切换侧边栏",
            Action::ToggleDarkMode => "切换明暗主题",
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
            Action::GotoLine => "跳转到行",
            Action::FocusSidebarConnections => "聚焦连接分区",
            Action::FocusSidebarDatabases => "聚焦数据库分区",
            Action::FocusSidebarTables => "聚焦表分区",
            Action::FocusSidebarFilters => "聚焦筛选分区",
            Action::FocusSidebarTriggers => "聚焦触发器分区",
            Action::FocusSidebarRoutines => "聚焦存储过程分区",
            Action::ZoomIn => "放大",
            Action::ZoomOut => "缩小",
            Action::ZoomReset => "重置缩放",
        }
    }

    /// 获取操作的分类
    pub fn category(&self) -> &'static str {
        match self {
            Action::NextFocusArea
            | Action::PrevFocusArea
            | Action::NewConnection
            | Action::CommandPalette
            | Action::OpenKeybindingsDialog
            | Action::ToggleSidebar
            | Action::ToggleEditor
            | Action::ToggleErDiagram
            | Action::ShowHelp
            | Action::ShowHistory
            | Action::Export
            | Action::Import
            | Action::Refresh
            | Action::ClearCommandLine
            | Action::ClearSearch => "全局",
            Action::NewTable | Action::NewDatabase | Action::NewUser => "创建",
            Action::NewTab | Action::CloseTab | Action::NextTab | Action::PrevTab => "Tab",
            Action::Save | Action::GotoLine => "编辑",
            Action::OpenThemeSelector | Action::ToggleDarkMode => "外观",
            Action::FocusSidebarConnections
            | Action::FocusSidebarDatabases
            | Action::FocusSidebarTables
            | Action::FocusSidebarFilters
            | Action::FocusSidebarTriggers
            | Action::FocusSidebarRoutines => "侧边栏",
            Action::ZoomIn | Action::ZoomOut | Action::ZoomReset => "缩放",
        }
    }

    /// keymap.toml 中使用的稳定键名
    pub fn keymap_name(&self) -> &'static str {
        match self {
            Action::NextFocusArea => "next_focus_area",
            Action::PrevFocusArea => "prev_focus_area",
            Action::NewConnection => "new_connection",
            Action::CommandPalette => "command_palette",
            Action::OpenKeybindingsDialog => "open_keybindings",
            Action::OpenThemeSelector => "open_theme_selector",
            Action::ToggleSidebar => "toggle_sidebar",
            Action::ToggleDarkMode => "toggle_dark_mode",
            Action::ToggleEditor => "toggle_editor",
            Action::ToggleErDiagram => "toggle_er_diagram",
            Action::ShowHelp => "show_help",
            Action::ShowHistory => "show_history",
            Action::Export => "export",
            Action::Import => "import",
            Action::Refresh => "refresh",
            Action::ClearCommandLine => "clear_command_line",
            Action::ClearSearch => "clear_search",
            Action::NewTable => "new_table",
            Action::NewDatabase => "new_database",
            Action::NewUser => "new_user",
            Action::NewTab => "new_tab",
            Action::CloseTab => "close_tab",
            Action::NextTab => "next_tab",
            Action::PrevTab => "prev_tab",
            Action::Save => "save",
            Action::GotoLine => "goto_line",
            Action::FocusSidebarConnections => "focus_sidebar_connections",
            Action::FocusSidebarDatabases => "focus_sidebar_databases",
            Action::FocusSidebarTables => "focus_sidebar_tables",
            Action::FocusSidebarFilters => "focus_sidebar_filters",
            Action::FocusSidebarTriggers => "focus_sidebar_triggers",
            Action::FocusSidebarRoutines => "focus_sidebar_routines",
            Action::ZoomIn => "zoom_in",
            Action::ZoomOut => "zoom_out",
            Action::ZoomReset => "zoom_reset",
        }
    }

    pub fn from_keymap_name(name: &str) -> Option<Self> {
        Some(match name {
            "next_focus_area" => Action::NextFocusArea,
            "prev_focus_area" => Action::PrevFocusArea,
            "new_connection" => Action::NewConnection,
            "command_palette" => Action::CommandPalette,
            "open_keybindings" => Action::OpenKeybindingsDialog,
            "open_theme_selector" => Action::OpenThemeSelector,
            "toggle_sidebar" => Action::ToggleSidebar,
            "toggle_dark_mode" => Action::ToggleDarkMode,
            "toggle_editor" => Action::ToggleEditor,
            "toggle_er_diagram" => Action::ToggleErDiagram,
            "show_help" => Action::ShowHelp,
            "show_history" => Action::ShowHistory,
            "export" => Action::Export,
            "import" => Action::Import,
            "refresh" => Action::Refresh,
            "clear_command_line" => Action::ClearCommandLine,
            "clear_search" => Action::ClearSearch,
            "new_table" => Action::NewTable,
            "new_database" => Action::NewDatabase,
            "new_user" => Action::NewUser,
            "new_tab" => Action::NewTab,
            "close_tab" => Action::CloseTab,
            "next_tab" => Action::NextTab,
            "prev_tab" => Action::PrevTab,
            "save" => Action::Save,
            "goto_line" => Action::GotoLine,
            "focus_sidebar_connections" => Action::FocusSidebarConnections,
            "focus_sidebar_databases" => Action::FocusSidebarDatabases,
            "focus_sidebar_tables" => Action::FocusSidebarTables,
            "focus_sidebar_filters" => Action::FocusSidebarFilters,
            "focus_sidebar_triggers" => Action::FocusSidebarTriggers,
            "focus_sidebar_routines" => Action::FocusSidebarRoutines,
            "zoom_in" => Action::ZoomIn,
            "zoom_out" => Action::ZoomOut,
            "zoom_reset" => Action::ZoomReset,
            _ => return None,
        })
    }
}

/// 快捷键绑定管理器
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyBindings {
    /// 操作到快捷键的映射
    bindings: HashMap<Action, KeyBinding>,
    /// 局部作用域快捷键覆盖（如 dialog.help.scroll_up）
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    local_bindings: HashMap<String, Vec<KeyBinding>>,
    /// 局部命令序列覆盖（如 grid.normal.copy_row）
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    local_sequences: HashMap<String, Vec<String>>,
    /// 运行时诊断信息，仅用于 UI 和日志，不写回磁盘。
    #[serde(default, skip_serializing, skip_deserializing)]
    diagnostics: Vec<KeymapDiagnostic>,
}

impl Default for KeyBindings {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // 全局操作
        bindings.insert(Action::NextFocusArea, KeyBinding::key_only(KeyCode::Tab));
        bindings.insert(
            Action::PrevFocusArea,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::SHIFT),
        );
        bindings.insert(Action::NewConnection, KeyBinding::ctrl(KeyCode::N));
        bindings.insert(Action::CommandPalette, KeyBinding::ctrl(KeyCode::P));
        bindings.insert(
            Action::OpenKeybindingsDialog,
            KeyBinding::new(KeyCode::K, KeyModifiers::ALT),
        );
        bindings.insert(
            Action::OpenThemeSelector,
            KeyBinding::ctrl_shift(KeyCode::T),
        );
        bindings.insert(Action::ToggleSidebar, KeyBinding::ctrl(KeyCode::B));
        bindings.insert(Action::ToggleDarkMode, KeyBinding::ctrl(KeyCode::D));
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
        bindings.insert(Action::NewTab, KeyBinding::ctrl(KeyCode::T));
        bindings.insert(Action::CloseTab, KeyBinding::ctrl(KeyCode::W));
        bindings.insert(
            Action::NextTab,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::CTRL),
        );
        bindings.insert(
            Action::PrevTab,
            KeyBinding::new(KeyCode::Tab, KeyModifiers::CTRL_SHIFT),
        );

        // 编辑操作
        bindings.insert(Action::Save, KeyBinding::ctrl(KeyCode::S));
        bindings.insert(Action::GotoLine, KeyBinding::ctrl(KeyCode::G));

        // 侧边栏焦点
        bindings.insert(
            Action::FocusSidebarConnections,
            KeyBinding::ctrl(KeyCode::Num1),
        );
        bindings.insert(
            Action::FocusSidebarDatabases,
            KeyBinding::ctrl(KeyCode::Num2),
        );
        bindings.insert(Action::FocusSidebarTables, KeyBinding::ctrl(KeyCode::Num3));
        bindings.insert(Action::FocusSidebarFilters, KeyBinding::ctrl(KeyCode::Num4));
        bindings.insert(
            Action::FocusSidebarTriggers,
            KeyBinding::ctrl(KeyCode::Num5),
        );
        bindings.insert(
            Action::FocusSidebarRoutines,
            KeyBinding::ctrl(KeyCode::Num6),
        );

        // 缩放
        bindings.insert(Action::ZoomIn, KeyBinding::ctrl(KeyCode::Plus));
        bindings.insert(Action::ZoomOut, KeyBinding::ctrl(KeyCode::Minus));
        bindings.insert(Action::ZoomReset, KeyBinding::ctrl(KeyCode::Num0));

        Self {
            bindings,
            local_bindings: HashMap::new(),
            local_sequences: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }
}

impl KeyBindings {
    const KNOWN_ACTION_SCOPE_PATHS: &[&'static str] = &[
        "toolbar",
        "query_tabs",
        "sidebar.connections",
        "sidebar.databases",
        "sidebar.tables",
        "sidebar.filters.list",
        "sidebar.filters.input",
        "sidebar.triggers",
        "sidebar.routines",
        "grid.normal",
        "grid.select",
        "grid.insert",
        "editor.normal",
        "editor.insert",
        "dialog.connection",
        "dialog.export",
        "dialog.import",
        "dialog.delete_confirm",
        "dialog.help",
        "dialog.about",
        "dialog.welcome_setup",
        "dialog.history",
        "dialog.ddl",
        "dialog.create_database",
        "dialog.create_user",
        "dialog.keybindings",
        "dialog.command_palette",
        "dialog.generic",
        "er_diagram",
    ];

    pub fn keymap_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("gridix"))
    }

    pub fn keymap_path() -> Option<PathBuf> {
        Self::keymap_dir().map(|p| p.join("keymap.toml"))
    }

    /// 创建新的快捷键管理器
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load_or_init(legacy: &KeyBindings) -> Self {
        let Some(path) = Self::keymap_path() else {
            tracing::warn!("无法找到 keymap.toml 路径，回退到内置快捷键");
            let mut bindings = Self::default();
            if Self::has_legacy_customizations(legacy) {
                bindings.push_diagnostic(
                    KeymapDiagnosticSeverity::Warning,
                    KeymapDiagnosticCode::LegacyConfigMigrationPending,
                    "config.toml.keybindings",
                    "检测到旧版 config.toml 内联快捷键，但当前会话无法定位 keymap.toml 路径；运行时已回退到默认 keymap，请手动迁移到 ~/.config/gridix/keymap.toml。",
                );
            }
            return bindings;
        };

        match Self::load_or_init_from_path(&path, legacy) {
            Ok(bindings) => bindings,
            Err(error) => {
                tracing::warn!(error = %error, path = ?path, "加载 keymap.toml 失败，回退到默认/迁移键位");
                let mut bindings = Self::default();
                if Self::has_legacy_customizations(legacy) {
                    bindings.push_diagnostic(
                        KeymapDiagnosticSeverity::Warning,
                        KeymapDiagnosticCode::LegacyConfigMigrationPending,
                        "config.toml.keybindings",
                        "检测到旧版 config.toml 内联快捷键，但 keymap.toml 加载失败；运行时已回退到默认 keymap，请手动迁移后重试。",
                    );
                }
                bindings
            }
        }
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

    pub fn diagnostics(&self) -> &[KeymapDiagnostic] {
        &self.diagnostics
    }

    pub fn diagnostics_for_action(&self, action: Action) -> Vec<&KeymapDiagnostic> {
        let action_key = action.keymap_name();
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.path == action_key)
            .collect()
    }

    pub fn diagnostics_for_command(&self, command_id: &str) -> Vec<&KeymapDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.path == command_id)
            .collect()
    }

    pub fn diagnostics_for_binding_path(&self, path: &str) -> Vec<&KeymapDiagnostic> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.path == path)
            .collect()
    }

    pub fn has_customizations(&self) -> bool {
        let defaults = Self::default();
        self.bindings != defaults.bindings
            || !self.local_bindings.is_empty()
            || !self.local_sequences.is_empty()
    }

    pub fn effective_scoped_bindings(&self, command_id: &str) -> Vec<KeyBinding> {
        self.local_bindings_for(command_id)
            .map(|bindings| bindings.to_vec())
            .unwrap_or_else(|| {
                scoped_commands()
                    .iter()
                    .find(|command| command.id == command_id)
                    .map(|command| {
                        command
                            .default_bindings
                            .iter()
                            .map(|binding| binding.key_binding())
                            .collect()
                    })
                    .unwrap_or_default()
            })
    }

    /// 获取局部作用域快捷键覆盖
    pub fn local_bindings_for(&self, key: &str) -> Option<&[KeyBinding]> {
        self.local_bindings.get(key).map(Vec::as_slice)
    }

    /// 获取某个作用域下动作对应的局部快捷键覆盖。
    ///
    /// 运行时路由使用 `scope_path.action_name`，例如
    /// `sidebar.tables.refresh` 或 `editor.normal.clear_command_line`。
    pub fn scoped_bindings_for_action(
        &self,
        scope_path: &str,
        action: Action,
    ) -> Option<&[KeyBinding]> {
        let key = format!("{}.{}", scope_path, action.keymap_name());
        self.local_bindings_for(&key)
    }

    /// 获取所有局部作用域快捷键覆盖
    pub fn all_local_bindings(&self) -> &HashMap<String, Vec<KeyBinding>> {
        &self.local_bindings
    }

    /// 获取局部命令序列覆盖
    pub fn local_sequences_for(&self, key: &str) -> Option<&[String]> {
        self.local_sequences.get(key).map(Vec::as_slice)
    }

    /// 获取所有局部命令序列覆盖
    pub fn all_local_sequences(&self) -> &HashMap<String, Vec<String>> {
        &self.local_sequences
    }

    /// 设置局部作用域快捷键覆盖
    pub fn set_local_bindings(&mut self, key: impl Into<String>, bindings: Vec<KeyBinding>) {
        self.local_bindings.insert(key.into(), bindings);
    }

    /// 设置局部命令序列覆盖
    pub fn set_local_sequences(&mut self, key: impl Into<String>, sequences: Vec<String>) {
        self.local_sequences.insert(key.into(), sequences);
    }

    /// 移除局部作用域快捷键覆盖，恢复默认行为
    pub fn remove_local_bindings(&mut self, key: &str) {
        self.local_bindings.remove(key);
    }

    /// 移除局部命令序列覆盖，恢复默认行为
    pub fn remove_local_sequences(&mut self, key: &str) {
        self.local_sequences.remove(key);
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

    pub fn save_to_disk(&self) -> Result<(), String> {
        let path = Self::keymap_path().ok_or("无法找到 keymap.toml 路径")?;
        self.save_to_path(&path)
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

    fn fill_missing_defaults(&mut self) {
        let defaults = Self::default();
        for action in Action::all() {
            if !self.bindings.contains_key(action)
                && let Some(default_binding) = defaults.get(*action)
            {
                self.bindings.insert(*action, default_binding.clone());
            }
        }
    }

    fn push_diagnostic(
        &mut self,
        severity: KeymapDiagnosticSeverity,
        code: KeymapDiagnosticCode,
        path: impl Into<String>,
        message: impl Into<String>,
    ) {
        let diagnostic = KeymapDiagnostic {
            severity,
            code,
            path: path.into(),
            message: message.into(),
        };
        if !self
            .diagnostics
            .iter()
            .any(|existing| existing == &diagnostic)
        {
            match diagnostic.severity {
                KeymapDiagnosticSeverity::Warning => {
                    tracing::warn!(path = %diagnostic.path, message = %diagnostic.message, "keymap warning");
                }
                KeymapDiagnosticSeverity::Error => {
                    tracing::warn!(path = %diagnostic.path, message = %diagnostic.message, "keymap error");
                }
            }
            self.diagnostics.push(diagnostic);
        }
    }

    fn has_legacy_customizations(legacy: &KeyBindings) -> bool {
        let defaults = Self::default();
        legacy.bindings != defaults.bindings
            || !legacy.local_bindings.is_empty()
            || !legacy.local_sequences.is_empty()
    }

    fn load_or_init_from_path(path: &Path, legacy: &KeyBindings) -> Result<Self, String> {
        if !path.exists() {
            let mut initial = Self::default();
            if Self::has_legacy_customizations(legacy) {
                initial.push_diagnostic(
                    KeymapDiagnosticSeverity::Warning,
                    KeymapDiagnosticCode::LegacyConfigMigrationPending,
                    "config.toml.keybindings",
                    "检测到旧版 config.toml 内联快捷键。已初始化默认 keymap.toml；当前运行时使用默认 keymap，旧字段迁移和清理留到兼容窗口结束后处理。",
                );
            }
            initial.save_to_path(path)?;
            return Ok(initial);
        }

        let content =
            fs::read_to_string(path).map_err(|e| format!("读取 keymap.toml 失败: {}", e))?;
        Self::parse_keymap(&content)
    }

    fn parse_keymap(content: &str) -> Result<Self, String> {
        let value: toml::Value =
            toml::from_str(content).map_err(|e| format!("解析 keymap.toml 失败: {}", e))?;
        let table = value
            .as_table()
            .ok_or("keymap.toml 必须是顶层 TOML 表".to_string())?;

        let mut bindings = Self::default();
        bindings.local_bindings.clear();
        bindings.local_sequences.clear();
        bindings.diagnostics.clear();

        for (raw_key, raw_value) in table {
            if raw_key == "global" {
                let Some(global_table) = raw_value.as_table() else {
                    bindings.push_diagnostic(
                        KeymapDiagnosticSeverity::Error,
                        KeymapDiagnosticCode::InvalidBinding,
                        raw_key,
                        "global 段必须是 TOML 表。",
                    );
                    continue;
                };
                for (action_key, action_value) in global_table {
                    bindings.parse_global_binding_entry(action_key, action_value);
                }
                continue;
            }

            if Action::from_keymap_name(raw_key).is_some() || raw_key == "major_area_switch" {
                bindings.parse_global_binding_entry(raw_key, raw_value);
                continue;
            }

            let Some(section_table) = raw_value.as_table() else {
                bindings.push_diagnostic(
                    KeymapDiagnosticSeverity::Warning,
                    KeymapDiagnosticCode::UnknownSection,
                    raw_key,
                    "忽略未知或非表结构的快捷键配置。",
                );
                continue;
            };

            bindings.parse_local_binding_section(raw_key, section_table);
        }

        bindings.fill_missing_defaults();
        bindings.collect_conflict_diagnostics();

        Ok(bindings)
    }

    fn save_to_path(&self, path: &Path) -> Result<(), String> {
        let dir = path.parent().ok_or("无法找到 keymap.toml 目录")?;
        fs::create_dir_all(dir).map_err(|e| format!("创建 keymap 目录失败: {}", e))?;

        let mut root = toml::map::Map::new();
        let mut global = BTreeMap::new();
        for action in Action::all() {
            if let Some(binding) = self.get(*action) {
                global.insert(
                    action.keymap_name().to_string(),
                    toml::Value::String(binding.display()),
                );
            }
        }

        root.insert(
            "global".to_string(),
            toml::Value::Table(global.into_iter().collect()),
        );

        let mut local_entries: Vec<_> = scoped_commands()
            .iter()
            .map(|command| {
                (
                    command.id.to_string(),
                    self.effective_scoped_bindings(command.id),
                )
            })
            .collect();
        local_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (path_key, bindings) in local_entries {
            Self::insert_nested_local_binding(
                &mut root,
                &path_key,
                Self::serialize_binding_list(&bindings),
            );
        }

        let mut extra_local_entries: Vec<_> = self
            .local_bindings
            .iter()
            .filter(|(path_key, _)| {
                !scoped_commands()
                    .iter()
                    .any(|command| command.id == path_key.as_str())
            })
            .collect();
        extra_local_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (path_key, bindings) in extra_local_entries {
            Self::insert_nested_local_binding(
                &mut root,
                path_key,
                Self::serialize_binding_list(bindings),
            );
        }

        let mut sequence_entries: Vec<_> = self.local_sequences.iter().collect();
        sequence_entries.sort_by(|(left, _), (right, _)| left.cmp(right));
        for (path_key, sequences) in sequence_entries {
            Self::insert_nested_local_binding(
                &mut root,
                path_key,
                Self::serialize_string_list(sequences),
            );
        }

        let content = toml::to_string_pretty(&toml::Value::Table(root))
            .map_err(|e| format!("序列化 keymap.toml 失败: {}", e))?;

        let temp_path = path.with_extension("toml.tmp");
        fs::write(&temp_path, &content).map_err(|e| format!("写入 keymap 临时文件失败: {}", e))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&temp_path, permissions)
                .map_err(|e| format!("设置 keymap 权限失败: {}", e))?;
        }

        fs::rename(&temp_path, path).map_err(|e| format!("写入 keymap.toml 失败: {}", e))?;
        Ok(())
    }

    fn parse_global_binding_entry(&mut self, raw_key: &str, raw_value: &toml::Value) {
        let action = if raw_key == "major_area_switch" {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Warning,
                KeymapDiagnosticCode::DeprecatedAlias,
                raw_key,
                "major_area_switch 已废弃，请改用 next_focus_area / prev_focus_area。当前会按 next_focus_area 处理。",
            );
            Action::NextFocusArea
        } else if let Some(action) = Action::from_keymap_name(raw_key) {
            action
        } else {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Warning,
                KeymapDiagnosticCode::UnknownAction,
                raw_key,
                "忽略未知的全局快捷键动作。",
            );
            return;
        };

        let Some(binding_text) = raw_value.as_str() else {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Error,
                KeymapDiagnosticCode::InvalidBinding,
                raw_key,
                "全局快捷键必须是字符串。",
            );
            return;
        };

        let Some(binding) = KeyBinding::parse(binding_text) else {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Error,
                KeymapDiagnosticCode::InvalidBinding,
                raw_key,
                format!("无法解析全局快捷键绑定 {binding_text:?}。"),
            );
            return;
        };

        self.set(action, binding);
    }

    fn parse_local_binding_section(
        &mut self,
        prefix: &str,
        table: &toml::map::Map<String, toml::Value>,
    ) {
        if !Self::is_known_scoped_prefix(prefix) {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Warning,
                KeymapDiagnosticCode::UnknownSection,
                prefix,
                "忽略未知的 keymap 作用域段。",
            );
            return;
        }

        for (raw_key, raw_value) in table {
            let path = format!("{prefix}.{raw_key}");
            if let Some(child_table) = raw_value.as_table() {
                self.parse_local_binding_section(&path, child_table);
                continue;
            }

            if Self::is_local_sequence_key(&path) {
                let Some(sequences) = self.parse_local_sequence_value(&path, raw_value) else {
                    continue;
                };

                self.local_sequences.insert(path, sequences);
                continue;
            }

            let Some(binding_path) = self.canonical_local_binding_path(&path) else {
                self.push_diagnostic(
                    KeymapDiagnosticSeverity::Warning,
                    KeymapDiagnosticCode::UnknownAction,
                    &path,
                    "忽略未知的作用域动作。",
                );
                continue;
            };

            let Some(bindings) = self.parse_local_binding_value(&binding_path, raw_value) else {
                continue;
            };

            self.local_bindings.insert(binding_path, bindings);
        }
    }

    fn is_local_sequence_key(path: &str) -> bool {
        path.starts_with("grid.")
    }

    fn is_known_scoped_prefix(prefix: &str) -> bool {
        prefix == "grid"
            || prefix.starts_with("grid.")
            || Self::KNOWN_ACTION_SCOPE_PATHS.iter().any(|scope| {
                *scope == prefix
                    || scope
                        .strip_prefix(prefix)
                        .is_some_and(|suffix| suffix.starts_with('.'))
            })
            || scoped_commands().iter().any(|command| {
                command.id == prefix
                    || command
                        .id
                        .strip_prefix(prefix)
                        .is_some_and(|suffix| suffix.starts_with('.'))
            })
    }

    fn canonical_command_id(&mut self, path: &str) -> Option<&'static str> {
        if let Some(command) = scoped_commands().iter().find(|command| command.id == path) {
            return Some(command.id);
        }

        let alias = match path {
            "editor.sql.execute" => Some("editor.insert.execute"),
            "editor.sql.explain" => Some("editor.insert.explain"),
            "editor.sql.clear" => Some("editor.insert.clear"),
            "editor.sql.autocomplete_trigger" => Some("editor.insert.trigger_completion"),
            "editor.sql.autocomplete_confirm" => Some("editor.insert.confirm_completion"),
            "editor.sql.history_prev" => Some("editor.insert.history_prev"),
            "editor.sql.history_next" => Some("editor.insert.history_next"),
            "editor.sql.history_browse" => Some("editor.insert.history_browse"),
            _ => None,
        }?;

        self.push_diagnostic(
            KeymapDiagnosticSeverity::Warning,
            KeymapDiagnosticCode::DeprecatedAlias,
            path,
            format!("旧命令 id {path} 已废弃，将按 {alias} 处理。"),
        );
        Some(alias)
    }

    fn is_known_action_scope(scope_path: &str) -> bool {
        Self::KNOWN_ACTION_SCOPE_PATHS.contains(&scope_path)
    }

    fn scope_resolution_chain(scope: &str) -> Vec<&str> {
        let mut chain = Vec::new();
        let mut current = Some(scope);
        while let Some(path) = current {
            if !chain.contains(&path) {
                chain.push(path);
            }
            current = path.rsplit_once('.').map(|(parent, _)| parent);
        }

        // Runtime dialog dispatch lets specific dialog scopes fall back to
        // dialog.common before the key returns to workspace/global routing.
        if scope.starts_with("dialog.")
            && scope != "dialog.common"
            && scope != "dialog.confirm"
            && !chain.contains(&"dialog.common")
        {
            chain.push("dialog.common");
        }

        // Filters list commands inherit the generic sidebar list traversal keys.
        if scope.starts_with("sidebar.filters") && !chain.contains(&"sidebar.list") {
            chain.push("sidebar.list");
        }

        chain
    }

    fn scopes_shadow_each_other(left_scope: &str, right_scope: &str) -> bool {
        if left_scope == right_scope {
            return false;
        }

        let left_chain = Self::scope_resolution_chain(left_scope);
        let right_chain = Self::scope_resolution_chain(right_scope);
        left_chain.iter().skip(1).any(|scope| *scope == right_scope)
            || right_chain.iter().skip(1).any(|scope| *scope == left_scope)
    }

    fn canonical_local_binding_path(&mut self, path: &str) -> Option<String> {
        if let Some(command_id) = self.canonical_command_id(path) {
            return Some(command_id.to_string());
        }

        let (scope_path, action_key) = path.rsplit_once('.')?;
        if Self::is_known_action_scope(scope_path) && Action::from_keymap_name(action_key).is_some()
        {
            return Some(path.to_string());
        }

        None
    }

    fn is_text_entry_scope(command_id: &str) -> bool {
        command_id.starts_with("editor.insert.") || command_id.starts_with("sidebar.filters.input.")
    }

    fn parse_local_binding_value(
        &mut self,
        path: &str,
        raw_value: &toml::Value,
    ) -> Option<Vec<KeyBinding>> {
        match raw_value {
            toml::Value::String(binding_text) => {
                let Some(binding) = KeyBinding::parse(binding_text) else {
                    self.push_diagnostic(
                        KeymapDiagnosticSeverity::Error,
                        KeymapDiagnosticCode::InvalidBinding,
                        path,
                        format!("无法解析局部快捷键绑定 {binding_text:?}。"),
                    );
                    return None;
                };
                let mut bindings = Vec::new();
                self.try_push_local_binding(path, &mut bindings, binding);
                (!bindings.is_empty()).then_some(bindings)
            }
            toml::Value::Array(values) => {
                let mut bindings = Vec::new();
                for value in values {
                    let Some(binding_text) = value.as_str() else {
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Error,
                            KeymapDiagnosticCode::InvalidBinding,
                            path,
                            "局部快捷键数组只能包含字符串。",
                        );
                        continue;
                    };
                    let Some(binding) = KeyBinding::parse(binding_text) else {
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Error,
                            KeymapDiagnosticCode::InvalidBinding,
                            path,
                            format!("无法解析局部快捷键绑定 {binding_text:?}。"),
                        );
                        continue;
                    };
                    self.try_push_local_binding(path, &mut bindings, binding);
                }
                if bindings.is_empty() {
                    self.push_diagnostic(
                        KeymapDiagnosticSeverity::Error,
                        KeymapDiagnosticCode::InvalidBinding,
                        path,
                        "局部快捷键数组为空或全部非法。",
                    );
                    None
                } else {
                    Some(bindings)
                }
            }
            _ => {
                self.push_diagnostic(
                    KeymapDiagnosticSeverity::Error,
                    KeymapDiagnosticCode::InvalidBinding,
                    path,
                    "局部快捷键必须是字符串或字符串数组。",
                );
                None
            }
        }
    }

    fn serialize_binding_list(bindings: &[KeyBinding]) -> toml::Value {
        if bindings.len() == 1 {
            toml::Value::String(bindings[0].display())
        } else {
            toml::Value::Array(
                bindings
                    .iter()
                    .map(|binding| toml::Value::String(binding.display()))
                    .collect(),
            )
        }
    }

    fn parse_local_sequence_value(
        &mut self,
        path: &str,
        raw_value: &toml::Value,
    ) -> Option<Vec<String>> {
        match raw_value {
            toml::Value::String(sequence) => {
                let sequence = sequence.trim();
                if sequence.is_empty() {
                    self.push_diagnostic(
                        KeymapDiagnosticSeverity::Error,
                        KeymapDiagnosticCode::InvalidBinding,
                        path,
                        "忽略空的局部命令序列。",
                    );
                    return None;
                }
                Some(vec![sequence.to_string()])
            }
            toml::Value::Array(values) => {
                let mut sequences = Vec::new();
                for value in values {
                    let Some(sequence) = value.as_str() else {
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Error,
                            KeymapDiagnosticCode::InvalidBinding,
                            path,
                            "局部命令序列数组只能包含字符串。",
                        );
                        continue;
                    };
                    let sequence = sequence.trim();
                    if sequence.is_empty() {
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Error,
                            KeymapDiagnosticCode::InvalidBinding,
                            path,
                            "忽略空白局部命令序列。",
                        );
                        continue;
                    }
                    if !sequences.iter().any(|existing| existing == sequence) {
                        sequences.push(sequence.to_string());
                    }
                }
                if sequences.is_empty() {
                    self.push_diagnostic(
                        KeymapDiagnosticSeverity::Error,
                        KeymapDiagnosticCode::InvalidBinding,
                        path,
                        "局部命令序列数组为空或全部非法。",
                    );
                    None
                } else {
                    Some(sequences)
                }
            }
            _ => {
                self.push_diagnostic(
                    KeymapDiagnosticSeverity::Error,
                    KeymapDiagnosticCode::InvalidBinding,
                    path,
                    "局部命令序列必须是字符串或字符串数组。",
                );
                None
            }
        }
    }

    fn serialize_string_list(values: &[String]) -> toml::Value {
        if values.len() == 1 {
            toml::Value::String(values[0].clone())
        } else {
            toml::Value::Array(
                values
                    .iter()
                    .map(|value| toml::Value::String(value.clone()))
                    .collect(),
            )
        }
    }

    fn insert_nested_local_binding(
        root: &mut toml::map::Map<String, toml::Value>,
        path_key: &str,
        value: toml::Value,
    ) {
        let mut parts = path_key.split('.').peekable();
        let mut current = root;

        while let Some(part) = parts.next() {
            if parts.peek().is_none() {
                current.insert(part.to_string(), value);
                break;
            }

            let entry = current
                .entry(part.to_string())
                .or_insert_with(|| toml::Value::Table(toml::map::Map::new()));
            if !entry.is_table() {
                *entry = toml::Value::Table(toml::map::Map::new());
            }
            current = entry
                .as_table_mut()
                .expect("entry was just normalized to a TOML table");
        }
    }

    fn try_push_local_binding(
        &mut self,
        path: &str,
        bindings: &mut Vec<KeyBinding>,
        binding: KeyBinding,
    ) {
        if Self::is_text_entry_scope(path) && binding.conflicts_with_text_entry() {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Error,
                KeymapDiagnosticCode::TextEntryPlainCharacterRejected,
                path,
                format!(
                    "文本输入作用域 {path} 不接受会直接写入字符的命令绑定 {}。",
                    binding.display()
                ),
            );
            return;
        }

        if !bindings.iter().any(|existing| existing == &binding) {
            bindings.push(binding);
        }
    }

    fn collect_conflict_diagnostics(&mut self) {
        for (left, right, binding) in self.find_conflicts() {
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Error,
                KeymapDiagnosticCode::ExactScopeConflict,
                left.keymap_name(),
                format!(
                    "{} 与同一作用域动作 {} 使用了相同绑定 {}。",
                    left.description(),
                    right.description(),
                    binding.display()
                ),
            );
            self.push_diagnostic(
                KeymapDiagnosticSeverity::Error,
                KeymapDiagnosticCode::ExactScopeConflict,
                right.keymap_name(),
                format!(
                    "{} 与同一作用域动作 {} 使用了相同绑定 {}。",
                    right.description(),
                    left.description(),
                    binding.display()
                ),
            );
        }

        let mut scoped_entries: Vec<(String, Vec<KeyBinding>)> = scoped_commands()
            .iter()
            .map(|command| {
                (
                    command.id.to_string(),
                    self.effective_scoped_bindings(command.id),
                )
            })
            .collect();
        scoped_entries.extend(self.local_bindings.iter().filter_map(|(path, bindings)| {
            if scoped_commands()
                .iter()
                .any(|command| command.id == path.as_str())
            {
                return None;
            }

            let (scope_path, action_key) = path.rsplit_once('.')?;
            if Self::is_known_action_scope(scope_path)
                && Action::from_keymap_name(action_key).is_some()
            {
                Some((path.clone(), bindings.clone()))
            } else {
                None
            }
        }));

        for i in 0..scoped_entries.len() {
            for j in (i + 1)..scoped_entries.len() {
                let (left_id, left_bindings) = &scoped_entries[i];
                let (right_id, right_bindings) = &scoped_entries[j];
                let left_scope = left_id
                    .rsplit_once('.')
                    .map(|(scope, _)| scope)
                    .unwrap_or(left_id.as_str());
                let right_scope = right_id
                    .rsplit_once('.')
                    .map(|(scope, _)| scope)
                    .unwrap_or(right_id.as_str());

                for binding in left_bindings {
                    if !right_bindings.iter().any(|candidate| candidate == binding) {
                        continue;
                    }

                    if left_scope == right_scope {
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Error,
                            KeymapDiagnosticCode::ExactScopeConflict,
                            left_id.as_str(),
                            format!(
                                "{} 与 {} 在同一作用域 {left_scope} 内重复使用 {}。",
                                left_id,
                                right_id,
                                binding.display()
                            ),
                        );
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Error,
                            KeymapDiagnosticCode::ExactScopeConflict,
                            right_id.as_str(),
                            format!(
                                "{} 与 {} 在同一作用域 {right_scope} 内重复使用 {}。",
                                right_id,
                                left_id,
                                binding.display()
                            ),
                        );
                    } else if Self::scopes_shadow_each_other(left_scope, right_scope) {
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Warning,
                            KeymapDiagnosticCode::ParentShadowing,
                            left_id.as_str(),
                            format!(
                                "{} 与层级父/子作用域命令 {} 共享 {}；运行时会优先由更具体的局部命令消费。",
                                left_id,
                                right_id,
                                binding.display()
                            ),
                        );
                        self.push_diagnostic(
                            KeymapDiagnosticSeverity::Warning,
                            KeymapDiagnosticCode::ParentShadowing,
                            right_id.as_str(),
                            format!(
                                "{} 与层级父/子作用域命令 {} 共享 {}；运行时会优先由更具体的局部命令消费。",
                                right_id,
                                left_id,
                                binding.display()
                            ),
                        );
                    }
                }
            }
        }

        let text_entry_commands: Vec<_> = scoped_commands()
            .iter()
            .filter(|command| Self::is_text_entry_scope(command.id))
            .collect();

        for action in Action::all() {
            let Some(global_binding) = self.get(*action).cloned() else {
                continue;
            };
            for command in &text_entry_commands {
                if self
                    .effective_scoped_bindings(command.id)
                    .iter()
                    .any(|binding| binding == &global_binding)
                {
                    self.push_diagnostic(
                        KeymapDiagnosticSeverity::Warning,
                        KeymapDiagnosticCode::WorkspaceFallbackShadowingTextEntry,
                        action.keymap_name(),
                        format!(
                            "{} 与文本输入作用域命令 {} 共享 {}。运行时会先由局部文本输入命令消费，再回退到 focus-area 切换或工作区动作。",
                            action.keymap_name(),
                            command.id,
                            global_binding.display()
                        ),
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, KeyBinding, KeyBindings, KeyCode, KeymapDiagnosticCode};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn missing_keymap_is_initialized_from_defaults_and_marks_legacy_migration() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keymap.toml");
        let mut legacy = KeyBindings::default();
        legacy.set(Action::NewConnection, KeyBinding::ctrl(KeyCode::P));

        let loaded = KeyBindings::load_or_init_from_path(&path, &legacy).unwrap();

        assert_eq!(loaded.display(Action::NewConnection), "Ctrl+N");
        assert!(path.exists());
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains("[global]"));
        assert!(content.contains("next_focus_area = \"Tab\""));
        assert!(content.contains("[editor.insert]"));
        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::LegacyConfigMigrationPending
        }));
    }

    #[test]
    fn partial_keymap_keeps_custom_values_and_fills_defaults() {
        let content = "new_connection = \"Alt+N\"\n";

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert_eq!(loaded.display(Action::NewConnection), "Alt+N");
        assert_eq!(loaded.display(Action::ShowHelp), "F1");
    }

    #[test]
    fn parse_plus_key_binding_from_ctrl_plus_plus() {
        let parsed = KeyBinding::parse("Ctrl++").expect("Ctrl++ should parse as Ctrl + Plus");

        assert_eq!(parsed.key, KeyCode::Plus);
        assert_eq!(parsed.display(), "Ctrl++");
    }

    #[test]
    fn invalid_entries_are_ignored_without_losing_defaults() {
        let content = r#"
unknown_action = "Ctrl+P"
new_connection = "NotAKey"
show_help = "F2"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert_eq!(loaded.display(Action::NewConnection), "Ctrl+N");
        assert_eq!(loaded.display(Action::ShowHelp), "F2");
    }

    #[test]
    fn invalid_key_produces_diagnostic_and_keeps_default_binding() {
        let content = r#"
[global]
next_focus_area = "NotAKey"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert_eq!(loaded.display(Action::NextFocusArea), "Tab");
        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::InvalidBinding
                && diagnostic.path == "next_focus_area"
        }));
    }

    #[test]
    fn structured_keymap_parses_global_and_local_sections() {
        let content = r#"
[global]
show_help = "F2"

[dialog.help]
scroll_up = ["K", "Up"]

[dialog.common]
dismiss = ["Esc", "Q"]

[grid.normal]
copy_row = ["yy", "Y"]
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert_eq!(loaded.display(Action::ShowHelp), "F2");
        let help_scroll = loaded.local_bindings_for("dialog.help.scroll_up").unwrap();
        assert_eq!(help_scroll.len(), 2);
        assert_eq!(help_scroll[0].display(), "K");
        assert_eq!(help_scroll[1].display(), "Up");
        let dismiss = loaded.local_bindings_for("dialog.common.dismiss").unwrap();
        assert_eq!(dismiss.len(), 2);
        assert_eq!(dismiss[0].display(), "Esc");
        assert_eq!(dismiss[1].display(), "Q");
        let copy_row = loaded.local_sequences_for("grid.normal.copy_row").unwrap();
        assert_eq!(copy_row, ["yy", "Y"]);
    }

    #[test]
    fn same_scope_conflict_detection_reports_error() {
        let content = r#"
[dialog.help]
scroll_up = "J"
scroll_down = "J"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::ExactScopeConflict
                && diagnostic.path == "dialog.help.scroll_up"
        }));
        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::ExactScopeConflict
                && diagnostic.path == "dialog.help.scroll_down"
        }));
    }

    #[test]
    fn same_scope_scoped_action_conflict_detection_reports_error() {
        let content = r#"
[toolbar]
refresh = "Ctrl+R"
save = "Ctrl+R"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::ExactScopeConflict
                && diagnostic.path == "toolbar.refresh"
        }));
        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::ExactScopeConflict
                && diagnostic.path == "toolbar.save"
        }));
    }

    #[test]
    fn inherited_conflict_detection_reports_warning() {
        let content = r#"
[dialog.common]
dismiss = "Esc"

[dialog.help]
scroll_up = "Esc"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::ParentShadowing
                && diagnostic.path == "dialog.help.scroll_up"
        }));
    }

    #[test]
    fn text_entry_scopes_reject_plain_character_commands() {
        let content = r#"
[editor.insert]
confirm_completion = "J"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        assert!(
            loaded
                .local_bindings_for("editor.insert.confirm_completion")
                .is_none()
        );
        assert!(loaded.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == KeymapDiagnosticCode::TextEntryPlainCharacterRejected
                && diagnostic.path == "editor.insert.confirm_completion"
        }));
    }

    #[test]
    fn scoped_bindings_for_action_reads_scope_action_paths() {
        let content = r#"
[toolbar]
refresh = "R"

[sidebar.tables]
show_help = "A"

[sidebar.filters.input]
show_help = "Ctrl+H"
"#;

        let loaded = KeyBindings::parse_keymap(content).unwrap();

        let toolbar_refresh = loaded
            .scoped_bindings_for_action("toolbar", Action::Refresh)
            .unwrap();
        assert_eq!(toolbar_refresh.len(), 1);
        assert_eq!(toolbar_refresh[0].display(), "R");

        let sidebar_table_help = loaded
            .scoped_bindings_for_action("sidebar.tables", Action::ShowHelp)
            .unwrap();
        assert_eq!(sidebar_table_help.len(), 1);
        assert_eq!(sidebar_table_help[0].display(), "A");

        let filters_input_help = loaded
            .scoped_bindings_for_action("sidebar.filters.input", Action::ShowHelp)
            .unwrap();
        assert_eq!(filters_input_help.len(), 1);
        assert_eq!(filters_input_help[0].display(), "Ctrl+H");

        assert!(
            loaded
                .scoped_bindings_for_action("sidebar.tables", Action::Refresh)
                .is_none()
        );
    }

    #[test]
    fn save_to_path_writes_structured_keymap_sections() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("keymap.toml");
        let mut bindings = KeyBindings::default();
        bindings.local_bindings.insert(
            "dialog.common.dismiss".to_string(),
            vec![
                KeyBinding::key_only(KeyCode::Escape),
                KeyBinding::key_only(KeyCode::Q),
            ],
        );
        bindings.local_bindings.insert(
            "dialog.help.scroll_up".to_string(),
            vec![KeyBinding::key_only(KeyCode::K)],
        );
        bindings.local_sequences.insert(
            "grid.normal.copy_row".to_string(),
            vec!["yy".to_string(), "Y".to_string()],
        );

        bindings.save_to_path(&path).unwrap();
        let content = fs::read_to_string(&path).unwrap();

        assert!(content.contains("[global]"));
        assert!(content.contains("[dialog.common]"));
        assert!(content.contains("dismiss = ["));
        assert!(content.contains("\"Esc\""));
        assert!(content.contains("\"Q\""));
        assert!(content.contains("[dialog.help]"));
        assert!(content.contains("scroll_up = \"K\""));
        assert!(content.contains("[editor.insert]"));
        assert!(content.contains("confirm_completion = ["));
        assert!(content.contains("\"Tab\""));
        assert!(content.contains("[grid.normal]"));
        assert!(content.contains("copy_row = ["));
        assert!(content.contains("\"yy\""));
        assert!(content.contains("\"Y\""));
    }
}
