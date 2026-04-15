use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStyle, DialogWindow,
    WorkspaceDialogShell,
};
use crate::ui::{LocalShortcut, local_shortcut_text, local_shortcuts_text};
use eframe::egui::{self, RichText, ScrollArea};
use std::cell::Cell;

const TOOLBAR_MENU_WIDTH: f32 = 760.0;
const TOOLBAR_MENU_HEIGHT: f32 = 420.0;
const TOOLBAR_MENU_LIST_WIDTH: f32 = 280.0;
const TOOLBAR_MENU_INLINE_HEADER_MIN_WIDTH: f32 = 700.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToolbarMenuItemId {
    Export,
    Import,
    ToggleErDiagram,
    ShowHistory,
    NewTable,
    NewDatabase,
    NewUser,
}

#[derive(Debug, Clone)]
pub struct ToolbarMenuDialogEntry {
    pub id: ToolbarMenuItemId,
    pub icon: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub shortcut: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ToolbarMenuDialogState {
    pub show: bool,
    pub selected_index: usize,
    reset_selection: bool,
}

impl ToolbarMenuDialogState {
    pub fn open(&mut self) {
        self.show = true;
        self.selected_index = 0;
        self.reset_selection = true;
    }

    pub fn close(&mut self) {
        self.show = false;
        self.selected_index = 0;
        self.reset_selection = false;
    }

    fn prepare_selection(&mut self, entries: &[ToolbarMenuDialogEntry]) {
        if entries.is_empty() {
            self.selected_index = 0;
            self.reset_selection = false;
            return;
        }

        if self.reset_selection {
            self.selected_index = entries.iter().position(|entry| entry.enabled).unwrap_or(0);
            self.reset_selection = false;
        }

        self.selected_index = self.selected_index.min(entries.len() - 1);
    }

    fn move_selection(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.selected_index = 0;
            return;
        }

        let current = self.selected_index as isize;
        let max_index = (len - 1) as isize;
        self.selected_index = (current + delta).clamp(0, max_index) as usize;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolbarMenuFrameAction {
    Prev,
    Next,
    Confirm,
    Dismiss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolbarMenuHeaderLayout {
    Inline,
    Stacked,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ToolbarMenuWindowKind {
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ToolbarMenuWindowProfile {
    kind: ToolbarMenuWindowKind,
    width: f32,
    height: f32,
}

pub struct ToolbarMenuDialog;

impl ToolbarMenuDialog {
    pub fn show(
        ctx: &egui::Context,
        state: &mut ToolbarMenuDialogState,
        title: &str,
        subtitle: &str,
        confirm_text: &str,
        entries: &[ToolbarMenuDialogEntry],
    ) -> Option<ToolbarMenuItemId> {
        if !state.show {
            return None;
        }

        state.prepare_selection(entries);
        let close_requested = Cell::new(false);
        let activated = Cell::new(None);
        let selected_enabled = entries
            .get(state.selected_index)
            .is_some_and(|entry| entry.enabled);

        if let Some(frame_action) = Self::consume_frame_action(ctx) {
            match frame_action {
                ToolbarMenuFrameAction::Prev => state.move_selection(-1, entries.len()),
                ToolbarMenuFrameAction::Next => state.move_selection(1, entries.len()),
                ToolbarMenuFrameAction::Confirm if selected_enabled => {
                    activated.set(entries.get(state.selected_index).map(|entry| entry.id));
                }
                ToolbarMenuFrameAction::Confirm => {}
                ToolbarMenuFrameAction::Dismiss => close_requested.set(true),
            }
        }

        let selected_entry = entries.get(state.selected_index).cloned();
        let style = DialogStyle::LARGE;
        let window_profile = Self::window_profile();
        let window = match window_profile.kind {
            ToolbarMenuWindowKind::Workspace => DialogWindow::workspace(
                ctx,
                title,
                &style,
                window_profile.width,
                window_profile.height,
            ),
        };
        window.open(&mut state.show).show(ctx, |ui| {
            WorkspaceDialogShell::show(
                ui,
                format!("{}_workspace_shell", title),
                |ui| {
                    Self::show_compact_header(ui, title, subtitle);
                },
                |_| {},
                |ui| {
                    DialogContent::split_workspace(
                        ui,
                        TOOLBAR_MENU_LIST_WIDTH,
                        |ui| {
                            DialogContent::workspace_pane(
                                ui,
                                "动作列表",
                                "保留顶部图标按钮作为 trigger，真正的选择在这里完成。",
                                |ui| {
                                    ScrollArea::vertical()
                                        .id_salt(ui.id().with("toolbar_menu_entries"))
                                        .show(ui, |ui| {
                                            for (index, entry) in entries.iter().enumerate() {
                                                let response = Self::show_entry(
                                                    ui,
                                                    entry,
                                                    index == state.selected_index,
                                                );
                                                if response.hovered() || response.clicked() {
                                                    state.selected_index = index;
                                                }
                                                if response.double_clicked() && entry.enabled {
                                                    activated.set(Some(entry.id));
                                                }
                                            }
                                        });
                                },
                            );
                        },
                        |ui| {
                            DialogContent::workspace_pane(
                                ui,
                                "当前项",
                                "查看当前条目的用途、快捷键和可用性。",
                                |ui| {
                                    if let Some(entry) = &selected_entry {
                                        ui.label(
                                            RichText::new(format!(
                                                "{} {}",
                                                entry.icon, entry.title
                                            ))
                                            .size(20.0)
                                            .strong(),
                                        );
                                        ui.add_space(6.0);
                                        ui.label(RichText::new(entry.description).weak());
                                        ui.add_space(12.0);

                                        if entry.shortcut.is_empty() {
                                            DialogContent::info_text(
                                                ui,
                                                "当前项没有显式主快捷键入口。",
                                            );
                                        } else {
                                            DialogContent::shortcut_hint(
                                                ui,
                                                &[("快捷键", entry.shortcut.as_str())],
                                            );
                                        }

                                        if let Some(reason) = &entry.disabled_reason {
                                            DialogContent::warning_text(ui, reason);
                                        } else if entry.enabled {
                                            DialogContent::success_text(ui, "当前项可以立即执行。");
                                        }
                                    } else {
                                        DialogContent::info_text(ui, "当前菜单没有可显示的条目。");
                                    }
                                },
                            );
                        },
                    );
                },
                |ui| {
                    let confirm_enabled =
                        selected_entry.as_ref().is_some_and(|entry| entry.enabled);
                    let footer =
                        DialogFooter::show(ui, confirm_text, "关闭", confirm_enabled, &style);
                    if footer.cancelled {
                        close_requested.set(true);
                    }
                    if footer.confirmed && confirm_enabled {
                        activated.set(selected_entry.as_ref().map(|entry| entry.id));
                    }
                },
            );
        });

        if activated.get().is_some() || close_requested.get() || !state.show {
            state.close();
        }

        activated.get()
    }

    fn consume_frame_action(ctx: &egui::Context) -> Option<ToolbarMenuFrameAction> {
        DialogShortcutContext::new(ctx).resolve(&[
            (LocalShortcut::ToolbarMenuPrev, ToolbarMenuFrameAction::Prev),
            (LocalShortcut::ToolbarMenuNext, ToolbarMenuFrameAction::Next),
            (
                LocalShortcut::ToolbarMenuConfirm,
                ToolbarMenuFrameAction::Confirm,
            ),
            (
                LocalShortcut::ToolbarMenuDismiss,
                ToolbarMenuFrameAction::Dismiss,
            ),
        ])
    }

    fn header_layout_for_width(width: f32) -> ToolbarMenuHeaderLayout {
        if width >= TOOLBAR_MENU_INLINE_HEADER_MIN_WIDTH {
            ToolbarMenuHeaderLayout::Inline
        } else {
            ToolbarMenuHeaderLayout::Stacked
        }
    }

    fn window_profile() -> ToolbarMenuWindowProfile {
        ToolbarMenuWindowProfile {
            kind: ToolbarMenuWindowKind::Workspace,
            width: TOOLBAR_MENU_WIDTH,
            height: TOOLBAR_MENU_HEIGHT,
        }
    }

    fn show_compact_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
        let layout = Self::header_layout_for_width(ui.available_width());
        DialogContent::toolbar(ui, |ui| match layout {
            ToolbarMenuHeaderLayout::Inline => {
                ui.columns(2, |columns| {
                    Self::show_header_title_block(&mut columns[0], title, subtitle);
                    Self::show_header_hint_block(&mut columns[1]);
                });
            }
            ToolbarMenuHeaderLayout::Stacked => {
                Self::show_header_title_block(ui, title, subtitle);
                ui.add_space(8.0);
                Self::show_header_hint_block(ui);
            }
        });
        ui.add_space(8.0);
    }

    fn show_header_title_block(ui: &mut egui::Ui, title: &str, subtitle: &str) {
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong());
            ui.add_space(6.0);
            ui.label(RichText::new(subtitle).small().weak());
        });
    }

    fn show_header_hint_block(ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new(format!(
                        "{} 选择",
                        local_shortcuts_text(&[
                            LocalShortcut::ToolbarMenuPrev,
                            LocalShortcut::ToolbarMenuNext,
                        ])
                    ))
                    .small(),
                );
                ui.label(
                    RichText::new(format!(
                        "{} 打开",
                        local_shortcut_text(LocalShortcut::ToolbarMenuConfirm)
                    ))
                    .small(),
                );
                ui.label(
                    RichText::new(format!(
                        "{} 关闭",
                        local_shortcut_text(LocalShortcut::ToolbarMenuDismiss)
                    ))
                    .small(),
                );
            });
            ui.add_space(6.0);
            DialogContent::mouse_hint(ui, &[("单击条目", "选中"), ("双击条目", "立即打开")]);
        });
    }

    fn show_entry(
        ui: &mut egui::Ui,
        entry: &ToolbarMenuDialogEntry,
        selected: bool,
    ) -> egui::Response {
        let text_color = if entry.enabled {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };
        let suffix = if entry.shortcut.is_empty() {
            String::new()
        } else {
            format!("    {}", entry.shortcut)
        };
        let disabled = entry
            .disabled_reason
            .as_deref()
            .map(|reason| format!("\n{}", reason))
            .unwrap_or_default();
        let label = format!(
            "{} {}{}\n{}{}",
            entry.icon, entry.title, suffix, entry.description, disabled
        );

        ui.add_sized(
            [ui.available_width(), 56.0],
            egui::Button::new(
                RichText::new(label)
                    .color(text_color)
                    .text_style(egui::TextStyle::Body),
            )
            .fill(fill),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ToolbarMenuDialog, ToolbarMenuDialogEntry, ToolbarMenuDialogState, ToolbarMenuFrameAction,
        ToolbarMenuHeaderLayout, ToolbarMenuItemId, ToolbarMenuWindowKind,
    };
    use egui::{Event, Key, Modifiers, RawInput};

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        ctx.begin_pass(RawInput {
            events: vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
    }

    fn entry(id: ToolbarMenuItemId, enabled: bool) -> ToolbarMenuDialogEntry {
        ToolbarMenuDialogEntry {
            id,
            icon: "*",
            title: "entry",
            description: "desc",
            shortcut: String::new(),
            enabled,
            disabled_reason: None,
        }
    }

    #[test]
    fn toolbar_menu_open_prefers_first_enabled_entry() {
        let mut state = ToolbarMenuDialogState {
            selected_index: 2,
            ..Default::default()
        };
        let entries = vec![
            entry(ToolbarMenuItemId::Export, false),
            entry(ToolbarMenuItemId::Import, true),
            entry(ToolbarMenuItemId::ShowHistory, true),
        ];

        state.open();
        state.prepare_selection(&entries);

        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn toolbar_menu_selection_clamps_when_entries_shrink() {
        let mut state = ToolbarMenuDialogState {
            show: true,
            selected_index: 4,
            reset_selection: false,
        };
        let entries = vec![
            entry(ToolbarMenuItemId::Export, true),
            entry(ToolbarMenuItemId::Import, true),
        ];

        state.prepare_selection(&entries);

        assert_eq!(state.selected_index, 1);
    }

    #[test]
    fn toolbar_menu_close_clears_visibility() {
        let mut state = ToolbarMenuDialogState {
            show: true,
            selected_index: 1,
            reset_selection: true,
        };

        state.close();

        assert!(!state.show);
        assert_eq!(state.selected_index, 0);
    }

    #[test]
    fn toolbar_menu_dismiss_accepts_q() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Q);

        assert_eq!(
            ToolbarMenuDialog::consume_frame_action(&ctx),
            Some(ToolbarMenuFrameAction::Dismiss)
        );

        let _ = ctx.end_pass();
    }

    #[test]
    fn toolbar_menu_dismiss_accepts_escape() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Escape);

        assert_eq!(
            ToolbarMenuDialog::consume_frame_action(&ctx),
            Some(ToolbarMenuFrameAction::Dismiss)
        );

        let _ = ctx.end_pass();
    }

    #[test]
    fn toolbar_menu_header_prefers_inline_layout_when_width_allows() {
        assert_eq!(
            ToolbarMenuDialog::header_layout_for_width(760.0),
            ToolbarMenuHeaderLayout::Inline
        );
        assert_eq!(
            ToolbarMenuDialog::header_layout_for_width(640.0),
            ToolbarMenuHeaderLayout::Stacked
        );
    }

    #[test]
    fn toolbar_menu_uses_workspace_window_profile() {
        let profile = ToolbarMenuDialog::window_profile();

        assert_eq!(profile.kind, ToolbarMenuWindowKind::Workspace);
        assert_eq!(profile.width, 760.0);
        assert_eq!(profile.height, 420.0);
    }
}
