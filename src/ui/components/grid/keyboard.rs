//! 键盘输入处理（Helix 风格）
//!
//! ## Normal 模式键位
//! - `h/j/k/l`: 移动光标（Grid 上下文：四向移动）
//! - `w/b`: 右/左移一列
//! - `e`: 跳转到行尾
//! - `gh/gl`: 行首/行尾
//! - `gg/G`: 文件首/尾
//! - `Ctrl+u`: 向上翻半页
//! - `PageUp/PageDown`: 翻页
//! - `i/a/c`: 进入插入模式
//! - `v`: 进入选择模式
//! - `x`: 选择整行
//! - `%`: 选择全部
//! - `;`: 折叠选择到单个光标
//! - `dd`: 标记删除当前行
//! - `yy`: 复制整行
//! - `p`: 粘贴
//! - `u/U`: 撤销/取消删除标记
//! - `/`: 打开筛选面板
//! - `f`: 为当前列添加筛选
//! - `o/O`: 添加新行
//! - `:w`: 保存修改
//! - `q`: 放弃修改
//! - `Ctrl+R`: 刷新表格数据
//! - `Space+d`: 标记删除行
//! - `Ctrl+S`: 保存修改
//!
//! ## 视图模式 (z 前缀)
//! - `zz/zc`: 将当前行滚动到屏幕中央
//! - `zt`: 将当前行滚动到屏幕顶部
//! - `zb`: 将当前行滚动到屏幕底部
//!
//! ## 数字计数
//! - `1-9`: 输入计数前缀（如 10j 向下移动10行）
//! - `0`: 追加到已有计数
//! - `Backspace`: 回退计数数字

#![allow(clippy::too_many_arguments)]

use super::actions::DataGridActions;
use super::filter::ColumnFilter;
use super::mode::GridMode;
use super::state::DataGridState;
use crate::core::{KeyBinding, KeyBindings, KeyCode, KeyModifiers};
use crate::database::QueryResult;
use egui::{self, Key};
use tracing::debug;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GridCommandShortcut {
    OpenFilter,
    AddRowBelow,
    AddRowAbove,
    Save,
    Discard,
    JumpFileStart,
    JumpFileEnd,
    JumpLineStart,
    JumpLineEnd,
    ScrollCenter,
    ScrollTop,
    ScrollBottom,
    DeleteRow,
    CopyRow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GridSequenceStep {
    binding: KeyBinding,
    token: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GridSequence {
    raw: String,
    steps: Vec<GridSequenceStep>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GridSequenceConflictKind {
    Exact,
    Prefix,
}

enum GridNormalInput {
    Action {
        action: GridKeyAction,
        sequence: String,
    },
    StartPrefix {
        token: String,
        preserve_count: bool,
    },
}

/// 命令缓冲区状态
#[derive(Default)]
struct CmdBuffer {
    keys: String,
    count: Option<usize>,
}

impl CmdBuffer {
    fn clear(&mut self) {
        self.keys.clear();
        self.count = None;
    }

    fn start_prefix(&mut self, prefix: &str) {
        self.keys.clear();
        self.keys.push_str(prefix);
        self.count = None;
    }

    fn get_count(&self) -> usize {
        self.count.unwrap_or(1)
    }
}

impl GridCommandShortcut {
    pub(crate) const ALL: [Self; 14] = [
        Self::OpenFilter,
        Self::AddRowBelow,
        Self::AddRowAbove,
        Self::Save,
        Self::Discard,
        Self::JumpFileStart,
        Self::JumpFileEnd,
        Self::JumpLineStart,
        Self::JumpLineEnd,
        Self::ScrollCenter,
        Self::ScrollTop,
        Self::ScrollBottom,
        Self::DeleteRow,
        Self::CopyRow,
    ];

    pub(crate) fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub(crate) fn config_key(self) -> &'static str {
        match self {
            Self::OpenFilter => "grid.normal.open_filter",
            Self::AddRowBelow => "grid.normal.add_row_below",
            Self::AddRowAbove => "grid.normal.add_row_above",
            Self::Save => "grid.normal.save",
            Self::Discard => "grid.normal.discard",
            Self::JumpFileStart => "grid.normal.jump_file_start",
            Self::JumpFileEnd => "grid.normal.jump_file_end",
            Self::JumpLineStart => "grid.normal.jump_line_start",
            Self::JumpLineEnd => "grid.normal.jump_line_end",
            Self::ScrollCenter => "grid.normal.scroll_center",
            Self::ScrollTop => "grid.normal.scroll_top",
            Self::ScrollBottom => "grid.normal.scroll_bottom",
            Self::DeleteRow => "grid.normal.delete_row",
            Self::CopyRow => "grid.normal.copy_row",
        }
    }

    pub(crate) fn default_sequences(self) -> &'static [&'static str] {
        match self {
            Self::OpenFilter => &["/"],
            Self::AddRowBelow => &["o"],
            Self::AddRowAbove => &["O"],
            Self::Save => &["Ctrl+S", ":w"],
            Self::Discard => &["q", ":q"],
            Self::JumpFileStart => &["gg"],
            Self::JumpFileEnd => &["G"],
            Self::JumpLineStart => &["gh"],
            Self::JumpLineEnd => &["gl"],
            Self::ScrollCenter => &["zz"],
            Self::ScrollTop => &["zt"],
            Self::ScrollBottom => &["zb"],
            Self::DeleteRow => &["dd", "Space+d"],
            Self::CopyRow => &["yy"],
        }
    }

    fn action(self) -> GridKeyAction {
        match self {
            Self::OpenFilter => GridKeyAction::OpenFilterPanel,
            Self::AddRowBelow => GridKeyAction::AddRowBelow,
            Self::AddRowAbove => GridKeyAction::AddRowAbove,
            Self::Save => GridKeyAction::SaveChanges,
            Self::Discard => GridKeyAction::DiscardChanges,
            Self::JumpFileStart => GridKeyAction::JumpFileStart,
            Self::JumpFileEnd => GridKeyAction::JumpFileEnd,
            Self::JumpLineStart => GridKeyAction::JumpLineStart,
            Self::JumpLineEnd => GridKeyAction::JumpLineEnd,
            Self::ScrollCenter => GridKeyAction::ScrollCurrentRowCenter,
            Self::ScrollTop => GridKeyAction::ScrollCurrentRowTop,
            Self::ScrollBottom => GridKeyAction::ScrollCurrentRowBottom,
            Self::DeleteRow => GridKeyAction::DeleteRow,
            Self::CopyRow => GridKeyAction::CopyRow,
        }
    }

    fn preserve_count_on_prefix(self) -> bool {
        matches!(
            self,
            Self::JumpFileStart | Self::JumpLineStart | Self::JumpLineEnd
        )
    }
}

fn effective_grid_command_sequences(
    keybindings: &KeyBindings,
    command: GridCommandShortcut,
) -> Vec<String> {
    if let Some(sequences) = keybindings.local_sequences_for(command.config_key()) {
        let mut values = Vec::new();
        for sequence in sequences {
            if !values.iter().any(|existing| existing == sequence) {
                values.push(sequence.clone());
            }
        }
        if !values.is_empty() {
            return values;
        }
    }

    command
        .default_sequences()
        .iter()
        .map(|sequence| (*sequence).to_string())
        .collect()
}

pub(crate) fn grid_command_shortcuts(
    keybindings: &KeyBindings,
    command: GridCommandShortcut,
) -> Vec<String> {
    effective_grid_command_sequences(keybindings, command)
}

pub(super) fn mode_help_text(mode: GridMode, keybindings: &KeyBindings) -> String {
    match mode {
        GridMode::Normal => format!(
            "hjkl 移动 | i/a/c 编辑 | v/x/% 选择 | {} 复制 | p 粘贴 | {} 筛选 | {}/{} 跳转",
            first_grid_shortcut(keybindings, GridCommandShortcut::CopyRow),
            first_grid_shortcut(keybindings, GridCommandShortcut::OpenFilter),
            first_grid_shortcut(keybindings, GridCommandShortcut::JumpFileStart),
            first_grid_shortcut(keybindings, GridCommandShortcut::JumpFileEnd),
        ),
        GridMode::Insert => "Esc 退出 | Enter 确认".to_string(),
        GridMode::Select => {
            "hjkl 扩展 | d 清空 | c 编辑 | y 复制 | x 整行 | Esc/; 退出".to_string()
        }
    }
}

fn first_grid_shortcut(keybindings: &KeyBindings, command: GridCommandShortcut) -> String {
    grid_command_shortcuts(keybindings, command)
        .into_iter()
        .next()
        .unwrap_or_default()
}

pub(crate) fn normalize_grid_command_sequence(sequence: &str) -> Option<String> {
    let parsed = parse_grid_sequence(sequence)?;
    Some(match parsed.steps.as_slice() {
        [step] => step.token.clone(),
        [first, second] if first.token == " " => format!("Space+{}", second.token),
        [first, second] if first.token == ":" => format!(":{}", second.token),
        steps => steps.iter().map(|step| step.token.as_str()).collect(),
    })
}

pub(crate) fn grid_command_sequence_conflict(
    left: &str,
    right: &str,
) -> Option<GridSequenceConflictKind> {
    let left = parse_grid_sequence(left)?;
    let right = parse_grid_sequence(right)?;

    if left.steps == right.steps {
        return Some(GridSequenceConflictKind::Exact);
    }

    if is_grid_sequence_prefix(&left.steps, &right.steps)
        || is_grid_sequence_prefix(&right.steps, &left.steps)
    {
        return Some(GridSequenceConflictKind::Prefix);
    }

    None
}

fn parse_grid_sequence(sequence: &str) -> Option<GridSequence> {
    let sequence = sequence.trim();
    if sequence.is_empty() {
        return None;
    }

    let steps = if let Some(rest) = sequence.strip_prefix("Space+") {
        vec![space_step(), parse_grid_step(rest)?]
    } else if let Some(rest) = sequence.strip_prefix(':') {
        if rest.is_empty() {
            return None;
        }
        vec![colon_step(), parse_grid_step(rest)?]
    } else if !sequence.contains('+') && sequence.chars().count() == 2 {
        sequence
            .chars()
            .map(parse_grid_compact_char)
            .collect::<Option<Vec<_>>>()?
    } else {
        vec![parse_grid_step(sequence)?]
    };

    Some(GridSequence {
        raw: sequence.to_string(),
        steps,
    })
}

fn parse_grid_compact_char(ch: char) -> Option<GridSequenceStep> {
    match ch {
        ':' => Some(colon_step()),
        ' ' => Some(space_step()),
        _ => parse_grid_step(&ch.to_string()),
    }
}

fn parse_grid_step(token: &str) -> Option<GridSequenceStep> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }

    let binding = if token == ":" {
        KeyBinding::new(KeyCode::Semicolon, KeyModifiers::SHIFT)
    } else if token.eq_ignore_ascii_case("space") {
        KeyBinding::key_only(KeyCode::Space)
    } else if token.chars().count() == 1
        && token.chars().next()?.is_ascii_uppercase()
        && token.chars().next()?.is_ascii_alphabetic()
    {
        KeyBinding::new(char_to_keycode(token.chars().next()?)?, KeyModifiers::SHIFT)
    } else {
        KeyBinding::parse(token)?
    };

    let normalized = normalize_grid_step_token(token, &binding);
    Some(GridSequenceStep {
        binding,
        token: normalized,
    })
}

fn normalize_grid_step_token(token: &str, binding: &KeyBinding) -> String {
    if binding.key == KeyCode::Space && binding.modifiers == KeyModifiers::NONE {
        " ".to_string()
    } else if binding.key == KeyCode::Semicolon && binding.modifiers.shift {
        ":".to_string()
    } else {
        token.to_string()
    }
}

fn char_to_keycode(ch: char) -> Option<KeyCode> {
    Some(match ch.to_ascii_uppercase() {
        'A' => KeyCode::A,
        'B' => KeyCode::B,
        'C' => KeyCode::C,
        'D' => KeyCode::D,
        'E' => KeyCode::E,
        'F' => KeyCode::F,
        'G' => KeyCode::G,
        'H' => KeyCode::H,
        'I' => KeyCode::I,
        'J' => KeyCode::J,
        'K' => KeyCode::K,
        'L' => KeyCode::L,
        'M' => KeyCode::M,
        'N' => KeyCode::N,
        'O' => KeyCode::O,
        'P' => KeyCode::P,
        'Q' => KeyCode::Q,
        'R' => KeyCode::R,
        'S' => KeyCode::S,
        'T' => KeyCode::T,
        'U' => KeyCode::U,
        'V' => KeyCode::V,
        'W' => KeyCode::W,
        'X' => KeyCode::X,
        'Y' => KeyCode::Y,
        'Z' => KeyCode::Z,
        _ => return None,
    })
}

fn is_grid_sequence_prefix(left: &[GridSequenceStep], right: &[GridSequenceStep]) -> bool {
    left.len() < right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| left.token == right.token)
}

fn space_step() -> GridSequenceStep {
    GridSequenceStep {
        binding: KeyBinding::key_only(KeyCode::Space),
        token: " ".to_string(),
    }
}

fn colon_step() -> GridSequenceStep {
    GridSequenceStep {
        binding: KeyBinding::new(KeyCode::Semicolon, KeyModifiers::SHIFT),
        token: ":".to_string(),
    }
}

fn input_matches_binding(i: &egui::InputState, binding: &KeyBinding) -> bool {
    binding.modifiers.matches(&i.modifiers) && i.key_pressed(binding.key.to_egui_key())
}

fn detect_configured_normal_input(
    i: &egui::InputState,
    cmd: &CmdBuffer,
    keybindings: &KeyBindings,
) -> Option<GridNormalInput> {
    let commands = [
        GridCommandShortcut::OpenFilter,
        GridCommandShortcut::AddRowBelow,
        GridCommandShortcut::AddRowAbove,
        GridCommandShortcut::Save,
        GridCommandShortcut::Discard,
        GridCommandShortcut::JumpFileStart,
        GridCommandShortcut::JumpFileEnd,
        GridCommandShortcut::JumpLineStart,
        GridCommandShortcut::JumpLineEnd,
        GridCommandShortcut::ScrollCenter,
        GridCommandShortcut::ScrollTop,
        GridCommandShortcut::ScrollBottom,
        GridCommandShortcut::DeleteRow,
        GridCommandShortcut::CopyRow,
    ];

    if !cmd.keys.is_empty() {
        for command in commands {
            for sequence in effective_grid_command_sequences(keybindings, command)
                .into_iter()
                .filter_map(|sequence| parse_grid_sequence(&sequence))
            {
                if sequence.steps.len() == 2
                    && sequence.steps[0].token == cmd.keys
                    && input_matches_binding(i, &sequence.steps[1].binding)
                {
                    return Some(GridNormalInput::Action {
                        action: if command == GridCommandShortcut::JumpFileStart
                            && cmd.count.is_some()
                        {
                            GridKeyAction::JumpToCountedRow
                        } else {
                            command.action()
                        },
                        sequence: sequence.raw,
                    });
                }
            }
        }
        return None;
    }

    for command in commands {
        let mut prefix_candidate: Option<String> = None;
        for sequence in effective_grid_command_sequences(keybindings, command)
            .into_iter()
            .filter_map(|sequence| parse_grid_sequence(&sequence))
        {
            if sequence.steps.len() == 1 && input_matches_binding(i, &sequence.steps[0].binding) {
                return Some(GridNormalInput::Action {
                    action: if command == GridCommandShortcut::JumpFileEnd && cmd.count.is_some() {
                        GridKeyAction::JumpToCountedRow
                    } else {
                        command.action()
                    },
                    sequence: sequence.raw,
                });
            }
            if sequence.steps.len() == 2
                && input_matches_binding(i, &sequence.steps[0].binding)
                && prefix_candidate.is_none()
            {
                prefix_candidate = Some(sequence.steps[0].token.clone());
            }
        }

        if let Some(token) = prefix_candidate {
            return Some(GridNormalInput::StartPrefix {
                token,
                preserve_count: command.preserve_count_on_prefix(),
            });
        }
    }

    None
}

fn has_pressed_key_event(events: &[egui::Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, egui::Event::Key { pressed: true, .. }))
}

fn should_clear_pending_command(events: &[egui::Event], cmd: &CmdBuffer) -> bool {
    has_pressed_key_event(events) && (!cmd.keys.is_empty() || cmd.count.is_some())
}

fn clear_pending_command_on_unhandled_key(i: &egui::InputState, cmd: &mut CmdBuffer) {
    if should_clear_pending_command(&i.events, cmd) {
        cmd.clear();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridKeyAction {
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    MoveWordRight,
    MoveWordLeft,
    JumpLineEnd,
    JumpLineStart,
    JumpFileStart,
    JumpFileEnd,
    JumpToCountedRow,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
    OpenFilterPanel,
    AddColumnFilter,
    AddRowBelow,
    AddRowAbove,
    SaveChanges,
    DiscardChanges,
    Refresh,
    EnterInsert,
    AppendInsert,
    ChangeCell,
    ReplaceCell,
    EnterSelect,
    SelectRow,
    SelectAll,
    CollapseSelection,
    Paste,
    UndoCellChange,
    UnmarkDelete,
    ScrollCurrentRowCenter,
    ScrollCurrentRowTop,
    ScrollCurrentRowBottom,
    DeleteRow,
    CopyRow,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridSelectAction {
    MoveLeft,
    MoveDown,
    MoveUp,
    MoveRight,
    MoveWordRight,
    MoveWordLeft,
    DeleteSelection,
    ChangeSelection,
    CopySelection,
    SelectRow,
    ExitSelect,
}

fn detect_normal_key_action(i: &egui::InputState, cmd: &mut CmdBuffer) -> Option<GridKeyAction> {
    if cmd.keys.is_empty() {
        if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
            return Some(GridKeyAction::MoveLeft);
        }
        if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
            return Some(GridKeyAction::MoveDown);
        }
        if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
            return Some(GridKeyAction::MoveUp);
        }
        if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
            return Some(GridKeyAction::MoveRight);
        }
        if i.key_pressed(Key::W) && !i.modifiers.ctrl {
            return Some(GridKeyAction::MoveWordRight);
        }
        if i.key_pressed(Key::B) && !i.modifiers.ctrl {
            return Some(GridKeyAction::MoveWordLeft);
        }
        if i.key_pressed(Key::E) && !i.modifiers.ctrl {
            return Some(GridKeyAction::JumpLineEnd);
        }
        if i.key_pressed(Key::Num0) && cmd.count.is_none() {
            return Some(GridKeyAction::JumpLineStart);
        }
        if i.key_pressed(Key::Num4) && i.modifiers.shift {
            return Some(GridKeyAction::JumpLineEnd);
        }
        if i.key_pressed(Key::Num6) && i.modifiers.shift {
            return Some(GridKeyAction::JumpLineStart);
        }
        if i.key_pressed(Key::Home) && i.modifiers.ctrl {
            return Some(GridKeyAction::JumpFileStart);
        }
        if i.key_pressed(Key::Home) {
            return Some(GridKeyAction::JumpLineStart);
        }
        if i.key_pressed(Key::End) && i.modifiers.ctrl {
            return Some(GridKeyAction::JumpFileEnd);
        }
        if i.key_pressed(Key::End) {
            return Some(GridKeyAction::JumpLineEnd);
        }
        if i.modifiers.ctrl && i.key_pressed(Key::U) {
            return Some(GridKeyAction::HalfPageUp);
        }
        if i.modifiers.ctrl && i.key_pressed(Key::D) {
            return Some(GridKeyAction::HalfPageDown);
        }
        if i.key_pressed(Key::PageUp) {
            return Some(GridKeyAction::PageUp);
        }
        if i.key_pressed(Key::PageDown) {
            return Some(GridKeyAction::PageDown);
        }
        if i.key_pressed(Key::F) && !i.modifiers.ctrl {
            return Some(GridKeyAction::AddColumnFilter);
        }
        if i.modifiers.ctrl && i.key_pressed(Key::R) {
            return Some(GridKeyAction::Refresh);
        }
        if i.key_pressed(Key::I) && !i.modifiers.ctrl {
            return Some(GridKeyAction::EnterInsert);
        }
        if i.key_pressed(Key::A) && !i.modifiers.ctrl {
            return Some(GridKeyAction::AppendInsert);
        }
        if i.key_pressed(Key::C) && !i.modifiers.ctrl {
            return Some(GridKeyAction::ChangeCell);
        }
        if i.key_pressed(Key::R) && !i.modifiers.ctrl {
            return Some(GridKeyAction::ReplaceCell);
        }
        if i.key_pressed(Key::V) && !i.modifiers.shift {
            return Some(GridKeyAction::EnterSelect);
        }
        if i.key_pressed(Key::X) && !i.modifiers.shift {
            return Some(GridKeyAction::SelectRow);
        }
        if i.key_pressed(Key::Num5) && i.modifiers.shift {
            return Some(GridKeyAction::SelectAll);
        }
        if i.key_pressed(Key::Semicolon) && !i.modifiers.shift {
            return Some(GridKeyAction::CollapseSelection);
        }
        if i.key_pressed(Key::P) {
            return Some(GridKeyAction::Paste);
        }
        if i.key_pressed(Key::U) && !i.modifiers.shift && !i.modifiers.ctrl {
            return Some(GridKeyAction::UndoCellChange);
        }
        if i.key_pressed(Key::U) && i.modifiers.shift {
            return Some(GridKeyAction::UnmarkDelete);
        }
        if i.key_pressed(Key::Escape) {
            return Some(GridKeyAction::Escape);
        }
    }

    None
}

fn detect_select_key_action(i: &egui::InputState) -> Option<GridSelectAction> {
    if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
        return Some(GridSelectAction::MoveLeft);
    }
    if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
        return Some(GridSelectAction::MoveDown);
    }
    if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
        return Some(GridSelectAction::MoveUp);
    }
    if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
        return Some(GridSelectAction::MoveRight);
    }
    if i.key_pressed(Key::W) {
        return Some(GridSelectAction::MoveWordRight);
    }
    if i.key_pressed(Key::B) {
        return Some(GridSelectAction::MoveWordLeft);
    }
    if i.key_pressed(Key::D) {
        return Some(GridSelectAction::DeleteSelection);
    }
    if i.key_pressed(Key::C) {
        return Some(GridSelectAction::ChangeSelection);
    }
    if i.key_pressed(Key::Y) {
        return Some(GridSelectAction::CopySelection);
    }
    if i.key_pressed(Key::X) {
        return Some(GridSelectAction::SelectRow);
    }
    if i.key_pressed(Key::Escape) || (i.key_pressed(Key::Semicolon) && !i.modifiers.shift) {
        return Some(GridSelectAction::ExitSelect);
    }

    None
}

fn mark_current_row_for_delete(
    state: &mut DataGridState,
    actions: &mut DataGridActions,
    shortcut: &str,
) {
    let row_idx = state.cursor.0;
    if !state.rows_to_delete.contains(&row_idx) {
        state.rows_to_delete.push(row_idx);
        actions.message = Some(format!("已标记删除第 {} 行 ({})", row_idx + 1, shortcut));
    }
}

fn copy_current_row(
    state: &mut DataGridState,
    filtered_rows: &[(usize, &Vec<String>)],
    actions: &mut DataGridActions,
) {
    if let Some((_, row_data)) = filtered_rows.get(state.cursor.0) {
        let row_text = row_data.join("\t");
        state.clipboard = Some(row_text);
        actions.message = Some(format!("已复制第 {} 行 (yy)", state.cursor.0 + 1));
    }
}

fn display_sequence<'a>(sequence: &'a str, fallback: &'a str) -> &'a str {
    if sequence.is_empty() {
        fallback
    } else {
        sequence
    }
}

fn discard_all_changes(state: &mut DataGridState, actions: &mut DataGridActions, shortcut: &str) {
    if state.has_changes() {
        state.clear_edits();
        actions.message = Some(format!("已放弃所有修改 ({})", shortcut));
    }
}

fn jump_to_counted_row(state: &mut DataGridState, count: usize, max_row: usize) {
    let target_row = count.saturating_sub(1).min(max_row.saturating_sub(1));
    state.cursor.0 = target_row;
    state.scroll_to_row = Some(target_row);
}

fn clear_selected_cells(state: &mut DataGridState) -> Option<usize> {
    let ((min_r, min_c), (max_r, max_c)) = state.get_selection()?;
    for r in min_r..=max_r {
        for c in min_c..=max_c {
            state.modified_cells.insert((r, c), String::new());
        }
    }
    Some((max_r - min_r + 1) * (max_c - min_c + 1))
}

fn copy_selected_cells(
    state: &DataGridState,
    filtered_rows: &[(usize, &Vec<String>)],
) -> Option<String> {
    let ((min_r, min_c), (max_r, max_c)) = state.get_selection()?;
    let mut text = String::new();
    for r in min_r..=max_r {
        if let Some((_, row_data)) = filtered_rows.get(r) {
            let row_text: Vec<&str> = (min_c..=max_c)
                .filter_map(|c| row_data.get(c).map(|s| s.as_str()))
                .collect();
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&row_text.join("\t"));
        }
    }
    Some(text)
}

fn exit_select_mode(state: &mut DataGridState, cmd: &mut CmdBuffer) {
    state.mode = GridMode::Normal;
    state.select_anchor = None;
    cmd.clear();
}

pub fn handle_keyboard(
    ui: &mut egui::Ui,
    state: &mut DataGridState,
    result: &QueryResult,
    filtered_rows: &[(usize, &Vec<String>)],
    keybindings: &KeyBindings,
    actions: &mut DataGridActions,
) {
    // 如果表格未聚焦或处于编辑模式，不处理表格快捷键
    if !state.focused || state.mode == GridMode::Insert {
        return;
    }

    let max_row = filtered_rows.len();
    let max_col = result.columns.len();

    if max_row == 0 || max_col == 0 {
        debug!(max_row, max_col, "DataGrid 为空，跳过键盘处理");
        return;
    }

    let half_page = (max_row / 2).max(1);

    // 从 state 同步命令缓冲区
    let mut cmd = CmdBuffer {
        keys: state.command_buffer.clone(),
        count: state.count,
    };

    ui.input(|i| {
        // === 数字前缀处理 ===
        if handle_number_input(i, &mut cmd) {
            state.count = cmd.count;
            return;
        }

        // 数字 + Enter: 切换到指定的查询Tab
        if i.key_pressed(Key::Enter)
            && cmd.keys.is_empty()
            && let Some(tab_number) = cmd.count
        {
            if tab_number > 0 {
                actions.switch_to_tab = Some(tab_number - 1);
                actions.message = Some(format!("切换到查询 {}", tab_number));
            }
            cmd.clear();
            return;
        }

        // Backspace 回退数字计数
        if i.key_pressed(Key::Backspace)
            && let Some(current) = cmd.count
        {
            cmd.count = if current < 10 {
                None
            } else {
                Some(current / 10)
            };
            return;
        }

        match state.mode {
            GridMode::Normal => {
                handle_normal_mode(
                    i,
                    state,
                    result,
                    filtered_rows,
                    keybindings,
                    actions,
                    max_row,
                    max_col,
                    half_page,
                    &mut cmd,
                );
            }
            GridMode::Select => {
                handle_select_mode(i, state, filtered_rows, actions, max_row, max_col, &mut cmd);
            }
            GridMode::Insert => {}
        }
    });

    // 同步命令缓冲区回 state
    state.command_buffer = cmd.keys;
    state.count = cmd.count;
}

/// 处理数字输入，返回 true 表示已处理
fn handle_number_input(i: &egui::InputState, cmd: &mut CmdBuffer) -> bool {
    // 检查修饰键，有修饰键时不处理数字
    if i.modifiers.ctrl || i.modifiers.alt || i.modifiers.shift {
        return false;
    }

    for digit in 0..=9u32 {
        let key = match digit {
            0 => Key::Num0,
            1 => Key::Num1,
            2 => Key::Num2,
            3 => Key::Num3,
            4 => Key::Num4,
            5 => Key::Num5,
            6 => Key::Num6,
            7 => Key::Num7,
            8 => Key::Num8,
            9 => Key::Num9,
            _ => continue,
        };

        if i.key_pressed(key) {
            // 0 只有在已有计数时才追加，否则作为跳转到行首
            if digit == 0 && cmd.count.is_none() {
                return false;
            }
            let current = cmd.count.unwrap_or(0);
            // 防止溢出，限制最大计数为 99999
            if current <= 9999 {
                cmd.count = Some(current * 10 + digit as usize);
            }
            return true;
        }
    }
    false
}

fn handle_normal_mode(
    i: &egui::InputState,
    state: &mut DataGridState,
    result: &QueryResult,
    filtered_rows: &[(usize, &Vec<String>)],
    keybindings: &KeyBindings,
    actions: &mut DataGridActions,
    max_row: usize,
    max_col: usize,
    half_page: usize,
    cmd: &mut CmdBuffer,
) {
    let repeat = cmd.get_count();

    if let Some(configured) = detect_configured_normal_input(i, cmd, keybindings) {
        match configured {
            GridNormalInput::StartPrefix {
                token,
                preserve_count,
            } => {
                if preserve_count {
                    cmd.keys = token;
                } else {
                    cmd.start_prefix(&token);
                }
                return;
            }
            GridNormalInput::Action { action, sequence } => {
                handle_detected_normal_action(
                    action,
                    &sequence,
                    repeat,
                    state,
                    result,
                    filtered_rows,
                    actions,
                    max_row,
                    max_col,
                    half_page,
                    cmd,
                );
                return;
            }
        }
    }

    if let Some(action) = detect_normal_key_action(i, cmd) {
        handle_detected_normal_action(
            action,
            "",
            repeat,
            state,
            result,
            filtered_rows,
            actions,
            max_row,
            max_col,
            half_page,
            cmd,
        );
        return;
    }

    clear_pending_command_on_unhandled_key(i, cmd);
}

fn handle_detected_normal_action(
    action: GridKeyAction,
    sequence: &str,
    repeat: usize,
    state: &mut DataGridState,
    result: &QueryResult,
    filtered_rows: &[(usize, &Vec<String>)],
    actions: &mut DataGridActions,
    max_row: usize,
    max_col: usize,
    half_page: usize,
    cmd: &mut CmdBuffer,
) {
    let mut should_clear_cmd = true;
    match action {
        GridKeyAction::MoveLeft => {
            if state.cursor.1 == 0 {
                actions.focus_transfer = Some(super::actions::FocusTransfer::Sidebar);
            } else {
                for _ in 0..repeat {
                    state.move_cursor(0, -1, max_row, max_col);
                }
            }
        }
        GridKeyAction::MoveDown => {
            if state.cursor.0 >= max_row.saturating_sub(1) {
                actions.focus_transfer = Some(super::actions::FocusTransfer::SqlEditor);
            } else {
                for _ in 0..repeat {
                    state.move_cursor(1, 0, max_row, max_col);
                }
            }
        }
        GridKeyAction::MoveUp => {
            if state.cursor.0 == 0 {
                actions.focus_transfer = Some(super::actions::FocusTransfer::QueryTabs);
            } else {
                for _ in 0..repeat {
                    state.move_cursor(-1, 0, max_row, max_col);
                }
            }
        }
        GridKeyAction::MoveRight => {
            for _ in 0..repeat {
                state.move_cursor(0, 1, max_row, max_col);
            }
        }
        GridKeyAction::MoveWordRight => {
            if state.cursor.1 >= max_col.saturating_sub(1) {
                actions.focus_transfer = Some(super::actions::FocusTransfer::SqlEditor);
            } else {
                state.move_cursor(0, 1, max_row, max_col);
            }
        }
        GridKeyAction::MoveWordLeft => {
            if state.cursor.1 == 0 {
                actions.focus_transfer = Some(super::actions::FocusTransfer::Sidebar);
            } else {
                state.move_cursor(0, -1, max_row, max_col);
            }
        }
        GridKeyAction::JumpLineEnd => state.goto_line_end(max_col),
        GridKeyAction::JumpLineStart => state.goto_line_start(),
        GridKeyAction::JumpFileStart => state.goto_file_start(),
        GridKeyAction::JumpFileEnd => state.goto_file_end(max_row),
        GridKeyAction::JumpToCountedRow => {
            jump_to_counted_row(state, repeat, max_row);
        }
        GridKeyAction::HalfPageUp => {
            let delta = half_page * repeat;
            state.cursor.0 = state.cursor.0.saturating_sub(delta);
            state.scroll_to_row = Some(state.cursor.0);
        }
        GridKeyAction::HalfPageDown => {
            let delta = half_page * repeat;
            state.cursor.0 = (state.cursor.0 + delta).min(max_row.saturating_sub(1));
            state.scroll_to_row = Some(state.cursor.0);
        }
        GridKeyAction::PageUp => {
            let delta = half_page * repeat;
            state.cursor.0 = state.cursor.0.saturating_sub(delta);
            state.scroll_to_row = Some(state.cursor.0);
        }
        GridKeyAction::PageDown => {
            let delta = half_page * repeat;
            state.cursor.0 = (state.cursor.0 + delta).min(max_row.saturating_sub(1));
            state.scroll_to_row = Some(state.cursor.0);
        }
        GridKeyAction::OpenFilterPanel => {
            actions.open_filter_panel = true;
        }
        GridKeyAction::AddColumnFilter => {
            if let Some(col_name) = result.columns.get(state.cursor.1)
                && !state.filters.iter().any(|f| &f.column == col_name)
            {
                state.filters.push(ColumnFilter::new(col_name.clone()));
                actions.message = Some(format!("为列 {} 添加筛选 (f)", col_name));
            }
        }
        GridKeyAction::AddRowBelow => {
            let new_row = vec!["".to_string(); result.columns.len()];
            state.new_rows.push(new_row);
            let new_row_idx = result.rows.len() + state.new_rows.len() - 1;
            state.cursor = (new_row_idx, 0);
            state.scroll_to_row = Some(new_row_idx);
            actions.message = Some(format!("已添加新行 ({})", display_sequence(sequence, "o")));
        }
        GridKeyAction::AddRowAbove => {
            let new_row = vec!["".to_string(); result.columns.len()];
            state.new_rows.insert(0, new_row);
            let new_row_idx = result.rows.len();
            state.cursor = (new_row_idx, 0);
            state.scroll_to_row = Some(new_row_idx);
            actions.message = Some(format!(
                "已在开头添加新行 ({})",
                display_sequence(sequence, "O")
            ));
        }
        GridKeyAction::SaveChanges => {
            state.pending_save = true;
            actions.message = Some(format!(
                "保存修改 ({})",
                display_sequence(sequence, "Ctrl+S")
            ));
        }
        GridKeyAction::DiscardChanges => {
            discard_all_changes(state, actions, display_sequence(sequence, "q"));
        }
        GridKeyAction::Refresh => {
            actions.refresh_requested = true;
            actions.message = Some("刷新表格数据 (Ctrl+R)".to_string());
        }
        GridKeyAction::EnterInsert | GridKeyAction::AppendInsert => {
            enter_insert_mode(state, filtered_rows);
        }
        GridKeyAction::ChangeCell => {
            state.mode = GridMode::Insert;
            state.editing_cell = Some(state.cursor);
            state.edit_text.clear();
            if let Some((_, row_data)) = filtered_rows.get(state.cursor.0)
                && let Some(cell) = row_data.get(state.cursor.1)
            {
                state.original_value = cell.to_string();
            }
            actions.message = Some("修改单元格 (c)".to_string());
        }
        GridKeyAction::ReplaceCell => {
            state.mode = GridMode::Insert;
            state.editing_cell = Some(state.cursor);
            state.edit_text.clear();
            state.original_value.clear();
        }
        GridKeyAction::EnterSelect => {
            state.mode = GridMode::Select;
            state.select_anchor = Some(state.cursor);
        }
        GridKeyAction::SelectRow => {
            state.mode = GridMode::Select;
            state.select_anchor = Some((state.cursor.0, 0));
            state.cursor.1 = max_col.saturating_sub(1);
            actions.message = Some("选择整行 (x)".to_string());
        }
        GridKeyAction::SelectAll => {
            state.mode = GridMode::Select;
            state.select_anchor = Some((0, 0));
            state.cursor = (max_row.saturating_sub(1), max_col.saturating_sub(1));
            actions.message = Some("选择全部 (%)".to_string());
        }
        GridKeyAction::CollapseSelection => {
            if state.mode == GridMode::Select {
                state.mode = GridMode::Normal;
                state.select_anchor = None;
                actions.message = Some("折叠选择 (;)".to_string());
            }
        }
        GridKeyAction::Paste => {
            if let Some(text) = &state.clipboard {
                state.modified_cells.insert(state.cursor, text.clone());
                actions.message = Some("已粘贴 (p)".to_string());
            }
        }
        GridKeyAction::UndoCellChange => {
            if state.modified_cells.remove(&state.cursor).is_some() {
                actions.message = Some("已撤销修改 (u)".to_string());
            }
        }
        GridKeyAction::UnmarkDelete => {
            if state.rows_to_delete.contains(&state.cursor.0) {
                state.rows_to_delete.retain(|&x| x != state.cursor.0);
                actions.message = Some("已取消删除标记 (U)".to_string());
            }
        }
        GridKeyAction::ScrollCurrentRowCenter => {
            state.scroll_to_row = Some(state.cursor.0);
            actions.scroll_to_center = true;
            actions.message = Some(format!("滚动到中央 ({})", display_sequence(sequence, "zz")));
        }
        GridKeyAction::ScrollCurrentRowTop => {
            state.scroll_to_row = Some(state.cursor.0);
            actions.scroll_to_top = true;
            actions.message = Some(format!("滚动到顶部 ({})", display_sequence(sequence, "zt")));
        }
        GridKeyAction::ScrollCurrentRowBottom => {
            state.scroll_to_row = Some(state.cursor.0);
            actions.scroll_to_bottom = true;
            actions.message = Some(format!("滚动到底部 ({})", display_sequence(sequence, "zb")));
        }
        GridKeyAction::DeleteRow => {
            mark_current_row_for_delete(state, actions, display_sequence(sequence, "dd"));
        }
        GridKeyAction::CopyRow => {
            copy_current_row(state, filtered_rows, actions);
            if let Some(message) = &mut actions.message {
                *message =
                    message.replace("(yy)", &format!("({})", display_sequence(sequence, "yy")));
            }
        }
        GridKeyAction::Escape => {
            if !cmd.keys.is_empty() || cmd.count.is_some() {
                cmd.clear();
            } else if !state.filters.is_empty() {
                state.filters.clear();
                actions.message = Some("已清空筛选条件 (Esc)".to_string());
            }
            should_clear_cmd = false;
        }
    }
    if should_clear_cmd {
        cmd.clear();
    }
}

fn handle_select_mode(
    i: &egui::InputState,
    state: &mut DataGridState,
    filtered_rows: &[(usize, &Vec<String>)],
    actions: &mut DataGridActions,
    max_row: usize,
    max_col: usize,
    cmd: &mut CmdBuffer,
) {
    if let Some(action) = detect_select_key_action(i) {
        match action {
            GridSelectAction::MoveLeft | GridSelectAction::MoveWordLeft => {
                state.move_cursor(0, -1, max_row, max_col);
            }
            GridSelectAction::MoveDown => {
                state.move_cursor(1, 0, max_row, max_col);
            }
            GridSelectAction::MoveUp => {
                state.move_cursor(-1, 0, max_row, max_col);
            }
            GridSelectAction::MoveRight | GridSelectAction::MoveWordRight => {
                state.move_cursor(0, 1, max_row, max_col);
            }
            GridSelectAction::DeleteSelection => {
                if let Some(cell_count) = clear_selected_cells(state) {
                    actions.message = Some(format!("已清空 {} 个单元格 (d)", cell_count));
                }
                exit_select_mode(state, cmd);
            }
            GridSelectAction::ChangeSelection => {
                let _ = clear_selected_cells(state);
                state.mode = GridMode::Insert;
                state.editing_cell = Some(state.cursor);
                state.edit_text.clear();
                state.original_value.clear();
                state.select_anchor = None;
            }
            GridSelectAction::CopySelection => {
                if let Some(text) = copy_selected_cells(state, filtered_rows) {
                    state.clipboard = Some(text);
                    actions.message = Some("已复制选中内容 (y)".to_string());
                }
                exit_select_mode(state, cmd);
            }
            GridSelectAction::SelectRow => {
                state.select_anchor = Some((state.cursor.0, 0));
                state.cursor.1 = max_col.saturating_sub(1);
            }
            GridSelectAction::ExitSelect => {
                exit_select_mode(state, cmd);
            }
        }
    }
}

fn enter_insert_mode(state: &mut DataGridState, filtered_rows: &[(usize, &Vec<String>)]) {
    state.mode = GridMode::Insert;
    state.editing_cell = Some(state.cursor);

    if let Some((_, row_data)) = filtered_rows.get(state.cursor.0)
        && let Some(cell) = row_data.get(state.cursor.1)
    {
        state.edit_text = state
            .modified_cells
            .get(&state.cursor)
            .cloned()
            .unwrap_or_else(|| cell.to_string());
        state.original_value = cell.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CmdBuffer, clear_selected_cells, copy_selected_cells, handle_keyboard,
        has_pressed_key_event, should_clear_pending_command,
    };
    use crate::core::KeyBindings;
    use crate::database::QueryResult;
    use crate::ui::DataGridState;
    use crate::ui::components::grid::GridMode;
    use crate::ui::components::grid::actions::DataGridActions;
    use egui::{Event, Key, Modifiers};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn key_event_with_modifiers(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn sample_result() -> QueryResult {
        QueryResult {
            columns: vec!["id".to_string(), "name".to_string(), "email".to_string()],
            rows: vec![
                vec![
                    "1".to_string(),
                    "alice".to_string(),
                    "a@example.com".to_string(),
                ],
                vec![
                    "2".to_string(),
                    "bob".to_string(),
                    "b@example.com".to_string(),
                ],
                vec![
                    "3".to_string(),
                    "carol".to_string(),
                    "c@example.com".to_string(),
                ],
            ],
            affected_rows: 0,
            truncated: false,
            original_row_count: None,
        }
    }

    fn send_key_with_bindings(
        state: &mut DataGridState,
        result: &QueryResult,
        keybindings: &KeyBindings,
        event: Event,
    ) -> DataGridActions {
        let modifiers = match &event {
            Event::Key { modifiers, .. } => *modifiers,
            _ => Modifiers::NONE,
        };
        let ctx = egui::Context::default();
        let raw_input = egui::RawInput {
            events: vec![event],
            modifiers,
            ..Default::default()
        };
        let filtered_rows: Vec<(usize, &Vec<String>)> = result.rows.iter().enumerate().collect();
        let mut actions = DataGridActions::default();

        ctx.begin_pass(raw_input);
        egui::Area::new(egui::Id::new("grid_keyboard_test_area")).show(&ctx, |ui| {
            handle_keyboard(ui, state, result, &filtered_rows, keybindings, &mut actions);
        });
        let _ = ctx.end_pass();

        actions
    }

    fn send_key(state: &mut DataGridState, result: &QueryResult, event: Event) -> DataGridActions {
        send_key_with_bindings(state, result, &KeyBindings::default(), event)
    }

    #[test]
    fn prefix_start_clears_existing_count() {
        let mut cmd = CmdBuffer {
            keys: String::new(),
            count: Some(12),
        };

        cmd.start_prefix("g");

        assert_eq!(cmd.keys, "g");
        assert_eq!(cmd.count, None);
    }

    #[test]
    fn pending_prefix_is_cleared_by_unhandled_key_press() {
        let cmd = CmdBuffer {
            keys: "z".to_string(),
            count: None,
        };

        assert!(should_clear_pending_command(&[key_event(Key::X)], &cmd));
    }

    #[test]
    fn pending_count_is_cleared_by_unhandled_key_press() {
        let cmd = CmdBuffer {
            keys: String::new(),
            count: Some(5),
        };

        assert!(should_clear_pending_command(&[key_event(Key::Q)], &cmd));
    }

    #[test]
    fn text_input_event_does_not_clear_pending_command() {
        let cmd = CmdBuffer {
            keys: "g".to_string(),
            count: None,
        };

        assert!(!has_pressed_key_event(&[Event::Text("x".to_string())]));
        assert!(!should_clear_pending_command(
            &[Event::Text("x".to_string())],
            &cmd
        ));
    }

    #[test]
    fn clear_selected_cells_marks_everything_in_selection() {
        let mut state = DataGridState::new();
        state.mode = GridMode::Select;
        state.select_anchor = Some((1, 1));
        state.cursor = (2, 3);

        let cleared = clear_selected_cells(&mut state);

        assert_eq!(cleared, Some(6));
        for row in 1..=2 {
            for col in 1..=3 {
                assert_eq!(state.modified_cells.get(&(row, col)), Some(&String::new()));
            }
        }
    }

    #[test]
    fn copy_selected_cells_builds_tsv_block() {
        let mut state = DataGridState::new();
        state.mode = GridMode::Select;
        state.select_anchor = Some((0, 1));
        state.cursor = (1, 2);

        let row1 = vec!["id".to_string(), "name".to_string(), "email".to_string()];
        let row2 = vec![
            "1".to_string(),
            "alice".to_string(),
            "a@example.com".to_string(),
        ];
        let filtered_rows = vec![(0usize, &row1), (1usize, &row2)];

        let copied = copy_selected_cells(&state, &filtered_rows);

        assert_eq!(copied.as_deref(), Some("name\temail\nalice\ta@example.com"));
    }

    #[test]
    fn slash_opens_filter_panel_in_normal_mode() {
        let mut state = DataGridState::new();
        let result = sample_result();

        let actions = send_key(&mut state, &result, key_event(Key::Slash));

        assert!(actions.open_filter_panel);
        assert!(state.command_buffer.is_empty());
    }

    #[test]
    fn count_prefix_moves_down_multiple_rows() {
        let mut state = DataGridState::new();
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Num2));
        let _ = send_key(&mut state, &result, key_event(Key::J));

        assert_eq!(state.cursor, (2, 0));
        assert!(state.command_buffer.is_empty());
        assert_eq!(state.count, None);
    }

    #[test]
    fn space_d_marks_current_row_for_delete() {
        let mut state = DataGridState::new();
        state.cursor = (1, 0);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Space));
        let actions = send_key(&mut state, &result, key_event(Key::D));

        assert_eq!(state.rows_to_delete, vec![1]);
        assert!(state.command_buffer.is_empty());
        assert_eq!(
            actions.message.as_deref(),
            Some("已标记删除第 2 行 (Space+d)")
        );
    }

    #[test]
    fn yy_copies_current_row_into_clipboard() {
        let mut state = DataGridState::new();
        state.cursor = (0, 0);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Y));
        let actions = send_key(&mut state, &result, key_event(Key::Y));

        assert_eq!(state.clipboard.as_deref(), Some("1\talice\ta@example.com"));
        assert!(state.command_buffer.is_empty());
        assert_eq!(actions.message.as_deref(), Some("已复制第 1 行 (yy)"));
    }

    #[test]
    fn select_mode_d_clears_selected_cells_and_returns_to_normal() {
        let mut state = DataGridState::new();
        state.mode = GridMode::Select;
        state.select_anchor = Some((0, 1));
        state.cursor = (1, 2);
        let result = sample_result();

        let actions = send_key(
            &mut state,
            &result,
            key_event_with_modifiers(Key::D, Modifiers::NONE),
        );

        assert_eq!(state.mode, GridMode::Normal);
        assert_eq!(state.select_anchor, None);
        assert_eq!(state.modified_cells.len(), 4);
        assert_eq!(actions.message.as_deref(), Some("已清空 4 个单元格 (d)"));
    }

    #[test]
    fn gg_jumps_to_file_start() {
        let mut state = DataGridState::new();
        state.cursor = (2, 2);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::G));
        let _ = send_key(&mut state, &result, key_event(Key::G));

        assert_eq!(state.cursor, (0, 0));
        assert!(state.command_buffer.is_empty());
        assert_eq!(state.scroll_to_row, Some(0));
    }

    #[test]
    fn count_prefix_before_gg_jumps_to_specific_row() {
        let mut state = DataGridState::new();
        state.cursor = (2, 2);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Num2));
        let _ = send_key(&mut state, &result, key_event(Key::G));
        let _ = send_key(&mut state, &result, key_event(Key::G));

        assert_eq!(state.cursor, (1, 2));
        assert!(state.command_buffer.is_empty());
        assert_eq!(state.count, None);
        assert_eq!(state.scroll_to_row, Some(1));
    }

    #[test]
    fn shift_g_jumps_to_file_end() {
        let mut state = DataGridState::new();
        state.cursor = (0, 0);
        let result = sample_result();

        let _ = send_key(
            &mut state,
            &result,
            key_event_with_modifiers(Key::G, Modifiers::SHIFT),
        );

        assert_eq!(state.cursor.0, result.rows.len() - 1);
        assert!(state.command_buffer.is_empty());
        assert_eq!(state.scroll_to_row, Some(result.rows.len() - 1));
    }

    #[test]
    fn count_prefix_before_shift_g_jumps_to_specific_row() {
        let mut state = DataGridState::new();
        state.cursor = (0, 1);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Num2));
        let _ = send_key(
            &mut state,
            &result,
            key_event_with_modifiers(Key::G, Modifiers::SHIFT),
        );

        assert_eq!(state.cursor, (1, 1));
        assert!(state.command_buffer.is_empty());
        assert_eq!(state.count, None);
        assert_eq!(state.scroll_to_row, Some(1));
    }

    #[test]
    fn zz_requests_center_scroll() {
        let mut state = DataGridState::new();
        state.cursor = (1, 1);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Z));
        let actions = send_key(&mut state, &result, key_event(Key::Z));

        assert!(actions.scroll_to_center);
        assert!(!actions.scroll_to_top);
        assert!(!actions.scroll_to_bottom);
        assert_eq!(state.scroll_to_row, Some(1));
        assert_eq!(actions.message.as_deref(), Some("滚动到中央 (zz)"));
    }

    #[test]
    fn zt_requests_top_scroll() {
        let mut state = DataGridState::new();
        state.cursor = (1, 0);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Z));
        let actions = send_key(&mut state, &result, key_event(Key::T));

        assert!(actions.scroll_to_top);
        assert!(!actions.scroll_to_center);
        assert!(!actions.scroll_to_bottom);
        assert_eq!(state.scroll_to_row, Some(1));
        assert_eq!(actions.message.as_deref(), Some("滚动到顶部 (zt)"));
    }

    #[test]
    fn zb_requests_bottom_scroll() {
        let mut state = DataGridState::new();
        state.cursor = (1, 0);
        let result = sample_result();

        let _ = send_key(&mut state, &result, key_event(Key::Z));
        let actions = send_key(&mut state, &result, key_event(Key::B));

        assert!(actions.scroll_to_bottom);
        assert!(!actions.scroll_to_center);
        assert!(!actions.scroll_to_top);
        assert_eq!(state.scroll_to_row, Some(1));
        assert_eq!(actions.message.as_deref(), Some("滚动到底部 (zb)"));
    }

    #[test]
    fn colon_w_sets_pending_save() {
        let mut state = DataGridState::new();
        state.modified_cells.insert((0, 1), "updated".to_string());
        let result = sample_result();

        let _ = send_key(
            &mut state,
            &result,
            key_event_with_modifiers(Key::Semicolon, Modifiers::SHIFT),
        );
        let actions = send_key(&mut state, &result, key_event(Key::W));

        assert!(state.pending_save);
        assert!(state.command_buffer.is_empty());
        assert_eq!(actions.message.as_deref(), Some("保存修改 (:w)"));
    }

    #[test]
    fn colon_q_discards_existing_changes() {
        let mut state = DataGridState::new();
        state.modified_cells.insert((0, 1), "updated".to_string());
        state.rows_to_delete.push(1);
        let result = sample_result();

        let _ = send_key(
            &mut state,
            &result,
            key_event_with_modifiers(Key::Semicolon, Modifiers::SHIFT),
        );
        let actions = send_key(&mut state, &result, key_event(Key::Q));

        assert!(state.modified_cells.is_empty());
        assert!(state.rows_to_delete.is_empty());
        assert!(state.command_buffer.is_empty());
        assert_eq!(actions.message.as_deref(), Some("已放弃所有修改 (:q)"));
    }

    #[test]
    fn custom_grid_sequence_overrides_copy_row() {
        let mut state = DataGridState::new();
        state.cursor = (0, 0);
        let result = sample_result();
        let mut keybindings = KeyBindings::default();
        keybindings.set_local_sequences("grid.normal.copy_row", vec!["cc".to_string()]);

        let first = send_key_with_bindings(&mut state, &result, &keybindings, key_event(Key::C));
        assert!(first.message.is_none());
        assert_eq!(state.command_buffer, "c");
        assert!(state.clipboard.is_none());

        let second = send_key_with_bindings(&mut state, &result, &keybindings, key_event(Key::C));
        assert_eq!(state.clipboard.as_deref(), Some("1\talice\ta@example.com"));
        assert!(state.command_buffer.is_empty());
        assert_eq!(second.message.as_deref(), Some("已复制第 1 行 (cc)"));
    }
}
