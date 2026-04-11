//! 确认对话框组件
//!
//! 支持的快捷键：
//! - `Enter` / `y` - 确认操作
//! - `Esc` / `n` - 取消操作

use super::common::{DialogContent, DialogFooter, DialogStyle, DialogWindow};
use crate::ui::{
    LocalShortcut, local_shortcut_text, local_shortcuts_text,
    styles::{DANGER, SPACING_MD},
};
use egui::{self, RichText};

pub struct ConfirmDialog;

impl ConfirmDialog {
    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        title: &str,
        message: &str,
        confirm_text: &str,
        on_confirm: &mut bool,
    ) {
        if !*show {
            return;
        }

        let style = DialogStyle::SMALL;
        DialogWindow::standard(ctx, title, &style).show(ctx, |ui| {
            DialogContent::card(ui, Some(DANGER), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("⚠").size(20.0).color(DANGER));
                    ui.add_space(SPACING_MD);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("此操作不可撤销").strong());
                        ui.add_space(4.0);
                        ui.label(RichText::new(message).size(14.0));
                    });
                });
            });

            ui.add_space(SPACING_MD);
            DialogContent::shortcut_hint(
                ui,
                &[
                    (
                        local_shortcuts_text(&[LocalShortcut::DangerConfirm]).as_str(),
                        "确认",
                    ),
                    (
                        local_shortcuts_text(&[LocalShortcut::DangerCancel]).as_str(),
                        "取消",
                    ),
                ],
            );

            let footer = DialogFooter::show_danger(
                ui,
                &format!(
                    "{} [{}]",
                    confirm_text,
                    local_shortcut_text(LocalShortcut::DangerConfirm)
                ),
                &format!(
                    "取消 [{}]",
                    local_shortcut_text(LocalShortcut::DangerCancel)
                ),
                &style,
            );

            if footer.confirmed {
                *on_confirm = true;
                *show = false;
            } else if footer.cancelled {
                *show = false;
            }
        });
    }
}
