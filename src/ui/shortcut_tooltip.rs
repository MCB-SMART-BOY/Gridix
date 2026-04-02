//! 快捷键提示文本工具
//!
//! 统一生成“功能 + 快捷键”的悬停提示，避免快捷键改配置后 UI 文案失真。

use crate::core::{Action, KeyBinding, KeyBindings, KeyCode, KeyModifiers};
use egui::InputState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalBinding {
    key: KeyCode,
    modifiers: KeyModifiers,
}

impl LocalBinding {
    pub const fn new(key: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    fn key_binding(self) -> KeyBinding {
        KeyBinding::new(self.key, self.modifiers)
    }

    pub fn display(self) -> String {
        self.key_binding().display()
    }

    pub fn is_pressed(self, ctx: &egui::Context) -> bool {
        self.key_binding().is_pressed(ctx)
    }

    pub fn consume(self, input: &mut InputState) -> bool {
        input.consume_key(self.modifiers.to_egui(), self.key.to_egui_key())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalShortcut {
    Confirm,
    Cancel,
    Dismiss,
    SqlExecute,
    SqlExplain,
    SqlClear,
    SqlAutocompleteTrigger,
    SqlAutocompleteConfirm,
    SqlHistoryPrev,
    SqlHistoryNext,
    SqlHistoryBrowse,
    ImportRefresh,
    ImportFormatSql,
    ImportFormatCsv,
    ImportFormatTsv,
    ImportFormatJson,
    ImportCyclePrev,
    ImportCycleNext,
    SqliteBrowseFile,
    FormatSelectionCycle,
    HistoryClear,
}

impl LocalShortcut {
    pub fn bindings(self) -> Vec<LocalBinding> {
        match self {
            LocalShortcut::Confirm => vec![LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE)],
            LocalShortcut::Cancel => vec![LocalBinding::new(KeyCode::Escape, KeyModifiers::NONE)],
            LocalShortcut::Dismiss => vec![
                LocalBinding::new(KeyCode::Escape, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Q, KeyModifiers::NONE),
            ],
            LocalShortcut::SqlExecute => vec![
                LocalBinding::new(KeyCode::Enter, KeyModifiers::CTRL),
                LocalBinding::new(KeyCode::F5, KeyModifiers::NONE),
            ],
            LocalShortcut::SqlExplain => vec![LocalBinding::new(KeyCode::F6, KeyModifiers::NONE)],
            LocalShortcut::SqlClear => vec![LocalBinding::new(KeyCode::D, KeyModifiers::SHIFT)],
            LocalShortcut::SqlAutocompleteTrigger => vec![
                LocalBinding::new(KeyCode::Space, KeyModifiers::CTRL),
                LocalBinding::new(KeyCode::L, KeyModifiers::ALT),
            ],
            LocalShortcut::SqlAutocompleteConfirm => vec![
                LocalBinding::new(KeyCode::Tab, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Enter, KeyModifiers::NONE),
            ],
            LocalShortcut::SqlHistoryPrev => vec![
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::K, KeyModifiers::SHIFT),
            ],
            LocalShortcut::SqlHistoryNext => vec![
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::J, KeyModifiers::SHIFT),
            ],
            LocalShortcut::SqlHistoryBrowse => vec![
                LocalBinding::new(KeyCode::ArrowUp, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::ArrowDown, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::K, KeyModifiers::SHIFT),
                LocalBinding::new(KeyCode::J, KeyModifiers::SHIFT),
            ],
            LocalShortcut::ImportRefresh => {
                vec![LocalBinding::new(KeyCode::R, KeyModifiers::CTRL)]
            }
            LocalShortcut::ImportFormatSql => {
                vec![LocalBinding::new(KeyCode::Num1, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportFormatCsv => {
                vec![LocalBinding::new(KeyCode::Num2, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportFormatTsv => {
                vec![LocalBinding::new(KeyCode::Num3, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportFormatJson => {
                vec![LocalBinding::new(KeyCode::Num4, KeyModifiers::NONE)]
            }
            LocalShortcut::ImportCyclePrev => vec![
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
            ],
            LocalShortcut::ImportCycleNext => vec![
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::SqliteBrowseFile => {
                vec![LocalBinding::new(KeyCode::O, KeyModifiers::CTRL)]
            }
            LocalShortcut::FormatSelectionCycle => vec![
                LocalBinding::new(KeyCode::Num1, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Num2, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Num3, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::Num4, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::H, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::L, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowLeft, KeyModifiers::NONE),
                LocalBinding::new(KeyCode::ArrowRight, KeyModifiers::NONE),
            ],
            LocalShortcut::HistoryClear => {
                vec![LocalBinding::new(KeyCode::Delete, KeyModifiers::CTRL)]
            }
        }
    }
}

/// 仅使用给定快捷键列表生成提示。
pub fn shortcut_tooltip(label: &str, shortcuts: &[&str]) -> String {
    if shortcuts.is_empty() {
        return label.to_string();
    }

    format!("{}\n快捷键: {}", label, shortcuts.join(" / "))
}

/// 基于动作与当前绑定生成提示。
pub fn action_tooltip(keybindings: &KeyBindings, action: Action) -> String {
    action_tooltip_with_extras(keybindings, action, action.description(), &[])
}

/// 基于动作、标签与额外快捷键生成提示。
pub fn action_tooltip_with_extras(
    keybindings: &KeyBindings,
    action: Action,
    label: &str,
    extra_shortcuts: &[&str],
) -> String {
    let mut shortcuts: Vec<String> = extra_shortcuts.iter().map(|s| (*s).to_string()).collect();
    let action_shortcut = keybindings.display(action);
    if !action_shortcut.is_empty() && !shortcuts.iter().any(|item| item == &action_shortcut) {
        shortcuts.push(action_shortcut);
    }

    let mut lines = vec![label.to_string()];
    if action.description() != label {
        lines.push(format!("功能: {}", action.description()));
    }
    if !shortcuts.is_empty() {
        lines.push(format!("快捷键: {}", shortcuts.join(" / ")));
    }

    lines.join("\n")
}

pub fn local_shortcut_text(shortcut: LocalShortcut) -> String {
    local_bindings_text(shortcut.bindings())
}

pub fn local_shortcut_tooltip(label: &str, shortcut: LocalShortcut) -> String {
    let shortcuts = local_binding_strings(shortcut.bindings());
    let shortcut_refs: Vec<&str> = shortcuts.iter().map(String::as_str).collect();
    shortcut_tooltip(label, &shortcut_refs)
}

pub fn local_shortcuts_tooltip(label: &str, shortcuts: &[LocalShortcut]) -> String {
    let mut values: Vec<String> = Vec::new();
    for shortcut in shortcuts {
        for value in local_binding_strings(shortcut.bindings()) {
            if !values.iter().any(|item| item == &value) {
                values.push(value);
            }
        }
    }
    let refs: Vec<&str> = values.iter().map(String::as_str).collect();
    shortcut_tooltip(label, &refs)
}

pub fn local_shortcut_pressed(ctx: &egui::Context, shortcut: LocalShortcut) -> bool {
    shortcut
        .bindings()
        .into_iter()
        .any(|binding| binding.is_pressed(ctx))
}

pub fn consume_local_shortcut(input: &mut InputState, shortcut: LocalShortcut) -> bool {
    shortcut
        .bindings()
        .into_iter()
        .any(|binding| binding.consume(input))
}

fn local_binding_strings(bindings: Vec<LocalBinding>) -> Vec<String> {
    let mut values = Vec::new();
    for binding in bindings {
        let display = binding.display();
        if !values.iter().any(|item| item == &display) {
            values.push(display);
        }
    }
    values
}

fn local_bindings_text(bindings: Vec<LocalBinding>) -> String {
    local_binding_strings(bindings).join(" / ")
}

#[cfg(test)]
mod tests {
    use super::{
        LocalShortcut, action_tooltip_with_extras, local_shortcut_pressed, local_shortcut_text,
        local_shortcut_tooltip, local_shortcuts_tooltip, shortcut_tooltip,
    };
    use crate::core::{Action, KeyBindings};

    #[test]
    fn action_tooltip_appends_current_binding() {
        let keybindings = KeyBindings::default();
        let tooltip =
            action_tooltip_with_extras(&keybindings, Action::AddFilter, "打开筛选面板", &["/"]);

        assert!(tooltip.contains("打开筛选面板"));
        assert!(tooltip.contains("Ctrl+F"));
        assert!(tooltip.contains("/"));
    }

    #[test]
    fn shortcut_tooltip_keeps_local_shortcuts() {
        let tooltip = shortcut_tooltip("取消", &["Esc"]);

        assert_eq!(tooltip, "取消\n快捷键: Esc");
    }

    #[test]
    fn local_shortcut_text_formats_combined_shortcuts() {
        let text = local_shortcut_text(LocalShortcut::SqlExecute);

        assert_eq!(text, "Ctrl+Enter / F5");
    }

    #[test]
    fn local_shortcut_tooltip_uses_named_shortcut_set() {
        let tooltip = local_shortcut_tooltip("关闭对话框", LocalShortcut::Dismiss);

        assert_eq!(tooltip, "关闭对话框\n快捷键: Esc / Q");
    }

    #[test]
    fn local_shortcuts_tooltip_deduplicates_entries() {
        let tooltip = local_shortcuts_tooltip(
            "补全",
            &[
                LocalShortcut::SqlAutocompleteTrigger,
                LocalShortcut::SqlAutocompleteConfirm,
                LocalShortcut::Confirm,
            ],
        );

        assert_eq!(tooltip, "补全\n快捷键: Ctrl+Space / Alt+L / Tab / Enter");
    }

    #[test]
    fn local_shortcut_text_uses_actual_binding_order() {
        let text = local_shortcut_text(LocalShortcut::SqlHistoryBrowse);

        assert_eq!(text, "Shift+Up / Shift+Down / Shift+K / Shift+J");
    }

    #[test]
    fn local_shortcut_pressed_is_false_without_input() {
        let ctx = egui::Context::default();

        assert!(!local_shortcut_pressed(&ctx, LocalShortcut::SqlExecute));
    }
}
