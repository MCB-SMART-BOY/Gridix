//! 帮助对话框 - 工具使用与数据库学习入口
//!
//! 支持的快捷键：
//! - `Esc` / `q` - 关闭对话框
//! - `j` / `k` - 滚动内容
//! - `Ctrl+d` / `Ctrl+u` - 快速滚动

use super::common::{DialogContent, DialogShortcutContext, DialogStyle, DialogWindow};
use super::picker_shell::{PickerDialogShell, PickerNavAction, PickerPaneFocus};
use crate::core::Action;
use crate::ui::styles::{theme_accent, theme_muted_text, theme_warn};
use crate::ui::{LocalShortcut, local_shortcuts_text};
use egui::{self, Color32, RichText, ScrollArea, Vec2};

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

use self::types::{HelpPickerItem, HelpPickerRoot};
use self::types::{LearningStage, LearningView, TOPIC_DEFINITIONS};

pub struct HelpDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpNavigationAction {
    SetLearningView(LearningView),
    SelectTopic(LearningTopic),
}

#[derive(Debug, Clone)]
enum HelpUiAction {
    Navigate(HelpNavigationAction),
    Dispatch(HelpAction),
    NavigateAndDispatch {
        navigation: HelpNavigationAction,
        action: HelpAction,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpFrameAction {
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HelpPickerUiAction {
    OpenRoot(HelpPickerRoot),
    OpenItem(HelpPickerItem),
}

impl HelpDialog {
    const WINDOW_WIDTH: f32 = 860.0;
    const WINDOW_HEIGHT: f32 = 620.0;
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
    fn consume_scroll_delta(ctx: &egui::Context) -> f32 {
        let shortcuts = DialogShortcutContext::new(ctx);
        let mut delta = 0.0f32;

        if shortcuts.consume(LocalShortcut::HelpScrollUp) {
            delta -= 50.0;
        }
        if shortcuts.consume(LocalShortcut::HelpScrollDown) {
            delta += 50.0;
        }
        if shortcuts.consume(LocalShortcut::HelpPageUp) {
            delta -= 300.0;
        }
        if shortcuts.consume(LocalShortcut::HelpPageDown) {
            delta += 300.0;
        }

        delta
    }

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

        let mut action = None;
        Self::sync_picker_from_navigation(state);
        let snapshot = state.clone();
        let mut content_ui_action = None;
        let mut root_picker_action = None;
        let mut item_picker_action = None;

        if matches!(
            Self::handle_keyboard_input(ctx, state),
            Some(HelpFrameAction::Close)
        ) {
            *open = false;
            return None;
        }

        let (nav_width, item_width, compact_nav, compact_items) =
            Self::pane_layout_for_state(&snapshot);

        DialogWindow::workspace(
            ctx,
            "帮助与学习",
            &DialogStyle::WORKSPACE,
            Self::WINDOW_WIDTH,
            Self::WINDOW_HEIGHT,
        )
        .open(open)
        .show(ctx, |ui| {
            DialogContent::toolbar(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(12.0, 0.0);
                    let help_binding = context.keybindings.display(Action::ShowHelp);
                    Self::hint(
                        ui,
                        local_shortcuts_text(&[
                            LocalShortcut::HelpScrollUp,
                            LocalShortcut::HelpScrollDown,
                        ])
                        .as_str(),
                        if snapshot.picker_focus == PickerPaneFocus::Detail {
                            "详情滚动"
                        } else {
                            "层级移动"
                        },
                    );
                    Self::hint(ui, "H / L / Enter", "返回 / 打开");
                    Self::hint(ui, "Tab", "切换列焦点");
                    Self::hint(
                        ui,
                        local_shortcuts_text(&[LocalShortcut::Dismiss]).as_str(),
                        "关闭",
                    );
                    Self::hint(
                        ui,
                        local_shortcuts_text(&[
                            LocalShortcut::HelpPageUp,
                            LocalShortcut::HelpPageDown,
                        ])
                        .as_str(),
                        "快速滚动",
                    );
                    Self::hint(
                        ui,
                        if help_binding.is_empty() {
                            "F1"
                        } else {
                            help_binding.as_str()
                        },
                        "切换帮助",
                    );
                });
                ui.add_space(8.0);
                DialogContent::mouse_hint(
                    ui,
                    &[
                        ("单击左侧导航", "切换帮助区域"),
                        ("单击知识点", "直接进入对应课程"),
                        ("单击卡片按钮", "执行示例或继续学习"),
                    ],
                );
            });
            ui.add_space(10.0);

            DialogContent::toolbar(ui, |ui| {
                PickerDialogShell::breadcrumb(ui, &Self::breadcrumb_segments(&snapshot));
                ui.add_space(6.0);
                DialogContent::mouse_hint(
                    ui,
                    &[
                        ("单击左列", "打开帮助主线"),
                        ("单击中列", "打开当前主题"),
                        ("滚轮 / 拖动", "浏览右列内容"),
                    ],
                );
            });

            ui.add_space(8.0);

            PickerDialogShell::split(
                ui,
                nav_width,
                item_width,
                |ui| Self::show_root_pane(ui, &snapshot, compact_nav, &mut root_picker_action),
                |ui| Self::show_item_pane(ui, &snapshot, compact_items, &mut item_picker_action),
                |ui| Self::show_detail_pane(ui, ctx, &snapshot, context, &mut content_ui_action),
            );
        });

        if let Some(ui_action) = content_ui_action {
            action = Self::apply_ui_action(state, ui_action);
        }

        if let Some(picker_action) = item_picker_action.or(root_picker_action) {
            Self::apply_picker_ui_action(state, picker_action);
        }

        action
    }

    fn handle_keyboard_input(
        ctx: &egui::Context,
        state: &mut HelpState,
    ) -> Option<HelpFrameAction> {
        let shortcuts = DialogShortcutContext::new(ctx);
        if shortcuts.consume(LocalShortcut::Dismiss) {
            return Some(HelpFrameAction::Close);
        }

        let nav_action = if state.picker_focus == PickerPaneFocus::Detail {
            Self::consume_detail_nav_action(ctx)
        } else {
            PickerDialogShell::consume_nav_action(ctx)
        };

        if let Some(action) = nav_action {
            Self::apply_picker_nav_action(state, action);
        }

        None
    }

    fn consume_detail_nav_action(ctx: &egui::Context) -> Option<PickerNavAction> {
        PickerDialogShell::consume_detail_nav_action(ctx)
    }

    fn pane_layout_for_state(state: &HelpState) -> (f32, f32, bool, bool) {
        match state.picker_focus {
            PickerPaneFocus::Navigator => (220.0, 280.0, false, false),
            PickerPaneFocus::Items => (132.0, 250.0, true, false),
            PickerPaneFocus::Detail => (104.0, 176.0, true, true),
        }
    }

    fn apply_ui_action(state: &mut HelpState, action: HelpUiAction) -> Option<HelpAction> {
        match action {
            HelpUiAction::Navigate(navigation) => {
                Self::apply_navigation_action(state, navigation);
                None
            }
            HelpUiAction::Dispatch(action) => Some(action),
            HelpUiAction::NavigateAndDispatch { navigation, action } => {
                Self::apply_navigation_action(state, navigation);
                Some(action)
            }
        }
    }

    fn apply_navigation_action(state: &mut HelpState, action: HelpNavigationAction) {
        match action {
            HelpNavigationAction::SetLearningView(view) => {
                state.active_tab = HelpTab::DatabaseLearning;
                state.learning_view = view;
            }
            HelpNavigationAction::SelectTopic(topic) => {
                state.active_tab = HelpTab::DatabaseLearning;
                state.learning_view = LearningView::TopicDetail;
                state.learning_topic = topic;
            }
        }

        Self::sync_picker_from_navigation(state);
    }

    fn apply_picker_ui_action(state: &mut HelpState, action: HelpPickerUiAction) {
        match action {
            HelpPickerUiAction::OpenRoot(root) => {
                Self::select_picker_root(state, root);
                state.picker_focus = PickerPaneFocus::Items;
            }
            HelpPickerUiAction::OpenItem(item) => {
                Self::select_picker_item(state, item);
                state.picker_focus = PickerPaneFocus::Detail;
            }
        }
    }

    fn apply_picker_nav_action(state: &mut HelpState, action: PickerNavAction) {
        match action {
            PickerNavAction::MovePrev => match state.picker_focus {
                PickerPaneFocus::Navigator => Self::move_root_selection(state, -1),
                PickerPaneFocus::Items => Self::move_item_selection(state, -1),
                PickerPaneFocus::Detail => {}
            },
            PickerNavAction::MoveNext => match state.picker_focus {
                PickerPaneFocus::Navigator => Self::move_root_selection(state, 1),
                PickerPaneFocus::Items => Self::move_item_selection(state, 1),
                PickerPaneFocus::Detail => {}
            },
            PickerNavAction::Open => match state.picker_focus {
                PickerPaneFocus::Navigator => {
                    Self::select_picker_root(state, state.picker_root);
                    state.picker_focus = PickerPaneFocus::Items;
                }
                PickerPaneFocus::Items => {
                    Self::select_picker_item(state, state.picker_item);
                    state.picker_focus = PickerPaneFocus::Detail;
                }
                PickerPaneFocus::Detail => {}
            },
            PickerNavAction::Back => match state.picker_focus {
                PickerPaneFocus::Navigator => {}
                PickerPaneFocus::Items => state.picker_focus = PickerPaneFocus::Navigator,
                PickerPaneFocus::Detail => state.picker_focus = PickerPaneFocus::Items,
            },
            PickerNavAction::FocusNext => {
                state.picker_focus = PickerDialogShell::next_focus(state.picker_focus);
            }
            PickerNavAction::FocusPrev => {
                state.picker_focus = PickerDialogShell::prev_focus(state.picker_focus);
            }
        }
    }

    fn sync_picker_from_navigation(state: &mut HelpState) {
        match state.active_tab {
            HelpTab::ToolQuickStart => {
                state.picker_root = HelpPickerRoot::ToolQuickStart;
                state.picker_item = HelpPickerItem::ToolQuickStartGuide;
            }
            HelpTab::DatabaseLearning => {
                state.picker_root = HelpPickerRoot::DatabaseLearning;
                state.picker_item = match state.learning_view {
                    LearningView::Overview => HelpPickerItem::LearningOverview,
                    LearningView::Roadmap => HelpPickerItem::LearningRoadmap,
                    LearningView::TopicDetail => {
                        HelpPickerItem::LearningTopic(state.learning_topic)
                    }
                };
            }
        }
    }

    fn select_picker_root(state: &mut HelpState, root: HelpPickerRoot) {
        match root {
            HelpPickerRoot::ToolQuickStart => {
                Self::select_picker_item(state, HelpPickerItem::ToolQuickStartGuide);
            }
            HelpPickerRoot::DatabaseLearning => {
                let item = match state.picker_item {
                    HelpPickerItem::LearningOverview
                    | HelpPickerItem::LearningRoadmap
                    | HelpPickerItem::LearningTopic(_) => state.picker_item,
                    HelpPickerItem::ToolQuickStartGuide => HelpPickerItem::LearningOverview,
                };
                Self::select_picker_item(state, item);
            }
        }
    }

    fn select_picker_item(state: &mut HelpState, item: HelpPickerItem) {
        match item {
            HelpPickerItem::ToolQuickStartGuide => {
                state.active_tab = HelpTab::ToolQuickStart;
                state.picker_root = HelpPickerRoot::ToolQuickStart;
                state.picker_item = HelpPickerItem::ToolQuickStartGuide;
            }
            HelpPickerItem::LearningOverview => {
                state.active_tab = HelpTab::DatabaseLearning;
                state.learning_view = LearningView::Overview;
                state.picker_root = HelpPickerRoot::DatabaseLearning;
                state.picker_item = HelpPickerItem::LearningOverview;
            }
            HelpPickerItem::LearningRoadmap => {
                state.active_tab = HelpTab::DatabaseLearning;
                state.learning_view = LearningView::Roadmap;
                state.picker_root = HelpPickerRoot::DatabaseLearning;
                state.picker_item = HelpPickerItem::LearningRoadmap;
            }
            HelpPickerItem::LearningTopic(topic) => {
                state.active_tab = HelpTab::DatabaseLearning;
                state.learning_view = LearningView::TopicDetail;
                state.learning_topic = topic;
                state.picker_root = HelpPickerRoot::DatabaseLearning;
                state.picker_item = HelpPickerItem::LearningTopic(topic);
            }
        }
    }

    fn move_root_selection(state: &mut HelpState, direction: isize) {
        let roots = [
            HelpPickerRoot::ToolQuickStart,
            HelpPickerRoot::DatabaseLearning,
        ];
        let Some(current_index) = roots.iter().position(|root| *root == state.picker_root) else {
            return;
        };
        let next_index = current_index as isize + direction;
        if !(0..roots.len() as isize).contains(&next_index) {
            return;
        }

        Self::select_picker_root(state, roots[next_index as usize]);
    }

    fn move_item_selection(state: &mut HelpState, direction: isize) {
        let items = Self::picker_items(state.picker_root);
        let Some(current_index) = items.iter().position(|item| *item == state.picker_item) else {
            return;
        };
        let next_index = current_index as isize + direction;
        if !(0..items.len() as isize).contains(&next_index) {
            return;
        }

        Self::select_picker_item(state, items[next_index as usize]);
    }

    fn picker_items(root: HelpPickerRoot) -> Vec<HelpPickerItem> {
        match root {
            HelpPickerRoot::ToolQuickStart => vec![HelpPickerItem::ToolQuickStartGuide],
            HelpPickerRoot::DatabaseLearning => {
                let mut items = vec![
                    HelpPickerItem::LearningOverview,
                    HelpPickerItem::LearningRoadmap,
                ];
                for definition in TOPIC_DEFINITIONS {
                    items.push(HelpPickerItem::LearningTopic(definition.topic));
                }
                items
            }
        }
    }

    fn picker_root_label(root: HelpPickerRoot) -> &'static str {
        match root {
            HelpPickerRoot::ToolQuickStart => "工具快速使用指南",
            HelpPickerRoot::DatabaseLearning => "数据库相关知识点学习",
        }
    }

    fn picker_root_meta(root: HelpPickerRoot) -> &'static str {
        match root {
            HelpPickerRoot::ToolQuickStart => "Gridix 工作流 / 焦点 / 常用动作",
            HelpPickerRoot::DatabaseLearning => "总览 / 路线图 / 主题课程",
        }
    }

    fn picker_item_label(item: HelpPickerItem) -> &'static str {
        match item {
            HelpPickerItem::ToolQuickStartGuide => "工具快速使用指南",
            HelpPickerItem::LearningOverview => "学习总览",
            HelpPickerItem::LearningRoadmap => "学习路线图",
            HelpPickerItem::LearningTopic(topic) => Self::topic_definition(topic).short_title,
        }
    }

    fn picker_item_detail(item: HelpPickerItem) -> &'static str {
        match item {
            HelpPickerItem::ToolQuickStartGuide => {
                "面向已经理解数据库概念、只想快速上手 Gridix 的用户。"
            }
            HelpPickerItem::LearningOverview => "先理解学习主线，再决定从哪一课开始。",
            HelpPickerItem::LearningRoadmap => "查看推荐依赖关系与完整知识地图。",
            HelpPickerItem::LearningTopic(topic) => Self::topic_definition(topic).summary,
        }
    }

    fn breadcrumb_segments(state: &HelpState) -> Vec<String> {
        let mut segments = vec![
            "帮助与学习".to_string(),
            Self::picker_root_label(state.picker_root).to_string(),
        ];
        if state.picker_root == HelpPickerRoot::DatabaseLearning {
            segments.push(Self::picker_item_label(state.picker_item).to_string());
        }
        segments
    }

    fn show_root_pane(
        ui: &mut egui::Ui,
        state: &HelpState,
        compact: bool,
        pending_action: &mut Option<HelpPickerUiAction>,
    ) {
        PickerDialogShell::pane(
            ui,
            "导航",
            if compact {
                "已选主线会收窄；h 返回，单击重新展开。"
            } else {
                "左列选择帮助主线；j/k 移动，l 或 Enter 打开。"
            },
            state.picker_focus == PickerPaneFocus::Navigator,
            |ui| {
                ScrollArea::vertical()
                    .id_salt("help_picker_roots")
                    .show(ui, |ui| {
                        for root in [
                            HelpPickerRoot::ToolQuickStart,
                            HelpPickerRoot::DatabaseLearning,
                        ] {
                            let is_selected = state.picker_root == root;
                            let response = PickerDialogShell::entry(
                                ui,
                                format!("help_root::{root:?}"),
                                is_selected,
                                is_selected && state.picker_focus == PickerPaneFocus::Navigator,
                                Self::picker_root_label(root),
                                (!compact).then_some(Self::picker_root_meta(root)),
                                None,
                            );
                            PickerDialogShell::reveal_selected(
                                &response,
                                is_selected && state.picker_focus == PickerPaneFocus::Navigator,
                            );
                            if response.clicked() {
                                *pending_action = Some(HelpPickerUiAction::OpenRoot(root));
                            }
                            ui.add_space(6.0);
                        }
                    });
            },
        );
    }

    fn show_item_pane(
        ui: &mut egui::Ui,
        state: &HelpState,
        compact: bool,
        pending_action: &mut Option<HelpPickerUiAction>,
    ) {
        PickerDialogShell::pane(
            ui,
            "当前层级",
            if compact {
                "选中后自动收窄，让正文拿到更多空间。"
            } else {
                "中列浏览当前主线内容；j/k 移动，l 或 Enter 打开。"
            },
            state.picker_focus == PickerPaneFocus::Items,
            |ui| {
                ScrollArea::vertical()
                    .id_salt("help_picker_items")
                    .show(ui, |ui| match state.picker_root {
                        HelpPickerRoot::ToolQuickStart => {
                            let item = HelpPickerItem::ToolQuickStartGuide;
                            let is_selected = state.picker_item == item;
                            let response = PickerDialogShell::entry(
                                ui,
                                "help_item::tool_quick_start",
                                is_selected,
                                is_selected && state.picker_focus == PickerPaneFocus::Items,
                                Self::picker_item_label(item),
                                (!compact).then_some("焦点 / 工作流 / 表格 / SQL 编辑器"),
                                (!compact).then_some(Self::picker_item_detail(item)),
                            );
                            PickerDialogShell::reveal_selected(
                                &response,
                                is_selected && state.picker_focus == PickerPaneFocus::Items,
                            );
                            if response.clicked() {
                                *pending_action = Some(HelpPickerUiAction::OpenItem(item));
                            }
                        }
                        HelpPickerRoot::DatabaseLearning => {
                            PickerDialogShell::section_label(ui, "入口");
                            for item in [
                                HelpPickerItem::LearningOverview,
                                HelpPickerItem::LearningRoadmap,
                            ] {
                                let is_selected = state.picker_item == item;
                                let response = PickerDialogShell::entry(
                                    ui,
                                    format!("help_item::{item:?}"),
                                    is_selected,
                                    is_selected && state.picker_focus == PickerPaneFocus::Items,
                                    Self::picker_item_label(item),
                                    None,
                                    (!compact).then_some(Self::picker_item_detail(item)),
                                );
                                PickerDialogShell::reveal_selected(
                                    &response,
                                    is_selected && state.picker_focus == PickerPaneFocus::Items,
                                );
                                if response.clicked() {
                                    *pending_action = Some(HelpPickerUiAction::OpenItem(item));
                                }
                                ui.add_space(6.0);
                            }

                            PickerDialogShell::section_label(ui, "知识点");
                            for definition in TOPIC_DEFINITIONS {
                                let item = HelpPickerItem::LearningTopic(definition.topic);
                                let is_selected = state.picker_item == item;
                                let response = PickerDialogShell::entry(
                                    ui,
                                    format!("help_item::topic::{:?}", definition.topic),
                                    is_selected,
                                    is_selected && state.picker_focus == PickerPaneFocus::Items,
                                    definition.short_title,
                                    (!compact).then_some(definition.dependency_text),
                                    (!compact).then_some(definition.summary),
                                );
                                PickerDialogShell::reveal_selected(
                                    &response,
                                    is_selected && state.picker_focus == PickerPaneFocus::Items,
                                );
                                if response.clicked() {
                                    *pending_action = Some(HelpPickerUiAction::OpenItem(item));
                                }
                                ui.add_space(6.0);
                            }
                        }
                    });
            },
        );
    }

    fn show_detail_pane(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        state: &HelpState,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        PickerDialogShell::pane(
            ui,
            Self::picker_item_label(state.picker_item),
            "右列显示正文；h 返回上一列，滚轮或 Ctrl+d / Ctrl+u 继续浏览。",
            state.picker_focus == PickerPaneFocus::Detail,
            |ui| {
                ScrollArea::vertical()
                    .id_salt("help_picker_detail")
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        if state.picker_focus == PickerPaneFocus::Detail {
                            let total_delta = Self::consume_scroll_delta(ctx);
                            if total_delta != 0.0 {
                                ui.scroll_with_delta(Vec2::new(0.0, -total_delta));
                            }
                        }

                        ui.add_space(6.0);
                        match state.picker_item {
                            HelpPickerItem::ToolQuickStartGuide => {
                                Self::show_tool_guide(ui, &context.keybindings);
                            }
                            HelpPickerItem::LearningOverview
                            | HelpPickerItem::LearningRoadmap
                            | HelpPickerItem::LearningTopic(_) => {
                                Self::show_learning_guide(ui, state, context, pending_ui_action);
                            }
                        }
                        ui.add_space(20.0);
                    });
            },
        );
    }

    fn accent_color(ui: &egui::Ui) -> Color32 {
        theme_accent(ui.visuals())
    }

    fn body_text_color(ui: &egui::Ui) -> Color32 {
        ui.visuals().text_color()
    }

    fn muted_text_color(ui: &egui::Ui) -> Color32 {
        theme_muted_text(ui.visuals())
    }

    fn key_text_color(ui: &egui::Ui) -> Color32 {
        theme_warn(ui.visuals())
    }

    fn hint(ui: &mut egui::Ui, key: &str, desc: &str) {
        ui.label(
            RichText::new(key)
                .monospace()
                .small()
                .color(Self::key_text_color(ui)),
        );
        ui.label(
            RichText::new(desc)
                .small()
                .color(Self::muted_text_color(ui)),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        HelpDialog, HelpNavigationAction, HelpPickerItem, HelpPickerRoot, HelpState, HelpTab,
        HelpUiAction, LearningTopic, LearningView,
    };
    use crate::ui::dialogs::HelpAction;
    use crate::ui::dialogs::picker_shell::{PickerNavAction, PickerPaneFocus};

    #[test]
    fn navigation_action_sets_learning_view() {
        let mut state = HelpState::default();
        HelpDialog::apply_navigation_action(
            &mut state,
            HelpNavigationAction::SetLearningView(LearningView::Roadmap),
        );

        assert_eq!(state.learning_view, LearningView::Roadmap);
        assert_eq!(state.picker_root, HelpPickerRoot::DatabaseLearning);
        assert_eq!(state.picker_item, HelpPickerItem::LearningRoadmap);
    }

    #[test]
    fn navigation_action_select_topic_enters_learning_detail() {
        let mut state = HelpState {
            active_tab: HelpTab::ToolQuickStart,
            ..Default::default()
        };

        HelpDialog::apply_navigation_action(
            &mut state,
            HelpNavigationAction::SelectTopic(LearningTopic::Join),
        );

        assert_eq!(state.active_tab, HelpTab::DatabaseLearning);
        assert_eq!(state.learning_view, LearningView::TopicDetail);
        assert_eq!(state.learning_topic, LearningTopic::Join);
        assert_eq!(state.picker_root, HelpPickerRoot::DatabaseLearning);
        assert_eq!(
            state.picker_item,
            HelpPickerItem::LearningTopic(LearningTopic::Join)
        );
    }

    #[test]
    fn ui_action_dispatch_returns_business_action_without_mutating_state() {
        let mut state = HelpState::default();

        let action = HelpDialog::apply_ui_action(
            &mut state,
            HelpUiAction::Dispatch(HelpAction::EnsureLearningSample { reset: true }),
        );

        assert!(matches!(
            action,
            Some(HelpAction::EnsureLearningSample { reset: true })
        ));
        assert_eq!(state.active_tab, HelpTab::ToolQuickStart);
        assert_eq!(state.learning_view, LearningView::Overview);
    }

    #[test]
    fn ui_action_navigate_and_dispatch_updates_state_and_returns_action() {
        let mut state = HelpState::default();

        let action = HelpDialog::apply_ui_action(
            &mut state,
            HelpUiAction::NavigateAndDispatch {
                navigation: HelpNavigationAction::SelectTopic(LearningTopic::Foundations),
                action: HelpAction::EnsureLearningSample { reset: false },
            },
        );

        assert!(matches!(
            action,
            Some(HelpAction::EnsureLearningSample { reset: false })
        ));
        assert_eq!(state.active_tab, HelpTab::DatabaseLearning);
        assert_eq!(state.learning_view, LearningView::TopicDetail);
        assert_eq!(state.learning_topic, LearningTopic::Foundations);
        assert_eq!(
            state.picker_item,
            HelpPickerItem::LearningTopic(LearningTopic::Foundations)
        );
    }

    #[test]
    fn picker_nav_moves_between_root_item_and_detail_focus() {
        let mut state = HelpState::default();

        HelpDialog::apply_picker_nav_action(&mut state, PickerNavAction::MoveNext);
        assert_eq!(state.picker_root, HelpPickerRoot::DatabaseLearning);
        assert_eq!(state.picker_item, HelpPickerItem::LearningOverview);

        HelpDialog::apply_picker_nav_action(&mut state, PickerNavAction::Open);
        assert_eq!(state.picker_focus, PickerPaneFocus::Items);

        HelpDialog::apply_picker_nav_action(&mut state, PickerNavAction::MoveNext);
        assert_eq!(state.picker_item, HelpPickerItem::LearningRoadmap);

        HelpDialog::apply_picker_nav_action(&mut state, PickerNavAction::Open);
        assert_eq!(state.picker_focus, PickerPaneFocus::Detail);

        HelpDialog::apply_picker_nav_action(&mut state, PickerNavAction::Back);
        assert_eq!(state.picker_focus, PickerPaneFocus::Items);
    }

    #[test]
    fn detail_focus_uses_compact_picker_layout() {
        let mut state = HelpState::default();
        let navigator_layout = HelpDialog::pane_layout_for_state(&state);

        state.picker_focus = PickerPaneFocus::Detail;
        let detail_layout = HelpDialog::pane_layout_for_state(&state);

        assert!(detail_layout.0 < navigator_layout.0);
        assert!(detail_layout.1 < navigator_layout.1);
        assert!(detail_layout.2);
        assert!(detail_layout.3);
    }
}
