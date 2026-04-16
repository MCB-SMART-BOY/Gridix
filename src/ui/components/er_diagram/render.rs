//! ER 图渲染

use super::graph::{ERLayoutStrategy, selected_neighborhood};
use super::state::{
    ERCardDisplayMode, ERDiagramInteractionMode, ERDiagramState, EREdgeDisplayMode, ERTable,
    RelationType, Relationship, RelationshipOrigin,
};
use crate::core::ThemePreset;
use crate::ui::styles::{
    theme_accent, theme_muted_text, theme_selection_fill, theme_subtle_stroke, theme_text,
};
use crate::ui::{
    LocalShortcut, consume_local_shortcut, local_shortcut_text, local_shortcut_tooltip,
};
use egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use std::collections::HashMap;

/// ER 图渲染响应
#[derive(Default)]
pub struct ERDiagramResponse {
    /// 是否需要刷新数据
    pub refresh_requested: bool,
    /// 是否需要重新布局
    pub layout_requested: bool,
    /// 是否需要适应视图
    pub fit_view_requested: bool,
    /// 是否请求将 ER 图设为当前焦点区域
    pub request_focus: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ERDiagramKeyAction {
    Refresh,
    Layout,
    FitView,
    ZoomIn,
    ZoomOut,
    PanLeft,
    PanDown,
    PanUp,
    PanRight,
}

/// 渲染颜色配置
struct RenderColors {
    background: Color32,
    grid_line: Color32,
    table_bg: Color32,
    table_header_bg: Color32,
    table_header_selected_bg: Color32,
    table_border: Color32,
    table_related_border: Color32,
    table_selected_border: Color32,
    table_shadow: Color32,
    text_primary: Color32,
    text_secondary: Color32,
    text_type: Color32,
    badge_text: Color32,
    pk_icon: Color32,
    fk_icon: Color32,
    relation_line_explicit: Color32,
    relation_line_inferred: Color32,
    relation_line_selected: Color32,
    row_separator: Color32,
}

impl RenderColors {
    fn blend(from: Color32, to: Color32, to_weight: f32) -> Color32 {
        let to_weight = to_weight.clamp(0.0, 1.0);
        let from_weight = 1.0 - to_weight;
        let mix = |a: u8, b: u8| ((a as f32 * from_weight) + (b as f32 * to_weight)).round() as u8;

        Color32::from_rgba_unmultiplied(
            mix(from.r(), to.r()),
            mix(from.g(), to.g()),
            mix(from.b(), to.b()),
            mix(from.a(), to.a()),
        )
    }

    fn from_theme(theme: &ThemePreset, visuals: &egui::Visuals) -> Self {
        let palette = theme.colors();
        let border = theme_subtle_stroke(visuals);
        let text_primary = theme_text(visuals);
        let text_secondary = theme_muted_text(visuals);
        let grid_line = Color32::from_rgba_unmultiplied(
            border.r(),
            border.g(),
            border.b(),
            if visuals.dark_mode { 14 } else { 12 },
        );
        let row_separator = Color32::from_rgba_unmultiplied(
            border.r(),
            border.g(),
            border.b(),
            if visuals.dark_mode { 24 } else { 20 },
        );
        let table_shadow = visuals
            .window_shadow
            .color
            .gamma_multiply(if visuals.dark_mode { 1.0 } else { 0.65 });
        let text_type = Self::blend(
            palette.fg_muted,
            palette.info,
            if visuals.dark_mode { 0.18 } else { 0.12 },
        );
        let badge_text = Self::blend(
            palette.fg_secondary,
            palette.info,
            if visuals.dark_mode { 0.08 } else { 0.06 },
        );
        let pk_icon = Self::blend(
            palette.warning,
            palette.fg_primary,
            if visuals.dark_mode { 0.08 } else { 0.16 },
        );
        let fk_icon = Self::blend(palette.info, palette.accent, 0.35);
        let relation_line_explicit = Self::blend(
            palette.border,
            palette.info,
            if visuals.dark_mode { 0.62 } else { 0.52 },
        );
        let relation_line_inferred = Self::blend(
            palette.border,
            palette.fg_muted,
            if visuals.dark_mode { 0.56 } else { 0.42 },
        );
        let relation_line_selected = Self::blend(
            palette.accent,
            palette.info,
            if visuals.dark_mode { 0.18 } else { 0.14 },
        );

        Self {
            background: palette.bg_primary,
            grid_line,
            table_bg: palette.bg_secondary,
            table_header_bg: palette.bg_tertiary,
            table_header_selected_bg: Self::blend(
                palette.bg_tertiary,
                palette.accent,
                if visuals.dark_mode { 0.20 } else { 0.12 },
            ),
            table_border: border,
            table_related_border: Self::blend(
                border,
                palette.info,
                if visuals.dark_mode { 0.34 } else { 0.24 },
            ),
            table_selected_border: visuals.selection.stroke.color,
            table_shadow,
            text_primary,
            text_secondary,
            text_type,
            badge_text,
            pk_icon,
            fk_icon,
            relation_line_explicit,
            relation_line_inferred,
            relation_line_selected,
            row_separator,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct TableVisualState {
    selected: bool,
    related: bool,
    dimmed: bool,
}

#[derive(Debug, Clone, Copy)]
struct TableRenderContext {
    canvas_rect: Rect,
    pan_offset: Vec2,
    zoom: f32,
    display_mode: ERCardDisplayMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RouteOrientation {
    Horizontal,
    Vertical,
    Mixed,
}

#[derive(Debug, Clone, Copy)]
struct RouteDescriptor {
    orientation: RouteOrientation,
    baseline_bucket: i32,
    span_start: f32,
    span_end: f32,
}

#[derive(Debug, Clone, Copy)]
struct RoutedRelationship {
    relationship_index: usize,
    from_screen: Pos2,
    to_screen: Pos2,
    from_dir: i32,
    to_dir: i32,
}

#[derive(Debug, Clone, Copy)]
struct ERToolbarButtonChrome {
    fill: Option<Color32>,
    stroke: Option<Stroke>,
    frame_when_inactive: bool,
    selected: bool,
}

fn er_toolbar_button_chrome(visuals: &egui::Visuals, is_selected: bool) -> ERToolbarButtonChrome {
    ERToolbarButtonChrome {
        fill: is_selected.then(|| theme_selection_fill(visuals, 56)),
        stroke: is_selected.then(|| Stroke::new(1.0, theme_accent(visuals))),
        frame_when_inactive: is_selected,
        selected: is_selected,
    }
}

fn er_toolbar_button(
    ui: &mut egui::Ui,
    icon: &str,
    icon_size: f32,
    button_size: Vec2,
    is_selected: bool,
    tooltip: impl Into<egui::WidgetText>,
) -> egui::Response {
    let chrome = er_toolbar_button_chrome(ui.visuals(), is_selected);
    let mut button = egui::Button::new(RichText::new(icon).size(icon_size))
        .min_size(button_size)
        .corner_radius(CornerRadius::same(6))
        .frame(true)
        .frame_when_inactive(chrome.frame_when_inactive)
        .selected(chrome.selected);
    if let Some(fill) = chrome.fill {
        button = button.fill(fill);
    }
    if let Some(stroke) = chrome.stroke {
        button = button.stroke(stroke);
    }

    ui.add(button).on_hover_text(tooltip)
}

fn er_toolbar_chip(
    ui: &mut egui::Ui,
    label: impl Into<egui::WidgetText>,
    is_selected: bool,
    tooltip: impl Into<egui::WidgetText>,
) -> egui::Response {
    let chrome = er_toolbar_button_chrome(ui.visuals(), is_selected);
    let mut button = egui::Button::new(label)
        .min_size(Vec2::new(0.0, 26.0))
        .corner_radius(CornerRadius::same(9))
        .frame(true)
        .frame_when_inactive(chrome.frame_when_inactive)
        .selected(chrome.selected);
    if let Some(fill) = chrome.fill {
        button = button.fill(fill);
    }
    if let Some(stroke) = chrome.stroke {
        button = button.stroke(stroke);
    }

    ui.add(button).on_hover_text(tooltip)
}

impl ERDiagramState {
    /// 渲染 ER 图
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        theme: &ThemePreset,
        is_focused: bool,
    ) -> ERDiagramResponse {
        let mut response = ERDiagramResponse::default();
        let colors = RenderColors::from_theme(theme, ui.visuals());
        let graph_summary = super::graph::build_er_graph(&self.tables, &self.relationships).summary;

        // 工具栏
        ui.horizontal(|ui| {
            let mode_label = if self.is_viewport_mode() {
                "视口模式"
            } else {
                "浏览模式"
            };
            if ui
                .add(
                    egui::Button::new(RichText::new(mode_label).size(11.0))
                        .frame(true)
                        .frame_when_inactive(self.is_viewport_mode())
                        .selected(self.is_viewport_mode())
                        .fill(if self.is_viewport_mode() {
                            theme_selection_fill(ui.visuals(), 56)
                        } else {
                            ui.visuals().widgets.inactive.bg_fill
                        })
                        .stroke(if self.is_viewport_mode() {
                            Stroke::new(1.0, theme_accent(ui.visuals()))
                        } else {
                            Stroke::NONE
                        }),
                )
                .on_hover_text(local_shortcut_tooltip(
                    if self.is_viewport_mode() {
                        "退出视口模式"
                    } else {
                        "进入视口模式"
                    },
                    LocalShortcut::ErDiagramViewportMode,
                ))
                .clicked()
            {
                self.toggle_interaction_mode();
                response.request_focus = true;
            }

            ui.add_space(10.0);

            if er_toolbar_button(
                ui,
                "🔄",
                14.0,
                Vec2::new(26.0, 26.0),
                false,
                local_shortcut_tooltip("刷新数据", LocalShortcut::ErDiagramRefresh),
            )
            .clicked()
            {
                response.refresh_requested = true;
                response.request_focus = true;
            }

            if er_toolbar_button(
                ui,
                "⊞",
                14.0,
                Vec2::new(26.0, 26.0),
                false,
                local_shortcut_tooltip("重新布局", LocalShortcut::ErDiagramLayout),
            )
            .clicked()
            {
                response.layout_requested = true;
                response.request_focus = true;
            }

            if er_toolbar_button(
                ui,
                "⛶",
                14.0,
                Vec2::new(26.0, 26.0),
                false,
                local_shortcut_tooltip("适应视图", LocalShortcut::ErDiagramFitView),
            )
            .clicked()
            {
                response.fit_view_requested = true;
                response.request_focus = true;
            }

            ui.add_space(10.0);

            if er_toolbar_chip(
                ui,
                RichText::new(format!("边: {}", self.edge_display_mode().label())).size(11.0),
                self.edge_display_mode() != EREdgeDisplayMode::All,
                "切换边显示模式：全部边 / 焦点边 / 显式边",
            )
            .clicked()
            {
                self.cycle_edge_display_mode();
                response.request_focus = true;
            }

            if er_toolbar_chip(
                ui,
                RichText::new(format!("列: {}", self.card_display_mode().label())).size(11.0),
                self.card_display_mode() == ERCardDisplayMode::KeysOnly,
                "切换表卡信息密度：关键列 / 完整列",
            )
            .clicked()
            {
                let display_mode = self.toggle_card_display_mode();
                for table in &mut self.tables {
                    calculate_table_size_for_mode(table, display_mode);
                }
                response.layout_requested = true;
                response.fit_view_requested = true;
                response.request_focus = true;
            }

            ui.add_space(6.0);

            let strategy_label = match graph_summary.strategy {
                ERLayoutStrategy::Grid => "网格完成态",
                ERLayoutStrategy::Relation => "关系完成态",
                ERLayoutStrategy::Component => "组件完成态",
                ERLayoutStrategy::DenseGraph => "高密度完成态",
                ERLayoutStrategy::StableIncremental => "稳定增量完成态",
            };
            ui.label(
                RichText::new(strategy_label)
                    .size(11.0)
                    .color(colors.text_secondary),
            );

            if er_toolbar_button(
                ui,
                "+",
                14.0,
                Vec2::new(22.0, 22.0),
                false,
                local_shortcut_tooltip("放大视图", LocalShortcut::ErDiagramZoomIn),
            )
            .clicked()
            {
                self.zoom_by(1.2);
                response.request_focus = true;
            }

            ui.label(
                RichText::new(format!("{:.0}%", self.zoom * 100.0))
                    .size(12.0)
                    .color(colors.text_secondary),
            );

            if er_toolbar_button(
                ui,
                "−",
                14.0,
                Vec2::new(22.0, 22.0),
                false,
                local_shortcut_tooltip("缩小视图", LocalShortcut::ErDiagramZoomOut),
            )
            .clicked()
            {
                self.zoom_by(0.8);
                response.request_focus = true;
            }

            if er_toolbar_button(
                ui,
                "↺",
                14.0,
                Vec2::new(26.0, 26.0),
                false,
                format!(
                    "重置视图\n缩放: {} / {}",
                    local_shortcut_text(LocalShortcut::ErDiagramZoomIn),
                    local_shortcut_text(LocalShortcut::ErDiagramZoomOut),
                ),
            )
            .clicked()
            {
                self.reset_view();
                response.request_focus = true;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} 张表", graph_summary.table_count))
                        .small()
                        .color(colors.text_secondary),
                );

                ui.add_space(8.0);

                ui.label(
                    RichText::new(format!("{} 关系", graph_summary.relationship_count))
                        .small()
                        .color(colors.text_secondary),
                );

                if graph_summary.component_count > 1 {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!("{} 组件", graph_summary.component_count))
                            .small()
                            .color(colors.text_secondary),
                    );
                }

                if graph_summary.inferred_relationship_count > 0 {
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new(format!(
                            "{} 推断",
                            graph_summary.inferred_relationship_count
                        ))
                        .small()
                        .color(colors.text_secondary),
                    );
                }

                ui.add_space(8.0);

                ui.label(
                    RichText::new(format!("主簇 {}", graph_summary.largest_component_size))
                        .small()
                        .color(colors.text_secondary),
                );
            });
        });

        ui.separator();

        // 画布区域
        let available = ui.available_rect_before_wrap();
        let (canvas_response, painter) =
            ui.allocate_painter(available.size(), Sense::click_and_drag());
        let canvas_rect = canvas_response.rect;
        if canvas_response.clicked() || canvas_response.dragged() {
            response.request_focus = true;
        }

        // 绘制背景
        painter.rect_filled(canvas_rect, CornerRadius::ZERO, colors.background);

        // 绘制网格
        self.draw_grid(&painter, canvas_rect, &colors);

        if self.loading {
            // 加载中
            painter.text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                "加载中...",
                FontId::proportional(18.0),
                colors.text_secondary,
            );
        } else if self.tables.is_empty() {
            // 空状态
            painter.text(
                canvas_rect.center(),
                egui::Align2::CENTER_CENTER,
                "无表数据\n选择数据库后刷新",
                FontId::proportional(16.0),
                colors.text_secondary,
            );
        } else {
            // 先计算所有表格尺寸（关系线绘制依赖尺寸数据）
            let display_mode = self.card_display_mode();
            for table in &mut self.tables {
                Self::calculate_table_size_for_mode(table, display_mode);
            }

            self.consume_pending_fit_to_view(canvas_rect.size());
            self.reveal_selected_table_in_view(canvas_rect.size());

            let selected_neighborhood = self
                .selected_table_name()
                .map(|table_name| selected_neighborhood(table_name, &self.relationships));

            // 绘制关系线（在表格下方）
            self.draw_relationships(
                &painter,
                canvas_rect,
                &colors,
                selected_neighborhood.as_ref(),
            );

            // 绘制表格
            for table in &self.tables {
                let visual_state = table_visual_state(
                    table,
                    selected_neighborhood.as_ref(),
                    self.edge_display_mode(),
                );
                let render_context = TableRenderContext {
                    canvas_rect,
                    pan_offset: self.pan_offset,
                    zoom: self.zoom,
                    display_mode: self.card_display_mode(),
                };
                Self::draw_table_static(&painter, table, render_context, &colors, visual_state);
            }
        }

        // 处理交互
        self.handle_interaction(ui, &canvas_response, canvas_rect);

        // 键盘快捷键
        if is_focused
            && let Some(action) = consume_er_diagram_key_action(ui, self.interaction_mode())
        {
            match action {
                ERDiagramKeyAction::Refresh => response.refresh_requested = true,
                ERDiagramKeyAction::Layout => response.layout_requested = true,
                ERDiagramKeyAction::FitView => response.fit_view_requested = true,
                ERDiagramKeyAction::ZoomIn => self.zoom_by(1.2),
                ERDiagramKeyAction::ZoomOut => self.zoom_by(0.8),
                ERDiagramKeyAction::PanLeft => self.pan_keyboard_left(),
                ERDiagramKeyAction::PanDown => self.pan_keyboard_down(),
                ERDiagramKeyAction::PanUp => self.pan_keyboard_up(),
                ERDiagramKeyAction::PanRight => self.pan_keyboard_right(),
            }
        }

        response
    }

    /// 绘制背景网格（点状网格，更现代）
    fn draw_grid(&self, painter: &egui::Painter, rect: Rect, colors: &RenderColors) {
        let grid_size = 24.0 * self.zoom;
        let dot_size = 1.5 * self.zoom;

        let offset_x = (self.pan_offset.x * self.zoom) % grid_size;
        let offset_y = (self.pan_offset.y * self.zoom) % grid_size;

        // 绘制点状网格
        let mut x = rect.left() + offset_x;
        while x < rect.right() {
            let mut y = rect.top() + offset_y;
            while y < rect.bottom() {
                painter.circle_filled(Pos2::new(x, y), dot_size, colors.grid_line);
                y += grid_size;
            }
            x += grid_size;
        }
    }

    /// 计算表格尺寸（根据内容与信息密度自适应宽度）
    fn calculate_table_size_for_mode(table: &mut ERTable, display_mode: ERCardDisplayMode) {
        calculate_table_size_for_mode(table, display_mode);
    }
}

fn consume_er_diagram_key_action(
    ui: &mut egui::Ui,
    interaction_mode: ERDiagramInteractionMode,
) -> Option<ERDiagramKeyAction> {
    ui.input_mut(|input| {
        if consume_local_shortcut(input, LocalShortcut::ErDiagramRefresh) {
            Some(ERDiagramKeyAction::Refresh)
        } else if consume_local_shortcut(input, LocalShortcut::ErDiagramLayout) {
            Some(ERDiagramKeyAction::Layout)
        } else if consume_local_shortcut(input, LocalShortcut::ErDiagramFitView) {
            Some(ERDiagramKeyAction::FitView)
        } else if consume_local_shortcut(input, LocalShortcut::ErDiagramZoomIn) {
            Some(ERDiagramKeyAction::ZoomIn)
        } else if consume_local_shortcut(input, LocalShortcut::ErDiagramZoomOut) {
            Some(ERDiagramKeyAction::ZoomOut)
        } else if interaction_mode == ERDiagramInteractionMode::Viewport
            && consume_local_shortcut(input, LocalShortcut::ErDiagramViewportPanLeft)
        {
            Some(ERDiagramKeyAction::PanLeft)
        } else if interaction_mode == ERDiagramInteractionMode::Viewport
            && consume_local_shortcut(input, LocalShortcut::ErDiagramViewportPanDown)
        {
            Some(ERDiagramKeyAction::PanDown)
        } else if interaction_mode == ERDiagramInteractionMode::Viewport
            && consume_local_shortcut(input, LocalShortcut::ErDiagramViewportPanUp)
        {
            Some(ERDiagramKeyAction::PanUp)
        } else if interaction_mode == ERDiagramInteractionMode::Viewport
            && consume_local_shortcut(input, LocalShortcut::ErDiagramViewportPanRight)
        {
            Some(ERDiagramKeyAction::PanRight)
        } else {
            None
        }
    })
}

fn table_visual_state(
    table: &ERTable,
    selected_neighborhood: Option<&std::collections::HashSet<String>>,
    edge_mode: EREdgeDisplayMode,
) -> TableVisualState {
    let selected = table.selected;
    let related = selected_neighborhood
        .is_some_and(|neighborhood| neighborhood.contains(table.name.as_str()) && !selected);
    let dimmed = edge_mode == EREdgeDisplayMode::Focus
        && selected_neighborhood.is_some()
        && !selected
        && !related;

    TableVisualState {
        selected,
        related,
        dimmed,
    }
}

fn visible_column_indices(table: &ERTable, display_mode: ERCardDisplayMode) -> Vec<usize> {
    match display_mode {
        ERCardDisplayMode::Standard => (0..table.columns.len()).collect(),
        ERCardDisplayMode::KeysOnly => {
            let mut indices: Vec<usize> = table
                .columns
                .iter()
                .enumerate()
                .filter_map(|(index, column)| {
                    (column.is_primary_key || column.is_foreign_key).then_some(index)
                })
                .collect();

            if indices.is_empty() {
                indices.extend((0..table.columns.len()).take(5));
            } else {
                for index in 0..table.columns.len() {
                    if indices.len() >= 6 {
                        break;
                    }
                    if !indices.contains(&index) {
                        indices.push(index);
                    }
                }
            }

            indices
        }
    }
}

pub fn calculate_table_size(table: &mut ERTable) {
    calculate_table_size_for_mode(table, ERCardDisplayMode::Standard);
}

pub fn calculate_table_size_for_mode(table: &mut ERTable, display_mode: ERCardDisplayMode) {
    let header_height = 42.0;
    let row_height = 22.0;
    let footer_height = 14.0;
    let padding = 12.0;
    let min_width = 190.0;
    let max_width = 360.0;
    let min_height = 92.0;
    let char_width = 7.0;
    let icon_width = 14.0;
    let type_gap = 22.0;
    let null_marker_width = 18.0;

    let visible_indices = visible_column_indices(table, display_mode);
    let hidden_column_count = table.columns.len().saturating_sub(visible_indices.len());
    let header_width = table.name.len() as f32 * char_width + padding * 5.0 + 76.0;

    let max_column_width = visible_indices
        .iter()
        .filter_map(|index| table.columns.get(*index))
        .map(|column| {
            let icons = if column.is_primary_key {
                icon_width
            } else {
                0.0
            } + if column.is_foreign_key {
                icon_width
            } else {
                0.0
            };
            let name_width = column.name.len() as f32 * char_width;
            let type_width = column.data_type.len() as f32 * char_width * 0.8;
            icons + name_width + type_gap + type_width + null_marker_width + padding * 2.0
        })
        .fold(0.0_f32, |left, right| left.max(right));

    let content_width = header_width
        .max(max_column_width + if hidden_column_count > 0 { 18.0 } else { 0.0 })
        .clamp(min_width, max_width);

    let row_count = visible_indices.len() + usize::from(hidden_column_count > 0);
    let content_height = header_height + (row_count as f32 * row_height) + footer_height;
    table.size = Vec2::new(content_width, content_height.max(min_height));
}

impl ERDiagramState {
    /// 绘制表格（静态方法）
    fn draw_table_static(
        painter: &egui::Painter,
        table: &ERTable,
        render_context: TableRenderContext,
        colors: &RenderColors,
        visual_state: TableVisualState,
    ) {
        let padding = 12.0;
        let header_height = 42.0;
        let row_height = 22.0;

        // 计算屏幕位置
        let screen_pos = Pos2::new(
            render_context.canvas_rect.left()
                + (table.position.x + render_context.pan_offset.x) * render_context.zoom,
            render_context.canvas_rect.top()
                + (table.position.y + render_context.pan_offset.y) * render_context.zoom,
        );
        let screen_size = table.size * render_context.zoom;
        let table_rect = Rect::from_min_size(screen_pos, screen_size);

        // 检查是否在可见区域
        if !render_context.canvas_rect.intersects(table_rect) {
            return;
        }

        let corner_radius = (8.0 * render_context.zoom) as u8;
        let table_fill = if visual_state.dimmed {
            RenderColors::blend(colors.table_bg, colors.background, 0.32)
        } else {
            colors.table_bg
        };
        let header_fill = if visual_state.selected {
            colors.table_header_selected_bg
        } else if visual_state.dimmed {
            RenderColors::blend(colors.table_header_bg, colors.background, 0.26)
        } else {
            colors.table_header_bg
        };
        let border_color = if visual_state.selected {
            colors.table_selected_border
        } else if visual_state.related {
            colors.table_related_border
        } else {
            colors.table_border
        };
        let text_primary = if visual_state.dimmed {
            RenderColors::blend(colors.text_primary, colors.text_secondary, 0.45)
        } else {
            colors.text_primary
        };
        let text_secondary = if visual_state.dimmed {
            RenderColors::blend(colors.text_secondary, colors.background, 0.30)
        } else {
            colors.text_secondary
        };

        // 绘制阴影
        let shadow_offset = 3.0 * render_context.zoom;
        let shadow_rect = Rect::from_min_size(
            screen_pos + Vec2::new(shadow_offset, shadow_offset),
            screen_size,
        );
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(corner_radius),
            if visual_state.dimmed {
                colors.table_shadow.gamma_multiply(0.45)
            } else {
                colors.table_shadow
            },
        );

        // 绘制表格背景
        painter.rect_filled(table_rect, CornerRadius::same(corner_radius), table_fill);

        // 绘制边框
        painter.rect_stroke(
            table_rect,
            CornerRadius::same(corner_radius),
            Stroke::new(
                if visual_state.selected {
                    2.0 * render_context.zoom
                } else if visual_state.related {
                    1.4 * render_context.zoom
                } else {
                    1.0
                },
                border_color,
            ),
            egui::StrokeKind::Inside,
        );

        // 绘制表头背景
        let header_rect = Rect::from_min_size(
            screen_pos,
            Vec2::new(screen_size.x, header_height * render_context.zoom),
        );
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: corner_radius,
                ne: corner_radius,
                sw: 0,
                se: 0,
            },
            header_fill,
        );

        // 表头分隔线
        painter.line_segment(
            [
                Pos2::new(
                    screen_pos.x,
                    screen_pos.y + header_height * render_context.zoom,
                ),
                Pos2::new(
                    screen_pos.x + screen_size.x,
                    screen_pos.y + header_height * render_context.zoom,
                ),
            ],
            Stroke::new(1.0, border_color),
        );

        let font_size = 13.0 * render_context.zoom;
        let title_pos = Pos2::new(
            screen_pos.x + padding * render_context.zoom,
            header_rect.center().y - 6.0 * render_context.zoom,
        );
        painter.text(
            title_pos,
            egui::Align2::LEFT_CENTER,
            &table.name,
            FontId::proportional(font_size),
            text_primary,
        );

        let primary_key_count = table
            .columns
            .iter()
            .filter(|column| column.is_primary_key)
            .count();
        let foreign_key_count = table
            .columns
            .iter()
            .filter(|column| column.is_foreign_key)
            .count();
        let badge_text = format!(
            "{} 列  {} PK  {} FK",
            table.columns.len(),
            primary_key_count,
            foreign_key_count
        );
        let badge_pos = Pos2::new(
            screen_pos.x + screen_size.x - padding * render_context.zoom,
            header_rect.center().y + 6.0 * render_context.zoom,
        );
        painter.text(
            badge_pos,
            egui::Align2::RIGHT_CENTER,
            badge_text,
            FontId::proportional(9.5 * render_context.zoom),
            colors.badge_text,
        );

        // 绘制列
        let small_font_size = 11.0 * render_context.zoom;
        let tiny_font_size = 9.0 * render_context.zoom;
        let icon_size = 12.0 * render_context.zoom;
        let visible_indices = visible_column_indices(table, render_context.display_mode);
        let hidden_column_count = table.columns.len().saturating_sub(visible_indices.len());

        for (row_index, column_index) in visible_indices.iter().enumerate() {
            let Some(col) = table.columns.get(*column_index) else {
                continue;
            };
            let row_y = screen_pos.y
                + (header_height + row_index as f32 * row_height) * render_context.zoom;
            let row_x = screen_pos.x + padding * render_context.zoom;
            let row_center_y = row_y + row_height * render_context.zoom / 2.0;

            if row_index > 0 {
                painter.line_segment(
                    [
                        Pos2::new(screen_pos.x + 8.0 * render_context.zoom, row_y),
                        Pos2::new(
                            screen_pos.x + screen_size.x - 8.0 * render_context.zoom,
                            row_y,
                        ),
                    ],
                    Stroke::new(1.0, colors.row_separator),
                );
            }

            // 图标区域
            let mut icon_x = row_x;

            // 主键图标
            if col.is_primary_key {
                painter.circle_filled(
                    Pos2::new(icon_x + 4.0 * render_context.zoom, row_center_y),
                    3.0 * render_context.zoom,
                    colors.pk_icon,
                );
                icon_x += icon_size + 2.0 * render_context.zoom;
            }

            // 外键图标
            if col.is_foreign_key {
                painter.circle_stroke(
                    Pos2::new(icon_x + 4.0 * render_context.zoom, row_center_y),
                    3.0 * render_context.zoom,
                    Stroke::new(1.5 * render_context.zoom, colors.fk_icon),
                );
                icon_x += icon_size + 2.0 * render_context.zoom;
            }

            // 列名（如果非空则加粗显示）
            let text_x = if col.is_primary_key || col.is_foreign_key {
                icon_x
            } else {
                row_x
            };

            let name_color = if !col.nullable {
                text_primary
            } else {
                text_secondary
            };

            painter.text(
                Pos2::new(text_x, row_center_y),
                egui::Align2::LEFT_CENTER,
                &col.name,
                FontId::proportional(small_font_size),
                name_color,
            );

            // 右侧信息区：数据类型 + 标记
            let right_x = screen_pos.x + screen_size.x - padding * render_context.zoom;

            // 构建标记字符串：NULL标记 + 默认值标记
            let mut markers = String::new();

            if col.default_value.is_some() {
                markers.push('=');
            }

            if col.nullable {
                markers.push('?');
            } else {
                markers.push('!');
            }

            let marker_color = if col.default_value.is_some() {
                colors.fk_icon
            } else if col.nullable {
                colors.text_type
            } else {
                colors.pk_icon
            };

            painter.text(
                Pos2::new(right_x, row_center_y),
                egui::Align2::RIGHT_CENTER,
                &markers,
                FontId::proportional(tiny_font_size),
                marker_color,
            );

            // 数据类型（在标记左边）
            let markers_width = markers.len() as f32 * 6.0 * render_context.zoom;
            painter.text(
                Pos2::new(
                    right_x - markers_width - 4.0 * render_context.zoom,
                    row_center_y,
                ),
                egui::Align2::RIGHT_CENTER,
                &col.data_type,
                FontId::proportional(small_font_size * 0.9),
                colors.text_type,
            );
        }

        if hidden_column_count > 0 {
            let row_y = screen_pos.y
                + (header_height + visible_indices.len() as f32 * row_height) * render_context.zoom;
            let row_center_y = row_y + row_height * render_context.zoom / 2.0;

            painter.line_segment(
                [
                    Pos2::new(screen_pos.x + 8.0 * render_context.zoom, row_y),
                    Pos2::new(
                        screen_pos.x + screen_size.x - 8.0 * render_context.zoom,
                        row_y,
                    ),
                ],
                Stroke::new(1.0, colors.row_separator),
            );

            painter.text(
                Pos2::new(screen_pos.x + padding * render_context.zoom, row_center_y),
                egui::Align2::LEFT_CENTER,
                format!("… 还有 {} 列", hidden_column_count),
                FontId::proportional(10.0 * render_context.zoom),
                text_secondary,
            );
        }
    }

    /// 计算列在表格中的Y偏移（从表格顶部开始）
    fn get_column_y_offset(table: &ERTable, column_name: &str) -> f32 {
        let header_height = 42.0;
        let row_height = 22.0;

        // 查找列索引
        let col_idx = table
            .columns
            .iter()
            .position(|c| c.name == column_name)
            .unwrap_or(0);

        // 计算Y偏移：表头 + 列索引 * 行高 + 行高/2（居中）
        header_height + col_idx as f32 * row_height + row_height / 2.0
    }

    /// 计算两个表之间的连接点（只使用左右连接，连接点在外键列位置）
    /// 返回 (from_point, to_point, from_direction, to_direction)
    /// direction: 0=右, 2=左
    fn calculate_connection_points_at_column(
        from: &ERTable,
        to: &ERTable,
        from_column: &str,
        to_column: &str,
        pan_offset: Vec2,
        zoom: f32,
        canvas_rect: Rect,
    ) -> (Pos2, Pos2, i32, i32) {
        // 计算外键列在from表中的Y位置
        let from_col_y = Self::get_column_y_offset(from, from_column);
        // 计算目标列在to表中的Y位置（通常是主键id列）
        let to_col_y = Self::get_column_y_offset(to, to_column);

        // 计算两个表的中心点X坐标
        let from_center_x = from.position.x + from.size.x / 2.0;
        let to_center_x = to.position.x + to.size.x / 2.0;
        let from_center_y = from.position.y + from.size.y / 2.0;
        let to_center_y = to.position.y + to.size.y / 2.0;
        let prefers_vertical =
            (to_center_y - from_center_y).abs() > (to_center_x - from_center_x).abs();

        // 根据几何方向选择左右或上下连接，避免上下堆叠的表仍被强行拉成长横折线。
        let (from_edge, to_edge, from_dir, to_dir) = if prefers_vertical {
            if to_center_y > from_center_y {
                (
                    Pos2::new(from_center_x, from.position.y + from.size.y),
                    Pos2::new(to_center_x, to.position.y),
                    1,
                    3,
                )
            } else {
                (
                    Pos2::new(from_center_x, from.position.y),
                    Pos2::new(to_center_x, to.position.y + to.size.y),
                    3,
                    1,
                )
            }
        } else if to_center_x > from_center_x {
            // to 在 from 的右边：from右边 -> to左边
            (
                Pos2::new(from.position.x + from.size.x, from.position.y + from_col_y),
                Pos2::new(to.position.x, to.position.y + to_col_y),
                0,
                2,
            )
        } else {
            // to 在 from 的左边：from左边 -> to右边
            (
                Pos2::new(from.position.x, from.position.y + from_col_y),
                Pos2::new(to.position.x + to.size.x, to.position.y + to_col_y),
                2,
                0,
            )
        };

        // 转换为屏幕坐标
        let from_screen = Pos2::new(
            canvas_rect.left() + (from_edge.x + pan_offset.x) * zoom,
            canvas_rect.top() + (from_edge.y + pan_offset.y) * zoom,
        );
        let to_screen = Pos2::new(
            canvas_rect.left() + (to_edge.x + pan_offset.x) * zoom,
            canvas_rect.top() + (to_edge.y + pan_offset.y) * zoom,
        );

        (from_screen, to_screen, from_dir, to_dir)
    }

    fn orthogonal_route_points(
        from: Pos2,
        to: Pos2,
        from_dir: i32,
        to_dir: i32,
        zoom: f32,
        lane_offset: f32,
    ) -> [Pos2; 6] {
        let stub = 18.0 * zoom;
        let from_stub = match from_dir {
            0 => from + Vec2::new(stub, 0.0),
            1 => from + Vec2::new(0.0, stub),
            2 => from - Vec2::new(stub, 0.0),
            3 => from - Vec2::new(0.0, stub),
            _ => from,
        };
        let to_stub = match to_dir {
            0 => to + Vec2::new(stub, 0.0),
            1 => to + Vec2::new(0.0, stub),
            2 => to - Vec2::new(stub, 0.0),
            3 => to - Vec2::new(0.0, stub),
            _ => to,
        };
        let horizontal = matches!(from_dir, 0 | 2) && matches!(to_dir, 0 | 2);
        let vertical = matches!(from_dir, 1 | 3) && matches!(to_dir, 1 | 3);

        if vertical {
            let mid_y = (from_stub.y + to_stub.y) / 2.0 + lane_offset;
            [
                from,
                from_stub,
                Pos2::new(from_stub.x, mid_y),
                Pos2::new(to_stub.x, mid_y),
                to_stub,
                to,
            ]
        } else if horizontal {
            let mid_x = (from_stub.x + to_stub.x) / 2.0 + lane_offset;
            [
                from,
                from_stub,
                Pos2::new(mid_x, from_stub.y),
                Pos2::new(mid_x, to_stub.y),
                to_stub,
                to,
            ]
        } else {
            let elbow_x = to_stub.x + lane_offset;
            [
                from,
                from_stub,
                Pos2::new(elbow_x, from_stub.y),
                Pos2::new(elbow_x, to_stub.y),
                to_stub,
                to,
            ]
        }
    }

    fn route_orientation(from_dir: i32, to_dir: i32) -> RouteOrientation {
        if matches!(from_dir, 0 | 2) && matches!(to_dir, 0 | 2) {
            RouteOrientation::Horizontal
        } else if matches!(from_dir, 1 | 3) && matches!(to_dir, 1 | 3) {
            RouteOrientation::Vertical
        } else {
            RouteOrientation::Mixed
        }
    }

    fn route_stubs(from: Pos2, to: Pos2, from_dir: i32, to_dir: i32, zoom: f32) -> (Pos2, Pos2) {
        let stub = 18.0 * zoom;
        let from_stub = match from_dir {
            0 => from + Vec2::new(stub, 0.0),
            1 => from + Vec2::new(0.0, stub),
            2 => from - Vec2::new(stub, 0.0),
            3 => from - Vec2::new(0.0, stub),
            _ => from,
        };
        let to_stub = match to_dir {
            0 => to + Vec2::new(stub, 0.0),
            1 => to + Vec2::new(0.0, stub),
            2 => to - Vec2::new(stub, 0.0),
            3 => to - Vec2::new(0.0, stub),
            _ => to,
        };

        (from_stub, to_stub)
    }

    fn route_descriptor(
        from: Pos2,
        to: Pos2,
        from_dir: i32,
        to_dir: i32,
        zoom: f32,
    ) -> RouteDescriptor {
        let orientation = Self::route_orientation(from_dir, to_dir);
        let (from_stub, to_stub) = Self::route_stubs(from, to, from_dir, to_dir, zoom);
        let (baseline, span_start, span_end) = match orientation {
            RouteOrientation::Horizontal => (
                (from_stub.x + to_stub.x) / 2.0,
                from_stub.y.min(to_stub.y),
                from_stub.y.max(to_stub.y),
            ),
            RouteOrientation::Vertical => (
                (from_stub.y + to_stub.y) / 2.0,
                from_stub.x.min(to_stub.x),
                from_stub.x.max(to_stub.x),
            ),
            // Mixed connectors still use an orthogonal elbow; lane separation should
            // operate on that shared vertical elbow column rather than skipping them.
            RouteOrientation::Mixed => (
                to_stub.x,
                from_stub.y.min(to_stub.y),
                from_stub.y.max(to_stub.y),
            ),
        };

        RouteDescriptor {
            orientation,
            baseline_bucket: (baseline / 12.0).round() as i32,
            span_start,
            span_end,
        }
    }

    fn route_lane_offsets(routes: &[RoutedRelationship], zoom: f32) -> Vec<f32> {
        let mut offsets = vec![0.0; routes.len()];
        let mut groups: HashMap<(RouteOrientation, i32), Vec<(usize, RouteDescriptor)>> =
            HashMap::new();

        for (index, route) in routes.iter().enumerate() {
            let descriptor = Self::route_descriptor(
                route.from_screen,
                route.to_screen,
                route.from_dir,
                route.to_dir,
                zoom,
            );

            groups
                .entry((descriptor.orientation, descriptor.baseline_bucket))
                .or_default()
                .push((index, descriptor));
        }

        let lane_spacing = (10.0 * zoom).clamp(8.0, 18.0);
        for entries in groups.values_mut() {
            entries.sort_by(|left, right| {
                left.1
                    .span_start
                    .total_cmp(&right.1.span_start)
                    .then(left.1.span_end.total_cmp(&right.1.span_end))
                    .then(left.0.cmp(&right.0))
            });

            let mut lane_last_end: Vec<f32> = Vec::new();
            let mut assigned_lanes = Vec::with_capacity(entries.len());
            for (_, descriptor) in entries.iter() {
                let lane = lane_last_end
                    .iter_mut()
                    .enumerate()
                    .find_map(|(lane_index, last_end)| {
                        (descriptor.span_start > *last_end).then(|| {
                            *last_end = descriptor.span_end;
                            lane_index
                        })
                    })
                    .unwrap_or_else(|| {
                        lane_last_end.push(descriptor.span_end);
                        lane_last_end.len() - 1
                    });
                assigned_lanes.push(lane);
            }

            if lane_last_end.len() <= 1 {
                continue;
            }

            let center = (lane_last_end.len() - 1) as f32 / 2.0;
            for ((route_index, _), lane) in entries.iter().zip(assigned_lanes.into_iter()) {
                offsets[*route_index] = (lane as f32 - center) * lane_spacing;
            }
        }

        offsets
    }

    fn relationship_is_visible(
        relationship: &Relationship,
        edge_mode: EREdgeDisplayMode,
        selected_table: Option<&str>,
    ) -> bool {
        match edge_mode {
            EREdgeDisplayMode::All => true,
            EREdgeDisplayMode::ExplicitOnly => relationship.origin == RelationshipOrigin::Explicit,
            EREdgeDisplayMode::Focus => selected_table.is_none_or(|table_name| {
                relationship.from_table == table_name || relationship.to_table == table_name
            }),
        }
    }

    /// 绘制关系线
    fn draw_relationships(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        colors: &RenderColors,
        selected_neighborhood: Option<&std::collections::HashSet<String>>,
    ) {
        let selected_table = self.selected_table_name();
        let mut routed_relationships = Vec::new();
        for (relationship_index, rel) in self.relationships.iter().enumerate() {
            if !Self::relationship_is_visible(rel, self.edge_display_mode(), selected_table) {
                continue;
            }

            let from_table = self.tables.iter().find(|t| t.name == rel.from_table);
            let to_table = self.tables.iter().find(|t| t.name == rel.to_table);

            if let (Some(from), Some(to)) = (from_table, to_table) {
                let (from_screen, to_screen, from_dir, to_dir) =
                    Self::calculate_connection_points_at_column(
                        from,
                        to,
                        &rel.from_column,
                        &rel.to_column,
                        self.pan_offset,
                        self.zoom,
                        canvas_rect,
                    );
                routed_relationships.push(RoutedRelationship {
                    relationship_index,
                    from_screen,
                    to_screen,
                    from_dir,
                    to_dir,
                });
            }
        }

        let lane_offsets = Self::route_lane_offsets(&routed_relationships, self.zoom);
        for (route, lane_offset) in routed_relationships.iter().zip(lane_offsets.into_iter()) {
            let rel = &self.relationships[route.relationship_index];
            let points = Self::orthogonal_route_points(
                route.from_screen,
                route.to_screen,
                route.from_dir,
                route.to_dir,
                self.zoom,
                lane_offset,
            );
            let highlight = selected_neighborhood.is_some_and(|neighborhood| {
                neighborhood.contains(rel.from_table.as_str())
                    && neighborhood.contains(rel.to_table.as_str())
            });
            let relation_color = if highlight {
                colors.relation_line_selected
            } else if rel.origin == RelationshipOrigin::Explicit {
                colors.relation_line_explicit
            } else {
                colors.relation_line_inferred
            };
            let stroke = Stroke::new(
                if highlight {
                    2.3
                } else if rel.origin == RelationshipOrigin::Explicit {
                    1.8
                } else {
                    1.1
                },
                relation_color,
            );

            for window in points.windows(2) {
                painter.line_segment([window[0], window[1]], stroke);
            }

            let arrow_size = 8.0 * self.zoom;
            let tail = points[points.len() - 2];
            let angle = (route.to_screen.y - tail.y).atan2(route.to_screen.x - tail.x);
            let arrow_p1 = Pos2::new(
                route.to_screen.x - arrow_size * (angle - 0.4).cos(),
                route.to_screen.y - arrow_size * (angle - 0.4).sin(),
            );
            let arrow_p2 = Pos2::new(
                route.to_screen.x - arrow_size * (angle + 0.4).cos(),
                route.to_screen.y - arrow_size * (angle + 0.4).sin(),
            );
            painter.line_segment([route.to_screen, arrow_p1], stroke);
            painter.line_segment([route.to_screen, arrow_p2], stroke);

            if highlight || self.zoom >= 0.9 {
                let mid = points[2].lerp(points[3], 0.5);
                let label = match rel.relation_type {
                    RelationType::OneToOne => "1:1",
                    RelationType::OneToMany => "1:N",
                    RelationType::ManyToMany => "N:M",
                };
                painter.text(
                    Pos2::new(mid.x, mid.y - 10.0 * self.zoom),
                    egui::Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(10.0 * self.zoom),
                    if highlight {
                        colors.text_primary
                    } else {
                        colors.text_secondary
                    },
                );
            }
        }
    }

    /// 处理交互
    fn handle_interaction(&mut self, ui: &egui::Ui, response: &egui::Response, canvas_rect: Rect) {
        // 滚轮缩放
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        if response.hovered() && scroll_delta.y != 0.0 {
            let factor = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
            self.zoom_by(factor);
        }

        // 点击选择表格
        if response.clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            let mut found = false;
            for (i, table) in self.tables.iter().enumerate() {
                let screen_pos = Pos2::new(
                    canvas_rect.left() + (table.position.x + self.pan_offset.x) * self.zoom,
                    canvas_rect.top() + (table.position.y + self.pan_offset.y) * self.zoom,
                );
                let screen_size = table.size * self.zoom;
                let table_rect = Rect::from_min_size(screen_pos, screen_size);

                if table_rect.contains(pos) {
                    self.start_drag(i, pos);
                    found = true;
                    break;
                }
            }
            if !found {
                // 取消选择
                self.selected_table = None;
                for table in &mut self.tables {
                    table.selected = false;
                }
            }
        }

        // 拖动
        if response.dragged()
            && let Some(pos) = response.interact_pointer_pos()
        {
            if self.dragging_table.is_some() {
                // 拖动表格
                let delta = response.drag_delta() / self.zoom;
                if let Some(idx) = self.dragging_table
                    && let Some(table) = self.tables.get_mut(idx)
                {
                    table.position.x += delta.x;
                    table.position.y += delta.y;
                }
            } else {
                // 检查是否开始拖动某个表
                for (i, table) in self.tables.iter().enumerate() {
                    let screen_pos = Pos2::new(
                        canvas_rect.left() + (table.position.x + self.pan_offset.x) * self.zoom,
                        canvas_rect.top() + (table.position.y + self.pan_offset.y) * self.zoom,
                    );
                    let screen_size = table.size * self.zoom;
                    let table_rect = Rect::from_min_size(screen_pos, screen_size);

                    if table_rect.contains(pos) {
                        self.start_drag(i, pos);
                        break;
                    }
                }

                // 如果没有拖动表格，则平移画布
                if self.dragging_table.is_none() {
                    let delta = response.drag_delta() / self.zoom;
                    self.pan_offset += delta;
                }
            }
        }

        // 拖动结束
        if response.drag_stopped() {
            self.end_drag();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ERCardDisplayMode, ERDiagramInteractionMode, ERDiagramState, EREdgeDisplayMode,
        Relationship, RelationshipOrigin, RenderColors, RoutedRelationship,
        er_toolbar_button_chrome, table_visual_state, visible_column_indices,
    };
    use crate::core::ThemePreset;
    use crate::ui::styles::{
        theme_accent, theme_muted_text, theme_selection_fill, theme_subtle_stroke, theme_text,
    };
    use crate::ui::{ERTable, RelationType};
    use eframe::egui::{Area, Context, Event, Id, Key, Modifiers, RawInput};
    use egui::{Color32, Pos2, Rect, Stroke, Vec2, Visuals};

    fn key_event_with_modifiers(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn run_diagram_key_with_modifiers_and_mode(
        key: Key,
        modifiers: Modifiers,
        is_focused: bool,
        interaction_mode: ERDiagramInteractionMode,
    ) -> (ERDiagramState, super::ERDiagramResponse) {
        let ctx = Context::default();
        ctx.begin_pass(RawInput {
            events: vec![key_event_with_modifiers(key, modifiers)],
            modifiers,
            ..Default::default()
        });
        let mut state = ERDiagramState::new();
        if interaction_mode == ERDiagramInteractionMode::Viewport {
            state.toggle_interaction_mode();
        }
        let mut response = super::ERDiagramResponse::default();
        Area::new(Id::new("er_diagram_key_test")).show(&ctx, |ui| {
            response = state.show(ui, &ThemePreset::default(), is_focused);
        });
        let _ = ctx.end_pass();
        (state, response)
    }

    fn run_diagram_key_with_modifiers(
        key: Key,
        modifiers: Modifiers,
        is_focused: bool,
    ) -> super::ERDiagramResponse {
        run_diagram_key_with_modifiers_and_mode(
            key,
            modifiers,
            is_focused,
            ERDiagramInteractionMode::Navigation,
        )
        .1
    }

    fn run_diagram_key(key: Key, is_focused: bool) -> super::ERDiagramResponse {
        run_diagram_key_with_modifiers(key, Modifiers::NONE, is_focused)
    }

    fn render_with_theme(
        state: &mut ERDiagramState,
        theme: ThemePreset,
        is_focused: bool,
    ) -> super::ERDiagramResponse {
        let ctx = Context::default();
        ctx.begin_pass(RawInput::default());
        let mut response = super::ERDiagramResponse::default();
        Area::new(Id::new(("er_diagram_theme_test", theme as u32))).show(&ctx, |ui| {
            response = state.show(ui, &theme, is_focused);
        });
        let _ = ctx.end_pass();
        response
    }

    fn relationship(from_table: &str, to_table: &str, origin: RelationshipOrigin) -> Relationship {
        Relationship {
            from_table: from_table.to_string(),
            from_column: "fk_id".to_string(),
            to_table: to_table.to_string(),
            to_column: "id".to_string(),
            relation_type: RelationType::OneToMany,
            origin,
        }
    }

    #[test]
    fn er_diagram_shortcuts_use_local_command_bindings_only_when_focused() {
        assert!(!run_diagram_key(Key::R, false).refresh_requested);
        assert!(run_diagram_key(Key::R, true).refresh_requested);

        assert!(!run_diagram_key(Key::L, false).layout_requested);
        assert!(!run_diagram_key(Key::L, true).layout_requested);
        assert!(!run_diagram_key_with_modifiers(Key::L, Modifiers::SHIFT, false).layout_requested);
        assert!(run_diagram_key_with_modifiers(Key::L, Modifiers::SHIFT, true).layout_requested);

        assert!(!run_diagram_key(Key::F, false).fit_view_requested);
        assert!(run_diagram_key(Key::F, true).fit_view_requested);
    }

    #[test]
    fn visible_column_indices_prioritize_key_columns_in_keys_only_mode() {
        let mut table = ERTable::new("orders".into());
        table.columns = vec![
            crate::ui::ERColumn {
                name: "id".into(),
                data_type: "INTEGER".into(),
                is_primary_key: true,
                is_foreign_key: false,
                nullable: false,
                default_value: None,
            },
            crate::ui::ERColumn {
                name: "customer_id".into(),
                data_type: "INTEGER".into(),
                is_primary_key: false,
                is_foreign_key: true,
                nullable: false,
                default_value: None,
            },
            crate::ui::ERColumn {
                name: "status".into(),
                data_type: "TEXT".into(),
                is_primary_key: false,
                is_foreign_key: false,
                nullable: false,
                default_value: None,
            },
        ];

        assert_eq!(
            visible_column_indices(&table, ERCardDisplayMode::KeysOnly),
            vec![0, 1, 2]
        );
    }

    #[test]
    fn calculate_table_size_for_mode_compacts_keys_only_cards() {
        let mut table = ERTable::new("orders".into());
        for index in 0..8 {
            table.columns.push(crate::ui::ERColumn {
                name: format!("col_{index}"),
                data_type: "TEXT".into(),
                is_primary_key: index == 0,
                is_foreign_key: index == 1,
                nullable: true,
                default_value: None,
            });
        }

        super::calculate_table_size_for_mode(&mut table, ERCardDisplayMode::Standard);
        let standard_height = table.size.y;
        super::calculate_table_size_for_mode(&mut table, ERCardDisplayMode::KeysOnly);

        assert!(table.size.y < standard_height);
    }

    #[test]
    fn relationship_visibility_hides_inferred_edges_in_explicit_only_mode() {
        let explicit = relationship("orders", "customers", RelationshipOrigin::Explicit);
        let inferred = relationship("payments", "orders", RelationshipOrigin::Inferred);

        assert!(ERDiagramState::relationship_is_visible(
            &explicit,
            EREdgeDisplayMode::ExplicitOnly,
            Some("orders")
        ));
        assert!(!ERDiagramState::relationship_is_visible(
            &inferred,
            EREdgeDisplayMode::ExplicitOnly,
            Some("orders")
        ));
    }

    #[test]
    fn calculate_connection_points_prefers_vertical_anchors_for_stacked_tables() {
        let mut parent = ERTable::new("customers".into());
        parent.position = Pos2::new(100.0, 80.0);
        parent.size = Vec2::new(180.0, 140.0);
        parent.columns.push(crate::ui::ERColumn {
            name: "id".into(),
            data_type: "INTEGER".into(),
            is_primary_key: true,
            is_foreign_key: false,
            nullable: false,
            default_value: None,
        });

        let mut child = ERTable::new("orders".into());
        child.position = Pos2::new(120.0, 320.0);
        child.size = Vec2::new(180.0, 160.0);
        child.columns.push(crate::ui::ERColumn {
            name: "customer_id".into(),
            data_type: "INTEGER".into(),
            is_primary_key: false,
            is_foreign_key: true,
            nullable: false,
            default_value: None,
        });

        let canvas_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1200.0, 900.0));
        let (from, to, from_dir, to_dir) = ERDiagramState::calculate_connection_points_at_column(
            &child,
            &parent,
            "customer_id",
            "id",
            Vec2::ZERO,
            1.0,
            canvas_rect,
        );

        assert_eq!(from_dir, 3);
        assert_eq!(to_dir, 1);
        assert_eq!(from.x, child.position.x + child.size.x * 0.5);
        assert_eq!(to.x, parent.position.x + parent.size.x * 0.5);
        assert_eq!(from.y, child.position.y);
        assert_eq!(to.y, parent.position.y + parent.size.y);
    }

    #[test]
    fn orthogonal_route_points_use_mid_y_for_vertical_connectors() {
        let points = ERDiagramState::orthogonal_route_points(
            Pos2::new(120.0, 260.0),
            Pos2::new(180.0, 80.0),
            3,
            1,
            1.0,
            0.0,
        );

        assert!(points[1].y < points[0].y);
        assert!(points[4].y > points[5].y);
        assert_eq!(points[2].y, points[3].y);
        assert_eq!(points[2].x, points[1].x);
        assert_eq!(points[3].x, points[4].x);
    }

    #[test]
    fn route_lane_offsets_separate_overlapping_horizontal_routes() {
        let routes = vec![
            RoutedRelationship {
                relationship_index: 0,
                from_screen: Pos2::new(200.0, 180.0),
                to_screen: Pos2::new(440.0, 260.0),
                from_dir: 0,
                to_dir: 2,
            },
            RoutedRelationship {
                relationship_index: 1,
                from_screen: Pos2::new(200.0, 220.0),
                to_screen: Pos2::new(440.0, 300.0),
                from_dir: 0,
                to_dir: 2,
            },
        ];

        let offsets = ERDiagramState::route_lane_offsets(&routes, 1.0);

        assert_eq!(offsets.len(), 2);
        assert!(offsets[0] != 0.0);
        assert!(offsets[1] != 0.0);
        assert_eq!(offsets[0], -offsets[1]);
    }

    #[test]
    fn route_lane_offsets_separate_overlapping_vertical_routes() {
        let routes = vec![
            RoutedRelationship {
                relationship_index: 0,
                from_screen: Pos2::new(280.0, 380.0),
                to_screen: Pos2::new(360.0, 140.0),
                from_dir: 3,
                to_dir: 1,
            },
            RoutedRelationship {
                relationship_index: 1,
                from_screen: Pos2::new(340.0, 380.0),
                to_screen: Pos2::new(420.0, 140.0),
                from_dir: 3,
                to_dir: 1,
            },
        ];

        let offsets = ERDiagramState::route_lane_offsets(&routes, 1.0);

        assert_eq!(offsets.len(), 2);
        assert!(offsets[0] != 0.0);
        assert!(offsets[1] != 0.0);
        assert_eq!(offsets[0], -offsets[1]);
    }

    #[test]
    fn route_lane_offsets_keep_separated_horizontal_routes_on_same_baseline() {
        let routes = vec![
            RoutedRelationship {
                relationship_index: 0,
                from_screen: Pos2::new(200.0, 180.0),
                to_screen: Pos2::new(440.0, 220.0),
                from_dir: 0,
                to_dir: 2,
            },
            RoutedRelationship {
                relationship_index: 1,
                from_screen: Pos2::new(200.0, 420.0),
                to_screen: Pos2::new(440.0, 460.0),
                from_dir: 0,
                to_dir: 2,
            },
        ];

        let offsets = ERDiagramState::route_lane_offsets(&routes, 1.0);

        assert_eq!(offsets, vec![0.0, 0.0]);
    }

    #[test]
    fn route_lane_offsets_separate_overlapping_mixed_routes() {
        let routes = vec![
            RoutedRelationship {
                relationship_index: 0,
                from_screen: Pos2::new(200.0, 180.0),
                to_screen: Pos2::new(440.0, 320.0),
                from_dir: 0,
                to_dir: 3,
            },
            RoutedRelationship {
                relationship_index: 1,
                from_screen: Pos2::new(240.0, 220.0),
                to_screen: Pos2::new(440.0, 360.0),
                from_dir: 0,
                to_dir: 3,
            },
        ];

        let offsets = ERDiagramState::route_lane_offsets(&routes, 1.0);

        assert_eq!(offsets.len(), 2);
        assert!(offsets[0] != 0.0);
        assert!(offsets[1] != 0.0);
        assert_eq!(offsets[0], -offsets[1]);
    }

    #[test]
    fn orthogonal_route_points_shift_mixed_connector_elbow_column_by_lane_offset() {
        let points = ERDiagramState::orthogonal_route_points(
            Pos2::new(180.0, 160.0),
            Pos2::new(420.0, 320.0),
            0,
            3,
            1.0,
            14.0,
        );

        assert_eq!(points[2].x, points[3].x);
        assert_eq!(points[2].x, points[4].x + 14.0);
        assert_eq!(points[2].y, points[1].y);
        assert_eq!(points[3].y, points[4].y);
    }

    #[test]
    fn calculate_connection_points_keep_horizontal_anchors_for_side_by_side_tables() {
        let mut left = ERTable::new("customers".into());
        left.position = Pos2::new(80.0, 200.0);
        left.size = Vec2::new(180.0, 140.0);
        left.columns.push(crate::ui::ERColumn {
            name: "id".into(),
            data_type: "INTEGER".into(),
            is_primary_key: true,
            is_foreign_key: false,
            nullable: false,
            default_value: None,
        });

        let mut right = ERTable::new("orders".into());
        right.position = Pos2::new(420.0, 220.0);
        right.size = Vec2::new(180.0, 160.0);
        right.columns.push(crate::ui::ERColumn {
            name: "customer_id".into(),
            data_type: "INTEGER".into(),
            is_primary_key: false,
            is_foreign_key: true,
            nullable: false,
            default_value: None,
        });

        let canvas_rect = Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::new(1200.0, 900.0));
        let (from, to, from_dir, to_dir) = ERDiagramState::calculate_connection_points_at_column(
            &right,
            &left,
            "customer_id",
            "id",
            Vec2::ZERO,
            1.0,
            canvas_rect,
        );

        assert_eq!(from_dir, 2);
        assert_eq!(to_dir, 0);
        assert_eq!(from.x, right.position.x);
        assert_eq!(to.x, left.position.x + left.size.x);
    }

    #[test]
    fn table_visual_state_dims_non_neighborhood_tables_in_focus_mode() {
        let mut selected = ERTable::new("orders".into());
        selected.selected = true;
        let related = ERTable::new("customers".into());
        let other = ERTable::new("event_logs".into());
        let neighborhood =
            std::collections::HashSet::from(["orders".to_string(), "customers".to_string()]);

        let selected_state =
            table_visual_state(&selected, Some(&neighborhood), EREdgeDisplayMode::Focus);
        let related_state =
            table_visual_state(&related, Some(&neighborhood), EREdgeDisplayMode::Focus);
        let other_state = table_visual_state(&other, Some(&neighborhood), EREdgeDisplayMode::Focus);

        assert!(selected_state.selected);
        assert!(related_state.related);
        assert!(other_state.dimmed);
    }

    #[test]
    fn er_diagram_viewport_pan_shortcuts_only_apply_in_viewport_mode() {
        let (navigation_state, _) = run_diagram_key_with_modifiers_and_mode(
            Key::H,
            Modifiers::NONE,
            true,
            ERDiagramInteractionMode::Navigation,
        );
        assert_eq!(navigation_state.pan_offset, egui::Vec2::ZERO);

        let (viewport_state, _) = run_diagram_key_with_modifiers_and_mode(
            Key::H,
            Modifiers::NONE,
            true,
            ERDiagramInteractionMode::Viewport,
        );
        assert!(viewport_state.pan_offset.x > 0.0);
    }

    #[test]
    fn er_diagram_refresh_and_layout_shortcuts_remain_available_in_viewport_mode() {
        let refresh = run_diagram_key_with_modifiers_and_mode(
            Key::R,
            Modifiers::NONE,
            true,
            ERDiagramInteractionMode::Viewport,
        )
        .1;
        assert!(refresh.refresh_requested);

        let fit = run_diagram_key_with_modifiers_and_mode(
            Key::F,
            Modifiers::NONE,
            true,
            ERDiagramInteractionMode::Viewport,
        )
        .1;
        assert!(fit.fit_view_requested);

        let layout = run_diagram_key_with_modifiers_and_mode(
            Key::L,
            Modifiers::SHIFT,
            true,
            ERDiagramInteractionMode::Viewport,
        )
        .1;
        assert!(layout.layout_requested);
    }

    #[test]
    fn er_toolbar_button_chrome_hides_inactive_frame_for_default_buttons() {
        let chrome = er_toolbar_button_chrome(&Visuals::dark(), false);

        assert!(!chrome.frame_when_inactive);
        assert!(!chrome.selected);
        assert!(chrome.fill.is_none());
        assert!(chrome.stroke.is_none());
    }

    #[test]
    fn er_toolbar_button_chrome_keeps_selected_mode_button_visible() {
        let visuals = Visuals::dark();
        let chrome = er_toolbar_button_chrome(&visuals, true);

        assert!(chrome.frame_when_inactive);
        assert!(chrome.selected);
        assert_eq!(chrome.fill, Some(theme_selection_fill(&visuals, 56)));
        assert_eq!(
            chrome.stroke,
            Some(Stroke::new(1.0, theme_accent(&visuals)))
        );
    }

    #[test]
    fn theme_switch_render_keeps_er_viewport_selection_and_mode_state() {
        let mut state = ERDiagramState::new();
        let mut orders = ERTable::new("orders".to_string());
        orders.position = Pos2::new(240.0, 160.0);
        orders.size = Vec2::new(200.0, 140.0);
        state.set_tables(vec![orders]);
        assert!(state.select_table(0));
        state.pan_offset = Vec2::new(48.0, -24.0);
        state.zoom = 1.25;
        state.toggle_interaction_mode();

        let first = render_with_theme(&mut state, ThemePreset::TokyoNightStorm, false);
        let settled_pan = state.pan_offset;
        let settled_zoom = state.zoom;
        let second = render_with_theme(&mut state, ThemePreset::GithubLight, false);

        assert!(!first.refresh_requested);
        assert!(!first.layout_requested);
        assert!(!first.fit_view_requested);
        assert!(!second.refresh_requested);
        assert!(!second.layout_requested);
        assert!(!second.fit_view_requested);

        assert!(state.is_viewport_mode());
        assert_eq!(state.selected_table_name(), Some("orders"));
        assert_eq!(state.pan_offset, settled_pan);
        assert_eq!(state.zoom, settled_zoom);
    }

    #[test]
    fn render_colors_use_theme_palette_beyond_dark_mode_binary() {
        let visuals = Visuals::dark();
        let tokyo = RenderColors::from_theme(&ThemePreset::TokyoNightStorm, &visuals);
        let nord = RenderColors::from_theme(&ThemePreset::Nord, &visuals);

        assert_ne!(tokyo.background, nord.background);
        assert_ne!(tokyo.table_bg, nord.table_bg);
        assert_ne!(tokyo.table_header_bg, nord.table_header_bg);
    }

    #[test]
    fn render_colors_follow_visual_helpers_for_text_selection_and_separator() {
        let mut visuals = Visuals::light();
        visuals.override_text_color = Some(Color32::from_rgb(11, 22, 33));
        visuals.selection.stroke.color = Color32::from_rgb(120, 140, 220);
        visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(88, 99, 111);

        let colors = RenderColors::from_theme(&ThemePreset::GithubLight, &visuals);
        let subtle = theme_subtle_stroke(&visuals);

        assert_eq!(colors.text_primary, theme_text(&visuals));
        assert_eq!(colors.text_secondary, theme_muted_text(&visuals));
        assert_eq!(colors.table_selected_border, visuals.selection.stroke.color);
        assert_eq!(colors.table_border, subtle);
        assert_eq!(
            colors.row_separator,
            Color32::from_rgba_unmultiplied(subtle.r(), subtle.g(), subtle.b(), 20)
        );
    }

    #[test]
    fn render_colors_second_wave_tokens_follow_theme_palette_and_window_shadow() {
        let mut visuals = Visuals::dark();
        visuals.widgets.noninteractive.bg_stroke.color = Color32::from_rgb(70, 80, 90);
        visuals.window_shadow.color = Color32::from_rgba_unmultiplied(3, 4, 5, 120);

        let tokyo = RenderColors::from_theme(&ThemePreset::TokyoNightStorm, &visuals);
        let nord = RenderColors::from_theme(&ThemePreset::Nord, &visuals);

        assert_eq!(
            tokyo.grid_line,
            Color32::from_rgba_unmultiplied(70, 80, 90, 14)
        );
        assert_eq!(tokyo.table_shadow, visuals.window_shadow.color);
        assert_ne!(tokyo.pk_icon, nord.pk_icon);
        assert_ne!(tokyo.fk_icon, nord.fk_icon);
        assert_ne!(tokyo.relation_line_explicit, nord.relation_line_explicit);
        assert_ne!(tokyo.relation_line_inferred, nord.relation_line_inferred);
        assert_ne!(tokyo.text_type, nord.text_type);
    }
}
