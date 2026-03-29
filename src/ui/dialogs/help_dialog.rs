//! 帮助对话框 - 工具使用与数据库学习入口
//!
//! 支持的快捷键：
//! - `Esc` / `q` - 关闭对话框
//! - `j` / `k` - 滚动内容
//! - `Ctrl+d` / `Ctrl+u` - 快速滚动

use super::keyboard::{self, ListNavigation};
use egui::{self, Color32, Key, RichText, ScrollArea, Vec2};

#[path = "help_dialog/learning.rs"]
mod learning;
#[path = "help_dialog/tool_guide.rs"]
mod tool_guide;
#[path = "help_dialog/topic_content.rs"]
mod topic_content;
#[path = "help_dialog/types.rs"]
mod types;

pub use self::types::{
    HelpAction, HelpContext, HelpOnboardingStep, HelpState, HelpTab, LearningTopic,
};

use self::types::LearningStage;

pub struct HelpDialog;

impl HelpDialog {
    const WINDOW_WIDTH: f32 = 920.0;
    const WINDOW_HEIGHT: f32 = 680.0;
    const CONTENT_WIDTH: f32 = 780.0;
    const LEARNING_SEQUENCE: [LearningTopic; 21] = [
        LearningTopic::Foundations,
        LearningTopic::DataTypes,
        LearningTopic::NullHandling,
        LearningTopic::SelectBasics,
        LearningTopic::FilterAndSort,
        LearningTopic::LikePattern,
        LearningTopic::Aggregate,
        LearningTopic::Relationships,
        LearningTopic::Join,
        LearningTopic::InsertData,
        LearningTopic::Constraints,
        LearningTopic::UpdateDelete,
        LearningTopic::Transactions,
        LearningTopic::SchemaDesign,
        LearningTopic::Views,
        LearningTopic::Indexes,
        LearningTopic::Subqueries,
        LearningTopic::WindowFunctions,
        LearningTopic::TriggersProcedures,
        LearningTopic::QueryPlans,
        LearningTopic::BackupPermissions,
    ];
    const ROADMAP_STAGES: [LearningStage; 6] = [
        LearningStage::Fundamentals,
        LearningStage::QueryBasics,
        LearningStage::RelationshipModel,
        LearningStage::Mutations,
        LearningStage::DesignQuality,
        LearningStage::Advanced,
    ];

    /// 显示帮助对话框
    pub fn show_with_scroll(
        ctx: &egui::Context,
        open: &mut bool,
        _scroll_offset: &mut f32,
        state: &mut HelpState,
        context: &HelpContext,
    ) -> Option<HelpAction> {
        if !*open {
            return None;
        }

        if keyboard::handle_close_keys(ctx) {
            *open = false;
            return None;
        }

        let mut action = None;

        egui::Window::new("帮助与学习")
            .open(open)
            .collapsible(false)
            .resizable(true)
            .default_size([Self::WINDOW_WIDTH, Self::WINDOW_HEIGHT])
            .default_pos(egui::pos2(96.0, 72.0))
            .min_width(760.0)
            .min_height(460.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
                    Self::hint(ui, "j/k", "滚动");
                    Self::hint(ui, "q/Esc", "关闭");
                    Self::hint(ui, "F1", "切换帮助");
                });
                ui.add_space(10.0);

                Self::show_tabs(ui, state);

                ui.add_space(8.0);
                ui.separator();

                ScrollArea::vertical()
                    .id_salt("help_scroll")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        let content_width = Self::CONTENT_WIDTH.min(ui.available_width());

                        let scroll_delta = match keyboard::handle_list_navigation(ctx) {
                            ListNavigation::Up => -50.0,
                            ListNavigation::Down => 50.0,
                            ListNavigation::PageUp => -300.0,
                            ListNavigation::PageDown => 300.0,
                            _ => 0.0,
                        };

                        let extra_delta = ctx.input(|i| {
                            let mut delta = 0.0f32;
                            if i.modifiers.ctrl && i.key_pressed(Key::D) {
                                delta += 300.0;
                            }
                            if i.modifiers.ctrl && i.key_pressed(Key::U) {
                                delta -= 300.0;
                            }
                            delta
                        });

                        let total_delta = scroll_delta + extra_delta;
                        if total_delta != 0.0 {
                            ui.scroll_with_delta(Vec2::new(0.0, -total_delta));
                        }

                        ui.add_space(12.0);

                        ui.horizontal(|ui| {
                            let offset = ((ui.available_width() - content_width) / 2.0).max(0.0);
                            ui.add_space(offset);

                            ui.allocate_ui_with_layout(
                                Vec2::new(content_width, 0.0),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    ui.set_min_width(content_width);
                                    ui.set_max_width(content_width);

                                    match state.active_tab {
                                        HelpTab::ToolQuickStart => Self::show_tool_guide(ui),
                                        HelpTab::DatabaseLearning => Self::show_learning_guide(
                                            ui,
                                            state,
                                            context,
                                            &mut action,
                                        ),
                                    }
                                },
                            );
                        });

                        ui.add_space(20.0);
                    });
            });

        action
    }

    fn show_tabs(ui: &mut egui::Ui, state: &mut HelpState) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;

            Self::tab_button(
                ui,
                &mut state.active_tab,
                HelpTab::ToolQuickStart,
                "工具快速使用指南",
                "已经了解数据库，只想快速上手 Gridix",
            );
            Self::tab_button(
                ui,
                &mut state.active_tab,
                HelpTab::DatabaseLearning,
                "数据库相关知识点学习指南",
                "通过示例库学习数据库概念与基础操作",
            );
        });
    }

    fn tab_button(
        ui: &mut egui::Ui,
        current: &mut HelpTab,
        target: HelpTab,
        label: &str,
        tooltip: &str,
    ) {
        let selected = *current == target;
        let response = ui.selectable_label(selected, label);
        if response.clicked() {
            *current = target;
        }
        response.on_hover_text(tooltip);
    }

    fn hint(ui: &mut egui::Ui, key: &str, desc: &str) {
        ui.label(
            RichText::new(key)
                .monospace()
                .small()
                .color(Color32::from_rgb(255, 200, 100)),
        );
        ui.label(RichText::new(desc).small().color(Color32::GRAY));
    }
}
