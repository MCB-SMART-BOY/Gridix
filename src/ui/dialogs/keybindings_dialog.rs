//! 快捷键编辑对话框
//!
//! 允许用户自定义全局快捷键与局部作用域快捷键。

use crate::core::{Action, KeyBinding, KeyBindings, KeyCode, KeyModifiers};
use crate::ui::shortcut_tooltip::LocalShortcut;
use eframe::egui::{self, Key, RichText};

const GLOBAL_CATEGORIES: &[&str] = &["全局", "创建", "Tab", "编辑", "缩放"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum BindingScope {
    #[default]
    Global,
    Local,
}

impl BindingScope {
    fn helper_text(self) -> &'static str {
        match self {
            Self::Global => {
                "提示: 全局动作会影响整个工作区；录制时若与其他全局动作冲突会即时提示。"
            }
            Self::Local => {
                "提示: 局部快捷键按作用域生效，可与其他区域重复。录制会覆盖当前作用域整组绑定；点“恢复默认”可回到内置多键方案。"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LocalScopeSection {
    Dialog,
    Sidebar,
    Editor,
}

impl LocalScopeSection {
    fn label(self) -> &'static str {
        match self {
            Self::Dialog => "对话框",
            Self::Sidebar => "侧边栏",
            Self::Editor => "编辑器",
        }
    }

    fn categories(self) -> &'static [&'static str] {
        match self {
            Self::Dialog => &[
                "通用对话框",
                "危险确认",
                "帮助",
                "导入",
                "导出",
                "连接",
                "DDL",
                "历史",
            ],
            Self::Sidebar => &["侧边栏", "筛选"],
            Self::Editor => &["SQL 编辑器"],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ScopeTreeSelection {
    #[default]
    GlobalAll,
    GlobalCategory(&'static str),
    LocalAll,
    LocalSection(LocalScopeSection),
    LocalCategory(&'static str),
}

impl ScopeTreeSelection {
    fn scope(self) -> BindingScope {
        match self {
            Self::GlobalAll | Self::GlobalCategory(_) => BindingScope::Global,
            Self::LocalAll | Self::LocalSection(_) | Self::LocalCategory(_) => BindingScope::Local,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::GlobalAll => "全局动作",
            Self::GlobalCategory(category) => category,
            Self::LocalAll => "局部快捷键",
            Self::LocalSection(section) => section.label(),
            Self::LocalCategory(category) => category,
        }
    }

    fn breadcrumb(self) -> String {
        match self {
            Self::GlobalAll => "全局动作".to_string(),
            Self::GlobalCategory(category) => format!("全局动作 / {category}"),
            Self::LocalAll => "局部快捷键".to_string(),
            Self::LocalSection(section) => format!("局部快捷键 / {}", section.label()),
            Self::LocalCategory(category) => {
                let section = local_scope_section_for_category(category);
                format!("局部快捷键 / {} / {category}", section.label())
            }
        }
    }

    fn matches(self, selection: BindingSelection) -> bool {
        match (self, selection) {
            (Self::GlobalAll, BindingSelection::Global(_)) => true,
            (Self::GlobalCategory(category), BindingSelection::Global(action)) => {
                action.category() == category
            }
            (Self::LocalAll, BindingSelection::Local(_)) => true,
            (Self::LocalSection(section), BindingSelection::Local(shortcut)) => {
                local_scope_section(shortcut) == section
            }
            (Self::LocalCategory(category), BindingSelection::Local(shortcut)) => {
                local_shortcut_category(shortcut) == category
            }
            _ => false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindingSelection {
    Global(Action),
    Local(LocalShortcut),
}

impl BindingSelection {
    fn label(self) -> &'static str {
        match self {
            Self::Global(action) => action.description(),
            Self::Local(shortcut) => local_shortcut_description(shortcut),
        }
    }

    fn category(self) -> &'static str {
        match self {
            Self::Global(action) => action.category(),
            Self::Local(shortcut) => local_shortcut_category(shortcut),
        }
    }

    fn detail(self) -> Option<&'static str> {
        match self {
            Self::Global(action) => Some(action.keymap_name()),
            Self::Local(shortcut) => Some(shortcut.config_key()),
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

/// 快捷键编辑对话框状态
#[derive(Default)]
pub struct KeyBindingsDialogState {
    /// 是否显示对话框
    pub show: bool,
    /// 当前快捷键绑定（编辑中的副本）
    bindings: KeyBindings,
    /// 当前选中的条目
    selected_binding: Option<BindingSelection>,
    /// 当前作用域树筛选
    current_tree: ScopeTreeSelection,
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
        self.show = true;
        self.bindings = bindings.clone();
        self.selected_binding = None;
        self.current_tree = ScopeTreeSelection::GlobalAll;
        self.recording = false;
        self.recorded_key = None;
        self.recorded_modifiers = KeyModifiers::NONE;
        self.recording_mode = RecordingMode::Replace;
        self.filter.clear();
        self.show_issue_only = false;
        self.has_changes = false;
        self.conflict_message = None;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.show = false;
        self.recording = false;
        self.recording_mode = RecordingMode::Replace;
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

    fn visible_bindings(&self) -> Vec<BindingSelection> {
        let filter_lower = self.filter.to_lowercase();
        let mut result = Vec::new();

        for action in Action::all() {
            let selection = BindingSelection::Global(*action);
            if self.matches_filter_for_tree(selection, &filter_lower, self.current_tree)
                && self.matches_issue_filter(selection)
            {
                result.push(selection);
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

        result
    }

    fn matches_search(&self, selection: BindingSelection, filter_lower: &str) -> bool {
        if !filter_lower.is_empty() {
            let label = selection.label().to_lowercase();
            let detail = selection.detail().unwrap_or_default().to_lowercase();
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

        match selection {
            BindingSelection::Global(_) => false,
            BindingSelection::Local(_) => !self.binding_issues(selection).is_empty(),
        }
    }

    fn select_binding(&mut self, selection: BindingSelection) {
        self.selected_binding = Some(selection);
        self.recording = false;
        self.conflict_message = None;
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
        if tree.scope() == BindingScope::Global {
            self.show_issue_only = false;
        }
        self.selected_binding = None;
        self.recording = false;
        self.recording_mode = RecordingMode::Replace;
        self.conflict_message = None;
    }

    fn set_show_issue_only(&mut self, enabled: bool) {
        self.show_issue_only = enabled && self.current_tree.scope() == BindingScope::Local;
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
            BindingSelection::Local(shortcut) => {
                let bindings = shortcut.bindings_for(&self.bindings);
                keybinding_list_text(&bindings)
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
            BindingSelection::Local(shortcut) => {
                if shortcut.is_overridden(&self.bindings) {
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
        }
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
            BindingSelection::Local(shortcut) => {
                self.bindings.remove_local_bindings(shortcut.config_key());
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

    fn clear_button_label(&self) -> Option<&'static str> {
        match self.selected_binding {
            Some(BindingSelection::Global(_)) => Some("清除快捷键"),
            Some(BindingSelection::Local(_)) => Some("恢复默认"),
            None => None,
        }
    }

    fn binding_issues(&self, selection: BindingSelection) -> Vec<BindingIssue> {
        match selection {
            BindingSelection::Global(_) => Vec::new(),
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
        }
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
        for action in Action::all() {
            let selection = BindingSelection::Global(*action);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                count += 1;
            }
        }
        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                count += 1;
            }
        }
        count
    }

    fn issue_count(&self, tree: ScopeTreeSelection) -> usize {
        let filter_lower = self.filter.to_lowercase();
        let mut total = 0;
        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if self.matches_filter_for_tree(selection, &filter_lower, tree) {
                total += self.binding_issues(selection).len();
            }
        }
        total
    }

    fn issue_summary(
        &self,
        tree: ScopeTreeSelection,
    ) -> Vec<(BindingSelection, Vec<BindingIssue>)> {
        let filter_lower = self.filter.to_lowercase();
        let mut summary = Vec::new();

        for shortcut in LocalShortcut::all() {
            let selection = BindingSelection::Local(*shortcut);
            if !self.matches_filter_for_tree(selection, &filter_lower, tree) {
                continue;
            }

            let issues = self.binding_issues(selection);
            if !issues.is_empty() {
                summary.push((selection, issues));
            }
        }

        summary
    }
}

/// 快捷键编辑对话框
pub struct KeyBindingsDialog;

impl KeyBindingsDialog {
    /// 显示对话框
    ///
    /// 返回 Some(KeyBindings) 表示用户保存了更改
    pub fn show(ctx: &egui::Context, state: &mut KeyBindingsDialogState) -> Option<KeyBindings> {
        if !state.show {
            return None;
        }

        let mut result = None;
        let mut should_close = false;

        egui::Window::new("快捷键设置")
            .collapsible(false)
            .resizable(true)
            .default_width(760.0)
            .default_height(560.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("搜索:");
                    ui.add(
                        egui::TextEdit::singleline(&mut state.filter)
                            .desired_width(180.0)
                            .hint_text("输入功能或 keymap 路径..."),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("重置全部为默认").clicked() {
                            state.reset_to_defaults();
                        }
                    });
                });

                ui.add_space(8.0);

                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(220.0);
                        ui.group(|ui| {
                            ui.set_width(220.0);
                            ui.label(RichText::new("作用域树").strong());
                            ui.label(
                                RichText::new("按区域逐层定位，不再靠平铺分类筛选。")
                                    .small()
                                    .weak(),
                            );
                            ui.add_space(6.0);
                            egui::ScrollArea::vertical()
                                .max_height(400.0)
                                .show(ui, |ui| {
                                    show_tree_entry(ui, state, ScopeTreeSelection::GlobalAll, 0);
                                    for category in GLOBAL_CATEGORIES {
                                        show_tree_entry(
                                            ui,
                                            state,
                                            ScopeTreeSelection::GlobalCategory(category),
                                            1,
                                        );
                                    }

                                    ui.add_space(4.0);
                                    show_tree_entry(ui, state, ScopeTreeSelection::LocalAll, 0);
                                    for section in [
                                        LocalScopeSection::Dialog,
                                        LocalScopeSection::Sidebar,
                                        LocalScopeSection::Editor,
                                    ] {
                                        show_tree_entry(
                                            ui,
                                            state,
                                            ScopeTreeSelection::LocalSection(section),
                                            1,
                                        );
                                        for category in section.categories() {
                                            show_tree_entry(
                                                ui,
                                                state,
                                                ScopeTreeSelection::LocalCategory(category),
                                                2,
                                            );
                                        }
                                    }
                                });
                        });
                    });

                    ui.add_space(8.0);

                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(state.current_tree.label()).strong());
                            ui.label(
                                RichText::new(state.current_tree.breadcrumb())
                                    .small()
                                    .weak()
                                    .monospace(),
                            );
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "当前共 {} 项",
                                    state.filtered_count(state.current_tree)
                                ))
                                .small()
                                .weak(),
                            );
                            let issues = state.issue_count(state.current_tree);
                            if issues > 0 {
                                ui.label(
                                    RichText::new(format!("{} 条作用域提醒", issues))
                                        .small()
                                        .color(egui::Color32::from_rgb(245, 189, 130)),
                                );
                            }
                            if state.current_tree.scope() == BindingScope::Local && issues > 0 {
                                let mut show_issue_only = state.show_issue_only;
                                if ui
                                    .checkbox(&mut show_issue_only, "只看有作用域提醒")
                                    .on_hover_text("只显示存在遮蔽或同作用域重叠的局部快捷键")
                                    .changed()
                                {
                                    state.set_show_issue_only(show_issue_only);
                                }
                            }
                        });

                        ui.add_space(4.0);

                        if let Some(msg) = &state.conflict_message {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("⚠").color(egui::Color32::YELLOW));
                                ui.label(RichText::new(msg).color(egui::Color32::YELLOW));
                            });
                            ui.add_space(4.0);
                        }

                        let summary = state.issue_summary(state.current_tree);
                        if !summary.is_empty() {
                            ui.group(|ui| {
                                ui.label(
                                    RichText::new("冲突摘要")
                                        .small()
                                        .strong()
                                        .color(egui::Color32::from_rgb(245, 189, 130)),
                                );
                                ui.label(
                                    RichText::new("点击条目可直接跳到对应快捷键，逐项调整。")
                                        .small()
                                        .weak(),
                                );
                                ui.add_space(4.0);
                                for (selection, issues) in summary.iter().take(6) {
                                    let label =
                                        format!("{} · {} 条提醒", selection.label(), issues.len());
                                    if ui
                                        .small_button(label)
                                        .on_hover_text(selection.detail().unwrap_or_default())
                                        .clicked()
                                    {
                                        state.select_binding(*selection);
                                    }
                                }
                                if summary.len() > 6 {
                                    ui.label(
                                        RichText::new(format!(
                                            "还有 {} 项未展开，请继续用左侧树和搜索缩小范围。",
                                            summary.len() - 6
                                        ))
                                        .small()
                                        .weak(),
                                    );
                                }
                            });
                            ui.add_space(6.0);
                        }

                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                egui::Grid::new("keybindings_grid")
                                    .num_columns(4)
                                    .spacing([20.0, 8.0])
                                    .striped(true)
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("功能").strong());
                                        ui.label(RichText::new("快捷键").strong());
                                        ui.label(RichText::new("分类").strong());
                                        ui.label(RichText::new("来源").strong());
                                        ui.end_row();

                                        for selection in state.visible_bindings() {
                                            let is_selected =
                                                state.selected_binding == Some(selection);
                                            let is_recording = state.recording
                                                && state.selected_binding == Some(selection);

                                            if ui
                                                .selectable_label(is_selected, selection.label())
                                                .clicked()
                                            {
                                                state.select_binding(selection);
                                            }

                                            if is_recording {
                                                let recording_text = if let Some(key) =
                                                    state.recorded_key
                                                {
                                                    KeyBinding::new(key, state.recorded_modifiers)
                                                        .display()
                                                } else if state.recorded_modifiers
                                                    != KeyModifiers::NONE
                                                {
                                                    format!("{}+...", state.recorded_modifiers)
                                                } else {
                                                    "按下快捷键...".to_string()
                                                };

                                                ui.label(
                                                    RichText::new(recording_text)
                                                        .color(egui::Color32::LIGHT_BLUE)
                                                        .italics(),
                                                );
                                            } else {
                                                let binding_text = state.binding_text(selection);
                                                if ui.button(binding_text).clicked() {
                                                    state.begin_recording(
                                                        selection,
                                                        RecordingMode::Replace,
                                                    );
                                                }
                                            }

                                            ui.label(selection.category());
                                            ui.label(state.binding_source(selection));
                                            ui.end_row();
                                        }
                                    });
                            });
                    });
                });

                ui.add_space(8.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if let Some(selection) = state.selected_binding {
                        ui.label(RichText::new(selection.label()).strong());
                        if let Some(detail) = selection.detail() {
                            ui.label(
                                RichText::new(format!("({detail})"))
                                    .small()
                                    .weak()
                                    .monospace(),
                            );
                        }
                    } else {
                        ui.label(RichText::new("请选择一项快捷键进行录制或恢复默认").weak());
                    }
                });

                ui.add_space(6.0);

                if let Some(BindingSelection::Local(shortcut)) = state.selected_binding {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("当前局部绑定:").small().weak());
                        for (index, binding) in state.local_bindings(shortcut).iter().enumerate() {
                            let remove_label = format!("{} ×", binding.display());
                            if ui
                                .small_button(remove_label)
                                .on_hover_text("移除这一组局部绑定")
                                .clicked()
                            {
                                state.remove_local_binding_at(shortcut, index);
                            }
                        }
                    });

                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui.button("替换绑定").clicked() {
                            state.begin_recording(
                                BindingSelection::Local(shortcut),
                                RecordingMode::Replace,
                            );
                        }
                        if ui.button("追加绑定").clicked() {
                            state.begin_recording(
                                BindingSelection::Local(shortcut),
                                RecordingMode::Append,
                            );
                        }
                    });

                    ui.add_space(4.0);

                    let issues = state.binding_issues(BindingSelection::Local(shortcut));
                    if !issues.is_empty() {
                        ui.group(|ui| {
                            ui.label(
                                RichText::new("作用域分析")
                                    .small()
                                    .strong()
                                    .color(egui::Color32::from_rgb(245, 189, 130)),
                            );
                            for issue in issues {
                                ui.label(
                                    RichText::new(format!("• {}", issue.message()))
                                        .small()
                                        .color(egui::Color32::from_rgb(245, 189, 130)),
                                );
                            }
                        });
                        ui.add_space(4.0);
                    }
                }

                if state.recording
                    && let Some(selection) = state.selected_binding
                {
                    ctx.input(|i| {
                        if i.key_pressed(Key::Escape) {
                            state.recording = false;
                            state.recorded_key = None;
                            state.recorded_modifiers = KeyModifiers::NONE;
                            return;
                        }

                        state.recorded_modifiers = KeyModifiers::from_egui(i.modifiers);
                        for event in &i.events {
                            if let egui::Event::Key {
                                key, pressed: true, ..
                            } = event
                                && let Some(key_code) = KeyCode::from_egui_key(*key)
                            {
                                state.recorded_key = Some(key_code);
                                let binding = KeyBinding::new(key_code, state.recorded_modifiers);
                                state.apply_recorded_binding(selection, binding);
                                state.recording = false;
                                break;
                            }
                        }
                    });
                }

                ui.horizontal(|ui| {
                    if let Some(label) = state.clear_button_label()
                        && ui.button(label).clicked()
                    {
                        state.clear_selected_binding();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("取消").clicked() {
                            should_close = true;
                        }

                        let save_text = if state.has_changes {
                            "保存 *"
                        } else {
                            "保存"
                        };
                        if ui.button(save_text).clicked() {
                            result = Some(state.bindings.clone());
                            should_close = true;
                        }
                    });
                });

                ui.add_space(4.0);
                ui.label(
                    RichText::new(if state.recording {
                        state.recording_hint()
                    } else {
                        state.current_tree.scope().helper_text()
                    })
                    .small()
                    .weak(),
                );
            });

        if should_close {
            state.close();
        }

        result
    }
}

fn show_tree_entry(
    ui: &mut egui::Ui,
    state: &mut KeyBindingsDialogState,
    tree: ScopeTreeSelection,
    depth: usize,
) {
    let count = state.filtered_count(tree);
    let issues = state.issue_count(tree);
    let mut label = format!("{} ({count})", tree.label());
    if issues > 0 {
        label.push_str(&format!(" · {issues}"));
    }

    ui.horizontal(|ui| {
        ui.add_space(depth as f32 * 14.0);
        let text = if issues > 0 {
            RichText::new(label).color(egui::Color32::from_rgb(245, 189, 130))
        } else {
            RichText::new(label)
        };
        let response = ui.selectable_label(state.current_tree == tree, text);
        if response.clicked() {
            state.select_tree(tree);
        }
        response.on_hover_text(tree.breadcrumb());
    });
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

fn local_shortcut_description(shortcut: LocalShortcut) -> &'static str {
    match shortcut {
        LocalShortcut::Confirm => "确认",
        LocalShortcut::Cancel => "取消",
        LocalShortcut::Dismiss => "关闭当前对话框",
        LocalShortcut::DangerConfirm => "危险确认",
        LocalShortcut::DangerCancel => "危险取消",
        LocalShortcut::HelpScrollUp => "帮助页向上滚动",
        LocalShortcut::HelpScrollDown => "帮助页向下滚动",
        LocalShortcut::HelpPageUp => "帮助页上翻",
        LocalShortcut::HelpPageDown => "帮助页下翻",
        LocalShortcut::SidebarItemPrev => "侧边栏上一项",
        LocalShortcut::SidebarItemNext => "侧边栏下一项",
        LocalShortcut::SidebarItemStart => "侧边栏跳到开头",
        LocalShortcut::SidebarItemEnd => "侧边栏跳到结尾",
        LocalShortcut::SidebarMoveLeft => "侧边栏向左返回",
        LocalShortcut::SidebarMoveRight => "侧边栏向右进入",
        LocalShortcut::SidebarToggle => "侧边栏切换/启用",
        LocalShortcut::SidebarDelete => "侧边栏删除",
        LocalShortcut::SidebarEdit => "侧边栏编辑连接",
        LocalShortcut::SidebarRename => "侧边栏重命名",
        LocalShortcut::SidebarRefresh => "侧边栏刷新",
        LocalShortcut::SidebarActivate => "侧边栏激活",
        LocalShortcut::FilterAdd => "新增筛选条件",
        LocalShortcut::FilterDelete => "删除筛选条件",
        LocalShortcut::FilterClearAll => "清空全部筛选",
        LocalShortcut::FilterColumnNext => "筛选列切到下一项",
        LocalShortcut::FilterColumnPrev => "筛选列切到上一项",
        LocalShortcut::FilterOperatorNext => "筛选运算符下一项",
        LocalShortcut::FilterOperatorPrev => "筛选运算符上一项",
        LocalShortcut::FilterLogicToggle => "切换 AND/OR",
        LocalShortcut::FilterFocusInput => "聚焦筛选输入框",
        LocalShortcut::FilterCaseToggle => "切换大小写敏感",
        LocalShortcut::ExportFormatCsv => "导出切到 CSV",
        LocalShortcut::ExportFormatTsv => "导出切到 TSV",
        LocalShortcut::ExportFormatSql => "导出切到 SQL",
        LocalShortcut::ExportFormatJson => "导出切到 JSON",
        LocalShortcut::ExportCyclePrev => "导出格式向前切换",
        LocalShortcut::ExportCycleNext => "导出格式向后切换",
        LocalShortcut::ExportColumnPrev => "导出列选择上一项",
        LocalShortcut::ExportColumnNext => "导出列选择下一项",
        LocalShortcut::ExportColumnStart => "导出列跳到开头",
        LocalShortcut::ExportColumnEnd => "导出列跳到结尾",
        LocalShortcut::ExportColumnToggle => "切换导出列选中",
        LocalShortcut::ExportColumnsToggleAll => "导出列全选/全不选",
        LocalShortcut::SqlExecute => "执行 SQL",
        LocalShortcut::SqlExplain => "执行 EXPLAIN",
        LocalShortcut::SqlClear => "清空 SQL 编辑器",
        LocalShortcut::SqlAutocompleteTrigger => "手动触发补全",
        LocalShortcut::SqlAutocompleteConfirm => "确认补全",
        LocalShortcut::SqlHistoryPrev => "SQL 历史上一条",
        LocalShortcut::SqlHistoryNext => "SQL 历史下一条",
        LocalShortcut::SqlHistoryBrowse => "打开 SQL 历史",
        LocalShortcut::ImportRefresh => "刷新导入预览",
        LocalShortcut::ImportFormatSql => "导入切到 SQL",
        LocalShortcut::ImportFormatCsv => "导入切到 CSV",
        LocalShortcut::ImportFormatTsv => "导入切到 TSV",
        LocalShortcut::ImportFormatJson => "导入切到 JSON",
        LocalShortcut::ImportCyclePrev => "导入格式向前切换",
        LocalShortcut::ImportCycleNext => "导入格式向后切换",
        LocalShortcut::ConnectionTypeSqlite => "连接切到 SQLite",
        LocalShortcut::ConnectionTypePostgres => "连接切到 PostgreSQL",
        LocalShortcut::ConnectionTypeMySql => "连接切到 MySQL",
        LocalShortcut::ConnectionTypePrev => "连接类型向前切换",
        LocalShortcut::ConnectionTypeNext => "连接类型向后切换",
        LocalShortcut::DdlColumnPrev => "DDL 列上一项",
        LocalShortcut::DdlColumnNext => "DDL 列下一项",
        LocalShortcut::DdlColumnStart => "DDL 列跳到开头",
        LocalShortcut::DdlColumnEnd => "DDL 列跳到结尾",
        LocalShortcut::DdlColumnDelete => "DDL 删除列",
        LocalShortcut::DdlColumnAddBelow => "DDL 在下方新增列",
        LocalShortcut::DdlColumnAddAbove => "DDL 在上方新增列",
        LocalShortcut::DdlColumnTogglePrimaryKey => "DDL 切换主键",
        LocalShortcut::SqliteBrowseFile => "浏览 SQLite 文件",
        LocalShortcut::FormatSelectionCycle => "循环切换格式选项",
        LocalShortcut::HistoryClear => "清空查询历史",
        LocalShortcut::HistoryPrev => "历史面板上一项",
        LocalShortcut::HistoryNext => "历史面板下一项",
        LocalShortcut::HistoryStart => "历史面板跳到开头",
        LocalShortcut::HistoryEnd => "历史面板跳到结尾",
        LocalShortcut::HistoryPageUp => "历史面板上翻",
        LocalShortcut::HistoryPageDown => "历史面板下翻",
        LocalShortcut::HistoryUse => "使用选中历史 SQL",
    }
}

fn local_shortcut_category(shortcut: LocalShortcut) -> &'static str {
    match shortcut {
        LocalShortcut::Confirm
        | LocalShortcut::Cancel
        | LocalShortcut::Dismiss
        | LocalShortcut::FormatSelectionCycle => "通用对话框",
        LocalShortcut::DangerConfirm | LocalShortcut::DangerCancel => "危险确认",
        LocalShortcut::HelpScrollUp
        | LocalShortcut::HelpScrollDown
        | LocalShortcut::HelpPageUp
        | LocalShortcut::HelpPageDown => "帮助",
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
        | LocalShortcut::SidebarActivate => "侧边栏",
        LocalShortcut::FilterAdd
        | LocalShortcut::FilterDelete
        | LocalShortcut::FilterClearAll
        | LocalShortcut::FilterColumnNext
        | LocalShortcut::FilterColumnPrev
        | LocalShortcut::FilterOperatorNext
        | LocalShortcut::FilterOperatorPrev
        | LocalShortcut::FilterLogicToggle
        | LocalShortcut::FilterFocusInput
        | LocalShortcut::FilterCaseToggle => "筛选",
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
        | LocalShortcut::ExportColumnsToggleAll => "导出",
        LocalShortcut::SqlExecute
        | LocalShortcut::SqlExplain
        | LocalShortcut::SqlClear
        | LocalShortcut::SqlAutocompleteTrigger
        | LocalShortcut::SqlAutocompleteConfirm
        | LocalShortcut::SqlHistoryPrev
        | LocalShortcut::SqlHistoryNext
        | LocalShortcut::SqlHistoryBrowse => "SQL 编辑器",
        LocalShortcut::ImportRefresh
        | LocalShortcut::ImportFormatSql
        | LocalShortcut::ImportFormatCsv
        | LocalShortcut::ImportFormatTsv
        | LocalShortcut::ImportFormatJson
        | LocalShortcut::ImportCyclePrev
        | LocalShortcut::ImportCycleNext => "导入",
        LocalShortcut::ConnectionTypeSqlite
        | LocalShortcut::ConnectionTypePostgres
        | LocalShortcut::ConnectionTypeMySql
        | LocalShortcut::ConnectionTypePrev
        | LocalShortcut::ConnectionTypeNext
        | LocalShortcut::SqliteBrowseFile => "连接",
        LocalShortcut::DdlColumnPrev
        | LocalShortcut::DdlColumnNext
        | LocalShortcut::DdlColumnStart
        | LocalShortcut::DdlColumnEnd
        | LocalShortcut::DdlColumnDelete
        | LocalShortcut::DdlColumnAddBelow
        | LocalShortcut::DdlColumnAddAbove
        | LocalShortcut::DdlColumnTogglePrimaryKey => "DDL",
        LocalShortcut::HistoryClear
        | LocalShortcut::HistoryPrev
        | LocalShortcut::HistoryNext
        | LocalShortcut::HistoryStart
        | LocalShortcut::HistoryEnd
        | LocalShortcut::HistoryPageUp
        | LocalShortcut::HistoryPageDown
        | LocalShortcut::HistoryUse => "历史",
    }
}

fn local_scope_section(shortcut: LocalShortcut) -> LocalScopeSection {
    local_scope_section_for_category(local_shortcut_category(shortcut))
}

fn local_scope_section_for_category(category: &str) -> LocalScopeSection {
    match category {
        "通用对话框" | "危险确认" | "帮助" | "导入" | "导出" | "连接" | "DDL" | "历史" => {
            LocalScopeSection::Dialog
        }
        "侧边栏" | "筛选" => LocalScopeSection::Sidebar,
        "SQL 编辑器" => LocalScopeSection::Editor,
        _ => LocalScopeSection::Dialog,
    }
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
        | LocalShortcut::FilterCaseToggle => &["sidebar.filters", "sidebar.list"],
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
        | LocalShortcut::SqlHistoryBrowse => &["editor.sql"],
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
    use super::{
        BindingIssue, BindingSelection, KeyBindingsDialogState, LocalScopeSection, RecordingMode,
        ScopeTreeSelection,
    };
    use crate::core::{KeyBinding, KeyBindings, KeyCode};
    use crate::ui::shortcut_tooltip::LocalShortcut;

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
    fn local_scope_tree_filters_to_selected_section() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::LocalSection(LocalScopeSection::Sidebar));

        let visible = state.visible_bindings();

        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::SidebarItemPrev)));
        assert!(visible.contains(&BindingSelection::Local(LocalShortcut::FilterAdd)));
        assert!(!visible.contains(&BindingSelection::Local(LocalShortcut::HelpScrollUp)));
        assert!(!visible.contains(&BindingSelection::Global(crate::core::Action::ShowHelp)));
    }

    #[test]
    fn search_and_tree_counts_follow_same_filter() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::LocalCategory("帮助"));
        state.filter = "page".to_string();

        assert_eq!(
            state.filtered_count(ScopeTreeSelection::LocalCategory("帮助")),
            2
        );
        let expected_issues: usize = state
            .visible_bindings()
            .into_iter()
            .map(|selection| state.binding_issues(selection).len())
            .sum();
        assert_eq!(
            state.issue_count(ScopeTreeSelection::LocalCategory("帮助")),
            expected_issues
        );
    }

    #[test]
    fn issue_only_filter_keeps_only_shortcuts_with_issues() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::LocalAll);
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
                .all(|selection| !state.binding_issues(*selection).is_empty())
        );
        assert!(visible.len() < all_visible.len());
    }

    #[test]
    fn issue_summary_matches_visible_issue_entries() {
        let mut state = KeyBindingsDialogState::default();
        state.open(&KeyBindings::default());
        state.select_tree(ScopeTreeSelection::LocalCategory("帮助"));

        let summary = state.issue_summary(ScopeTreeSelection::LocalCategory("帮助"));

        assert!(summary.iter().all(|(selection, issues)| matches!(
            selection,
            BindingSelection::Local(_)
        ) && !issues.is_empty()));
        assert_eq!(
            summary
                .iter()
                .map(|(_, issues)| issues.len())
                .sum::<usize>(),
            state.issue_count(ScopeTreeSelection::LocalCategory("帮助"))
        );
    }
}
