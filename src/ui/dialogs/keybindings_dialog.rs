//! 快捷键编辑对话框
//!
//! 允许用户自定义全局快捷键与局部作用域快捷键。

use std::path::PathBuf;

use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStyle, DialogWindow,
};
use super::picker_shell::{PickerDialogShell, PickerNavAction, PickerPaneFocus};
use crate::core::{Action, KeyBinding, KeyBindings, KeyCode, KeyModifiers, KeymapDiagnosticCode};
use crate::ui::components::{
    GridCommandShortcut, GridSequenceConflictKind, grid_command_sequence_conflict,
    grid_command_shortcuts, normalize_grid_command_sequence,
};
use crate::ui::shortcut_tooltip::LocalShortcut;
use eframe::egui::{self, Key, RichText};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ScopeTreeSelection {
    #[default]
    Global,
    Scope(&'static str),
}

impl ScopeTreeSelection {
    fn helper_text(self) -> &'static str {
        match self {
            Self::Global => {
                "提示: 全局动作会影响整个工作区；next_focus_area / prev_focus_area 只是 fallback action，不是无条件 global-first 抢键。"
            }
            Self::Scope(scope) if is_text_entry_scope_path(scope) => {
                "提示: 这是文本输入作用域；这里只暴露不会和输入语义冲突的动作，普通字符命令会被拒绝并在诊断区提示。"
            }
            Self::Scope(_) => {
                "提示: 局部快捷键按 scope 生效；文本输入作用域会拒绝普通字符命令，诊断区会显示冲突和遮蔽提醒。"
            }
        }
    }

    fn breadcrumb(self) -> String {
        match self {
            Self::Global => "全局动作".to_string(),
            Self::Scope(scope) => format!("作用域 / {scope}"),
        }
    }

    fn matches(self, selection: BindingSelection) -> bool {
        self.scope_path() == selection.scope_path()
    }

    fn is_global(self) -> bool {
        matches!(self, Self::Global)
    }

    fn scope_path(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Scope(scope) => scope,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindingSelection {
    Global(Action),
    ScopedAction(&'static str, Action),
    Local(LocalShortcut),
    GridCommand(GridCommandShortcut),
}

impl BindingSelection {
    fn scope_path(self) -> &'static str {
        match self {
            Self::Global(_) => "global",
            Self::ScopedAction(scope_path, _) => scope_path,
            Self::Local(shortcut) => local_shortcut_scope_tags(shortcut)
                .first()
                .copied()
                .unwrap_or_else(|| {
                    shortcut
                        .config_key()
                        .rsplit_once('.')
                        .map(|(scope, _)| scope)
                        .unwrap_or(shortcut.config_key())
                }),
            Self::GridCommand(_) => "grid.normal",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Global(action) => action.description(),
            Self::ScopedAction(_, action) => action.description(),
            Self::Local(shortcut) => local_shortcut_description(shortcut),
            Self::GridCommand(command) => grid_command_description(command),
        }
    }

    fn category(self) -> &'static str {
        match self {
            Self::Global(action) => action.category(),
            Self::ScopedAction(_, _) => "作用域动作",
            Self::Local(shortcut) => local_shortcut_category(shortcut),
            Self::GridCommand(command) => grid_command_category(command),
        }
    }

    fn detail(self) -> String {
        match self {
            Self::Global(action) => action.keymap_name().to_string(),
            Self::ScopedAction(scope_path, action) => {
                format!("{}.{}", scope_path, action.keymap_name())
            }
            Self::Local(shortcut) => shortcut.config_key().to_string(),
            Self::GridCommand(command) => command.config_key().to_string(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum BindingIssue {
    GlobalShadow {
        binding: KeyBinding,
        action: Action,
    },
    LocalConflict {
        binding: KeyBinding,
        shortcut: LocalShortcut,
    },
    GridConflict {
        sequence: String,
        conflicting_sequence: String,
        command: GridCommandShortcut,
        kind: GridSequenceConflictKind,
    },
}

impl BindingIssue {
    fn message(&self) -> String {
        match self {
            Self::GlobalShadow { binding, action } => format!(
                "{} 会遮蔽全局动作“{}”",
                binding.display(),
                action.description()
            ),
            Self::LocalConflict { binding, shortcut } => format!(
                "{} 与同作用域快捷键“{}”重叠",
                binding.display(),
                local_shortcut_description(*shortcut)
            ),
            Self::GridConflict {
                sequence,
                conflicting_sequence,
                command,
                kind,
            } => match kind {
                GridSequenceConflictKind::Exact => format!(
                    "{} 与表格命令“{}”的序列 {} 完全重叠",
                    sequence,
                    grid_command_description(*command),
                    conflicting_sequence
                ),
                GridSequenceConflictKind::Prefix => format!(
                    "{} 与表格命令“{}”的序列 {} 存在前缀重叠，可能互相吞键",
                    sequence,
                    grid_command_description(*command),
                    conflicting_sequence
                ),
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum RecordingMode {
    #[default]
    Replace,
    Append,
}

impl RecordingMode {}

const WORKSPACE_SCOPE_ACTION_CANDIDATES: &[Action] = &[
    Action::ShowHelp,
    Action::CommandPalette,
    Action::NewConnection,
    Action::NewTable,
    Action::NewDatabase,
    Action::NewUser,
    Action::Export,
    Action::Import,
    Action::ShowHistory,
    Action::ToggleErDiagram,
    Action::Refresh,
    Action::ClearCommandLine,
    Action::ToggleEditor,
    Action::ToggleSidebar,
    Action::ClearSearch,
    Action::Save,
    Action::GotoLine,
    Action::NewTab,
    Action::NextTab,
    Action::PrevTab,
    Action::CloseTab,
    Action::OpenThemeSelector,
    Action::OpenKeybindingsDialog,
    Action::ToggleDarkMode,
    Action::FocusSidebarConnections,
    Action::FocusSidebarDatabases,
    Action::FocusSidebarTables,
    Action::FocusSidebarFilters,
    Action::FocusSidebarTriggers,
    Action::FocusSidebarRoutines,
];

const EDITOR_INSERT_SCOPE_ACTION_CANDIDATES: &[Action] = &[
    Action::ShowHelp,
    Action::ClearCommandLine,
    Action::ToggleEditor,
];

const FILTERS_INPUT_SCOPE_ACTION_CANDIDATES: &[Action] = &[Action::ShowHelp];

fn is_text_entry_scope_path(scope_path: &str) -> bool {
    matches!(scope_path, "editor.insert" | "sidebar.filters.input")
}

fn scope_action_candidates(scope_path: &str) -> &'static [Action] {
    match scope_path {
        "toolbar"
        | "query_tabs"
        | "sidebar.connections"
        | "sidebar.databases"
        | "sidebar.tables"
        | "sidebar.filters.list"
        | "sidebar.triggers"
        | "sidebar.routines"
        | "grid.normal"
        | "grid.select"
        | "grid.insert"
        | "editor.normal" => WORKSPACE_SCOPE_ACTION_CANDIDATES,
        "editor.insert" => EDITOR_INSERT_SCOPE_ACTION_CANDIDATES,
        "sidebar.filters.input" => FILTERS_INPUT_SCOPE_ACTION_CANDIDATES,
        _ => &[],
    }
}

fn scope_supports_action_overrides(scope_path: &str) -> bool {
    !scope_action_candidates(scope_path).is_empty()
}

/// 快捷键编辑对话框状态
#[derive(Clone, Default)]
pub struct KeyBindingsDialogState {
    /// 是否显示对话框
    pub show: bool,
    /// 当前快捷键绑定（编辑中的副本）
    bindings: KeyBindings,
    /// 当前选中的条目
    selected_binding: Option<BindingSelection>,
    /// 当前作用域树筛选
    current_tree: ScopeTreeSelection,
    /// 当前 picker 焦点
    pane_focus: PickerPaneFocus,
    /// 旧版 config.toml 内联快捷键的兼容副本，仅用于显式迁移。
    legacy_bindings: Option<KeyBindings>,
    /// 当前 keymap.toml 路径，用于 UI 展示和复制。
    keymap_path: Option<PathBuf>,
    /// 是否正在录制快捷键
    recording: bool,
    /// 录制的按键
    recorded_key: Option<KeyCode>,
    /// 录制的修饰键
    recorded_modifiers: KeyModifiers,
    /// 当前录制模式
    recording_mode: RecordingMode,
    /// 搜索过滤
    filter: String,
    /// 表格命令序列输入
    sequence_input: String,
    /// 是否仅显示带作用域提醒的局部快捷键
    show_issue_only: bool,
    /// 是否有未保存的更改
    has_changes: bool,
    /// 冲突提示
    conflict_message: Option<String>,
}

impl KeyBindingsDialogState {
    /// 打开对话框
    pub fn open(&mut self, bindings: &KeyBindings) {
        self.open_with_legacy(bindings, &KeyBindings::default());
    }

    /// 打开对话框，并保留旧版 config.toml 快捷键作为显式迁移来源。
    pub fn open_with_legacy(&mut self, bindings: &KeyBindings, legacy: &KeyBindings) {
        self.show = true;
        self.bindings = bindings.clone();
        self.selected_binding = None;
        self.current_tree = ScopeTreeSelection::Global;
        self.pane_focus = PickerPaneFocus::Navigator;
        self.legacy_bindings = legacy.has_customizations().then(|| legacy.clone());
        self.keymap_path = KeyBindings::keymap_path();
        self.recording = false;
        self.recorded_key = None;
        self.recorded_modifiers = KeyModifiers::NONE;
        self.recording_mode = RecordingMode::Replace;
        self.filter.clear();
        self.sequence_input.clear();
        self.show_issue_only = false;
        self.has_changes = false;
        self.conflict_message = None;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.show = false;
        self.recording = false;
        self.recording_mode = RecordingMode::Replace;
        self.pane_focus = PickerPaneFocus::Navigator;
    }

    /// 重置为默认快捷键
    pub fn reset_to_defaults(&mut self) {
        self.bindings = KeyBindings::default();
        self.has_changes = true;
        self.conflict_message = None;
        self.recording = false;
        self.recording_mode = RecordingMode::Replace;
    }

    /// 获取编辑后的快捷键绑定
    #[allow(dead_code)]
    pub fn get_bindings(&self) -> &KeyBindings {
        &self.bindings
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    fn migration_notice(&self) -> Option<&str> {
        self.bindings
            .diagnostics()
            .iter()
            .find(|diagnostic| {
                diagnostic.code == KeymapDiagnosticCode::LegacyConfigMigrationPending
            })
            .map(|diagnostic| diagnostic.message.as_str())
    }

    fn import_legacy_bindings(&mut self) {
        let Some(legacy) = &self.legacy_bindings else {
            return;
        };

        self.bindings = legacy.clone();
        self.has_changes = true;
        self.recording = false;
        self.recording_mode = RecordingMode::Replace;
        self.conflict_message =
            Some("已导入旧版 config.toml 快捷键；点击保存后会写入 keymap.toml。".to_string());
    }

    /// 消费录制态键盘输入。
    ///
    /// 返回 true 表示本次输入被录制逻辑消费（取消录制或写入了绑定）。
    pub fn consume_recording_input(&mut self, input: &egui::InputState) -> bool {
        if !self.recording {
            return false;
        }

        let Some(selection) = self.selected_binding else {
            self.recording = false;
            self.recorded_key = None;
            self.recorded_modifiers = KeyModifiers::NONE;
            return false;
        };

        if input.key_pressed(Key::Escape) {
            self.recording = false;
            self.recorded_key = None;
            self.recorded_modifiers = KeyModifiers::NONE;
            return true;
        }

        self.recorded_modifiers = KeyModifiers::from_egui(input.modifiers);
        for event in &input.events {
            if let egui::Event::Key {
                key, pressed: true, ..
            } = event
                && let Some(key_code) = KeyCode::from_egui_key(*key)
            {
                self.recorded_key = Some(key_code);
                let binding = KeyBinding::new(key_code, self.recorded_modifiers);
                self.apply_recorded_binding(selection, binding);
                self.recording = false;
                return true;
            }
        }

        false
    }

    fn visible_bindings(&self) -> Vec<BindingSelection> {
        let filter_lower = self.filter.to_lowercase();
        let mut result = Vec::new();

        if self.current_tree.is_global() {
            for action in Action::all() {
                let selection = BindingSelection::Global(*action);
                if self.matches_filter_for_tree(selection, &filter_lower, self.current_tree)
                    && self.matches_issue_filter(selection)
                {
                    result.push(selection);
                }
            }
        } else if let ScopeTreeSelection::Scope(scope_path) = self.current_tree
            && scope_supports_action_overrides(scope_path)
        {
            for action in scope_action_candidates(scope_path) {
                let selection = BindingSelection::ScopedAction(scope_path, *action);
                if self.matches_filter_for_tree(selection, &filter_lower, self.current_tree)
                    && self.matches_issue_filter(selection)
                {
                    result.push(selection);
                }
            }
        }

        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if self.matches_filter_for_tree(selection, &filter_lower, self.current_tree)
                && self.matches_issue_filter(selection)
            {
                result.push(selection);
            }
        }

        for command in GridCommandShortcut::all() {
            let selection = BindingSelection::GridCommand(*command);
            if self.matches_filter_for_tree(selection, &filter_lower, self.current_tree)
                && self.matches_issue_filter(selection)
            {
                result.push(selection);
            }
        }

        result
    }

    fn matches_search(&self, selection: BindingSelection, filter_lower: &str) -> bool {
        if !filter_lower.is_empty() {
            let label = selection.label().to_lowercase();
            let detail = selection.detail().to_lowercase();
            if !label.contains(filter_lower) && !detail.contains(filter_lower) {
                return false;
            }
        }

        true
    }

    fn matches_filter_for_tree(
        &self,
        selection: BindingSelection,
        filter_lower: &str,
        tree: ScopeTreeSelection,
    ) -> bool {
        self.matches_search(selection, filter_lower) && tree.matches(selection)
    }

    fn matches_issue_filter(&self, selection: BindingSelection) -> bool {
        if !self.show_issue_only {
            return true;
        }

        !self.selection_issue_messages(selection).is_empty()
    }

    fn select_binding(&mut self, selection: BindingSelection) {
        self.selected_binding = Some(selection);
        self.recording = false;
        self.conflict_message = None;
        self.sequence_input.clear();
    }

    fn begin_recording(&mut self, selection: BindingSelection, mode: RecordingMode) {
        self.selected_binding = Some(selection);
        self.recording = true;
        self.recorded_key = None;
        self.recorded_modifiers = KeyModifiers::NONE;
        self.recording_mode = mode;
        self.conflict_message = None;
    }

    fn select_tree(&mut self, tree: ScopeTreeSelection) {
        if self.current_tree == tree {
            return;
        }
        self.current_tree = tree;
        if tree.is_global() {
            self.show_issue_only = false;
        }
        self.selected_binding = None;
        self.recording = false;
        self.recording_mode = RecordingMode::Replace;
        self.conflict_message = None;
        self.sequence_input.clear();
        self.selected_binding = self.visible_bindings().into_iter().next();
    }

    fn set_show_issue_only(&mut self, enabled: bool) {
        self.show_issue_only = enabled && !self.current_tree.is_global();
        if let Some(selection) = self.selected_binding
            && !self.visible_bindings().contains(&selection)
        {
            self.selected_binding = None;
        }
    }

    fn binding_text(&self, selection: BindingSelection) -> String {
        match selection {
            BindingSelection::Global(action) => self
                .bindings
                .get(action)
                .map(KeyBinding::display)
                .unwrap_or_else(|| "未设置".to_string()),
            BindingSelection::ScopedAction(scope_path, action) => self
                .scoped_action_bindings(scope_path, action)
                .map(|bindings| keybinding_list_text(&bindings))
                .unwrap_or_else(|| "未设置".to_string()),
            BindingSelection::Local(shortcut) => {
                let bindings = shortcut.bindings_for(&self.bindings);
                keybinding_list_text(&bindings)
            }
            BindingSelection::GridCommand(command) => {
                keybinding_list_text_strings(&self.grid_sequences(command))
            }
        }
    }

    fn binding_source(&self, selection: BindingSelection) -> &'static str {
        match selection {
            BindingSelection::Global(action) => {
                let defaults = KeyBindings::default();
                match (defaults.get(action), self.bindings.get(action)) {
                    (_, None) => "已清除",
                    (Some(default), Some(current)) if default == current => "默认",
                    _ => "自定义",
                }
            }
            BindingSelection::ScopedAction(scope_path, action) => {
                if self
                    .bindings
                    .scoped_bindings_for_action(scope_path, action)
                    .is_some()
                {
                    "局部覆盖"
                } else if self.bindings.get(action).is_some() {
                    "继承全局"
                } else {
                    "未设置"
                }
            }
            BindingSelection::Local(shortcut) => {
                if shortcut.is_overridden(&self.bindings) {
                    "自定义"
                } else {
                    "默认"
                }
            }
            BindingSelection::GridCommand(command) => {
                if self
                    .bindings
                    .local_sequences_for(command.config_key())
                    .is_some()
                {
                    "自定义"
                } else {
                    "默认"
                }
            }
        }
    }

    fn check_conflict(&self, action: Action, binding: &KeyBinding) -> Option<Action> {
        for candidate in Action::all() {
            if *candidate != action
                && let Some(existing) = self.bindings.get(*candidate)
                && existing == binding
            {
                return Some(*candidate);
            }
        }
        None
    }

    fn local_bindings(&self, shortcut: LocalShortcut) -> Vec<KeyBinding> {
        shortcut.bindings_for(&self.bindings)
    }

    fn scoped_action_bindings(
        &self,
        scope_path: &'static str,
        action: Action,
    ) -> Option<Vec<KeyBinding>> {
        if let Some(bindings) = self.bindings.scoped_bindings_for_action(scope_path, action) {
            Some(bindings.to_vec())
        } else {
            self.bindings
                .get(action)
                .cloned()
                .map(|binding| vec![binding])
        }
    }

    fn grid_sequences(&self, command: GridCommandShortcut) -> Vec<String> {
        grid_command_shortcuts(&self.bindings, command)
    }

    fn scoped_action_runtime_issues(
        &self,
        scope_path: &'static str,
        action: Action,
    ) -> Vec<String> {
        let mut issues = Vec::new();
        let Some(bindings) = self.scoped_action_bindings(scope_path, action) else {
            return issues;
        };

        for binding in &bindings {
            for other in scope_action_candidates(scope_path) {
                if *other == action {
                    continue;
                }

                let Some(other_bindings) = self.scoped_action_bindings(scope_path, *other) else {
                    continue;
                };
                if other_bindings.iter().any(|candidate| candidate == binding) {
                    let message = format!(
                        "{} 与同作用域动作“{}”在 {} 中重复使用 {}。",
                        action.description(),
                        other.description(),
                        scope_path,
                        binding.display()
                    );
                    if !issues.iter().any(|existing| existing == &message) {
                        issues.push(message);
                    }
                }
            }

            for shortcut in LocalShortcut::all() {
                if BindingSelection::Local(*shortcut).scope_path() != scope_path {
                    continue;
                }
                if self
                    .local_bindings(*shortcut)
                    .iter()
                    .any(|candidate| candidate == binding)
                {
                    let message = format!(
                        "{} 与同作用域命令“{}”在 {} 中重复使用 {}。",
                        action.description(),
                        local_shortcut_description(*shortcut),
                        scope_path,
                        binding.display()
                    );
                    if !issues.iter().any(|existing| existing == &message) {
                        issues.push(message);
                    }
                }
            }
        }

        issues
    }

    fn set_effective_scoped_action_bindings(
        &mut self,
        scope_path: &'static str,
        action: Action,
        bindings: Vec<KeyBinding>,
    ) {
        let mut unique = Vec::new();
        for binding in bindings {
            if !unique.iter().any(|existing| existing == &binding) {
                unique.push(binding);
            }
        }

        let key = format!("{}.{}", scope_path, action.keymap_name());
        let inherited = self
            .bindings
            .get(action)
            .cloned()
            .map(|binding| vec![binding])
            .unwrap_or_default();
        if unique.is_empty() || unique == inherited {
            self.bindings.remove_local_bindings(&key);
        } else {
            self.bindings.set_local_bindings(key, unique);
        }
    }

    fn set_effective_local_bindings(&mut self, shortcut: LocalShortcut, bindings: Vec<KeyBinding>) {
        let mut unique = Vec::new();
        for binding in bindings {
            if !unique.iter().any(|existing| existing == &binding) {
                unique.push(binding);
            }
        }

        let defaults = shortcut.default_keybindings();
        if unique == defaults || unique.is_empty() {
            self.bindings.remove_local_bindings(shortcut.config_key());
        } else {
            self.bindings
                .set_local_bindings(shortcut.config_key(), unique);
        }
    }

    fn set_effective_grid_sequences(
        &mut self,
        command: GridCommandShortcut,
        sequences: Vec<String>,
    ) {
        let mut unique = Vec::new();
        for sequence in sequences {
            if !unique.iter().any(|existing| existing == &sequence) {
                unique.push(sequence);
            }
        }

        let defaults: Vec<String> = command
            .default_sequences()
            .iter()
            .map(|sequence| (*sequence).to_string())
            .collect();

        if unique.is_empty() || unique == defaults {
            self.bindings.remove_local_sequences(command.config_key());
        } else {
            self.bindings
                .set_local_sequences(command.config_key(), unique);
        }
    }

    fn apply_recorded_binding(&mut self, selection: BindingSelection, binding: KeyBinding) {
        match selection {
            BindingSelection::Global(action) => {
                if let Some(conflict_action) = self.check_conflict(action, &binding) {
                    self.conflict_message = Some(format!(
                        "快捷键 {} 已被“{}”使用",
                        binding.display(),
                        conflict_action.description()
                    ));
                    return;
                }
                self.bindings.set(action, binding);
                self.has_changes = true;
                self.conflict_message = None;
            }
            BindingSelection::ScopedAction(scope_path, action) => {
                let mut bindings = self
                    .scoped_action_bindings(scope_path, action)
                    .unwrap_or_default();
                match self.recording_mode {
                    RecordingMode::Replace => bindings = vec![binding.clone()],
                    RecordingMode::Append => {
                        if bindings.iter().any(|existing| existing == &binding) {
                            self.conflict_message = Some(format!(
                                "“{}”已经包含 {}，无需重复追加。",
                                action.description(),
                                binding.display()
                            ));
                            return;
                        }
                        bindings.push(binding.clone());
                    }
                }

                self.set_effective_scoped_action_bindings(scope_path, action, bindings);
                self.has_changes = true;
                self.conflict_message = Some(match self.recording_mode {
                    RecordingMode::Replace => format!(
                        "已将 {} 在 {} 中替换为 {}。",
                        action.description(),
                        scope_path,
                        binding.display()
                    ),
                    RecordingMode::Append => format!(
                        "已为 {} 在 {} 中追加 {}。",
                        action.description(),
                        scope_path,
                        binding.display()
                    ),
                });
            }
            BindingSelection::Local(shortcut) => {
                let mut bindings = self.local_bindings(shortcut);
                match self.recording_mode {
                    RecordingMode::Replace => bindings = vec![binding.clone()],
                    RecordingMode::Append => {
                        if bindings.iter().any(|existing| existing == &binding) {
                            self.conflict_message = Some(format!(
                                "“{}”已经包含 {}，无需重复追加。",
                                local_shortcut_description(shortcut),
                                binding.display()
                            ));
                            return;
                        }
                        bindings.push(binding.clone());
                    }
                }

                self.set_effective_local_bindings(shortcut, bindings);
                self.has_changes = true;
                self.conflict_message = Some(match self.recording_mode {
                    RecordingMode::Replace => format!(
                        "已将“{}”替换为 {}。如需恢复多键默认方案，请点“恢复默认”。",
                        local_shortcut_description(shortcut),
                        binding.display()
                    ),
                    RecordingMode::Append => format!(
                        "已为“{}”追加 {}。当前可以保留多组绑定。",
                        local_shortcut_description(shortcut),
                        binding.display()
                    ),
                });
            }
            BindingSelection::GridCommand(_) => {}
        }
    }

    fn apply_grid_sequence_input(&mut self, command: GridCommandShortcut, mode: RecordingMode) {
        let Some(sequence) = normalize_grid_command_sequence(&self.sequence_input) else {
            self.conflict_message =
                Some("无效的表格命令序列。示例: yy、:w、Space+d、Ctrl+S".to_string());
            return;
        };

        let mut sequences = self.grid_sequences(command);
        match mode {
            RecordingMode::Replace => sequences = vec![sequence.clone()],
            RecordingMode::Append => {
                if sequences.iter().any(|existing| existing == &sequence) {
                    self.conflict_message = Some(format!(
                        "“{}”已经包含 {}，无需重复追加。",
                        grid_command_description(command),
                        sequence
                    ));
                    return;
                }
                sequences.push(sequence.clone());
            }
        }

        self.set_effective_grid_sequences(command, sequences);
        self.sequence_input.clear();
        self.has_changes = true;
        self.conflict_message = Some(match mode {
            RecordingMode::Replace => format!(
                "已将“{}”替换为 {}。",
                grid_command_description(command),
                sequence
            ),
            RecordingMode::Append => format!(
                "已为“{}”追加 {}。",
                grid_command_description(command),
                sequence
            ),
        });
    }

    fn clear_selected_binding(&mut self) {
        let Some(selection) = self.selected_binding else {
            return;
        };

        match selection {
            BindingSelection::Global(action) => {
                self.bindings.remove(action);
                self.has_changes = true;
                self.conflict_message = None;
            }
            BindingSelection::ScopedAction(scope_path, action) => {
                self.bindings.remove_local_bindings(&format!(
                    "{}.{}",
                    scope_path,
                    action.keymap_name()
                ));
                self.has_changes = true;
                self.conflict_message = None;
            }
            BindingSelection::Local(shortcut) => {
                self.bindings.remove_local_bindings(shortcut.config_key());
                self.has_changes = true;
                self.conflict_message = None;
            }
            BindingSelection::GridCommand(command) => {
                self.bindings.remove_local_sequences(command.config_key());
                self.has_changes = true;
                self.conflict_message = None;
            }
        }
    }

    fn remove_local_binding_at(&mut self, shortcut: LocalShortcut, index: usize) {
        let mut bindings = self.local_bindings(shortcut);
        if bindings.len() <= 1 {
            self.conflict_message =
                Some("局部快捷键至少保留一个绑定；如需回到默认方案，请点“恢复默认”。".to_string());
            return;
        }
        if index >= bindings.len() {
            return;
        }

        let removed = bindings.remove(index);
        self.set_effective_local_bindings(shortcut, bindings);
        self.has_changes = true;
        self.conflict_message = Some(format!(
            "已移除“{}”的绑定 {}。",
            local_shortcut_description(shortcut),
            removed.display()
        ));
    }

    fn remove_scoped_action_binding_at(
        &mut self,
        scope_path: &'static str,
        action: Action,
        index: usize,
    ) {
        let mut bindings = self
            .scoped_action_bindings(scope_path, action)
            .unwrap_or_default();
        if bindings.len() <= 1 {
            self.conflict_message =
                Some("作用域动作至少保留一个绑定；如需回到全局方案，请点“恢复全局”。".to_string());
            return;
        }
        if index >= bindings.len() {
            return;
        }

        let removed = bindings.remove(index);
        self.set_effective_scoped_action_bindings(scope_path, action, bindings);
        self.has_changes = true;
        self.conflict_message = Some(format!(
            "已移除 {} 在 {} 中的绑定 {}。",
            action.description(),
            scope_path,
            removed.display()
        ));
    }

    fn remove_grid_sequence_at(&mut self, command: GridCommandShortcut, index: usize) {
        let mut sequences = self.grid_sequences(command);
        if sequences.len() <= 1 {
            self.conflict_message =
                Some("表格命令至少保留一个序列；如需回到默认方案，请点“恢复默认”。".to_string());
            return;
        }
        if index >= sequences.len() {
            return;
        }

        let removed = sequences.remove(index);
        self.set_effective_grid_sequences(command, sequences);
        self.has_changes = true;
        self.conflict_message = Some(format!(
            "已移除“{}”的序列 {}。",
            grid_command_description(command),
            removed
        ));
    }

    fn clear_button_label(&self) -> Option<&'static str> {
        match self.selected_binding {
            Some(BindingSelection::Global(_)) => Some("清除快捷键"),
            Some(BindingSelection::ScopedAction(_, _)) => Some("恢复全局"),
            Some(BindingSelection::Local(_) | BindingSelection::GridCommand(_)) => Some("恢复默认"),
            None => None,
        }
    }

    fn binding_issues(&self, selection: BindingSelection) -> Vec<BindingIssue> {
        match selection {
            BindingSelection::Global(_) => Vec::new(),
            BindingSelection::ScopedAction(_, _) => Vec::new(),
            BindingSelection::Local(shortcut) => {
                let mut issues = Vec::new();
                let bindings = self.local_bindings(shortcut);

                for binding in &bindings {
                    for action in Action::all() {
                        if let Some(global_binding) = self.bindings.get(*action)
                            && global_binding == binding
                        {
                            issues.push(BindingIssue::GlobalShadow {
                                binding: binding.clone(),
                                action: *action,
                            });
                        }
                    }

                    for other in LocalShortcut::all() {
                        if *other == shortcut || !local_shortcuts_overlap(shortcut, *other) {
                            continue;
                        }

                        let other_bindings = self.local_bindings(*other);
                        if other_bindings.iter().any(|existing| existing == binding) {
                            issues.push(BindingIssue::LocalConflict {
                                binding: binding.clone(),
                                shortcut: *other,
                            });
                        }
                    }
                }

                let mut deduped = Vec::new();
                for issue in issues {
                    if !deduped.iter().any(|existing| existing == &issue) {
                        deduped.push(issue);
                    }
                }
                deduped
            }
            BindingSelection::GridCommand(command) => {
                let mut issues = Vec::new();
                let sequences = self.grid_sequences(command);

                for sequence in &sequences {
                    for other in GridCommandShortcut::all() {
                        if *other == command {
                            continue;
                        }

                        let other_sequences = self.grid_sequences(*other);
                        for other_sequence in other_sequences {
                            if let Some(kind) =
                                grid_command_sequence_conflict(sequence, &other_sequence)
                            {
                                issues.push(BindingIssue::GridConflict {
                                    sequence: sequence.clone(),
                                    conflicting_sequence: other_sequence,
                                    command: *other,
                                    kind,
                                });
                            }
                        }
                    }
                }

                let mut deduped = Vec::new();
                for issue in issues {
                    if !deduped.iter().any(|existing| existing == &issue) {
                        deduped.push(issue);
                    }
                }
                deduped
            }
        }
    }

    fn selection_issue_messages(&self, selection: BindingSelection) -> Vec<String> {
        let mut messages = Vec::new();

        match selection {
            BindingSelection::Global(action) => {
                for diagnostic in self.bindings.diagnostics_for_action(action) {
                    messages.push(diagnostic.message.clone());
                }
            }
            BindingSelection::ScopedAction(scope_path, action) => {
                let path = format!("{}.{}", scope_path, action.keymap_name());
                for diagnostic in self.bindings.diagnostics_for_binding_path(&path) {
                    messages.push(diagnostic.message.clone());
                }
                for issue in self.scoped_action_runtime_issues(scope_path, action) {
                    if !messages.iter().any(|existing| existing == &issue) {
                        messages.push(issue);
                    }
                }
            }
            BindingSelection::Local(shortcut) => {
                for diagnostic in self.bindings.diagnostics_for_command(shortcut.config_key()) {
                    messages.push(diagnostic.message.clone());
                }
            }
            BindingSelection::GridCommand(_) => {}
        }

        for issue in self.binding_issues(selection) {
            let message = issue.message();
            if !messages.iter().any(|existing| existing == &message) {
                messages.push(message);
            }
        }

        messages
    }

    fn recording_hint(&self) -> &'static str {
        match (self.selected_binding, self.recording_mode) {
            (Some(BindingSelection::Local(_)), RecordingMode::Append) => {
                "录制中：按下一个按键将追加到当前局部快捷键组，按 Esc 取消。"
            }
            (Some(BindingSelection::Local(_)), RecordingMode::Replace) => {
                "录制中：按下一个按键将替换当前局部快捷键组，按 Esc 取消。"
            }
            _ => "录制中：按下一个按键将替换当前快捷键，按 Esc 取消。",
        }
    }

    fn filtered_count(&self, tree: ScopeTreeSelection) -> usize {
        let filter_lower = self.filter.to_lowercase();
        let mut count = 0;
        if tree.is_global() {
            for action in Action::all() {
                let selection = BindingSelection::Global(*action);
                if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                    count += 1;
                }
            }
        } else if let ScopeTreeSelection::Scope(scope_path) = tree
            && scope_supports_action_overrides(scope_path)
        {
            for action in scope_action_candidates(scope_path) {
                let selection = BindingSelection::ScopedAction(scope_path, *action);
                if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                    count += 1;
                }
            }
        }
        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                count += 1;
            }
        }
        for command in GridCommandShortcut::all() {
            let selection = BindingSelection::GridCommand(*command);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                count += 1;
            }
        }
        count
    }

    fn issue_count(&self, tree: ScopeTreeSelection) -> usize {
        let filter_lower = self.filter.to_lowercase();
        let mut total = 0;
        if tree.is_global() {
            for action in Action::all() {
                let selection = BindingSelection::Global(*action);
                if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                    total += self.selection_issue_messages(selection).len();
                }
            }
        } else if let ScopeTreeSelection::Scope(scope_path) = tree
            && scope_supports_action_overrides(scope_path)
        {
            for action in scope_action_candidates(scope_path) {
                let selection = BindingSelection::ScopedAction(scope_path, *action);
                if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                    total += self.selection_issue_messages(selection).len();
                }
            }
        }
        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                total += self.selection_issue_messages(selection).len();
            }
        }
        for command in GridCommandShortcut::all() {
            let selection = BindingSelection::GridCommand(*command);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                total += self.selection_issue_messages(selection).len();
            }
        }
        total
    }

    fn issue_summary(&self, tree: ScopeTreeSelection) -> Vec<(BindingSelection, Vec<String>)> {
        let filter_lower = self.filter.to_lowercase();
        let mut summary = Vec::new();

        if tree.is_global() {
            for action in Action::all() {
                let selection = BindingSelection::Global(*action);
                if !self.matches_filter_for_tree(selection, &filter_lower, tree) {
                    continue;
                }

                let issues = self.selection_issue_messages(selection);
                if !issues.is_empty() {
                    summary.push((selection, issues));
                }
            }
        } else if let ScopeTreeSelection::Scope(scope_path) = tree
            && scope_supports_action_overrides(scope_path)
        {
            for action in scope_action_candidates(scope_path) {
                let selection = BindingSelection::ScopedAction(scope_path, *action);
                if !self.matches_filter_for_tree(selection, &filter_lower, tree) {
                    continue;
                }

                let issues = self.selection_issue_messages(selection);
                if !issues.is_empty() {
                    summary.push((selection, issues));
                }
            }
        }

        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if !self.matches_filter_for_tree(selection, &filter_lower, tree) {
                continue;
            }

            let issues = self.selection_issue_messages(selection);
            if !issues.is_empty() {
                summary.push((selection, issues));
            }
        }

        for command in GridCommandShortcut::all() {
            let selection = BindingSelection::GridCommand(*command);
            if !self.matches_filter_for_tree(selection, &filter_lower, tree) {
                continue;
            }

            let issues = self.selection_issue_messages(selection);
            if !issues.is_empty() {
                summary.push((selection, issues));
            }
        }

        summary
    }

    fn cycle_focus_next(&mut self) {
        self.pane_focus = PickerDialogShell::next_focus(self.pane_focus);
    }

    fn cycle_focus_prev(&mut self) {
        self.pane_focus = PickerDialogShell::prev_focus(self.pane_focus);
    }
}

fn navigator_entries() -> Vec<ScopeTreeSelection> {
    scope_tree_entries()
        .iter()
        .map(|entry| entry.selection)
        .collect()
}

#[derive(Clone, Copy)]
struct ScopeTreeEntry {
    section: &'static str,
    selection: ScopeTreeSelection,
    title: &'static str,
}

fn scope_tree_entries() -> &'static [ScopeTreeEntry] {
    &[
        ScopeTreeEntry {
            section: "全局",
            selection: ScopeTreeSelection::Global,
            title: "global",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.common"),
            title: "dialog.common",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.confirm"),
            title: "dialog.confirm",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.help"),
            title: "dialog.help",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.picker"),
            title: "dialog.picker",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.export"),
            title: "dialog.export",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.import"),
            title: "dialog.import",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.connection"),
            title: "dialog.connection",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.ddl"),
            title: "dialog.ddl",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.history"),
            title: "dialog.history",
        },
        ScopeTreeEntry {
            section: "对话框",
            selection: ScopeTreeSelection::Scope("dialog.command_palette"),
            title: "dialog.command_palette",
        },
        ScopeTreeEntry {
            section: "工作区动作",
            selection: ScopeTreeSelection::Scope("toolbar"),
            title: "toolbar",
        },
        ScopeTreeEntry {
            section: "工作区动作",
            selection: ScopeTreeSelection::Scope("er_diagram"),
            title: "er_diagram",
        },
        ScopeTreeEntry {
            section: "工作区动作",
            selection: ScopeTreeSelection::Scope("query_tabs"),
            title: "query_tabs",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.list"),
            title: "sidebar.list",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.connections"),
            title: "sidebar.connections",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.databases"),
            title: "sidebar.databases",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.tables"),
            title: "sidebar.tables",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.filters.list"),
            title: "sidebar.filters.list",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.filters.input"),
            title: "sidebar.filters.input",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.triggers"),
            title: "sidebar.triggers",
        },
        ScopeTreeEntry {
            section: "侧边栏",
            selection: ScopeTreeSelection::Scope("sidebar.routines"),
            title: "sidebar.routines",
        },
        ScopeTreeEntry {
            section: "编辑器",
            selection: ScopeTreeSelection::Scope("editor.normal"),
            title: "editor.normal",
        },
        ScopeTreeEntry {
            section: "编辑器",
            selection: ScopeTreeSelection::Scope("editor.insert"),
            title: "editor.insert",
        },
        ScopeTreeEntry {
            section: "表格",
            selection: ScopeTreeSelection::Scope("grid.normal"),
            title: "grid.normal",
        },
        ScopeTreeEntry {
            section: "表格",
            selection: ScopeTreeSelection::Scope("grid.select"),
            title: "grid.select",
        },
        ScopeTreeEntry {
            section: "表格",
            selection: ScopeTreeSelection::Scope("grid.insert"),
            title: "grid.insert",
        },
    ]
}

fn navigator_title(entry: ScopeTreeSelection) -> &'static str {
    scope_tree_entries()
        .iter()
        .find(|candidate| candidate.selection == entry)
        .map(|entry| entry.title)
        .unwrap_or("unknown")
}

/// 快捷键编辑对话框
pub struct KeyBindingsDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyBindingsDialogShortcutAction {
    CloseDialog,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KeyBindingsDialogFrameAction {
    Close,
    SaveAndClose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyBindingsDialogUiAction {
    ResetToDefaults,
    ImportLegacyBindings,
    SetFilter(String),
    SetSequenceInput(String),
    SelectTree(ScopeTreeSelection),
    SetShowIssueOnly(bool),
    SelectBinding(BindingSelection),
    BeginRecording(BindingSelection, RecordingMode),
    ClearSelectedBinding,
    RemoveScopedActionBindingAt(&'static str, Action, usize),
    RemoveLocalBindingAt(LocalShortcut, usize),
    RemoveGridSequenceAt(GridCommandShortcut, usize),
    ApplyGridSequenceInput(GridCommandShortcut, RecordingMode),
}

impl KeyBindingsDialog {
    const WINDOW_WIDTH: f32 = 900.0;
    const WINDOW_HEIGHT: f32 = 640.0;

    fn resolve_frame_action(
        actions: &[KeyBindingsDialogFrameAction],
    ) -> Option<KeyBindingsDialogFrameAction> {
        if actions.contains(&KeyBindingsDialogFrameAction::SaveAndClose) {
            Some(KeyBindingsDialogFrameAction::SaveAndClose)
        } else if actions.contains(&KeyBindingsDialogFrameAction::Close) {
            Some(KeyBindingsDialogFrameAction::Close)
        } else {
            None
        }
    }

    fn apply_ui_actions(
        state: &mut KeyBindingsDialogState,
        actions: Vec<KeyBindingsDialogUiAction>,
    ) {
        for action in actions {
            match action {
                KeyBindingsDialogUiAction::ResetToDefaults => state.reset_to_defaults(),
                KeyBindingsDialogUiAction::ImportLegacyBindings => state.import_legacy_bindings(),
                KeyBindingsDialogUiAction::SetFilter(filter) => {
                    state.filter = filter;
                }
                KeyBindingsDialogUiAction::SetSequenceInput(sequence_input) => {
                    state.sequence_input = sequence_input;
                }
                KeyBindingsDialogUiAction::SelectTree(tree) => state.select_tree(tree),
                KeyBindingsDialogUiAction::SetShowIssueOnly(enabled) => {
                    state.set_show_issue_only(enabled);
                }
                KeyBindingsDialogUiAction::SelectBinding(selection) => {
                    state.select_binding(selection);
                }
                KeyBindingsDialogUiAction::BeginRecording(selection, mode) => {
                    state.begin_recording(selection, mode);
                }
                KeyBindingsDialogUiAction::ClearSelectedBinding => state.clear_selected_binding(),
                KeyBindingsDialogUiAction::RemoveScopedActionBindingAt(
                    scope_path,
                    action,
                    index,
                ) => {
                    state.remove_scoped_action_binding_at(scope_path, action, index);
                }
                KeyBindingsDialogUiAction::RemoveLocalBindingAt(shortcut, index) => {
                    state.remove_local_binding_at(shortcut, index);
                }
                KeyBindingsDialogUiAction::RemoveGridSequenceAt(command, index) => {
                    state.remove_grid_sequence_at(command, index);
                }
                KeyBindingsDialogUiAction::ApplyGridSequenceInput(command, mode) => {
                    state.apply_grid_sequence_input(command, mode);
                }
            }
        }
    }

    fn detect_dialog_shortcut(
        ctx: &egui::Context,
        state: &KeyBindingsDialogState,
    ) -> Option<KeyBindingsDialogShortcutAction> {
        if state.recording {
            return None;
        }

        let shortcuts = DialogShortcutContext::new(ctx);
        if shortcuts.consume_command(LocalShortcut::Dismiss.config_key()) {
            Some(KeyBindingsDialogShortcutAction::CloseDialog)
        } else {
            None
        }
    }

    fn handle_keyboard_input(
        ctx: &egui::Context,
        state: &mut KeyBindingsDialogState,
    ) -> Option<KeyBindingsDialogFrameAction> {
        let recording_handled = ctx.input(|input| state.consume_recording_input(input));
        if recording_handled {
            return None;
        }

        if let Some(nav_action) = PickerDialogShell::consume_nav_action(ctx) {
            Self::apply_picker_nav_action(state, nav_action);
            return None;
        }

        matches!(
            Self::detect_dialog_shortcut(ctx, state),
            Some(KeyBindingsDialogShortcutAction::CloseDialog)
        )
        .then_some(KeyBindingsDialogFrameAction::Close)
    }

    fn apply_picker_nav_action(state: &mut KeyBindingsDialogState, action: PickerNavAction) {
        match action {
            PickerNavAction::MovePrev => match state.pane_focus {
                PickerPaneFocus::Navigator => Self::move_navigator_selection(state, -1),
                PickerPaneFocus::Items => Self::move_binding_selection(state, -1),
                PickerPaneFocus::Detail => {}
            },
            PickerNavAction::MoveNext => match state.pane_focus {
                PickerPaneFocus::Navigator => Self::move_navigator_selection(state, 1),
                PickerPaneFocus::Items => Self::move_binding_selection(state, 1),
                PickerPaneFocus::Detail => {}
            },
            PickerNavAction::Open => match state.pane_focus {
                PickerPaneFocus::Navigator => {
                    if state.selected_binding.is_none() {
                        state.selected_binding = state.visible_bindings().into_iter().next();
                    }
                    state.pane_focus = PickerPaneFocus::Items;
                }
                PickerPaneFocus::Items => {
                    if state.selected_binding.is_some() {
                        state.pane_focus = PickerPaneFocus::Detail;
                    }
                }
                PickerPaneFocus::Detail => {}
            },
            PickerNavAction::Back => match state.pane_focus {
                PickerPaneFocus::Navigator => {}
                PickerPaneFocus::Items => state.pane_focus = PickerPaneFocus::Navigator,
                PickerPaneFocus::Detail => state.pane_focus = PickerPaneFocus::Items,
            },
            PickerNavAction::FocusNext => state.cycle_focus_next(),
            PickerNavAction::FocusPrev => state.cycle_focus_prev(),
        }
    }

    fn move_navigator_selection(state: &mut KeyBindingsDialogState, direction: isize) {
        let entries = navigator_entries();
        let Some(current_index) = entries
            .iter()
            .position(|entry| *entry == state.current_tree)
        else {
            return;
        };
        let next_index = current_index as isize + direction;
        if !(0..entries.len() as isize).contains(&next_index) {
            return;
        }

        state.select_tree(entries[next_index as usize]);
    }

    fn move_binding_selection(state: &mut KeyBindingsDialogState, direction: isize) {
        let visible = state.visible_bindings();
        if visible.is_empty() {
            state.selected_binding = None;
            return;
        }

        let current_index = state
            .selected_binding
            .and_then(|selection| visible.iter().position(|entry| *entry == selection))
            .unwrap_or(0);
        let next_index = (current_index as isize + direction).clamp(0, visible.len() as isize - 1);
        state.select_binding(visible[next_index as usize]);
    }

    /// 显示对话框
    ///
    /// 返回 Some(KeyBindings) 表示用户保存了更改
    pub fn show(ctx: &egui::Context, state: &mut KeyBindingsDialogState) -> Option<KeyBindings> {
        if !state.show {
            return None;
        }

        let mut result = None;
        let snapshot = state.clone();
        let mut ui_actions = Vec::new();
        let mut tree_actions = Vec::new();
        let mut list_actions = Vec::new();
        let mut editor_actions = Vec::new();
        let mut frame_actions = Vec::new();
        let mut filter_input = state.filter.clone();
        let mut sequence_input = state.sequence_input.clone();
        let mut copied_text = None;

        if let Some(frame_action) = Self::handle_keyboard_input(ctx, state) {
            frame_actions.push(frame_action);
        }

        if matches!(
            Self::resolve_frame_action(&frame_actions),
            Some(KeyBindingsDialogFrameAction::Close)
        ) {
            state.close();
            return result;
        }

        let style = DialogStyle::WORKSPACE;
        DialogWindow::workspace(
            ctx,
            "快捷键设置",
            &style,
            Self::WINDOW_WIDTH,
            Self::WINDOW_HEIGHT,
        )
        .show(ctx, |ui| {
            DialogContent::toolbar(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("搜索:");
                    ui.add(
                        egui::TextEdit::singleline(&mut filter_input)
                            .desired_width(220.0)
                            .hint_text("输入功能或 keymap 路径..."),
                    );

                    if ui.button("重置全部为默认").clicked() {
                        ui_actions.push(KeyBindingsDialogUiAction::ResetToDefaults);
                    }
                });
            });

            ui.add_space(8.0);
            DialogContent::toolbar(ui, |ui| {
                let mut breadcrumb =
                    vec!["快捷键设置".to_string(), state.current_tree.breadcrumb()];
                if let Some(selection) = state.selected_binding {
                    breadcrumb.push(selection.label().to_string());
                }
                PickerDialogShell::breadcrumb(ui, &breadcrumb);
                ui.add_space(6.0);
                DialogContent::mouse_hint(
                    ui,
                    &[
                        ("单击导航项", "打开分组"),
                        ("单击列表项", "预览当前快捷键"),
                        ("单击右侧按钮", "替换 / 追加 / 恢复"),
                    ],
                );
            });

            ui.add_space(8.0);

            PickerDialogShell::split(
                ui,
                250.0,
                330.0,
                |ui| Self::show_scope_tree_pane(ui, &snapshot, &mut tree_actions),
                |ui| Self::show_binding_list_pane(ui, &snapshot, &mut list_actions),
                |ui| {
                    Self::show_binding_editor_pane(
                        ui,
                        &snapshot,
                        &mut sequence_input,
                        &mut editor_actions,
                        &mut copied_text,
                    )
                },
            );

            DialogContent::toolbar(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if let Some(label) = state.clear_button_label()
                        && ui.button(label).clicked()
                    {
                        ui_actions.push(KeyBindingsDialogUiAction::ClearSelectedBinding);
                    }

                    ui.label(
                        RichText::new(if state.recording {
                            state.recording_hint()
                        } else {
                            state.current_tree.helper_text()
                        })
                        .small()
                        .weak(),
                    );
                });
            });

            ui.add_space(8.0);

            let save_text = if state.has_changes {
                "保存 *"
            } else {
                "保存"
            };
            let footer = DialogFooter::show(ui, save_text, "取消", true, &style);
            if footer.cancelled {
                frame_actions.push(KeyBindingsDialogFrameAction::Close);
            }
            if footer.confirmed {
                frame_actions.push(KeyBindingsDialogFrameAction::SaveAndClose);
            }
        });

        if filter_input != state.filter {
            ui_actions.push(KeyBindingsDialogUiAction::SetFilter(filter_input));
        }
        if sequence_input != state.sequence_input {
            ui_actions.push(KeyBindingsDialogUiAction::SetSequenceInput(sequence_input));
        }
        if let Some(text) = copied_text {
            ctx.copy_text(text);
        }
        ui_actions.extend(tree_actions);
        ui_actions.extend(list_actions);
        ui_actions.extend(editor_actions);
        Self::apply_ui_actions(state, ui_actions);

        match Self::resolve_frame_action(&frame_actions) {
            Some(KeyBindingsDialogFrameAction::SaveAndClose) => {
                result = Some(state.bindings.clone());
                state.close();
            }
            Some(KeyBindingsDialogFrameAction::Close) => {
                state.close();
            }
            None => {}
        }

        result
    }

    fn show_scope_tree_pane(
        ui: &mut egui::Ui,
        state: &KeyBindingsDialogState,
        actions: &mut Vec<KeyBindingsDialogUiAction>,
    ) {
        PickerDialogShell::pane(
            ui,
            "导航",
            "左列负责打开分组；j/k 移动，l 或 Enter 打开。",
            state.pane_focus == PickerPaneFocus::Navigator,
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("keybindings_nav")
                    .show(ui, |ui| {
                        let mut current_section = "";
                        for entry in scope_tree_entries() {
                            if entry.section != current_section {
                                if !current_section.is_empty() {
                                    ui.add_space(10.0);
                                }
                                current_section = entry.section;
                                PickerDialogShell::section_label(ui, current_section);
                            }

                            let issues = state.issue_count(entry.selection);
                            let meta = if issues > 0 {
                                Some(format!(
                                    "{} 项 · {} 条诊断",
                                    state.filtered_count(entry.selection),
                                    issues
                                ))
                            } else {
                                Some(format!("{} 项", state.filtered_count(entry.selection)))
                            };

                            let is_selected = state.current_tree == entry.selection;
                            let response = PickerDialogShell::entry(
                                ui,
                                format!("nav::{:?}", entry.selection),
                                is_selected,
                                is_selected && state.pane_focus == PickerPaneFocus::Navigator,
                                entry.title,
                                meta.as_deref(),
                                None,
                            );
                            PickerDialogShell::reveal_selected(
                                &response,
                                is_selected && state.pane_focus == PickerPaneFocus::Navigator,
                            );
                            if response.clicked() {
                                actions
                                    .push(KeyBindingsDialogUiAction::SelectTree(entry.selection));
                            }
                            ui.add_space(6.0);
                        }
                    });
            },
        );
    }

    fn show_binding_list_pane(
        ui: &mut egui::Ui,
        state: &KeyBindingsDialogState,
        actions: &mut Vec<KeyBindingsDialogUiAction>,
    ) {
        let issues = state.issue_count(state.current_tree);
        let summary = state.issue_summary(state.current_tree);

        PickerDialogShell::pane(
            ui,
            "当前层级",
            "中列负责浏览当前分组；j/k 移动，l 或 Enter 查看详情。",
            state.pane_focus == PickerPaneFocus::Items,
            |ui| {
                DialogContent::toolbar(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(navigator_title(state.current_tree)).strong());
                        ui.label(
                            RichText::new(state.current_tree.breadcrumb())
                                .small()
                                .weak()
                                .monospace(),
                        );
                        ui.label(
                            RichText::new(format!(
                                "当前共 {} 项",
                                state.filtered_count(state.current_tree)
                            ))
                            .small()
                            .weak(),
                        );
                        if issues > 0 {
                            ui.label(
                                RichText::new(format!("{} 条作用域提醒", issues))
                                    .small()
                                    .color(egui::Color32::from_rgb(245, 189, 130)),
                            );
                        }
                        if !state.current_tree.is_global() && issues > 0 {
                            let mut show_issue_only = state.show_issue_only;
                            if ui
                                .checkbox(&mut show_issue_only, "只看有作用域提醒")
                                .on_hover_text("只显示存在遮蔽或同作用域重叠的局部快捷键")
                                .changed()
                            {
                                actions.push(KeyBindingsDialogUiAction::SetShowIssueOnly(
                                    show_issue_only,
                                ));
                            }
                        }
                    });
                });

                if !summary.is_empty() {
                    ui.add_space(8.0);
                    DialogContent::card(ui, Some(egui::Color32::from_rgb(245, 189, 130)), |ui| {
                        ui.label(RichText::new("冲突摘要").small().strong());
                        ui.add_space(4.0);
                        for (selection, issues) in summary.iter().take(5) {
                            let label = format!("{} · {} 条提醒", selection.label(), issues.len());
                            if ui
                                .small_button(label)
                                .on_hover_text(selection.detail())
                                .clicked()
                            {
                                actions.push(KeyBindingsDialogUiAction::SelectBinding(*selection));
                            }
                        }
                    });
                }

                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .id_salt("keybindings_items")
                    .show(ui, |ui| {
                        for selection in state.visible_bindings() {
                            Self::show_binding_list_entry(ui, state, selection, actions);
                            ui.add_space(8.0);
                        }
                    });
            },
        );
    }

    fn show_binding_list_entry(
        ui: &mut egui::Ui,
        state: &KeyBindingsDialogState,
        selection: BindingSelection,
        actions: &mut Vec<KeyBindingsDialogUiAction>,
    ) {
        let is_selected = state.selected_binding == Some(selection);
        let issue_count = state.selection_issue_messages(selection).len();

        let mut meta = state.binding_text(selection);
        if state.recording && is_selected {
            meta = if let Some(key) = state.recorded_key {
                KeyBinding::new(key, state.recorded_modifiers).display()
            } else if state.recorded_modifiers != KeyModifiers::NONE {
                format!("{}+...", state.recorded_modifiers)
            } else {
                "按下快捷键...".to_string()
            };
        }

        let detail = if issue_count > 0 {
            format!(
                "{} · {} · {} 条作用域提醒",
                selection.category(),
                state.binding_source(selection),
                issue_count
            )
        } else {
            format!(
                "{} · {}",
                selection.category(),
                state.binding_source(selection)
            )
        };

        let response = PickerDialogShell::entry(
            ui,
            selection.detail(),
            is_selected,
            is_selected && state.pane_focus == PickerPaneFocus::Items,
            selection.label(),
            Some(&meta),
            Some(&detail),
        );
        PickerDialogShell::reveal_selected(
            &response,
            is_selected && state.pane_focus == PickerPaneFocus::Items,
        );
        if response.clicked() {
            actions.push(KeyBindingsDialogUiAction::SelectBinding(selection));
        }
    }

    fn show_binding_editor_pane(
        ui: &mut egui::Ui,
        state: &KeyBindingsDialogState,
        sequence_input: &mut String,
        actions: &mut Vec<KeyBindingsDialogUiAction>,
        copied_text: &mut Option<String>,
    ) {
        PickerDialogShell::pane(
            ui,
            "详情与编辑",
            "右列负责查看详情、录制、恢复默认和处理作用域提醒。",
            state.pane_focus == PickerPaneFocus::Detail,
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("keybindings_detail")
                    .show(ui, |ui| {
                        if let Some(path) = &state.keymap_path {
                            DialogContent::card(ui, None, |ui| {
                                ui.label(RichText::new("keymap.toml").small().strong());
                                ui.add_space(4.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(path.display().to_string()).monospace(),
                                    )
                                    .wrap(),
                                );
                                ui.add_space(6.0);
                                if ui.button("复制路径").clicked() {
                                    *copied_text = Some(path.display().to_string());
                                }
                            });
                            ui.add_space(8.0);
                        }

                        if let Some(message) = state.migration_notice() {
                            DialogContent::card(
                                ui,
                                Some(egui::Color32::from_rgb(245, 189, 130)),
                                |ui| {
                                    ui.label(RichText::new("兼容迁移").small().strong());
                                    ui.add_space(4.0);
                                    DialogContent::warning_text(ui, message);
                                    if state.legacy_bindings.is_some() {
                                        ui.add_space(6.0);
                                        if ui.button("导入旧版 config.toml 键位").clicked() {
                                            actions.push(
                                                KeyBindingsDialogUiAction::ImportLegacyBindings,
                                            );
                                        }
                                    }
                                },
                            );
                            ui.add_space(8.0);
                        }

                        if let Some(msg) = &state.conflict_message {
                            DialogContent::card(
                                ui,
                                Some(egui::Color32::from_rgb(245, 189, 130)),
                                |ui| {
                                    DialogContent::warning_text(ui, msg);
                                },
                            );
                            ui.add_space(8.0);
                        }

                        if let Some(selection) = state.selected_binding {
                            ui.vertical(|ui| {
                                ui.label(RichText::new(selection.label()).strong());
                                let detail = selection.detail();
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(detail).small().weak().monospace(),
                                    )
                                    .wrap(),
                                );
                            });
                        } else {
                            ui.label(RichText::new("请选择一项快捷键进行录制或恢复默认").weak());
                        }

                        ui.add_space(8.0);

                        if let Some(selection) = state.selected_binding {
                            DialogContent::card(ui, None, |ui| {
                                ui.label(RichText::new("当前绑定").small().strong());
                                ui.add_space(4.0);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(state.binding_text(selection)).monospace(),
                                    )
                                    .wrap(),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(format!(
                                            "来源: {} · scope: {}",
                                            state.binding_source(selection),
                                            selection.scope_path()
                                        ))
                                        .small()
                                        .weak(),
                                    )
                                    .wrap(),
                                );
                            });
                            ui.add_space(8.0);

                            let diagnostics = state.selection_issue_messages(selection);
                            if diagnostics.is_empty() {
                                DialogContent::card(ui, None, |ui| {
                                    ui.label(RichText::new("诊断").small().strong());
                                    ui.add_space(4.0);
                                    ui.label(
                                        RichText::new("当前未发现作用域冲突或 keymap 诊断。")
                                            .small()
                                            .weak(),
                                    );
                                });
                            } else {
                                Self::show_issue_card(ui, diagnostics);
                            }
                            ui.add_space(8.0);
                        }

                        if let Some(BindingSelection::ScopedAction(scope_path, action)) =
                            state.selected_binding
                        {
                            if let Some(bindings) = state.scoped_action_bindings(scope_path, action) {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new("当前作用域绑定:").small().weak());
                                    for (index, binding) in bindings.iter().enumerate() {
                                        let remove_label = format!("{} ×", binding.display());
                                        ui.push_id((scope_path, action.keymap_name(), index), |ui| {
                                            if ui
                                                .small_button(remove_label)
                                                .on_hover_text("移除这一组作用域动作绑定")
                                                .clicked()
                                            {
                                                actions.push(
                                                    KeyBindingsDialogUiAction::RemoveScopedActionBindingAt(
                                                        scope_path,
                                                        action,
                                                        index,
                                                    ),
                                                );
                                            }
                                        });
                                    }
                                });
                            }

                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("替换绑定").clicked() {
                                    actions.push(KeyBindingsDialogUiAction::BeginRecording(
                                        BindingSelection::ScopedAction(scope_path, action),
                                        RecordingMode::Replace,
                                    ));
                                }
                                if ui.button("追加绑定").clicked() {
                                    actions.push(KeyBindingsDialogUiAction::BeginRecording(
                                        BindingSelection::ScopedAction(scope_path, action),
                                        RecordingMode::Append,
                                    ));
                                }
                            });

                            ui.add_space(8.0);
                            Self::show_issue_card(
                                ui,
                                state.selection_issue_messages(BindingSelection::ScopedAction(
                                    scope_path, action,
                                )),
                            );
                        } else if let Some(BindingSelection::Local(shortcut)) = state.selected_binding
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("当前局部绑定:").small().weak());
                                for (index, binding) in
                                    state.local_bindings(shortcut).iter().enumerate()
                                {
                                    let remove_label = format!("{} ×", binding.display());
                                    ui.push_id((shortcut.config_key(), index), |ui| {
                                        if ui
                                            .small_button(remove_label)
                                            .on_hover_text("移除这一组局部绑定")
                                            .clicked()
                                        {
                                            actions.push(
                                                KeyBindingsDialogUiAction::RemoveLocalBindingAt(
                                                    shortcut, index,
                                                ),
                                            );
                                        }
                                    });
                                }
                            });

                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("替换绑定").clicked() {
                                    actions.push(KeyBindingsDialogUiAction::BeginRecording(
                                        BindingSelection::Local(shortcut),
                                        RecordingMode::Replace,
                                    ));
                                }
                                if ui.button("追加绑定").clicked() {
                                    actions.push(KeyBindingsDialogUiAction::BeginRecording(
                                        BindingSelection::Local(shortcut),
                                        RecordingMode::Append,
                                    ));
                                }
                            });

                            ui.add_space(8.0);
                            Self::show_issue_card(
                                ui,
                                state.selection_issue_messages(BindingSelection::Local(shortcut)),
                            );
                        } else if let Some(BindingSelection::GridCommand(command)) =
                            state.selected_binding
                        {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new("当前命令序列:").small().weak());
                                for (index, sequence) in
                                    state.grid_sequences(command).iter().enumerate()
                                {
                                    let remove_label = format!("{} ×", sequence);
                                    ui.push_id((command.config_key(), index), |ui| {
                                        if ui
                                            .small_button(remove_label)
                                            .on_hover_text("移除这一组表格命令序列")
                                            .clicked()
                                        {
                                            actions.push(
                                                KeyBindingsDialogUiAction::RemoveGridSequenceAt(
                                                    command, index,
                                                ),
                                            );
                                        }
                                    });
                                }
                            });

                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label("序列:");
                                ui.add(
                                    egui::TextEdit::singleline(sequence_input)
                                        .desired_width(220.0)
                                        .hint_text("如: yy / :w / Space+d / Ctrl+S"),
                                );
                            });
                            ui.add_space(8.0);
                            ui.horizontal_wrapped(|ui| {
                                if ui.button("替换序列").clicked() {
                                    actions.push(
                                        KeyBindingsDialogUiAction::ApplyGridSequenceInput(
                                            command,
                                            RecordingMode::Replace,
                                        ),
                                    );
                                }
                                if ui.button("追加序列").clicked() {
                                    actions.push(
                                        KeyBindingsDialogUiAction::ApplyGridSequenceInput(
                                            command,
                                            RecordingMode::Append,
                                        ),
                                    );
                                }
                            });

                            ui.add_space(8.0);
                            Self::show_issue_card(
                                ui,
                                state.selection_issue_messages(BindingSelection::GridCommand(
                                    command,
                                )),
                            );
                        }
                    });
            },
        );
    }

    fn show_issue_card(ui: &mut egui::Ui, issues: Vec<String>) {
        if issues.is_empty() {
            return;
        }

        DialogContent::card(ui, Some(egui::Color32::from_rgb(245, 189, 130)), |ui| {
            ui.label(
                RichText::new("作用域分析")
                    .small()
                    .strong()
                    .color(egui::Color32::from_rgb(245, 189, 130)),
            );
            ui.add_space(4.0);
            for issue in issues {
                ui.label(
                    RichText::new(format!("• {issue}"))
                        .small()
                        .color(egui::Color32::from_rgb(245, 189, 130)),
                );
            }
        });
    }
}

fn keybinding_list_text(bindings: &[KeyBinding]) -> String {
    let mut values: Vec<String> = Vec::new();
    for binding in bindings {
        let display = binding.display();
        if !values.iter().any(|item| item == &display) {
            values.push(display);
        }
    }
    values.join(" / ")
}

fn keybinding_list_text_strings(bindings: &[String]) -> String {
    let mut values: Vec<String> = Vec::new();
    for binding in bindings {
        if !values.iter().any(|item| item == binding) {
            values.push(binding.clone());
        }
    }
    values.join(" / ")
}

fn local_shortcut_description(shortcut: LocalShortcut) -> &'static str {
    shortcut.description()
}

fn grid_command_description(command: GridCommandShortcut) -> &'static str {
    match command {
        GridCommandShortcut::OpenFilter => "表格打开筛选面板",
        GridCommandShortcut::AddRowBelow => "表格在下方新增行",
        GridCommandShortcut::AddRowAbove => "表格在上方新增行",
        GridCommandShortcut::Save => "表格保存修改",
        GridCommandShortcut::Discard => "表格放弃修改",
        GridCommandShortcut::JumpFileStart => "表格跳到开头",
        GridCommandShortcut::JumpFileEnd => "表格跳到结尾",
        GridCommandShortcut::JumpLineStart => "表格跳到行首",
        GridCommandShortcut::JumpLineEnd => "表格跳到行尾",
        GridCommandShortcut::ScrollCenter => "表格滚动到居中",
        GridCommandShortcut::ScrollTop => "表格滚动到顶部",
        GridCommandShortcut::ScrollBottom => "表格滚动到底部",
        GridCommandShortcut::DeleteRow => "表格删除当前行",
        GridCommandShortcut::CopyRow => "表格复制当前行",
    }
}

fn grid_command_category(_: GridCommandShortcut) -> &'static str {
    "表格命令"
}

fn local_shortcut_category(shortcut: LocalShortcut) -> &'static str {
    shortcut.category()
}

fn local_shortcut_scope_tags(shortcut: LocalShortcut) -> &'static [&'static str] {
    match shortcut {
        LocalShortcut::Confirm
        | LocalShortcut::Cancel
        | LocalShortcut::Dismiss
        | LocalShortcut::FormatSelectionCycle => &["dialog.common"],
        LocalShortcut::DangerConfirm | LocalShortcut::DangerCancel => &["dialog.confirm"],
        LocalShortcut::HelpScrollUp
        | LocalShortcut::HelpScrollDown
        | LocalShortcut::HelpPageUp
        | LocalShortcut::HelpPageDown => &["dialog.help", "dialog.common"],
        LocalShortcut::PickerMovePrev
        | LocalShortcut::PickerMoveNext
        | LocalShortcut::PickerOpen
        | LocalShortcut::PickerBack
        | LocalShortcut::PickerFocusNext
        | LocalShortcut::PickerFocusPrev => &["dialog.picker"],
        LocalShortcut::CommandPalettePrev
        | LocalShortcut::CommandPaletteNext
        | LocalShortcut::CommandPaletteConfirm
        | LocalShortcut::CommandPaletteDismiss => &["dialog.command_palette"],
        LocalShortcut::ToolbarPrev
        | LocalShortcut::ToolbarNext
        | LocalShortcut::ToolbarToQueryTabs
        | LocalShortcut::ToolbarActivate
        | LocalShortcut::ToolbarDismiss
        | LocalShortcut::ToolbarMenuPrev
        | LocalShortcut::ToolbarMenuNext
        | LocalShortcut::ToolbarMenuConfirm
        | LocalShortcut::ToolbarMenuDismiss
        | LocalShortcut::ToolbarThemePrev
        | LocalShortcut::ToolbarThemeNext
        | LocalShortcut::ToolbarThemeConfirm
        | LocalShortcut::ToolbarThemeDismiss
        | LocalShortcut::ToolbarThemeStart
        | LocalShortcut::ToolbarThemeEnd => &["toolbar"],
        LocalShortcut::QueryTabPrev
        | LocalShortcut::QueryTabNext
        | LocalShortcut::QueryTabToDataGrid
        | LocalShortcut::QueryTabToToolbar
        | LocalShortcut::QueryTabActivate
        | LocalShortcut::QueryTabClose
        | LocalShortcut::QueryTabDismiss => &["query_tabs"],
        LocalShortcut::ErDiagramRefresh
        | LocalShortcut::ErDiagramLayout
        | LocalShortcut::ErDiagramFitView
        | LocalShortcut::ErDiagramZoomIn
        | LocalShortcut::ErDiagramZoomOut => &["er_diagram"],
        LocalShortcut::SidebarItemPrev
        | LocalShortcut::SidebarItemNext
        | LocalShortcut::SidebarItemStart
        | LocalShortcut::SidebarItemEnd
        | LocalShortcut::SidebarMoveLeft
        | LocalShortcut::SidebarMoveRight
        | LocalShortcut::SidebarToggle
        | LocalShortcut::SidebarDelete
        | LocalShortcut::SidebarEdit
        | LocalShortcut::SidebarRename
        | LocalShortcut::SidebarRefresh
        | LocalShortcut::SidebarActivate => &["sidebar.list"],
        LocalShortcut::FilterAdd
        | LocalShortcut::FilterDelete
        | LocalShortcut::FilterClearAll
        | LocalShortcut::FilterColumnNext
        | LocalShortcut::FilterColumnPrev
        | LocalShortcut::FilterOperatorNext
        | LocalShortcut::FilterOperatorPrev
        | LocalShortcut::FilterLogicToggle
        | LocalShortcut::FilterFocusInput
        | LocalShortcut::FilterCaseToggle => &["sidebar.filters.list", "sidebar.list"],
        LocalShortcut::FilterInputDismiss => &["sidebar.filters.input"],
        LocalShortcut::ExportFormatCsv
        | LocalShortcut::ExportFormatTsv
        | LocalShortcut::ExportFormatSql
        | LocalShortcut::ExportFormatJson
        | LocalShortcut::ExportCyclePrev
        | LocalShortcut::ExportCycleNext
        | LocalShortcut::ExportColumnPrev
        | LocalShortcut::ExportColumnNext
        | LocalShortcut::ExportColumnStart
        | LocalShortcut::ExportColumnEnd
        | LocalShortcut::ExportColumnToggle
        | LocalShortcut::ExportColumnsToggleAll => &["dialog.export", "dialog.common"],
        LocalShortcut::SqlExecute
        | LocalShortcut::SqlExplain
        | LocalShortcut::SqlClear
        | LocalShortcut::SqlAutocompleteTrigger
        | LocalShortcut::SqlAutocompleteConfirm
        | LocalShortcut::SqlHistoryPrev
        | LocalShortcut::SqlHistoryNext
        | LocalShortcut::SqlHistoryBrowse => &["editor.insert"],
        LocalShortcut::GridEditFinish => &["grid.insert"],
        LocalShortcut::ImportRefresh
        | LocalShortcut::ImportFormatSql
        | LocalShortcut::ImportFormatCsv
        | LocalShortcut::ImportFormatTsv
        | LocalShortcut::ImportFormatJson
        | LocalShortcut::ImportCyclePrev
        | LocalShortcut::ImportCycleNext => &["dialog.import", "dialog.common"],
        LocalShortcut::ConnectionTypeSqlite
        | LocalShortcut::ConnectionTypePostgres
        | LocalShortcut::ConnectionTypeMySql
        | LocalShortcut::ConnectionTypePrev
        | LocalShortcut::ConnectionTypeNext
        | LocalShortcut::SqliteBrowseFile => &["dialog.connection", "dialog.common"],
        LocalShortcut::DdlColumnPrev
        | LocalShortcut::DdlColumnNext
        | LocalShortcut::DdlColumnStart
        | LocalShortcut::DdlColumnEnd
        | LocalShortcut::DdlColumnDelete
        | LocalShortcut::DdlColumnAddBelow
        | LocalShortcut::DdlColumnAddAbove
        | LocalShortcut::DdlColumnTogglePrimaryKey => &["dialog.ddl", "dialog.common"],
        LocalShortcut::HistoryClear
        | LocalShortcut::HistoryPrev
        | LocalShortcut::HistoryNext
        | LocalShortcut::HistoryStart
        | LocalShortcut::HistoryEnd
        | LocalShortcut::HistoryPageUp
        | LocalShortcut::HistoryPageDown
        | LocalShortcut::HistoryUse => &["dialog.history", "dialog.common"],
    }
}

fn local_shortcuts_overlap(left: LocalShortcut, right: LocalShortcut) -> bool {
    let left_tags = local_shortcut_scope_tags(left);
    let right_tags = local_shortcut_scope_tags(right);
    left_tags
        .iter()
        .any(|left_tag| right_tags.iter().any(|right_tag| right_tag == left_tag))
}

#[cfg(test)]
mod tests {
    use eframe::egui::{Event, Key, Modifiers, RawInput};

    use super::{
        BindingIssue, BindingSelection, KeyBindingsDialog, KeyBindingsDialogFrameAction,
        KeyBindingsDialogState, KeyBindingsDialogUiAction, RecordingMode, ScopeTreeSelection,
    };
    use crate::core::{KeyBinding, KeyBindings, KeyCode, KeyModifiers};
    use crate::ui::components::{GridCommandShortcut, GridSequenceConflictKind};
    use crate::ui::shortcut_tooltip::LocalShortcut;

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        ctx.begin_pass(RawInput {
            events: vec![key_event(key)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
    }

    fn focus_text_input(ctx: &egui::Context) {
        let mut text = String::new();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("keybindings dialog test input").show(ctx, |ui| {
            let response = ui.add(egui::TextEdit::singleline(&mut text).id_salt("kbd_dialog_text"));
            response.request_focus();
        });
        let _ = ctx.end_pass();
    }

    #[test]
    fn local_shortcut_uses_default_text_without_override() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        let text = state.binding_text(BindingSelection::Local(LocalShortcut::Dismiss));
        let source = state.binding_source(BindingSelection::Local(LocalShortcut::Dismiss));

        assert_eq!(text, "Esc / Q");
        assert_eq!(source, "默认");
    }

    #[test]
    fn grid_command_uses_default_text_without_override() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        let text = state.binding_text(BindingSelection::GridCommand(GridCommandShortcut::CopyRow));
        let source =
            state.binding_source(BindingSelection::GridCommand(GridCommandShortcut::CopyRow));

        assert_eq!(text, "yy");
        assert_eq!(source, "默认");
    }

    #[test]
    fn clearing_local_shortcut_restores_default_source() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_bindings(
            LocalShortcut::Confirm.config_key(),
            vec![KeyBinding::ctrl(KeyCode::Enter)],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);
        state.selected_binding = Some(BindingSelection::Local(LocalShortcut::Confirm));

        assert_eq!(
            state.binding_source(BindingSelection::Local(LocalShortcut::Confirm)),
            "自定义"
        );

        state.clear_selected_binding();

        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Confirm)),
            "Enter"
        );
        assert_eq!(
            state.binding_source(BindingSelection::Local(LocalShortcut::Confirm)),
            "默认"
        );
    }

    #[test]
    fn append_local_shortcut_keeps_existing_bindings() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.selected_binding = Some(BindingSelection::Local(LocalShortcut::Confirm));
        state.recording_mode = RecordingMode::Append;

        state.apply_recorded_binding(
            BindingSelection::Local(LocalShortcut::Confirm),
            KeyBinding::ctrl(KeyCode::Enter),
        );

        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Confirm)),
            "Enter / Ctrl+Enter"
        );
        assert_eq!(
            state.binding_source(BindingSelection::Local(LocalShortcut::Confirm)),
            "自定义"
        );
    }

    #[test]
    fn remove_one_local_binding_keeps_other_overrides() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_bindings(
            LocalShortcut::Dismiss.config_key(),
            vec![
                KeyBinding::key_only(KeyCode::Escape),
                KeyBinding::key_only(KeyCode::Q),
                KeyBinding::ctrl(KeyCode::W),
            ],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);

        state.remove_local_binding_at(LocalShortcut::Dismiss, 1);

        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Dismiss)),
            "Esc / Ctrl+W"
        );
        assert_eq!(
            state.binding_source(BindingSelection::Local(LocalShortcut::Dismiss)),
            "自定义"
        );
    }

    #[test]
    fn remove_one_scoped_action_binding_keeps_other_overrides() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_bindings(
            "toolbar.refresh",
            vec![
                KeyBinding::key_only(KeyCode::R),
                KeyBinding::ctrl(KeyCode::R),
            ],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);

        state.remove_scoped_action_binding_at("toolbar", crate::core::Action::Refresh, 0);

        assert_eq!(
            state.binding_text(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "Ctrl+R"
        );
        assert_eq!(
            state.binding_source(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "局部覆盖"
        );
    }

    #[test]
    fn append_grid_sequence_keeps_existing_sequences() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.sequence_input = "cc".to_string();

        state.apply_grid_sequence_input(GridCommandShortcut::CopyRow, RecordingMode::Append);

        assert_eq!(
            state.binding_text(BindingSelection::GridCommand(GridCommandShortcut::CopyRow)),
            "yy / cc"
        );
        assert_eq!(
            state.binding_source(BindingSelection::GridCommand(GridCommandShortcut::CopyRow)),
            "自定义"
        );
    }

    #[test]
    fn remove_one_grid_sequence_keeps_other_overrides() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_sequences(
            GridCommandShortcut::CopyRow.config_key(),
            vec!["yy".to_string(), "cc".to_string()],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);

        state.remove_grid_sequence_at(GridCommandShortcut::CopyRow, 0);

        assert_eq!(
            state.binding_text(BindingSelection::GridCommand(GridCommandShortcut::CopyRow)),
            "cc"
        );
        assert_eq!(
            state.binding_source(BindingSelection::GridCommand(GridCommandShortcut::CopyRow)),
            "自定义"
        );
    }

    #[test]
    fn grid_issues_include_prefix_conflicts() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_sequences(
            GridCommandShortcut::AddRowBelow.config_key(),
            vec!["g".to_string()],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);

        let issues = state.binding_issues(BindingSelection::GridCommand(
            GridCommandShortcut::JumpFileStart,
        ));

        assert!(issues.iter().any(|issue| matches!(
            issue,
            BindingIssue::GridConflict {
                command,
                kind: GridSequenceConflictKind::Prefix,
                ..
            } if *command == GridCommandShortcut::AddRowBelow
        )));
    }

    #[test]
    fn local_issues_include_global_shadowing() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        let issues = state.binding_issues(BindingSelection::Local(LocalShortcut::SqlExecute));

        assert!(issues.iter().any(|issue| matches!(
            issue,
            BindingIssue::GlobalShadow { action, .. } if *action == crate::core::Action::Refresh
        )));
    }

    #[test]
    fn local_issues_include_same_scope_conflicts() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_bindings(
            LocalShortcut::HelpScrollDown.config_key(),
            vec![KeyBinding::key_only(KeyCode::K)],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);

        let issues = state.binding_issues(BindingSelection::Local(LocalShortcut::HelpScrollUp));

        assert!(issues.iter().any(|issue| matches!(
            issue,
            BindingIssue::LocalConflict { shortcut, .. } if *shortcut == LocalShortcut::HelpScrollDown
        )));
    }

    #[test]
    fn scoped_action_issue_messages_include_same_scope_runtime_conflicts() {
        let mut bindings = KeyBindings::default();
        bindings.set_local_bindings("toolbar.refresh", vec![KeyBinding::ctrl(KeyCode::R)]);
        bindings.set_local_bindings("toolbar.save", vec![KeyBinding::ctrl(KeyCode::R)]);

        let mut state = KeyBindingsDialogState::default();
        state.open(&bindings);

        let issues = state.selection_issue_messages(BindingSelection::ScopedAction(
            "toolbar",
            crate::core::Action::Refresh,
        ));

        assert!(
            issues
                .iter()
                .any(|message| message.contains("同作用域动作") && message.contains("Ctrl+R"))
        );
    }

    #[test]
    fn local_scope_tree_filters_to_selected_section() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("sidebar.list"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::SidebarItemPrev)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::FilterAdd)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::HelpScrollUp)));
        assert!(!visible.contains(&BindingSelection::Global(crate::core::Action::ShowHelp)));
    }

    #[test]
    fn grid_scope_tree_filters_to_data_grid_section() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("grid.normal"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::GridCommand(GridCommandShortcut::CopyRow)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::SqlExecute)));
        assert!(!visible.contains(&BindingSelection::Global(crate::core::Action::ShowHelp)));
    }

    #[test]
    fn toolbar_scope_lists_scoped_actions() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("toolbar"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::ScopedAction(
            "toolbar",
            crate::core::Action::Refresh
        )));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::HelpScrollUp)));
    }

    #[test]
    fn toolbar_scope_lists_toolbar_local_shortcuts() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("toolbar"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::ToolbarPrev)));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::ToolbarMenuConfirm)));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::ToolbarThemeNext)));
    }

    #[test]
    fn query_tabs_scope_lists_query_tab_local_shortcuts() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("query_tabs"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::QueryTabPrev)));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::QueryTabClose)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::ToolbarPrev)));
    }

    #[test]
    fn command_palette_scope_lists_palette_local_shortcuts() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("dialog.command_palette"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::CommandPalettePrev)));
        assert!(visible.contains(&BindingSelection::Local(
            LocalShortcut::CommandPaletteConfirm
        )));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::ToolbarPrev)));
    }

    #[test]
    fn er_diagram_scope_lists_er_local_shortcuts() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("er_diagram"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::ErDiagramRefresh)));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::ErDiagramZoomIn)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::QueryTabPrev)));
    }

    #[test]
    fn filters_scope_tree_uses_runtime_filters_list_scope() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("sidebar.filters.list"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::FilterAdd)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::SidebarItemPrev)));
    }

    #[test]
    fn filters_input_scope_lists_only_text_entry_safe_actions() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("sidebar.filters.input"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::ScopedAction(
            "sidebar.filters.input",
            crate::core::Action::ShowHelp,
        )));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::FilterInputDismiss,)));
        assert!(!visible.contains(&BindingSelection::ScopedAction(
            "sidebar.filters.input",
            crate::core::Action::Refresh,
        )));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::FilterAdd)));
    }

    #[test]
    fn editor_insert_scope_lists_text_entry_safe_actions_and_local_commands() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("editor.insert"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::ScopedAction(
            "editor.insert",
            crate::core::Action::ClearCommandLine,
        )));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::SqlExecute)));
        assert!(!visible.contains(&BindingSelection::ScopedAction(
            "editor.insert",
            crate::core::Action::Refresh,
        )));
    }

    #[test]
    fn grid_insert_scope_lists_grid_edit_local_shortcuts() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("grid.insert"));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::GridEditFinish)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::SqlExecute)));
    }

    #[test]
    fn editor_insert_scoped_action_uses_inherited_global_binding_without_override() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        assert_eq!(
            state.binding_text(BindingSelection::ScopedAction(
                "editor.insert",
                crate::core::Action::ClearCommandLine,
            )),
            "Ctrl+L"
        );
        assert_eq!(
            state.binding_source(BindingSelection::ScopedAction(
                "editor.insert",
                crate::core::Action::ClearCommandLine,
            )),
            "继承全局"
        );
    }

    #[test]
    fn search_and_tree_counts_follow_same_filter() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("dialog.help"));
        state.filter = "page".to_string();

        assert_eq!(
            state.filtered_count(ScopeTreeSelection::Scope("dialog.help")),
            2
        );
        let expected_issues: usize = state
            .visible_bindings()
            .into_iter()
            .map(|selection| state.selection_issue_messages(selection).len())
            .sum();
        assert_eq!(
            state.issue_count(ScopeTreeSelection::Scope("dialog.help")),
            expected_issues
        );
    }

    #[test]
    fn issue_only_filter_keeps_only_shortcuts_with_issues() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("editor.insert"));
        let all_visible = state.visible_bindings();
        state.set_show_issue_only(true);

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::SqlExecute)));
        assert!(
            !visible
                .iter()
                .any(|selection| matches!(selection, BindingSelection::Global(_)))
        );
        assert!(
            visible
                .iter()
                .all(|selection| !state.selection_issue_messages(*selection).is_empty())
        );
        assert!(visible.len() < all_visible.len());
    }

    #[test]
    fn issue_summary_matches_visible_issue_entries() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::Scope("dialog.help"));

        let summary = state.issue_summary(ScopeTreeSelection::Scope("dialog.help"));

        assert!(summary.iter().all(|(selection, issues)| matches!(
            selection,
            BindingSelection::Local(_)
        ) && !issues.is_empty()));
        assert_eq!(
            summary
                .iter()
                .map(|(_, issues)| issues.len())
                .sum::<usize>(),
            state.issue_count(ScopeTreeSelection::Scope("dialog.help"))
        );
    }

    #[test]
    fn consume_recording_input_escape_cancels_recording() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.begin_recording(
            BindingSelection::Local(LocalShortcut::Confirm),
            RecordingMode::Replace,
        );

        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Escape);
        let handled = ctx.input(|i| state.consume_recording_input(i));
        let _ = ctx.end_pass();

        assert!(handled);
        assert!(!state.is_recording());
        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Confirm)),
            "Enter"
        );
    }

    #[test]
    fn consume_recording_input_applies_binding() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.begin_recording(
            BindingSelection::Local(LocalShortcut::Confirm),
            RecordingMode::Replace,
        );

        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Q);
        let handled = ctx.input(|i| state.consume_recording_input(i));
        let _ = ctx.end_pass();

        assert!(handled);
        assert!(!state.is_recording());
        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Confirm)),
            "Q"
        );
    }

    #[test]
    fn dialog_dismiss_shortcut_closes_when_not_recording() {
        let ctx = egui::Context::default();
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        begin_key_pass(&ctx, Key::Escape);
        let frame_action = KeyBindingsDialog::handle_keyboard_input(&ctx, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(frame_action, Some(KeyBindingsDialogFrameAction::Close));
    }

    #[test]
    fn dismiss_shortcut_is_blocked_when_text_input_has_priority() {
        let ctx = egui::Context::default();
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        focus_text_input(&ctx);
        begin_key_pass(&ctx, Key::Q);
        let frame_action = KeyBindingsDialog::handle_keyboard_input(&ctx, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(frame_action, None);
    }

    #[test]
    fn dialog_dismiss_shortcut_cancels_recording_without_closing_dialog() {
        let ctx = egui::Context::default();
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.begin_recording(
            BindingSelection::Local(LocalShortcut::Confirm),
            RecordingMode::Replace,
        );

        begin_key_pass(&ctx, Key::Escape);
        let frame_action = KeyBindingsDialog::handle_keyboard_input(&ctx, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(frame_action, None);
        assert!(!state.is_recording());
    }

    #[test]
    fn recording_key_is_applied_without_closing_dialog() {
        let ctx = egui::Context::default();
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.begin_recording(
            BindingSelection::Local(LocalShortcut::Confirm),
            RecordingMode::Replace,
        );

        begin_key_pass(&ctx, Key::Q);
        let frame_action = KeyBindingsDialog::handle_keyboard_input(&ctx, &mut state);
        let _ = ctx.end_pass();

        assert_eq!(frame_action, None);
        assert!(!state.is_recording());
        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Confirm)),
            "Q"
        );
    }

    #[test]
    fn resolve_frame_action_prefers_save_over_close() {
        let resolved = KeyBindingsDialog::resolve_frame_action(&[
            KeyBindingsDialogFrameAction::Close,
            KeyBindingsDialogFrameAction::SaveAndClose,
        ]);

        assert_eq!(resolved, Some(KeyBindingsDialogFrameAction::SaveAndClose));
    }

    #[test]
    fn apply_ui_actions_can_clear_selected_binding() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_binding(BindingSelection::Local(LocalShortcut::Dismiss));
        state.begin_recording(
            BindingSelection::Local(LocalShortcut::Dismiss),
            RecordingMode::Replace,
        );
        state.apply_recorded_binding(
            BindingSelection::Local(LocalShortcut::Dismiss),
            KeyBinding::new(KeyCode::Q, KeyModifiers::NONE),
        );

        assert_eq!(
            state.binding_source(BindingSelection::Local(LocalShortcut::Dismiss)),
            "自定义"
        );

        KeyBindingsDialog::apply_ui_actions(
            &mut state,
            vec![KeyBindingsDialogUiAction::ClearSelectedBinding],
        );

        assert_eq!(
            state.binding_source(BindingSelection::Local(LocalShortcut::Dismiss)),
            "默认"
        );
    }

    #[test]
    fn open_with_legacy_customizations_tracks_migration_source() {
        let mut legacy = KeyBindings::default();
        legacy.set(
            crate::core::Action::NewConnection,
            KeyBinding::ctrl(KeyCode::P),
        );

        let mut runtime = KeyBindings::default();
        runtime.set(
            crate::core::Action::NewConnection,
            KeyBinding::ctrl(KeyCode::N),
        );

        let mut state = KeyBindingsDialogState::default();
        state.open_with_legacy(&runtime, &legacy);

        assert!(state.legacy_bindings.is_some());
        assert!(state.keymap_path.is_some());
    }

    #[test]
    fn scoped_action_uses_inherited_global_binding_without_override() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        assert_eq!(
            state.binding_text(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "F5"
        );
        assert_eq!(
            state.binding_source(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "继承全局"
        );
    }

    #[test]
    fn scoped_action_recording_creates_local_override() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.recording_mode = RecordingMode::Replace;

        state.apply_recorded_binding(
            BindingSelection::ScopedAction("toolbar", crate::core::Action::Refresh),
            KeyBinding::key_only(KeyCode::R),
        );

        assert_eq!(
            state.binding_text(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "R"
        );
        assert_eq!(
            state.binding_source(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "局部覆盖"
        );
    }

    #[test]
    fn clearing_scoped_action_restores_global_source() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_binding(BindingSelection::ScopedAction(
            "toolbar",
            crate::core::Action::Refresh,
        ));
        state.apply_recorded_binding(
            BindingSelection::ScopedAction("toolbar", crate::core::Action::Refresh),
            KeyBinding::key_only(KeyCode::R),
        );

        state.clear_selected_binding();

        assert_eq!(
            state.binding_text(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "F5"
        );
        assert_eq!(
            state.binding_source(BindingSelection::ScopedAction(
                "toolbar",
                crate::core::Action::Refresh,
            )),
            "继承全局"
        );
    }

    #[test]
    fn import_legacy_bindings_replaces_editing_copy_and_marks_changes() {
        let mut legacy = KeyBindings::default();
        legacy.set(
            crate::core::Action::NewConnection,
            KeyBinding::ctrl(KeyCode::P),
        );
        legacy.set_local_bindings(
            LocalShortcut::Dismiss.config_key(),
            vec![KeyBinding::ctrl(KeyCode::W)],
        );

        let mut state = KeyBindingsDialogState::default();
        state.open_with_legacy(&KeyBindings::default(), &legacy);

        state.import_legacy_bindings();

        assert!(state.has_changes);
        assert_eq!(
            state.binding_text(BindingSelection::Global(crate::core::Action::NewConnection)),
            "Ctrl+P"
        );
        assert_eq!(
            state.binding_text(BindingSelection::Local(LocalShortcut::Dismiss)),
            "Ctrl+W"
        );
    }

    #[test]
    fn apply_ui_actions_can_apply_grid_sequence_from_editor_input() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_binding(BindingSelection::GridCommand(GridCommandShortcut::CopyRow));
        state.sequence_input = "zr".to_string();

        KeyBindingsDialog::apply_ui_actions(
            &mut state,
            vec![KeyBindingsDialogUiAction::ApplyGridSequenceInput(
                GridCommandShortcut::CopyRow,
                RecordingMode::Append,
            )],
        );

        let sequences = state.grid_sequences(GridCommandShortcut::CopyRow);
        assert!(sequences.iter().any(|sequence| sequence == "yy"));
        assert!(sequences.iter().any(|sequence| sequence == "zr"));
    }

    #[test]
    fn apply_ui_actions_can_update_filter_and_sequence_input() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());

        KeyBindingsDialog::apply_ui_actions(
            &mut state,
            vec![
                KeyBindingsDialogUiAction::SetFilter("page".to_string()),
                KeyBindingsDialogUiAction::SetSequenceInput("zr".to_string()),
            ],
        );

        assert_eq!(state.filter, "page");
        assert_eq!(state.sequence_input, "zr");
    }
}
