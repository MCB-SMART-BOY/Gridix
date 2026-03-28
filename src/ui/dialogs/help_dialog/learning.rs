use super::types::{
    HelpAction, HelpContext, HelpOnboardingStep, HelpState, LearningStage, LearningTopic,
    LearningTopicDefinition, LearningTopicStatus, LearningView, TOPIC_DEFINITIONS,
};
use super::*;
use egui::Stroke;

impl HelpDialog {
    pub(super) fn show_learning_guide(
        ui: &mut egui::Ui,
        state: &mut HelpState,
        context: &HelpContext,
        action: &mut Option<HelpAction>,
    ) {
        match state.learning_view {
            LearningView::Overview => Self::show_learning_overview(ui, state, context, action),
            LearningView::Roadmap => Self::show_learning_roadmap(ui, state),
            LearningView::TopicDetail => {
                Self::show_learning_topic_detail(ui, state, context, action)
            }
        }
    }

    fn show_learning_overview(
        ui: &mut egui::Ui,
        state: &mut HelpState,
        context: &HelpContext,
        action: &mut Option<HelpAction>,
    ) {
        let accent = Color32::from_rgb(130, 180, 255);
        let text = Color32::from_rgb(220, 220, 220);
        let muted = Color32::from_rgb(145, 145, 155);

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

        Self::show_onboarding_flow_card(ui, context, action);
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
        Self::learning_overview_flow(ui);
        ui.add_space(14.0);

        if Self::overview_action_card(
            ui,
            "查看学习路线图",
            "先看一张真正的依赖路线图，再看完整知识地图。这样你能同时知道当前该学什么，以及后面还有哪些主题。",
            &["核心依赖图", "完整知识地图", "可学习 / 规划中 / 进阶主题"],
            "进入路线图",
        ) {
            state.learning_view = LearningView::Roadmap;
        }

        ui.add_space(12.0);

        if Self::overview_action_card(
            ui,
            "从学习示例开始第一课",
            "如果你想直接动手，Gridix 会自动创建本地 SQLite 学习示例库，并带你进入第一课。",
            &["不会改动真实连接", "适合第一次使用", "可随时重置示例库"],
            "打开示例并开始第一课",
        ) {
            state.learning_topic = LearningTopic::Foundations;
            state.learning_view = LearningView::TopicDetail;
            *action = Some(HelpAction::EnsureLearningSample { reset: false });
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

    fn show_learning_roadmap(ui: &mut egui::Ui, state: &mut HelpState) {
        let muted = Color32::from_rgb(145, 145, 155);

        Self::learning_nav(ui, state, "数据库知识点路线图");
        ui.add_space(12.0);

        ui.label(
            RichText::new("先看核心依赖图，再看完整知识地图。")
                .color(Color32::from_rgb(220, 220, 220)),
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
        if let Some(topic) = Self::core_roadmap_graph(ui, &mut state.learning_topic) {
            state.learning_topic = topic;
            state.learning_view = LearningView::TopicDetail;
        }
        ui.add_space(18.0);

        ui.label(
            RichText::new("完整知识地图")
                .strong()
                .color(Color32::from_rgb(220, 225, 235)),
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
            if let Some(topic) = Self::roadmap_stage_card(ui, stage, &mut state.learning_topic) {
                state.learning_topic = topic;
                state.learning_view = LearningView::TopicDetail;
            }
            ui.add_space(12.0);
        }

        ui.add_space(10.0);
        ui.label(
            RichText::new("连线代表推荐依赖关系；上面的主干图讲“先学什么”，下面的完整地图讲“后面还能学什么”。")
                .small()
                .color(Color32::from_rgb(130, 180, 255)),
        );
    }

    fn show_learning_topic_detail(
        ui: &mut egui::Ui,
        state: &mut HelpState,
        context: &HelpContext,
        action: &mut Option<HelpAction>,
    ) {
        let muted = Color32::from_rgb(145, 145, 155);

        Self::learning_nav(ui, state, Self::topic_title(state.learning_topic));
        ui.add_space(12.0);
        Self::learning_status_card(ui, context);
        ui.add_space(16.0);
        Self::topic_learning_path_card(ui, state.learning_topic);
        ui.add_space(16.0);

        match state.learning_topic {
            LearningTopic::Foundations => Self::show_foundations_topic(ui, action),
            LearningTopic::DataTypes => Self::show_data_types_topic(ui, action),
            LearningTopic::NullHandling => Self::show_null_handling_topic(ui, action),
            LearningTopic::SelectBasics => Self::show_select_topic(ui, action),
            LearningTopic::FilterAndSort => Self::show_filter_sort_topic(ui, action),
            LearningTopic::LikePattern => Self::show_like_topic(ui, action),
            LearningTopic::Aggregate => Self::show_aggregate_topic(ui, action),
            LearningTopic::Relationships => Self::show_relationships_topic(ui, action),
            LearningTopic::Join => Self::show_join_topic(ui, action),
            LearningTopic::InsertData => Self::show_insert_topic(ui, action),
            LearningTopic::Constraints => Self::show_constraints_topic(ui, action),
            LearningTopic::UpdateDelete => Self::show_update_delete_topic(ui, action),
            LearningTopic::Transactions => Self::show_transactions_topic(ui, action),
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
        Self::topic_navigation_row(ui, state);
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

    fn learning_nav(ui: &mut egui::Ui, state: &mut HelpState, title: &str) {
        let accent = Color32::from_rgb(130, 180, 255);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);

            if Self::nav_button(ui, "返回总览") {
                state.learning_view = LearningView::Overview;
            }
            if Self::nav_button(ui, "查看学习路线图") {
                state.learning_view = LearningView::Roadmap;
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
                        .color(Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("先看主干，再决定是否延伸到设计、性能和系统主题。")
                        .small()
                        .color(Color32::from_rgb(182, 186, 194)),
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

    fn core_roadmap_graph(
        ui: &mut egui::Ui,
        selected: &mut LearningTopic,
    ) -> Option<LearningTopic> {
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
                        .color(Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        "这张图只画当前主干课，目的是先把数据库入门最关键的一条路径看清楚。",
                    )
                    .small()
                    .color(Color32::from_rgb(180, 184, 194)),
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
                        Color32::from_rgb(186, 190, 198),
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
                    let is_selected = *selected == topic;
                    let (fill, stroke) =
                        Self::topic_fill_and_stroke(definition.status, is_selected);

                    let response = ui.put(
                        node_rect,
                        egui::Button::new(
                            RichText::new(definition.short_title)
                                .strong()
                                .color(Color32::from_rgb(235, 238, 245)),
                        )
                        .fill(fill)
                        .stroke(Stroke::new(1.0, stroke))
                        .corner_radius(egui::CornerRadius::same(10)),
                    );
                    let response = Self::topic_hover_preview(response, topic);

                    if response.clicked() {
                        *selected = topic;
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
        selected: &mut LearningTopic,
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
                        .color(Color32::from_rgb(180, 184, 194)),
                );
                ui.add_space(12.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);
                    for definition in TOPIC_DEFINITIONS.iter().filter(|item| item.stage == stage) {
                        if let Some(topic) = Self::roadmap_topic_button(
                            ui,
                            definition,
                            *selected == definition.topic,
                        ) {
                            *selected = topic;
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
                    .color(Color32::from_rgb(235, 238, 245)),
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
                                .color(Color32::from_rgb(228, 232, 240)),
                        );
                        Self::status_chip(ui, definition.status);
                        Self::stage_chip(ui, definition.stage);
                    });
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(Self::topic_stage_label(topic))
                            .small()
                            .color(Color32::from_rgb(186, 190, 198)),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("摘要").small().strong().color(accent));
                    ui.label(
                        RichText::new(definition.summary).color(Color32::from_rgb(210, 214, 222)),
                    );
                    ui.add_space(8.0);
                    ui.label(RichText::new("依赖").small().strong().color(accent));
                    ui.label(
                        RichText::new(definition.dependency_text)
                            .small()
                            .color(Color32::from_rgb(186, 190, 198)),
                    );
                    ui.add_space(8.0);
                    Self::topic_relation_chip_row(ui, "前置", Self::topic_prerequisites(topic));
                    ui.add_space(6.0);
                    Self::topic_relation_chip_row(ui, "后续", Self::topic_next_topics(topic));
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(definition.follow_up_text)
                            .small()
                            .color(Color32::from_rgb(186, 190, 198)),
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
                    .color(Color32::from_rgb(235, 238, 245)),
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
                        .color(Color32::from_rgb(205, 208, 216)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(Self::topic_stage_label(topic))
                        .color(Color32::from_rgb(205, 208, 216)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("学习目标：{}", definition.summary))
                        .color(Color32::from_rgb(205, 208, 216)),
                );
                ui.add_space(4.0);
                ui.label(
                    RichText::new(definition.dependency_text)
                        .color(Color32::from_rgb(205, 208, 216)),
                );
                ui.add_space(8.0);
                Self::topic_relation_chip_row(ui, "前置知识", Self::topic_prerequisites(topic));
                ui.add_space(8.0);
                Self::topic_relation_chip_row(ui, "后续延伸", Self::topic_next_topics(topic));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(definition.follow_up_text)
                        .color(Color32::from_rgb(205, 208, 216)),
                );
            });
    }

    fn topic_navigation_row(ui: &mut egui::Ui, state: &mut HelpState) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

            if let Some(previous) = Self::topic_previous(state.learning_topic) {
                let label = format!("上一课：{}", Self::topic_short_title(previous));
                if Self::action_button(ui, &label, false) {
                    state.learning_topic = previous;
                }
            }

            if Self::action_button(ui, "返回路线图", false) {
                state.learning_view = LearningView::Roadmap;
            }

            if let Some(next) = Self::topic_next(state.learning_topic) {
                let label = format!("下一课：{}", Self::topic_short_title(next));
                if Self::action_button(ui, &label, true) {
                    state.learning_topic = next;
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
                        .color(Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(8.0);

                for item in items {
                    ui.label(
                        RichText::new(format!("• {}", item))
                            .color(Color32::from_rgb(205, 208, 216)),
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
                        .color(Color32::from_rgb(220, 225, 235)),
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
        action: &mut Option<HelpAction>,
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
                        .color(Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("这一段原先在欢迎页。为了避免欢迎页拥挤，已迁到学习指南总览。")
                        .color(Color32::from_rgb(205, 208, 216)),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("已完成 {}/{} 步", completed, total))
                        .small()
                        .color(Color32::from_rgb(145, 145, 155)),
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
                        *action = Some(HelpAction::ContinueOnboarding(step));
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
                        .color(Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(summary).color(Color32::from_rgb(205, 208, 216)));
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

    fn step_chip(ui: &mut egui::Ui, label: &str) {
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
                        .color(Color32::from_rgb(216, 220, 230)),
                );
            });
    }

    fn topic_definition(topic: LearningTopic) -> &'static LearningTopicDefinition {
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

    fn topic_title(topic: LearningTopic) -> &'static str {
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
                    .color(Color32::from_rgb(200, 204, 214)),
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

    fn show_foundations_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "数据库、表、行、列分别是什么？",
            "先建立一个正确的心智模型，再学 SQL 才不会乱。",
        );

        Self::concept_card(
            ui,
            "核心概念",
            &[
                "数据库可以理解为一组有关联的数据集合。",
                "表是数据库里的一个主题区域，例如 customers、orders。",
                "行是一条记录，列是这条记录的一个字段。",
                "学习数据库时，先学会“从表里读数据”，再学更复杂的关系和聚合。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先点下方“打开学习示例库”。",
                "2. 在左侧连接列表里选中 `Gridix 学习示例`。",
                "3. 在表列表里打开 `customers` 表。",
                "4. 观察结果区：每一行是一个客户，每一列是客户的属性。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "打开学习示例库",
                HelpAction::EnsureLearningSample { reset: false },
            )),
            Some((
                "自动查看 customers 表",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, city, level FROM customers ORDER BY id LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开新建连接窗口", HelpAction::OpenConnectionDialog)),
        );
    }

    fn show_data_types_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "数据类型：每一列为什么不能什么都塞",
            "数据库列有类型，不只是为了规范书写，更是为了约束存储和比较行为。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "常见类型包括整数、文本、浮点数、日期时间等。",
                "同一张表里，不同列通常表示不同含义，所以会有不同类型。",
                "类型会影响排序、比较和写入；例如数字和文本的比较方式不同。",
                "看懂类型，是理解表结构和写 INSERT / UPDATE 的前提。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开学习示例库里的 `products` 表。",
                "2. 执行 `PRAGMA table_info('products');` 查看列定义。",
                "3. 再执行 `SELECT id, name, price, typeof(price) AS price_type FROM products ORDER BY id LIMIT 5;`。",
                "4. 观察：`price` 是数值列，不是普通文本。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动查看 products 列类型",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "PRAGMA table_info('products');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动演示 typeof(price)",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT id, name, price, typeof(price) AS price_type FROM products ORDER BY id LIMIT 5;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开学习示例库", HelpAction::EnsureLearningSample { reset: false })),
        );
    }

    fn show_null_handling_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "NULL：缺失值不是空字符串，也不是 0",
            "很多数据库初学者的问题，不是 SQL 语法错，而是把 NULL 当成普通值来理解。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`NULL` 表示“当前没有值”或“未知”，不是空文本。",
                "判断 NULL 要用 `IS NULL` / `IS NOT NULL`，而不是 `= NULL`。",
                "NULL 经常出现在可选字段里，例如邮箱、发货时间、备注。",
                "学会处理 NULL，查询结果才不会漏掉或误判数据。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `customers` 表，执行 `SELECT id, name, email FROM customers WHERE email IS NULL;`。",
                "2. 再执行 `SELECT id, status, shipped_at FROM orders WHERE shipped_at IS NULL ORDER BY id;`。",
                "3. 观察哪些记录因为“还没有值”而显示为空。",
                "4. 再尝试把 `IS NULL` 改成 `IS NOT NULL`，比较结果差异。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示 email 的 NULL 查询",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, email FROM customers WHERE email IS NULL ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动演示 shipped_at 的 NULL 查询",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT id, status, shipped_at FROM orders WHERE shipped_at IS NULL ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("重置学习示例库", HelpAction::EnsureLearningSample { reset: true })),
        );
    }

    fn show_select_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "SELECT 基础：从表里读取你需要的列",
            "数据库学习的第一步，不是修改数据，而是读懂数据。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`SELECT` 用来取数据。",
                "`FROM` 指定从哪张表取数据。",
                "`LIMIT` 控制先看多少行，适合新手避免结果太长。",
                "一次只挑 2 到 4 列最容易观察数据结构。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `customers` 表。",
                "2. 按 Ctrl+J 打开 SQL 编辑器。",
                "3. 输入 `SELECT id, name, city FROM customers LIMIT 5;`。",
                "4. 按 Ctrl+Enter，看结果区是否出现 5 条客户记录。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示 SELECT",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, city FROM customers ORDER BY id LIMIT 5;".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
            None,
        );
    }

    fn show_like_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "LIKE：在文本里按关键字模糊匹配",
            "当你记不住完整值，只知道一部分文本时，LIKE 是最直接的入口。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`LIKE` 常用于文本列匹配。",
                "`%` 表示任意长度字符，`_` 表示单个字符。",
                "`LIKE '%ing%'` 的意思是“包含 ing 这段文本”。",
                "模糊匹配适合搜索，但通常比精准条件更宽，所以更要注意结果范围。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `customers` 表。",
                "2. 执行 `SELECT id, name, city FROM customers WHERE city LIKE '%ing%' ORDER BY id;`。",
                "3. 观察哪些城市名里包含 `ing`。",
                "4. 再执行 `SELECT id, name FROM products WHERE name LIKE '%Mouse%';`，体验另一种文本搜索。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示城市 LIKE 查询",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "SELECT id, name, city FROM customers WHERE city LIKE '%ing%' ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动演示商品名 LIKE 查询",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT id, name, category FROM products WHERE name LIKE '%Mouse%' ORDER BY id;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            None,
        );
    }

    fn show_filter_sort_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "WHERE 与 ORDER BY：筛选你想看的数据，再排序",
            "真实数据库查询通常不是“全表扫一遍”，而是先筛再排。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`WHERE` 用来筛掉不需要的行。",
                "`ORDER BY` 决定结果呈现顺序。",
                "`DESC` 表示从大到小，`ASC` 表示从小到大。",
                "筛选与排序组合后，才是日常工作里最常用的查询。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `products` 表。",
                "2. 在编辑器输入 `SELECT id, name, category, price FROM products WHERE price >= 80 ORDER BY price DESC LIMIT 8;`。",
                "3. 执行后观察：结果只保留价格较高的商品，并按价格从高到低排序。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示筛选与排序",
                HelpAction::RunLearningQuery {
                    table: Some("products".to_string()),
                    sql: "SELECT id, name, category, price FROM products WHERE price >= 80 ORDER BY price DESC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开学习示例库", HelpAction::EnsureLearningSample { reset: false })),
            None,
        );
    }

    fn show_aggregate_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "GROUP BY：从明细数据里提炼出统计结论",
            "数据库不仅能列记录，还能帮你总结规律。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`COUNT` 用来计数，`SUM` 用来求和。",
                "`GROUP BY` 决定按什么维度汇总。",
                "只看明细时你看到的是“发生了什么”，做聚合后你看到的是“整体规律”。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开 `orders` 表。",
                "2. 输入 `SELECT status, COUNT(*) AS order_count, ROUND(SUM(total_amount), 2) AS total_sales FROM orders GROUP BY status ORDER BY total_sales DESC;`。",
                "3. 执行后观察：每种订单状态对应多少笔订单、累计销售额是多少。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示 GROUP BY",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT status, COUNT(*) AS order_count, ROUND(SUM(total_amount), 2) AS total_sales FROM orders GROUP BY status ORDER BY total_sales DESC;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("重置学习示例库", HelpAction::EnsureLearningSample { reset: true })),
            None,
        );
    }

    fn show_relationships_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "主键、外键、关系图：理解表为什么能连起来",
            "如果不理解主键和外键，JOIN 只是会写，不算真正理解关系型数据库。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "主键是每一行的唯一标识，例如 `customers.id`。",
                "外键指向另一张表的主键，例如 `orders.customer_id` 指向 `customers.id`。",
                "ER 图把这些关系用图形方式表现出来，非常适合新手建立全局理解。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 打开学习示例库。",
                "2. 按 Ctrl+R 打开 ER 图。",
                "3. 找到 `customers -> orders -> order_items -> products` 这条关系链。",
                "4. 再执行 `PRAGMA foreign_key_list('order_items');`，观察外键具体指向哪张表。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some(("自动打开学习示例 ER 图", HelpAction::ShowLearningErDiagram)),
            Some((
                "自动查看 order_items 外键",
                HelpAction::RunLearningQuery {
                    table: Some("order_items".to_string()),
                    sql: "PRAGMA foreign_key_list('order_items');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "打开学习示例库",
                HelpAction::EnsureLearningSample { reset: false },
            )),
        );
    }

    fn show_join_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "JOIN：把分散在不同表里的信息拼起来",
            "关系型数据库最重要的价值之一，就是通过外键和 JOIN 组合数据。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "订单和客户通常不会放在一张超大表里。",
                "`orders.customer_id = customers.id` 这样的字段关系，就是表之间的连接点。",
                "`JOIN` 允许你在一张结果里同时看到客户和订单信息。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先理解 `orders` 保存订单，`customers` 保存客户。",
                "2. 在编辑器输入 `SELECT o.id AS order_id, c.name AS customer, o.status, o.total_amount FROM orders o JOIN customers c ON c.id = o.customer_id ORDER BY o.total_amount DESC LIMIT 8;`。",
                "3. 执行后观察：订单信息和客户姓名已经出现在同一张结果表里。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示 JOIN",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT o.id AS order_id, c.name AS customer, o.status, o.total_amount FROM orders o JOIN customers c ON c.id = o.customer_id ORDER BY o.total_amount DESC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "打开 ER 图辅助理解",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "SELECT o.id AS order_id, c.name AS customer, o.status, o.total_amount FROM orders o JOIN customers c ON c.id = o.customer_id ORDER BY o.total_amount DESC LIMIT 8;"
                        .to_string(),
                    open_er_diagram: true,
                },
            )),
            None,
        );
    }

    fn show_insert_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "INSERT：向表里新增一条记录",
            "写入数据之前，先确认要写入哪张表、哪些列，以及值的顺序是否对应。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`INSERT INTO table (列...) VALUES (值...)` 用来新增一行数据。",
                "显式写出列名，比只写 `VALUES (...)` 更安全，也更适合新手。",
                "插入的数据必须和列定义匹配，例如文本列要给文本，数值列要给数值。",
                "写操作会改变数据库状态，所以学习时最好先在示例库中练习。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先打开学习示例库，选中 `customers` 表。",
                "2. 在编辑器输入 `INSERT INTO customers (id, name, city, level) VALUES (7, 'Grace He', 'Suzhou', 'Silver');`。",
                "3. 执行后，再运行 `SELECT id, name, city, level FROM customers ORDER BY id DESC LIMIT 3;`。",
                "4. 观察结果区：新增客户已经出现在表中。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示 INSERT",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql: "INSERT INTO customers (id, name, city, level) VALUES (7, 'Grace He', 'Suzhou', 'Silver');"
                        .to_string(),
                    preview_table: Some("customers".to_string()),
                    preview_sql:
                        "SELECT id, name, city, level FROM customers ORDER BY id DESC LIMIT 3;"
                            .to_string(),
                    success_message: "INSERT 演示已完成，已为学习示例库新增一条客户记录。"
                        .to_string(),
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
            None,
        );
    }

    fn show_constraints_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "约束：数据库用什么规则保护数据质量",
            "约束不是语法装饰，而是数据库层面最重要的自我保护机制之一。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`PRIMARY KEY` 保证唯一标识。",
                "`NOT NULL` 要求这一列必须有值，`DEFAULT` 提供默认值。",
                "`FOREIGN KEY` 让表之间的关系真正被数据库认识。",
                "约束的价值在于：即使应用代码写错，数据库也能拦住一部分坏数据。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先执行 `PRAGMA table_info('customers');`，观察哪些列不允许为空、哪些列带默认值。",
                "2. 再执行 `PRAGMA foreign_key_list('orders');`，观察订单表如何指向客户表。",
                "3. 如果想更直观，再打开 ER 图，把图形关系和外键信息对上。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动查看 customers 约束",
                HelpAction::RunLearningQuery {
                    table: Some("customers".to_string()),
                    sql: "PRAGMA table_info('customers');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some((
                "自动查看 orders 外键",
                HelpAction::RunLearningQuery {
                    table: Some("orders".to_string()),
                    sql: "PRAGMA foreign_key_list('orders');".to_string(),
                    open_er_diagram: false,
                },
            )),
            Some(("打开学习示例 ER 图", HelpAction::ShowLearningErDiagram)),
        );
    }

    fn show_update_delete_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "UPDATE 与 DELETE：先筛选，再修改或删除",
            "真正危险的不是写操作本身，而是不带条件地改整张表。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`UPDATE` 修改已有记录，`DELETE` 删除已有记录。",
                "这两类语句几乎都应该先配合 `WHERE` 使用，否则容易误改整张表。",
                "在真实环境里，最好先写一条 `SELECT ... WHERE ...` 预览受影响的行。",
                "学习时可以随时重置示例库，所以这里适合反复练习。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先运行 `SELECT id, status FROM orders WHERE id = 1004;`，确认要修改的是哪一行。",
                "2. 再执行 `UPDATE orders SET status = 'SHIPPED' WHERE id = 1004;`。",
                "3. 然后执行 `SELECT id, status FROM orders WHERE id = 1004;`，观察状态是否变化。",
                "4. 如果要练习删除，先重置示例库，再执行 `DELETE FROM orders WHERE id = 1006;`，最后用 `SELECT COUNT(*) FROM orders WHERE id = 1006;` 验证。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示 UPDATE",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql: "UPDATE orders SET status = 'SHIPPED' WHERE id = 1004;"
                        .to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql: "SELECT id, status, total_amount FROM orders WHERE id = 1004;"
                        .to_string(),
                    success_message: "UPDATE 演示已完成，订单 1004 的状态已更新。".to_string(),
                },
            )),
            Some((
                "自动演示 DELETE",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql: "DELETE FROM orders WHERE id = 1006;".to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql:
                        "SELECT COUNT(*) AS deleted_row_count FROM orders WHERE id = 1006;"
                            .to_string(),
                    success_message: "DELETE 演示已完成，订单 1006 已从学习示例库移除。"
                        .to_string(),
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
        );
    }

    fn show_transactions_topic(ui: &mut egui::Ui, action: &mut Option<HelpAction>) {
        Self::topic_header(
            ui,
            "事务：一批操作为什么要么全成功、要么全撤销",
            "事务是数据库最关键的安全能力之一，它保护的是“多步修改”的一致性。",
        );

        Self::concept_card(
            ui,
            "你要理解的点",
            &[
                "`BEGIN` 表示事务开始，`COMMIT` 表示提交，`ROLLBACK` 表示撤销。",
                "当几步修改必须一起成功时，事务能避免“只改到一半”的中间状态。",
                "新手最重要的习惯不是背 ACID，而是先知道事务能保护写操作。",
                "在学习示例库中，你可以安全地演示提交和回滚。",
            ],
        );

        Self::practice_card(
            ui,
            "手动练习",
            &[
                "1. 先查看 `SELECT id, status FROM orders WHERE id = 1004;`。",
                "2. 执行 `BEGIN; UPDATE orders SET status = 'PAID' WHERE id = 1004; ROLLBACK;`。",
                "3. 再查一次同一条记录，观察状态没有变化。",
                "4. 如果把 `ROLLBACK` 换成 `COMMIT`，结果才会真正保留下来。",
            ],
        );

        Self::action_row(
            ui,
            action,
            Some((
                "自动演示事务回滚",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql:
                        "BEGIN;\nUPDATE orders SET status = 'PAID' WHERE id = 1004;\nROLLBACK;"
                            .to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql: "SELECT id, status, total_amount FROM orders WHERE id = 1004;"
                        .to_string(),
                    success_message: "事务回滚演示已完成，订单 1004 保持原始状态。".to_string(),
                },
            )),
            Some((
                "自动演示事务提交",
                HelpAction::RunLearningMutationDemo {
                    reset: true,
                    mutation_sql:
                        "BEGIN;\nUPDATE orders SET status = 'PAID' WHERE id = 1004;\nCOMMIT;"
                            .to_string(),
                    preview_table: Some("orders".to_string()),
                    preview_sql: "SELECT id, status, total_amount FROM orders WHERE id = 1004;"
                        .to_string(),
                    success_message: "事务提交演示已完成，订单 1004 的状态已真正更新。".to_string(),
                },
            )),
            Some((
                "重置学习示例库",
                HelpAction::EnsureLearningSample { reset: true },
            )),
        );
    }

    fn show_roadmap_preview_topic(ui: &mut egui::Ui, topic: LearningTopic) {
        Self::topic_header(
            ui,
            Self::topic_title(topic),
            "这个知识点已经放进完整路线图里，但当前阶段先展示它的位置、价值和前置依赖。",
        );

        Self::concept_card(
            ui,
            "为什么它重要",
            &[
                Self::topic_definition(topic).summary,
                Self::topic_definition(topic).dependency_text,
                Self::topic_definition(topic).follow_up_text,
            ],
        );

        let preview_hint = match Self::topic_definition(topic).status {
            LearningTopicStatus::Planned => {
                "这是下一阶段会逐步补齐的主题，后续会增加示例、练习和自动演示。"
            }
            LearningTopicStatus::Advanced => {
                "这是进阶主题，先知道它存在和依赖关系即可，不建议现在跳过去硬学。"
            }
            LearningTopicStatus::Available => "这个主题已经可以学习。",
        };

        Self::practice_card(
            ui,
            "当前建议",
            &[
                preview_hint,
                "先完成前置知识点，再回到这里继续推进整条学习路线。",
                "如果你只是想建立全局认知，这一页已经足够告诉你它为什么重要。",
            ],
        );
    }

    fn topic_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
        egui::Frame::NONE
            .fill(Color32::from_rgba_unmultiplied(95, 125, 180, 18))
            .stroke(Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(130, 170, 230, 36),
            ))
            .corner_radius(egui::CornerRadius::same(12))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(title)
                        .size(19.0)
                        .strong()
                        .color(Color32::from_rgb(130, 180, 255)),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(subtitle).color(Color32::from_rgb(205, 208, 216)));
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    Self::step_chip(ui, "先理解概念");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "再手动练习");
                    ui.label(RichText::new(">").color(Color32::GRAY));
                    Self::step_chip(ui, "不会时点自动演示");
                });
            });
        ui.add_space(14.0);
    }

    fn concept_card(ui: &mut egui::Ui, title: &str, items: &[&str]) {
        Self::info_card(
            ui,
            title,
            "理解概念",
            "先把概念和边界想清楚，再去下面实际操作。",
            Color32::from_rgba_unmultiplied(90, 140, 210, 20),
            Color32::from_rgba_unmultiplied(120, 170, 230, 44),
            Color32::from_rgb(130, 180, 255),
            items,
        );
        ui.add_space(12.0);
    }

    fn practice_card(ui: &mut egui::Ui, title: &str, items: &[&str]) {
        Self::info_card(
            ui,
            title,
            "动手练习",
            "按顺序操作；如果卡住了，直接用下面的自动演示验证。",
            Color32::from_rgba_unmultiplied(92, 180, 118, 18),
            Color32::from_rgba_unmultiplied(100, 190, 126, 40),
            Color32::from_rgb(146, 214, 160),
            items,
        );
        ui.add_space(12.0);
    }

    fn info_card(
        ui: &mut egui::Ui,
        title: &str,
        section_label: &str,
        intro: &str,
        fill: Color32,
        stroke: Color32,
        accent: Color32,
        items: &[&str],
    ) {
        let width = ui.available_width();
        egui::Frame::NONE
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(10))
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_min_width((width - 32.0).max(260.0));
                ui.set_max_width((width - 32.0).max(260.0));
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(8.0, 8.0);
                    egui::Frame::NONE
                        .fill(Color32::from_rgba_unmultiplied(
                            accent.r(),
                            accent.g(),
                            accent.b(),
                            26,
                        ))
                        .stroke(Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 46),
                        ))
                        .corner_radius(egui::CornerRadius::same(255))
                        .inner_margin(egui::Margin::symmetric(8, 4))
                        .show(ui, |ui| {
                            ui.label(RichText::new(section_label).small().strong().color(accent));
                        });
                    ui.label(
                        RichText::new(title)
                            .size(15.0)
                            .strong()
                            .color(Color32::from_rgb(224, 228, 236)),
                    );
                });
                ui.add_space(6.0);
                ui.label(
                    RichText::new(intro)
                        .small()
                        .color(Color32::from_rgb(176, 180, 190)),
                );
                ui.add_space(10.0);
                for item in items {
                    Self::topic_card_item(ui, item, accent);
                    ui.add_space(6.0);
                }
            });
    }

    fn topic_card_item(ui: &mut egui::Ui, item: &str, accent: Color32) {
        if let Some((step_no, text)) = Self::split_step_item(item) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(10.0, 8.0);
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(
                        accent.r(),
                        accent.g(),
                        accent.b(),
                        28,
                    ))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(accent.r(), accent.g(), accent.b(), 44),
                    ))
                    .corner_radius(egui::CornerRadius::same(8))
                    .inner_margin(egui::Margin::symmetric(8, 5))
                    .show(ui, |ui| {
                        ui.label(RichText::new(step_no).small().strong().color(accent));
                    });
                ui.add(
                    egui::Label::new(RichText::new(text).color(Color32::from_rgb(204, 208, 216)))
                        .wrap(),
                );
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
            ui.label(RichText::new("•").strong().color(accent));
            ui.add(
                egui::Label::new(RichText::new(item).color(Color32::from_rgb(204, 208, 216)))
                    .wrap(),
            );
        });
    }

    fn split_step_item(item: &str) -> Option<(&str, &str)> {
        let trimmed = item.trim_start();
        let digits_len = trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count();

        if digits_len == 0 {
            return None;
        }

        let bytes = trimmed.as_bytes();
        if bytes.get(digits_len) != Some(&b'.') || bytes.get(digits_len + 1) != Some(&b' ') {
            return None;
        }

        Some((&trimmed[..digits_len], &trimmed[(digits_len + 2)..]))
    }

    fn action_row(
        ui: &mut egui::Ui,
        action: &mut Option<HelpAction>,
        primary: Option<(&str, HelpAction)>,
        secondary: Option<(&str, HelpAction)>,
        tertiary: Option<(&str, HelpAction)>,
    ) {
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
                    RichText::new("直接在 Gridix 里试一遍")
                        .strong()
                        .color(Color32::from_rgb(220, 225, 235)),
                );
                ui.add_space(6.0);
                ui.label(
                    RichText::new("不会做时先点自动演示；想自己练时再切回编辑器手动操作。")
                        .small()
                        .color(Color32::from_rgb(182, 186, 194)),
                );
                ui.add_space(10.0);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(10.0, 10.0);

                    if let Some((label, value)) = primary {
                        if Self::action_button(ui, label, true) {
                            *action = Some(value);
                        }
                    }
                    if let Some((label, value)) = secondary {
                        if Self::action_button(ui, label, false) {
                            *action = Some(value);
                        }
                    }
                    if let Some((label, value)) = tertiary {
                        if Self::action_button(ui, label, false) {
                            *action = Some(value);
                        }
                    }
                });
            });

        ui.add_space(12.0);
    }

    fn action_button(ui: &mut egui::Ui, label: &str, primary: bool) -> bool {
        let fill = if primary {
            Color32::from_rgb(60, 112, 190)
        } else {
            Color32::from_rgba_unmultiplied(120, 120, 130, 28)
        };
        let stroke = if primary {
            Color32::from_rgba_unmultiplied(150, 205, 255, 48)
        } else {
            Color32::from_rgba_unmultiplied(170, 176, 194, 24)
        };

        ui.add(
            egui::Button::new(
                RichText::new(label)
                    .strong()
                    .color(Color32::from_rgb(245, 245, 248)),
            )
            .fill(fill)
            .stroke(Stroke::new(1.0, stroke))
            .corner_radius(egui::CornerRadius::same(8)),
        )
        .clicked()
    }
}
