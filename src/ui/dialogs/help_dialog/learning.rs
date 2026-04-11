use super::types::{
    HelpAction, HelpContext, HelpOnboardingStep, HelpState, LearningStage, LearningTopic,
    LearningTopicDefinition, LearningTopicStatus, LearningView, TOPIC_DEFINITIONS,
};
use super::*;
use egui::Stroke;

impl HelpDialog {
    pub(super) fn show_learning_guide(
        ui: &mut egui::Ui,
        state: &HelpState,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        match state.learning_view {
            LearningView::Overview => {
                Self::show_learning_overview(ui, state, context, pending_ui_action)
            }
            LearningView::Roadmap => Self::show_learning_roadmap(ui, state, pending_ui_action),
            LearningView::TopicDetail => {
                Self::show_learning_topic_detail(ui, state, context, pending_ui_action)
            }
        }
    }

    fn show_learning_overview(
        ui: &mut egui::Ui,
        _state: &HelpState,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let accent = Self::accent_color(ui);
        let text = Self::body_text_color(ui);
        let muted = Self::muted_text_color(ui);

        ui.label(
            RichText::new("数据库相关知识点学习指南")
                .size(20.0)
                .strong()
                .color(accent),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "这里不是工具说明书，而是数据库学习入口。先看总览，再打开路线图，最后进入具体知识点。",
            )
            .color(text),
        );
        ui.add_space(16.0);

        Self::show_onboarding_flow_card(ui, context, pending_ui_action);
        ui.add_space(14.0);

        Self::learning_status_card(ui, context);
        ui.add_space(16.0);

        Self::overview_card(
            ui,
            "为什么先看总览",
            &[
                "数据库知识不是平铺目录，而是有前置依赖的学习路径。",
                "先理解基础概念，再进入查询、聚合、关系和写操作，学习会稳定得多。",
                "路线图会告诉你先学什么、后学什么，以及哪些主题现在只是预告。",
            ],
            None,
        );
        ui.add_space(12.0);
        Self::overview_card(
            ui,
            "这套学习示例库能学什么",
            &[
                "内置 SQLite 教学数据集包含 8 张主表，每张表至少 100 行、15 列以上。",
                "关系覆盖客户、地址、供应商、分类、商品、订单、订单明细、支付，适合练 JOIN、GROUP BY、NULL、事务。",
                "数据里既有一对多，也有层级分类、级联删除和多跳外键，不是只有几条演示记录的空壳样例。",
            ],
            None,
        );
        ui.add_space(12.0);
        Self::learning_overview_flow(ui);
        ui.add_space(14.0);

        if Self::overview_action_card(
            ui,
            "查看学习路线图",
            "先看一张真正的依赖路线图，再看完整知识地图。这样你能同时知道当前该学什么，以及后面还有哪些主题。",
            &["核心依赖图", "完整知识地图", "可学习 / 规划中 / 进阶主题"],
            "进入路线图",
        ) {
            *pending_ui_action = Some(HelpUiAction::Navigate(
                HelpNavigationAction::SetLearningView(LearningView::Roadmap),
            ));
        }

        ui.add_space(12.0);

        if Self::overview_action_card(
            ui,
            "从学习示例开始第一课",
            "如果你想直接动手，Gridix 会自动创建本地 SQLite 学习示例库：8 张主表、每表 100+ 行，并直接带你进入第一课。",
            &[
                "不会改动真实连接",
                "适合第一次使用",
                "8 张主表 / 每表 100+ 行",
                "可随时重置示例库",
            ],
            "打开示例并开始第一课",
        ) {
            *pending_ui_action = Some(HelpUiAction::NavigateAndDispatch {
                navigation: HelpNavigationAction::SelectTopic(LearningTopic::Foundations),
                action: HelpAction::EnsureLearningSample { reset: false },
            });
        }

        ui.add_space(10.0);
        ui.label(
            RichText::new(
                "主干建议：概念 -> 数据类型 / NULL -> SELECT -> WHERE / LIKE -> GROUP BY -> 主键 / 外键 -> JOIN -> INSERT -> 约束 -> UPDATE / DELETE -> 事务",
            )
            .small()
            .color(muted),
        );
    }

    fn show_learning_roadmap(
        ui: &mut egui::Ui,
        state: &HelpState,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let muted = Self::muted_text_color(ui);

        Self::learning_nav(ui, state, "数据库知识点路线图", pending_ui_action);
        ui.add_space(12.0);

        ui.label(
            RichText::new("先看核心依赖图，再看完整知识地图。").color(Self::body_text_color(ui)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new("鼠标悬停查看简介，点击节点直接进入对应知识点。")
                .small()
                .color(muted),
        );
        ui.add_space(16.0);

        Self::roadmap_legend(ui);
        ui.add_space(12.0);
        Self::roadmap_summary_strip(ui);
        ui.add_space(12.0);
        if let Some(topic) = Self::core_roadmap_graph(ui, state.learning_topic) {
            *pending_ui_action = Some(HelpUiAction::Navigate(HelpNavigationAction::SelectTopic(
                topic,
            )));
        }
        ui.add_space(18.0);

        ui.label(
            RichText::new("完整知识地图")
                .strong()
                .color(Self::body_text_color(ui)),
        );
        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "下面按阶段展开全部主题，路线图不会因为当前还没做完课程就把后面的方向藏起来。",
            )
            .small()
            .color(muted),
        );
        ui.add_space(12.0);

        for stage in Self::ROADMAP_STAGES {
            if let Some(topic) = Self::roadmap_stage_card(ui, stage, state.learning_topic) {
                *pending_ui_action = Some(HelpUiAction::Navigate(
                    HelpNavigationAction::SelectTopic(topic),
                ));
            }
            ui.add_space(12.0);
        }

        ui.add_space(10.0);
        ui.label(
            RichText::new("连线代表推荐依赖关系；上面的主干图讲“先学什么”，下面的完整地图讲“后面还能学什么”。")
                .small()
                .color(Self::accent_color(ui)),
        );
    }

    fn show_learning_topic_detail(
        ui: &mut egui::Ui,
        state: &HelpState,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let muted = Self::muted_text_color(ui);

        Self::learning_nav(
            ui,
            state,
            Self::topic_title(state.learning_topic),
            pending_ui_action,
        );
        ui.add_space(12.0);
        Self::learning_status_card(ui, context);
        ui.add_space(16.0);
        Self::topic_learning_path_card(ui, state.learning_topic);
        ui.add_space(16.0);

        match state.learning_topic {
            LearningTopic::Foundations => Self::show_foundations_topic(ui, pending_ui_action),
            LearningTopic::DataTypes => Self::show_data_types_topic(ui, pending_ui_action),
            LearningTopic::NullHandling => Self::show_null_handling_topic(ui, pending_ui_action),
            LearningTopic::SelectBasics => Self::show_select_topic(ui, context, pending_ui_action),
            LearningTopic::FilterAndSort => Self::show_filter_sort_topic(ui, pending_ui_action),
            LearningTopic::LikePattern => Self::show_like_topic(ui, pending_ui_action),
            LearningTopic::Aggregate => Self::show_aggregate_topic(ui, pending_ui_action),
            LearningTopic::Relationships => {
                Self::show_relationships_topic(ui, context, pending_ui_action)
            }
            LearningTopic::Join => Self::show_join_topic(ui, pending_ui_action),
            LearningTopic::InsertData => Self::show_insert_topic(ui, pending_ui_action),
            LearningTopic::Constraints => Self::show_constraints_topic(ui, pending_ui_action),
            LearningTopic::UpdateDelete => Self::show_update_delete_topic(ui, pending_ui_action),
            LearningTopic::Transactions => Self::show_transactions_topic(ui, pending_ui_action),
            LearningTopic::SchemaDesign
            | LearningTopic::Views
            | LearningTopic::Indexes
            | LearningTopic::Subqueries
            | LearningTopic::WindowFunctions
            | LearningTopic::TriggersProcedures
            | LearningTopic::QueryPlans
            | LearningTopic::BackupPermissions => {
                Self::show_roadmap_preview_topic(ui, state.learning_topic)
            }
        }

        ui.add_space(16.0);
        Self::topic_navigation_row(ui, state, pending_ui_action);
        ui.add_space(12.0);

        match Self::topic_definition(state.learning_topic).status {
            LearningTopicStatus::Available => {
                ui.label(
                    RichText::new("提示：所有“自动演示”都会切换到内置的 SQLite 学习示例库，不会修改你的真实数据库连接。")
                        .small()
                        .color(muted),
                );
            }
            _ => {
                ui.label(
                    RichText::new("提示：这个主题当前主要用于建立全局认知，详细课程与自动演示会在后续阶段补齐。")
                        .small()
                        .color(muted),
                );
            }
        }
    }

    fn learning_nav(
        ui: &mut egui::Ui,
        _state: &HelpState,
        title: &str,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let accent = Color32::from_rgb(130, 180, 255);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);

            if Self::nav_button(ui, "返回总览") {
                *pending_ui_action = Some(HelpUiAction::Navigate(
                    HelpNavigationAction::SetLearningView(LearningView::Overview),
                ));
            }
            if Self::nav_button(ui, "查看学习路线图") {
                *pending_ui_action = Some(HelpUiAction::Navigate(
                    HelpNavigationAction::SetLearningView(LearningView::Roadmap),
                ));
            }

            ui.label(RichText::new(">").color(Color32::GRAY));
            ui.label(RichText::new(title).strong().color(accent));
        });
    }

    fn roadmap_legend(ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            Self::step_chip(ui, "蓝色：可学习");
            Self::step_chip(ui, "灰色：规划中");
            Self::step_chip(ui, "紫色：进阶主题");
        });
    }

    fn roadmap_summary_strip(ui: &mut egui::Ui) {
        let total = TOPIC_DEFINITIONS.len();
        let available = TOPIC_DEFINITIONS
            .iter()
            .filter(|topic| topic.status == LearningTopicStatus::Available)
            .count();
        let planned = TOPIC_DEFINITIONS
            .iter()
            .filter(|topic| topic.status == LearningTopicStatus::Planned)
            .count();
        let advanced = TOPIC_DEFINITIONS
            .iter()
            .filter(|topic| topic.status == LearningTopicStatus::Advanced)
            .count();

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(88, 108, 150, 14))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 28),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("路线图总览")
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("先看主干，再决定是否延伸到设计、性能和系统主题。")
                        .small()
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    Self::step_chip(ui, &format!("共 {total} 个知识点"));
                    Self::status_count_chip(ui, LearningTopicStatus::Available, available);
                    Self::status_count_chip(ui, LearningTopicStatus::Planned, planned);
                    Self::status_count_chip(ui, LearningTopicStatus::Advanced, advanced);
                    Self::step_chip(ui, "推荐起点：数据库 / 表 / 行 / 列");
                });
            });
    }

    fn core_roadmap_graph(ui: &mut egui::Ui, selected: LearningTopic) -> Option<LearningTopic> {
        let desired_size = Vec2::new(ui.available_width().min(760.0), 500.0);
        let mut clicked_topic = None;

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(95, 125, 180, 14))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 34),
            ))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(18, 18))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("核心依赖路线图")
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "这张图只画当前主干课，目的是先把数据库入门最关键的一条路径看清楚。",
                    )
                    .small()
                    .color(Self::muted_text_color(ui)),
                );
                ui.add_space(14.0);

                let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
                let painter = ui.painter();
                let lanes = [
                    (
                        LearningStage::Fundamentals,
                        &[
                            LearningTopic::Foundations,
                            LearningTopic::DataTypes,
                            LearningTopic::NullHandling,
                        ][..],
                    ),
                    (
                        LearningStage::QueryBasics,
                        &[
                            LearningTopic::SelectBasics,
                            LearningTopic::FilterAndSort,
                            LearningTopic::LikePattern,
                            LearningTopic::Aggregate,
                        ][..],
                    ),
                    (
                        LearningStage::RelationshipModel,
                        &[LearningTopic::Relationships, LearningTopic::Join][..],
                    ),
                    (
                        LearningStage::Mutations,
                        &[
                            LearningTopic::InsertData,
                            LearningTopic::Constraints,
                            LearningTopic::UpdateDelete,
                            LearningTopic::Transactions,
                        ][..],
                    ),
                ];
                let lane_gap = 12.0;
                let lane_height = (rect.height()
                    - lane_gap * (lanes.len().saturating_sub(1) as f32))
                    / lanes.len() as f32;
                let mut nodes = Vec::new();

                for (index, (stage, topics)) in lanes.iter().enumerate() {
                    let lane_top = rect.top() + index as f32 * (lane_height + lane_gap);
                    let lane_rect = egui::Rect::from_min_max(
                        egui::pos2(rect.left(), lane_top),
                        egui::pos2(rect.right(), lane_top + lane_height),
                    );
                    let (fill, stroke, accent) = Self::stage_palette(*stage);

                    painter.rect_filled(lane_rect, egui::CornerRadius::same(14), fill);
                    painter.line_segment(
                        [
                            egui::pos2(lane_rect.left() + 18.0, lane_rect.top() + 16.0),
                            egui::pos2(lane_rect.left() + 18.0, lane_rect.bottom() - 16.0),
                        ],
                        Stroke::new(3.0, accent),
                    );
                    painter.text(
                        egui::pos2(lane_rect.left() + 34.0, lane_rect.top() + 18.0),
                        egui::Align2::LEFT_TOP,
                        Self::stage_title(*stage),
                        egui::FontId::proportional(15.0),
                        accent,
                    );
                    painter.text(
                        egui::pos2(lane_rect.left() + 34.0, lane_rect.top() + 42.0),
                        egui::Align2::LEFT_TOP,
                        match stage {
                            LearningStage::Fundamentals => "先建立概念、类型和空值理解",
                            LearningStage::QueryBasics => "开始控制读取、筛选和统计结果",
                            LearningStage::RelationshipModel => "理解关系型数据库为什么能连接",
                            LearningStage::Mutations => "进入写入、约束、修改与事务",
                            _ => "",
                        },
                        egui::FontId::proportional(12.0),
                        Self::muted_text_color(ui),
                    );

                    nodes.extend(Self::roadmap_lane_nodes(lane_rect, topics));

                    painter.line_segment(
                        [
                            egui::pos2(lane_rect.left() + 190.0, lane_rect.top() + 18.0),
                            egui::pos2(lane_rect.right() - 18.0, lane_rect.top() + 18.0),
                        ],
                        Stroke::new(1.0, stroke),
                    );
                }

                for (from, to) in [
                    (LearningTopic::Foundations, LearningTopic::DataTypes),
                    (LearningTopic::DataTypes, LearningTopic::NullHandling),
                    (LearningTopic::DataTypes, LearningTopic::SelectBasics),
                    (LearningTopic::Foundations, LearningTopic::SelectBasics),
                    (LearningTopic::Foundations, LearningTopic::Relationships),
                    (LearningTopic::SelectBasics, LearningTopic::FilterAndSort),
                    (LearningTopic::FilterAndSort, LearningTopic::LikePattern),
                    (LearningTopic::FilterAndSort, LearningTopic::Aggregate),
                    (LearningTopic::Relationships, LearningTopic::Join),
                    (LearningTopic::FilterAndSort, LearningTopic::Join),
                    (LearningTopic::SelectBasics, LearningTopic::InsertData),
                    (LearningTopic::FilterAndSort, LearningTopic::UpdateDelete),
                    (LearningTopic::InsertData, LearningTopic::Constraints),
                    (LearningTopic::InsertData, LearningTopic::UpdateDelete),
                    (LearningTopic::Constraints, LearningTopic::Transactions),
                    (LearningTopic::UpdateDelete, LearningTopic::Transactions),
                ] {
                    let from_pos = nodes
                        .iter()
                        .find(|(topic, _)| *topic == from)
                        .map(|(_, node_rect)| node_rect.center())
                        .unwrap_or_default();
                    let to_pos = nodes
                        .iter()
                        .find(|(topic, _)| *topic == to)
                        .map(|(_, node_rect)| node_rect.center())
                        .unwrap_or_default();

                    painter.line_segment(
                        [from_pos, to_pos],
                        Stroke::new(1.8, Color32::from_rgba_unmultiplied(130, 170, 230, 78)),
                    );
                }

                for (topic, node_rect) in nodes {
                    let definition = Self::topic_definition(topic);
                    let is_selected = selected == topic;
                    let (fill, stroke) =
                        Self::topic_fill_and_stroke(definition.status, is_selected);

                    let response = ui.put(
                        node_rect,
                        egui::Button::new(
                            RichText::new(definition.short_title)
                                .strong()
                                .color(Self::body_text_color(ui)),
                        )
                        .fill(fill)
                        .stroke(Stroke::new(1.0, stroke))
                        .corner_radius(egui::CornerRadius::same(10)),
                    );
                    let response = Self::topic_hover_preview(response, topic);

                    if response.clicked() {
                        clicked_topic = Some(topic);
                    }
                }
            });

        clicked_topic
    }

    fn roadmap_lane_nodes(
        lane_rect: egui::Rect,
        topics: &[LearningTopic],
    ) -> Vec<(LearningTopic, egui::Rect)> {
        let node_height = 40.0;
        let gap = 12.0;
        let widths: Vec<f32> = topics
            .iter()
            .map(|topic| {
                (Self::topic_short_title(*topic).chars().count().clamp(4, 12) as f32 * 10.0 + 30.0)
                    .clamp(84.0, 148.0)
            })
            .collect();
        let total_width = widths.iter().sum::<f32>() + gap * topics.len().saturating_sub(1) as f32;
        let node_left = lane_rect.left() + 204.0;
        let node_right = lane_rect.right() - 18.0;
        let available_width = (node_right - node_left).max(total_width);
        let start_x = node_left + ((available_width - total_width) / 2.0).max(0.0);
        let center_y = lane_rect.center().y + 10.0;
        let mut cursor_x = start_x;

        topics
            .iter()
            .zip(widths)
            .map(|(topic, width)| {
                let rect = egui::Rect::from_center_size(
                    egui::pos2(cursor_x + width / 2.0, center_y),
                    egui::vec2(width, node_height),
                );
                cursor_x += width + gap;
                (*topic, rect)
            })
            .collect()
    }

    fn roadmap_stage_card(
        ui: &mut egui::Ui,
        stage: LearningStage,
        selected: LearningTopic,
    ) -> Option<LearningTopic> {
        let mut clicked_topic = None;
        let (fill, stroke, accent) = Self::stage_palette(stage);

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(18, 16))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                    Self::stage_chip(ui, stage);
                    ui.label(
                        RichText::new(Self::stage_title(stage))
                            .strong()
                            .color(accent),
                    );
                    Self::stage_count_chip(ui, stage);
                });
                ui.add_space(4.0);
                ui.label(
                    RichText::new(Self::stage_summary(stage))
                        .small()
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(12.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                    for definition in TOPIC_DEFINITIONS.iter().filter(|item| item.stage == stage) {
                        if let Some(topic) =
                            Self::roadmap_topic_button(ui, definition, selected == definition.topic)
                        {
                            clicked_topic = Some(topic);
                        }
                    }
                });
            });

        clicked_topic
    }

    fn roadmap_topic_button(
        ui: &mut egui::Ui,
        definition: &LearningTopicDefinition,
        selected: bool,
    ) -> Option<LearningTopic> {
        let (fill, stroke) = Self::topic_fill_and_stroke(definition.status, selected);
        let width = (definition.short_title.chars().count().clamp(6, 14) as f32 * 11.0 + 34.0)
            .clamp(92.0, 176.0);

        let response = ui.add_sized(
            [width, 36.0],
            egui::Button::new(
                RichText::new(definition.short_title)
                    .strong()
                    .color(Self::body_text_color(ui)),
            )
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(10)),
        );
        let response = Self::topic_hover_preview(response, definition.topic);

        if response.clicked() {
            Some(definition.topic)
        } else {
            None
        }
    }

    fn topic_hover_preview(response: egui::Response, topic: LearningTopic) -> egui::Response {
        let definition = Self::topic_definition(topic);
        let (_, _, accent) = Self::stage_palette(definition.stage);
        response.on_hover_ui(|ui| {
            ui.set_max_width(360.0);

            egui::Frame::NONE
                .fill(Color32::from_rgba_unmultiplied(34, 40, 56, 244))
                .stroke(Stroke::new(
                    1.0,
                    Color32::from_rgba_unmultiplied(130, 170, 230, 56),
                ))
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(14, 12))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                        ui.label(
                            RichText::new(definition.title)
                                .strong()
                                .color(Self::body_text_color(ui)),
                        );
                        Self::status_chip(ui, definition.status);
                        Self::stage_chip(ui, definition.stage);
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Self::topic_stage_label(topic))
                            .small()
                            .color(Self::muted_text_color(ui)),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("摘要").small().strong().color(accent));
                    ui.label(RichText::new(definition.summary).color(Self::muted_text_color(ui)));
                    ui.add_space(8.0);
                    ui.label(RichText::new("依赖").small().strong().color(accent));
                    ui.label(
                        RichText::new(definition.dependency_text)
                            .small()
                            .color(Self::muted_text_color(ui)),
                    );
                    ui.add_space(8.0);
                    Self::topic_relation_chip_row(ui, "前置", Self::topic_prerequisites(topic));
                    ui.add_space(6.0);
                    Self::topic_relation_chip_row(ui, "后续", Self::topic_next_topics(topic));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(definition.follow_up_text)
                            .small()
                            .color(Self::muted_text_color(ui)),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("点击直接进入").small().strong().color(accent));
                });
        })
    }

    fn stage_chip(ui: &mut egui::Ui, stage: LearningStage) {
        let (fill, stroke, accent) = Self::stage_palette(stage);

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(Self::stage_title(stage))
                        .small()
                        .strong()
                        .color(accent),
                );
            });
    }

    fn status_chip(ui: &mut egui::Ui, status: LearningTopicStatus) {
        let (fill, stroke, text) = Self::status_palette(status);

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(Self::topic_status_short_label(status))
                        .small()
                        .strong()
                        .color(text),
                );
            });
    }

    fn status_count_chip(ui: &mut egui::Ui, status: LearningTopicStatus, count: usize) {
        let (fill, stroke, text) = Self::status_palette(status);

        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} {count}",
                        Self::topic_status_short_label(status)
                    ))
                    .small()
                    .strong()
                    .color(text),
                );
            });
    }

    fn stage_count_chip(ui: &mut egui::Ui, stage: LearningStage) {
        let topic_count = TOPIC_DEFINITIONS
            .iter()
            .filter(|topic| topic.stage == stage)
            .count();
        let available_count = TOPIC_DEFINITIONS
            .iter()
            .filter(|topic| topic.stage == stage && topic.status == LearningTopicStatus::Available)
            .count();
        let (_, stroke, accent) = Self::stage_palette(stage);

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(
                accent.r(),
                accent.g(),
                accent.b(),
                20,
            ))
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(255))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(format!("{available_count}/{topic_count} 已开放"))
                        .small()
                        .strong()
                        .color(accent),
                );
            });
    }

    fn nav_button(ui: &mut egui::Ui, label: &str) -> bool {
        ui.add(
            egui::Button::new(
                RichText::new(label)
                    .small()
                    .strong()
                    .color(Self::body_text_color(ui)),
            )
            .fill(Color32::from_rgba_unmultiplied(120, 120, 130, 24))
            .corner_radius(egui::CornerRadius::same(8)),
        )
        .clicked()
    }

    fn topic_learning_path_card(ui: &mut egui::Ui, topic: LearningTopic) {
        let definition = Self::topic_definition(topic);
        let (fill, stroke, accent) = Self::stage_palette(definition.stage);
        egui::Frame::NONE
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                    ui.label(
                        RichText::new("这一课在整条学习路径中的位置")
                            .strong()
                            .color(accent),
                    );
                    Self::stage_chip(ui, definition.stage);
                    Self::status_chip(ui, definition.status);
                });
                ui.add_space(8.0);
                ui.label(
                    RichText::new(Self::topic_status_label(topic))
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(Self::topic_stage_label(topic)).color(Self::muted_text_color(ui)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("学习目标：{}", definition.summary))
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(definition.dependency_text).color(Self::muted_text_color(ui)),
                );
                ui.add_space(8.0);
                Self::topic_relation_chip_row(ui, "前置知识", Self::topic_prerequisites(topic));
                ui.add_space(8.0);
                Self::topic_relation_chip_row(ui, "后续延伸", Self::topic_next_topics(topic));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(definition.follow_up_text).color(Self::muted_text_color(ui)),
                );
            });
    }

    fn topic_navigation_row(
        ui: &mut egui::Ui,
        state: &HelpState,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

            if let Some(previous) = Self::topic_previous(state.learning_topic) {
                let label = format!("上一课：{}", Self::topic_short_title(previous));
                if Self::action_button(ui, &label, false) {
                    *pending_ui_action = Some(HelpUiAction::Navigate(
                        HelpNavigationAction::SelectTopic(previous),
                    ));
                }
            }

            if Self::action_button(ui, "返回路线图", false) {
                *pending_ui_action = Some(HelpUiAction::Navigate(
                    HelpNavigationAction::SetLearningView(LearningView::Roadmap),
                ));
            }

            if let Some(next) = Self::topic_next(state.learning_topic) {
                let label = format!("下一课：{}", Self::topic_short_title(next));
                if Self::action_button(ui, &label, true) {
                    *pending_ui_action = Some(HelpUiAction::Navigate(
                        HelpNavigationAction::SelectTopic(next),
                    ));
                }
            }
        });
    }

    fn overview_card(
        ui: &mut egui::Ui,
        title: &str,
        items: &[&str],
        button_label: Option<&str>,
    ) -> bool {
        let width = ui.available_width();
        let mut clicked = false;

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(95, 125, 180, 16))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 34),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_min_width((width - 32.0).max(220.0));
                ui.set_max_width((width - 32.0).max(220.0));
                ui.label(
                    RichText::new(title)
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(8.0);

                for item in items {
                    ui.label(
                        RichText::new(format!("• {}", item)).color(Self::muted_text_color(ui)),
                    );
                    ui.add_space(4.0);
                }

                if let Some(label) = button_label {
                    ui.add_space(8.0);
                    if Self::action_button(ui, label, true) {
                        clicked = true;
                    }
                }
            });

        clicked
    }

    fn learning_overview_flow(ui: &mut egui::Ui) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(80, 100, 150, 14))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 28),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(14, 12))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("推荐学习流程")
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(8.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    Self::step_chip(ui, "1. 看总览");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "2. 进路线图");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "3. 学知识点");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "4. 自动演示");
                });
            });
    }

    fn show_onboarding_flow_card(
        ui: &mut egui::Ui,
        context: &HelpContext,
        pending_ui_action: &mut Option<HelpUiAction>,
    ) {
        let width = ui.available_width();
        let mut steps = vec![
            (
                HelpOnboardingStep::EnvironmentCheck,
                "1. 环境检测",
                context.onboarding_environment_checked,
            ),
            (
                HelpOnboardingStep::CreateConnection,
                "2. 新建连接",
                context.onboarding_connection_created,
            ),
            (
                HelpOnboardingStep::InitializeDatabase,
                "3. 初始化数据库",
                context.onboarding_database_initialized,
            ),
        ];

        if context.onboarding_require_user_step {
            steps.push((
                HelpOnboardingStep::CreateUser,
                "4. 创建用户",
                context.onboarding_user_created,
            ));
        }

        steps.push((
            HelpOnboardingStep::RunFirstQuery,
            "5. 执行首条查询",
            context.onboarding_first_query_executed,
        ));

        let completed = steps.iter().filter(|(_, _, done)| *done).count();
        let total = steps.len().max(1);
        let next_step = steps
            .iter()
            .find(|(_, _, done)| !*done)
            .map(|(step, _, _)| *step);

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(84, 124, 210, 14))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(120, 160, 232, 36),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_min_width((width - 32.0).max(260.0));
                ui.set_max_width((width - 32.0).max(260.0));

                ui.label(
                    RichText::new("新手上手闭环流程")
                        .size(17.0)
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("这一段原先在欢迎页。为了避免欢迎页拥挤，已迁到学习指南总览。")
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("已完成 {}/{} 步", completed, total))
                        .small()
                        .color(Self::muted_text_color(ui)),
                );
                ui.add_space(6.0);

                ui.add(
                    egui::ProgressBar::new(completed as f32 / total as f32)
                        .desired_width(ui.available_width())
                        .show_percentage(),
                );
                ui.add_space(10.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    for (_, label, done) in &steps {
                        let text = if *done {
                            format!("✓ {}", label)
                        } else {
                            format!("○ {}", label)
                        };
                        Self::step_chip(ui, &text);
                    }
                });

                if let Some(step) = next_step {
                    ui.add_space(12.0);
                    if Self::action_button(ui, Self::onboarding_action_label(step), true) {
                        *pending_ui_action =
                            Some(HelpUiAction::Dispatch(HelpAction::ContinueOnboarding(step)));
                    }
                } else {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("闭环流程已完成，可以直接进入路线图学习。")
                            .small()
                            .strong()
                            .color(Color32::from_rgb(160, 220, 170)),
                    );
                }
            });
    }

    fn onboarding_action_label(step: HelpOnboardingStep) -> &'static str {
        match step {
            HelpOnboardingStep::EnvironmentCheck => "继续：检测本机环境",
            HelpOnboardingStep::CreateConnection => "继续：新建连接",
            HelpOnboardingStep::InitializeDatabase => "继续：初始化数据库",
            HelpOnboardingStep::CreateUser => "继续：创建用户",
            HelpOnboardingStep::RunFirstQuery => "继续：执行首条查询",
        }
    }

    fn overview_action_card(
        ui: &mut egui::Ui,
        title: &str,
        summary: &str,
        tags: &[&str],
        button_label: &str,
    ) -> bool {
        let width = ui.available_width();
        let mut clicked = false;

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(95, 125, 180, 16))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 34),
            ))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(18, 16))
            .show(ui, |ui| {
                ui.set_min_width((width - 36.0).max(260.0));
                ui.set_max_width((width - 36.0).max(260.0));

                ui.label(
                    RichText::new(title)
                        .size(17.0)
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(summary).color(Self::muted_text_color(ui)));
                ui.add_space(10.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    for tag in tags {
                        Self::step_chip(ui, tag);
                    }
                });

                ui.add_space(12.0);
                if Self::action_button(ui, button_label, true) {
                    clicked = true;
                }
            });

        clicked
    }

    pub(super) fn step_chip(ui: &mut egui::Ui, label: &str) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(120, 130, 160, 26))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(150, 170, 210, 34),
            ))
            .corner_radius(egui::CornerRadius::same(255))
            .inner_margin(egui::Margin::symmetric(10, 5))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(label)
                        .small()
                        .strong()
                        .color(Self::body_text_color(ui)),
                );
            });
    }

    pub(super) fn topic_definition(topic: LearningTopic) -> &'static LearningTopicDefinition {
        TOPIC_DEFINITIONS
            .iter()
            .find(|definition| definition.topic == topic)
            .unwrap_or(&TOPIC_DEFINITIONS[0])
    }

    fn stage_title(stage: LearningStage) -> &'static str {
        match stage {
            LearningStage::Fundamentals => "1. 基础认知",
            LearningStage::QueryBasics => "2. 基础查询",
            LearningStage::RelationshipModel => "3. 关系模型",
            LearningStage::Mutations => "4. 数据写入",
            LearningStage::DesignQuality => "5. 设计与质量",
            LearningStage::Advanced => "6. 性能与进阶",
        }
    }

    fn stage_summary(stage: LearningStage) -> &'static str {
        match stage {
            LearningStage::Fundamentals => "先建立数据库、数据类型和 NULL 的正确心智模型。",
            LearningStage::QueryBasics => {
                "先学会读取、筛选、排序、匹配和统计，这是最常用的基础功。"
            }
            LearningStage::RelationshipModel => {
                "理解主键、外键和 JOIN，才算真正开始理解关系型数据库。"
            }
            LearningStage::Mutations => "新增、修改、删除和事务是最容易犯错的部分，必须单独学习。",
            LearningStage::DesignQuality => {
                "当你能写查询后，下一步是理解结构设计、视图和索引的价值。"
            }
            LearningStage::Advanced => "这些是路线图后段的主题，先知道它们存在，再按需深入。",
        }
    }

    fn stage_palette(stage: LearningStage) -> (Color32, Color32, Color32) {
        match stage {
            LearningStage::Fundamentals => (
                Color32::from_rgba_unmultiplied(68, 104, 166, 24),
                Color32::from_rgba_unmultiplied(130, 180, 255, 48),
                Color32::from_rgb(130, 180, 255),
            ),
            LearningStage::QueryBasics => (
                Color32::from_rgba_unmultiplied(64, 128, 136, 24),
                Color32::from_rgba_unmultiplied(122, 204, 210, 44),
                Color32::from_rgb(142, 214, 220),
            ),
            LearningStage::RelationshipModel => (
                Color32::from_rgba_unmultiplied(94, 88, 146, 22),
                Color32::from_rgba_unmultiplied(176, 168, 238, 42),
                Color32::from_rgb(194, 182, 255),
            ),
            LearningStage::Mutations => (
                Color32::from_rgba_unmultiplied(128, 96, 72, 22),
                Color32::from_rgba_unmultiplied(228, 188, 142, 42),
                Color32::from_rgb(234, 198, 150),
            ),
            LearningStage::DesignQuality => (
                Color32::from_rgba_unmultiplied(82, 118, 90, 20),
                Color32::from_rgba_unmultiplied(168, 212, 170, 40),
                Color32::from_rgb(178, 220, 180),
            ),
            LearningStage::Advanced => (
                Color32::from_rgba_unmultiplied(92, 84, 124, 22),
                Color32::from_rgba_unmultiplied(188, 168, 224, 40),
                Color32::from_rgb(204, 188, 238),
            ),
        }
    }

    pub(super) fn topic_title(topic: LearningTopic) -> &'static str {
        Self::topic_definition(topic).title
    }

    fn topic_short_title(topic: LearningTopic) -> &'static str {
        Self::topic_definition(topic).short_title
    }

    fn topic_stage_label(topic: LearningTopic) -> &'static str {
        match Self::topic_definition(topic).stage {
            LearningStage::Fundamentals => "阶段：建立最小心智模型。",
            LearningStage::QueryBasics => "阶段：开始读取并控制查询结果。",
            LearningStage::RelationshipModel => "阶段：开始理解表之间为什么能连起来。",
            LearningStage::Mutations => "阶段：开始安全地写入、修改和回滚数据。",
            LearningStage::DesignQuality => "阶段：开始理解结构设计和数据质量。",
            LearningStage::Advanced => "阶段：进入进阶主题，开始关注性能与系统能力。",
        }
    }

    fn topic_status_short_label(status: LearningTopicStatus) -> &'static str {
        match status {
            LearningTopicStatus::Available => "可学习",
            LearningTopicStatus::Planned => "规划中",
            LearningTopicStatus::Advanced => "进阶主题",
        }
    }

    fn status_palette(status: LearningTopicStatus) -> (Color32, Color32, Color32) {
        match status {
            LearningTopicStatus::Available => (
                Color32::from_rgba_unmultiplied(72, 104, 152, 52),
                Color32::from_rgba_unmultiplied(140, 188, 255, 58),
                Color32::from_rgb(214, 226, 242),
            ),
            LearningTopicStatus::Planned => (
                Color32::from_rgba_unmultiplied(112, 114, 128, 42),
                Color32::from_rgba_unmultiplied(186, 192, 214, 44),
                Color32::from_rgb(222, 224, 232),
            ),
            LearningTopicStatus::Advanced => (
                Color32::from_rgba_unmultiplied(110, 86, 142, 44),
                Color32::from_rgba_unmultiplied(204, 170, 240, 48),
                Color32::from_rgb(232, 216, 248),
            ),
        }
    }

    fn topic_status_label(topic: LearningTopic) -> &'static str {
        match Self::topic_definition(topic).status {
            LearningTopicStatus::Available => "当前状态：可学习，已提供讲解、练习和可执行演示。",
            LearningTopicStatus::Planned => {
                "当前状态：规划中，已纳入知识体系，但详细课程稍后补齐。"
            }
            LearningTopicStatus::Advanced => "当前状态：进阶主题，建议先完成主干课再深入。",
        }
    }

    fn topic_prerequisites(topic: LearningTopic) -> &'static [LearningTopic] {
        match topic {
            LearningTopic::Foundations => &[],
            LearningTopic::DataTypes => &[LearningTopic::Foundations],
            LearningTopic::NullHandling => &[LearningTopic::Foundations, LearningTopic::DataTypes],
            LearningTopic::SelectBasics => &[LearningTopic::Foundations, LearningTopic::DataTypes],
            LearningTopic::FilterAndSort => &[LearningTopic::SelectBasics],
            LearningTopic::LikePattern => &[LearningTopic::FilterAndSort],
            LearningTopic::Aggregate => {
                &[LearningTopic::SelectBasics, LearningTopic::FilterAndSort]
            }
            LearningTopic::Relationships => &[LearningTopic::Foundations],
            LearningTopic::Join => &[LearningTopic::Relationships, LearningTopic::FilterAndSort],
            LearningTopic::InsertData => &[LearningTopic::SelectBasics, LearningTopic::DataTypes],
            LearningTopic::Constraints => {
                &[LearningTopic::Relationships, LearningTopic::InsertData]
            }
            LearningTopic::UpdateDelete => {
                &[LearningTopic::FilterAndSort, LearningTopic::InsertData]
            }
            LearningTopic::Transactions => {
                &[LearningTopic::InsertData, LearningTopic::UpdateDelete]
            }
            LearningTopic::SchemaDesign => {
                &[LearningTopic::Relationships, LearningTopic::Constraints]
            }
            LearningTopic::Views => &[LearningTopic::SelectBasics, LearningTopic::Join],
            LearningTopic::Indexes => &[LearningTopic::FilterAndSort],
            LearningTopic::Subqueries => &[LearningTopic::SelectBasics, LearningTopic::Aggregate],
            LearningTopic::WindowFunctions => &[LearningTopic::Aggregate],
            LearningTopic::TriggersProcedures => {
                &[LearningTopic::Constraints, LearningTopic::Transactions]
            }
            LearningTopic::QueryPlans => &[LearningTopic::Indexes, LearningTopic::Join],
            LearningTopic::BackupPermissions => &[LearningTopic::Transactions],
        }
    }

    fn topic_next_topics(topic: LearningTopic) -> &'static [LearningTopic] {
        match topic {
            LearningTopic::Foundations => &[LearningTopic::DataTypes, LearningTopic::SelectBasics],
            LearningTopic::DataTypes => &[LearningTopic::NullHandling, LearningTopic::SelectBasics],
            LearningTopic::NullHandling => {
                &[LearningTopic::SelectBasics, LearningTopic::FilterAndSort]
            }
            LearningTopic::SelectBasics => {
                &[LearningTopic::FilterAndSort, LearningTopic::InsertData]
            }
            LearningTopic::FilterAndSort => &[
                LearningTopic::LikePattern,
                LearningTopic::Aggregate,
                LearningTopic::UpdateDelete,
            ],
            LearningTopic::LikePattern => &[LearningTopic::Aggregate, LearningTopic::Join],
            LearningTopic::Aggregate => &[LearningTopic::Relationships, LearningTopic::Views],
            LearningTopic::Relationships => &[LearningTopic::Join, LearningTopic::Constraints],
            LearningTopic::Join => &[LearningTopic::Views, LearningTopic::QueryPlans],
            LearningTopic::InsertData => &[LearningTopic::Constraints, LearningTopic::UpdateDelete],
            LearningTopic::Constraints => {
                &[LearningTopic::Transactions, LearningTopic::SchemaDesign]
            }
            LearningTopic::UpdateDelete => &[LearningTopic::Transactions],
            LearningTopic::Transactions => &[
                LearningTopic::SchemaDesign,
                LearningTopic::BackupPermissions,
            ],
            LearningTopic::SchemaDesign => &[LearningTopic::Views, LearningTopic::Indexes],
            LearningTopic::Views => &[LearningTopic::Subqueries],
            LearningTopic::Indexes => &[LearningTopic::QueryPlans],
            LearningTopic::Subqueries => &[LearningTopic::WindowFunctions],
            LearningTopic::WindowFunctions => &[LearningTopic::TriggersProcedures],
            LearningTopic::TriggersProcedures => &[LearningTopic::BackupPermissions],
            LearningTopic::QueryPlans => &[LearningTopic::BackupPermissions],
            LearningTopic::BackupPermissions => &[],
        }
    }

    fn topic_relation_chip_row(ui: &mut egui::Ui, label: &str, topics: &[LearningTopic]) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
            ui.label(
                RichText::new(label)
                    .small()
                    .strong()
                    .color(Self::muted_text_color(ui)),
            );

            if topics.is_empty() {
                Self::step_chip(ui, "无");
            } else {
                for topic in topics {
                    Self::step_chip(ui, Self::topic_short_title(*topic));
                }
            }
        });
    }

    fn topic_fill_and_stroke(status: LearningTopicStatus, selected: bool) -> (Color32, Color32) {
        if selected {
            return (
                Color32::from_rgb(64, 112, 190),
                Color32::from_rgb(140, 200, 255),
            );
        }

        match status {
            LearningTopicStatus::Available => (
                Color32::from_rgba_unmultiplied(58, 84, 122, 220),
                Color32::from_rgba_unmultiplied(126, 176, 238, 54),
            ),
            LearningTopicStatus::Planned => (
                Color32::from_rgba_unmultiplied(86, 90, 106, 214),
                Color32::from_rgba_unmultiplied(182, 188, 210, 42),
            ),
            LearningTopicStatus::Advanced => (
                Color32::from_rgba_unmultiplied(78, 70, 98, 214),
                Color32::from_rgba_unmultiplied(196, 164, 228, 46),
            ),
        }
    }

    fn topic_previous(topic: LearningTopic) -> Option<LearningTopic> {
        let index = Self::LEARNING_SEQUENCE
            .iter()
            .position(|item| *item == topic)?;
        index
            .checked_sub(1)
            .map(|previous_index| Self::LEARNING_SEQUENCE[previous_index])
    }

    fn topic_next(topic: LearningTopic) -> Option<LearningTopic> {
        let index = Self::LEARNING_SEQUENCE
            .iter()
            .position(|item| *item == topic)?;
        Self::LEARNING_SEQUENCE.get(index + 1).copied()
    }

    fn learning_status_card(ui: &mut egui::Ui, context: &HelpContext) {
        let accent = Color32::from_rgb(130, 180, 255);
        let ok = Color32::from_rgb(160, 220, 170);
        let muted = Color32::from_rgb(150, 150, 160);
        let width = ui.available_width();

        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(95, 125, 180, 18))
            .stroke(egui::Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 40),
            ))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_min_width((width - 32.0).max(260.0));
                ui.set_max_width((width - 32.0).max(260.0));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(22.0, 10.0);

                    Self::status_item(
                        ui,
                        "当前连接",
                        context
                            .active_connection_name
                            .as_deref()
                            .unwrap_or("未连接"),
                        accent,
                    );
                    Self::status_item(
                        ui,
                        "当前表",
                        context.selected_table.as_deref().unwrap_or("未选择"),
                        muted,
                    );
                    Self::status_item(
                        ui,
                        "结果区",
                        if context.has_result {
                            "已有结果"
                        } else {
                            "暂无结果"
                        },
                        if context.has_result { ok } else { muted },
                    );
                    Self::status_item(
                        ui,
                        "SQL 编辑器",
                        if context.show_sql_editor {
                            "已展开"
                        } else {
                            "未展开"
                        },
                        if context.show_sql_editor { ok } else { muted },
                    );
                    Self::status_item(
                        ui,
                        "ER 图",
                        if context.show_er_diagram {
                            "已打开"
                        } else {
                            "未打开"
                        },
                        if context.show_er_diagram { ok } else { muted },
                    );
                });
            });
    }

    fn status_item(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
        ui.vertical(|ui| {
            ui.label(RichText::new(label).small().color(Color32::GRAY));
            ui.label(RichText::new(value).strong().color(color));
        });
    }
}
