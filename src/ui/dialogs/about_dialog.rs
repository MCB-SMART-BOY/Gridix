//! 关于对话框 - 显示项目信息

use super::common::{DialogContent, DialogFooter, DialogStyle, DialogWindow};
use crate::ui::styles::{theme_muted_text, theme_text};
use crate::ui::{LocalShortcut, local_shortcuts_text};
use egui::{self, Color32, CornerRadius, Margin, RichText, Stroke};

const ABOUT_BRAND_ACCENT: Color32 = Color32::from_rgb(122, 162, 247);
const ABOUT_FACT_TWO_COLUMN_MIN_WIDTH: f32 = 420.0;
const ABOUT_TAGLINE: &str = "Grid-first database manager";
const ABOUT_PLAYFUL_CAPTION: &str = "开源，不订阅，也不想把数据库工作流塞进一堆设置页。";
const ABOUT_MANIFESTO_TITLE: &str = "开源、键盘优先、面向数据库学习与日常使用";
const ABOUT_MANIFESTO_BODY: &str = "支持 SQLite / PostgreSQL / MySQL(MariaDB)，并提供帮助与学习体系、导入导出、ER 图与筛选工作流。";
const ABOUT_MANIFESTO_FOOTNOTE: &str =
    "把查询、浏览、学习和操作串成顺手的工作流，比把一切都塞进配置页更重要。";
const ABOUT_COMMUNITY_HINT: &str = "开源项目，欢迎 Star / issue / PR。";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AboutFactLayout {
    TwoColumn,
    Stacked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AboutFact {
    label: &'static str,
    value: &'static str,
    monospace: bool,
}

pub struct AboutDialog;

impl AboutDialog {
    pub fn show(ctx: &egui::Context, show: &mut bool) {
        if !*show {
            return;
        }

        let close_shortcuts = Self::close_shortcuts();
        let style = DialogStyle::MEDIUM;
        DialogWindow::standard(ctx, "关于 Gridix", &style).show(ctx, |ui| {
            let body_text = theme_text(ui.visuals());
            let muted_text = theme_muted_text(ui.visuals());

            ui.vertical_centered(|ui| {
                Self::show_brand_hero(ui, body_text, muted_text);
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(12.0);

            Self::show_manifesto_card(ui, body_text, muted_text);
            ui.add_space(12.0);
            Self::show_project_facts(ui, body_text, muted_text);
            ui.add_space(10.0);

            ui.vertical_centered(|ui| {
                ui.label(
                    RichText::new(ABOUT_COMMUNITY_HINT)
                        .small()
                        .color(muted_text),
                );
            });
            ui.add_space(8.0);

            DialogContent::shortcut_hint(
                ui,
                &[(local_shortcuts_text(&close_shortcuts).as_str(), "关闭")],
            );

            if DialogFooter::show_close_only(
                ui,
                &format!("关闭 [{}]", local_shortcuts_text(&close_shortcuts)),
                &style,
            ) {
                *show = false;
            }
        });
    }

    fn close_shortcuts() -> [LocalShortcut; 2] {
        [LocalShortcut::Dismiss, LocalShortcut::Confirm]
    }

    fn fact_layout_for_width(width: f32) -> AboutFactLayout {
        if width >= ABOUT_FACT_TWO_COLUMN_MIN_WIDTH {
            AboutFactLayout::TwoColumn
        } else {
            AboutFactLayout::Stacked
        }
    }

    fn project_facts() -> [AboutFact; 4] {
        [
            AboutFact {
                label: "仓库",
                value: "github.com/MCB-SMART-BOY/Gridix",
                monospace: true,
            },
            AboutFact {
                label: "作者",
                value: "MCB-SMART-BOY",
                monospace: false,
            },
            AboutFact {
                label: "支持",
                value: "SQLite / PostgreSQL / MySQL(MariaDB)",
                monospace: false,
            },
            AboutFact {
                label: "工作流",
                value: "帮助 / 导入导出 / ER / 筛选",
                monospace: false,
            },
        ]
    }

    fn show_brand_hero(ui: &mut egui::Ui, body_text: Color32, muted_text: Color32) {
        ui.label(
            RichText::new("GRIDIX")
                .size(34.0)
                .strong()
                .color(ABOUT_BRAND_ACCENT),
        );
        ui.add_space(4.0);
        ui.label(RichText::new(ABOUT_TAGLINE).size(18.0).color(body_text));
        ui.add_space(8.0);

        egui::Frame::NONE
            .fill(ABOUT_BRAND_ACCENT.gamma_multiply(0.16))
            .stroke(Stroke::new(1.0, ABOUT_BRAND_ACCENT.gamma_multiply(0.26)))
            .corner_radius(CornerRadius::same(127))
            .inner_margin(Margin::symmetric(10, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .strong()
                        .color(body_text),
                );
            });

        ui.add_space(10.0);
        ui.label(
            RichText::new(ABOUT_PLAYFUL_CAPTION)
                .small()
                .italics()
                .color(muted_text),
        );
    }

    fn show_manifesto_card(ui: &mut egui::Ui, body_text: Color32, muted_text: Color32) {
        egui::Frame::NONE
            .fill(ABOUT_BRAND_ACCENT.gamma_multiply(0.08))
            .stroke(Stroke::new(1.0, ABOUT_BRAND_ACCENT.gamma_multiply(0.24)))
            .corner_radius(CornerRadius::same(12))
            .inner_margin(Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.label(RichText::new(ABOUT_MANIFESTO_TITLE).strong());
                ui.add_space(6.0);
                ui.label(RichText::new(ABOUT_MANIFESTO_BODY).color(body_text));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(ABOUT_MANIFESTO_FOOTNOTE)
                        .small()
                        .color(muted_text),
                );
            });
    }

    fn show_project_facts(ui: &mut egui::Ui, body_text: Color32, muted_text: Color32) {
        let facts = Self::project_facts();
        let layout = Self::fact_layout_for_width(ui.available_width());

        ui.label(RichText::new("项目速览").small().strong().color(muted_text));
        ui.add_space(8.0);

        match layout {
            AboutFactLayout::TwoColumn => {
                ui.columns(2, |columns| {
                    for (index, fact) in facts.iter().enumerate() {
                        Self::show_fact_row(&mut columns[index % 2], *fact, body_text, muted_text);
                        if index + 2 < facts.len() {
                            columns[index % 2].add_space(10.0);
                        }
                    }
                });
            }
            AboutFactLayout::Stacked => {
                for (index, fact) in facts.iter().enumerate() {
                    Self::show_fact_row(ui, *fact, body_text, muted_text);
                    if index + 1 < facts.len() {
                        ui.add_space(10.0);
                    }
                }
            }
        }
    }

    fn show_fact_row(ui: &mut egui::Ui, fact: AboutFact, body_text: Color32, muted_text: Color32) {
        ui.vertical(|ui| {
            ui.label(RichText::new(fact.label).small().strong().color(muted_text));
            ui.add_space(2.0);
            let text = RichText::new(fact.value).color(body_text);
            if fact.monospace {
                ui.label(text.monospace().color(ABOUT_BRAND_ACCENT));
            } else {
                ui.label(text);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{AboutDialog, AboutFactLayout};
    use crate::ui::LocalShortcut;

    #[test]
    fn about_fact_layout_prefers_two_columns_when_width_allows() {
        assert_eq!(
            AboutDialog::fact_layout_for_width(520.0),
            AboutFactLayout::TwoColumn
        );
        assert_eq!(
            AboutDialog::fact_layout_for_width(360.0),
            AboutFactLayout::Stacked
        );
    }

    #[test]
    fn about_project_facts_keep_repository_author_and_supported_databases() {
        let facts = AboutDialog::project_facts();

        assert!(
            facts
                .iter()
                .any(|fact| fact.label == "仓库" && fact.value.contains("Gridix"))
        );
        assert!(
            facts
                .iter()
                .any(|fact| fact.label == "作者" && fact.value == "MCB-SMART-BOY")
        );
        assert!(facts.iter().any(|fact| {
            fact.label == "支持"
                && fact.value.contains("SQLite")
                && fact.value.contains("PostgreSQL")
                && fact.value.contains("MySQL")
        }));
    }

    #[test]
    fn about_copy_keeps_brand_tagline_and_playful_caption() {
        assert_eq!(super::ABOUT_TAGLINE, "Grid-first database manager");
        assert!(super::ABOUT_PLAYFUL_CAPTION.contains("开源"));
        assert!(super::ABOUT_PLAYFUL_CAPTION.contains("设置页"));
        assert!(super::ABOUT_COMMUNITY_HINT.contains("Star"));
    }

    #[test]
    fn about_close_shortcuts_remain_dismiss_and_confirm() {
        assert_eq!(
            AboutDialog::close_shortcuts(),
            [LocalShortcut::Dismiss, LocalShortcut::Confirm]
        );
    }
}
