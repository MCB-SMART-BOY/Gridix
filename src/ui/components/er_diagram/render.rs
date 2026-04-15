//! ER 图渲染

use super::state::{ERDiagramInteractionMode, ERDiagramState, ERTable, RelationType};
use crate::core::ThemePreset;
use crate::ui::styles::{
    theme_accent, theme_muted_text, theme_selection_fill, theme_subtle_stroke, theme_text,
};
use crate::ui::{
    LocalShortcut, consume_local_shortcut, local_shortcut_text, local_shortcut_tooltip,
};
use egui::{self, Color32, CornerRadius, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2};

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
    ToggleViewportMode,
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
    table_border: Color32,
    table_selected_border: Color32,
    table_shadow: Color32,
    text_primary: Color32,
    text_secondary: Color32,
    text_type: Color32,
    pk_icon: Color32,
    fk_icon: Color32,
    relation_line: Color32,
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
        let pk_icon = Self::blend(
            palette.warning,
            palette.fg_primary,
            if visuals.dark_mode { 0.08 } else { 0.16 },
        );
        let fk_icon = Self::blend(palette.info, palette.accent, 0.35);
        let relation_line = Self::blend(
            palette.border,
            palette.info,
            if visuals.dark_mode { 0.62 } else { 0.52 },
        );

        Self {
            background: palette.bg_primary,
            grid_line,
            table_bg: palette.bg_secondary,
            table_header_bg: palette.bg_tertiary,
            table_border: border,
            table_selected_border: visuals.selection.stroke.color,
            table_shadow,
            text_primary,
            text_secondary,
            text_type,
            pk_icon,
            fk_icon,
            relation_line,
            row_separator,
        }
    }
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

        // 工具栏
        ui.horizontal(|ui| {
            let mode_label = if self.is_viewport_mode() {
                "视口模式"
            } else {
                "浏览模式"
            };
            let chrome = er_toolbar_button_chrome(ui.visuals(), self.is_viewport_mode());
            let mut mode_button = egui::Button::new(RichText::new(mode_label).size(11.0))
                .frame(true)
                .frame_when_inactive(chrome.frame_when_inactive)
                .selected(chrome.selected);
            if let Some(fill) = chrome.fill {
                mode_button = mode_button.fill(fill);
            }
            if let Some(stroke) = chrome.stroke {
                mode_button = mode_button.stroke(stroke);
            }

            if ui
                .add(mode_button)
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

            ui.add_space(8.0);

            // 刷新按钮
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

            // 布局按钮
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

            // 适应视图按钮
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

            ui.add_space(8.0);

            // 缩放控制
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

            // 重置视图按钮
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
                    RichText::new(format!("{} 张表", self.tables.len()))
                        .small()
                        .color(colors.text_secondary),
                );

                ui.add_space(8.0);

                // 图例说明
                ui.label(RichText::new("ℹ").size(13.0).color(colors.text_secondary))
                    .on_hover_text(
                        "图例说明:\n● = 主键\n○ = 外键\n! = NOT NULL\n? = 可空\n= = 有默认值",
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
            for table in &mut self.tables {
                Self::calculate_table_size(table);
            }

            self.reveal_selected_table_in_view(canvas_rect.size());

            // 绘制关系线（在表格下方）
            self.draw_relationships(&painter, canvas_rect, &colors);

            // 绘制表格
            for table in &self.tables {
                Self::draw_table_static(
                    &painter,
                    table,
                    canvas_rect,
                    &colors,
                    self.pan_offset,
                    self.zoom,
                );
            }
        }

        // 处理交互
        self.handle_interaction(ui, &canvas_response, canvas_rect);

        // 键盘快捷键
        if is_focused
            && let Some(action) = consume_er_diagram_key_action(ui, self.interaction_mode())
        {
            match action {
                ERDiagramKeyAction::ToggleViewportMode => {
                    self.toggle_interaction_mode();
                }
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

    /// 计算表格尺寸（根据内容自适应宽度）
    fn calculate_table_size(table: &mut ERTable) {
        calculate_table_size(table);
    }
}

fn consume_er_diagram_key_action(
    ui: &mut egui::Ui,
    interaction_mode: ERDiagramInteractionMode,
) -> Option<ERDiagramKeyAction> {
    ui.input_mut(|input| {
        if consume_local_shortcut(input, LocalShortcut::ErDiagramViewportMode) {
            Some(ERDiagramKeyAction::ToggleViewportMode)
        } else if consume_local_shortcut(input, LocalShortcut::ErDiagramRefresh) {
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

/// 计算表格尺寸（根据内容自适应宽度）
///
/// 公开函数，可在数据加载后立即调用以确保布局正确
pub fn calculate_table_size(table: &mut ERTable) {
    let header_height = 36.0;
    let row_height = 24.0;
    let padding = 12.0;
    let min_width = 180.0;
    let max_width = 320.0;
    let min_height = 80.0;
    let char_width = 7.0; // 等宽字体每字符宽度估算
    let icon_width = 14.0; // 主键/外键图标宽度
    let type_gap = 24.0; // 列名和类型之间的间距
    let null_marker_width = 16.0; // NULL 标记宽度

    // 计算表名宽度
    let header_width = table.name.len() as f32 * char_width + padding * 4.0;

    // 计算每列需要的宽度（列名 + 图标 + 类型 + NULL标记）
    let max_column_width = table
        .columns
        .iter()
        .map(|col| {
            let icons = if col.is_primary_key { icon_width } else { 0.0 }
                + if col.is_foreign_key { icon_width } else { 0.0 };
            let name_width = col.name.len() as f32 * char_width;
            let type_width = col.data_type.len() as f32 * char_width * 0.8;
            icons + name_width + type_gap + type_width + null_marker_width + padding * 2.0
        })
        .fold(0.0_f32, |a, b| a.max(b));

    // 取表名和列中的最大宽度
    let content_width = header_width
        .max(max_column_width)
        .clamp(min_width, max_width);

    let num_columns = table.columns.len();
    let content_height = header_height + (num_columns as f32 * row_height) + padding;
    table.size = Vec2::new(content_width, content_height.max(min_height));
}

impl ERDiagramState {
    /// 绘制表格（静态方法）
    fn draw_table_static(
        painter: &egui::Painter,
        table: &ERTable,
        canvas_rect: Rect,
        colors: &RenderColors,
        pan_offset: Vec2,
        zoom: f32,
    ) {
        let padding = 12.0;
        let header_height = 36.0;
        let row_height = 24.0;

        // 计算屏幕位置
        let screen_pos = Pos2::new(
            canvas_rect.left() + (table.position.x + pan_offset.x) * zoom,
            canvas_rect.top() + (table.position.y + pan_offset.y) * zoom,
        );
        let screen_size = table.size * zoom;
        let table_rect = Rect::from_min_size(screen_pos, screen_size);

        // 检查是否在可见区域
        if !canvas_rect.intersects(table_rect) {
            return;
        }

        let corner_radius = (8.0 * zoom) as u8;

        // 绘制阴影
        let shadow_offset = 3.0 * zoom;
        let shadow_rect = Rect::from_min_size(
            screen_pos + Vec2::new(shadow_offset, shadow_offset),
            screen_size,
        );
        painter.rect_filled(
            shadow_rect,
            CornerRadius::same(corner_radius),
            colors.table_shadow,
        );

        // 绘制表格背景
        painter.rect_filled(
            table_rect,
            CornerRadius::same(corner_radius),
            colors.table_bg,
        );

        // 绘制边框
        painter.rect_stroke(
            table_rect,
            CornerRadius::same(corner_radius),
            Stroke::new(
                if table.selected { 2.0 * zoom } else { 1.0 },
                if table.selected {
                    colors.table_selected_border
                } else {
                    colors.table_border
                },
            ),
            egui::StrokeKind::Inside,
        );

        // 绘制表头背景
        let header_rect =
            Rect::from_min_size(screen_pos, Vec2::new(screen_size.x, header_height * zoom));
        painter.rect_filled(
            header_rect,
            CornerRadius {
                nw: corner_radius,
                ne: corner_radius,
                sw: 0,
                se: 0,
            },
            colors.table_header_bg,
        );

        // 表头分隔线
        painter.line_segment(
            [
                Pos2::new(screen_pos.x, screen_pos.y + header_height * zoom),
                Pos2::new(
                    screen_pos.x + screen_size.x,
                    screen_pos.y + header_height * zoom,
                ),
            ],
            Stroke::new(1.0, colors.table_border),
        );

        // 表名（加粗）
        let font_size = 13.0 * zoom;
        painter.text(
            header_rect.center(),
            egui::Align2::CENTER_CENTER,
            &table.name,
            FontId::proportional(font_size),
            colors.text_primary,
        );

        // 绘制列
        let small_font_size = 11.0 * zoom;
        let tiny_font_size = 9.0 * zoom;
        let icon_size = 12.0 * zoom;

        for (i, col) in table.columns.iter().enumerate() {
            let row_y = screen_pos.y + (header_height + i as f32 * row_height) * zoom;
            let row_x = screen_pos.x + padding * zoom;
            let row_center_y = row_y + row_height * zoom / 2.0;

            // 行分隔线（除了第一行）
            if i > 0 {
                painter.line_segment(
                    [
                        Pos2::new(screen_pos.x + 8.0 * zoom, row_y),
                        Pos2::new(screen_pos.x + screen_size.x - 8.0 * zoom, row_y),
                    ],
                    Stroke::new(1.0, colors.row_separator),
                );
            }

            // 图标区域
            let mut icon_x = row_x;

            // 主键图标
            if col.is_primary_key {
                painter.circle_filled(
                    Pos2::new(icon_x + 4.0 * zoom, row_center_y),
                    3.0 * zoom,
                    colors.pk_icon,
                );
                icon_x += icon_size + 2.0 * zoom;
            }

            // 外键图标
            if col.is_foreign_key {
                painter.circle_stroke(
                    Pos2::new(icon_x + 4.0 * zoom, row_center_y),
                    3.0 * zoom,
                    Stroke::new(1.5 * zoom, colors.fk_icon),
                );
                icon_x += icon_size + 2.0 * zoom;
            }

            // 列名（如果非空则加粗显示）
            let text_x = if col.is_primary_key || col.is_foreign_key {
                icon_x
            } else {
                row_x
            };

            // 列名颜色：NOT NULL 用主色，可空用次级色
            let name_color = if !col.nullable {
                colors.text_primary
            } else {
                colors.text_secondary
            };

            painter.text(
                Pos2::new(text_x, row_center_y),
                egui::Align2::LEFT_CENTER,
                &col.name,
                FontId::proportional(small_font_size),
                name_color,
            );

            // 右侧信息区：数据类型 + 标记
            let right_x = screen_pos.x + screen_size.x - padding * zoom;

            // 构建标记字符串：NULL标记 + 默认值标记
            let mut markers = String::new();

            // 默认值标记 (=)
            if col.default_value.is_some() {
                markers.push('=');
            }

            // NULL/NOT NULL 标记
            if col.nullable {
                markers.push('?');
            } else {
                markers.push('!');
            }

            // 标记颜色：如有默认值用蓝色，否则按 nullable 区分
            let marker_color = if col.default_value.is_some() {
                colors.fk_icon // 蓝色表示有默认值
            } else if col.nullable {
                colors.text_type
            } else {
                colors.pk_icon // 金色强调 NOT NULL
            };

            painter.text(
                Pos2::new(right_x, row_center_y),
                egui::Align2::RIGHT_CENTER,
                &markers,
                FontId::proportional(tiny_font_size),
                marker_color,
            );

            // 数据类型（在标记左边）
            let markers_width = markers.len() as f32 * 6.0 * zoom;
            painter.text(
                Pos2::new(right_x - markers_width - 4.0 * zoom, row_center_y),
                egui::Align2::RIGHT_CENTER,
                &col.data_type,
                FontId::proportional(small_font_size * 0.9),
                colors.text_type,
            );
        }
    }

    /// 计算列在表格中的Y偏移（从表格顶部开始）
    fn get_column_y_offset(table: &ERTable, column_name: &str) -> f32 {
        let header_height = 36.0;
        let row_height = 24.0;

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

        // 只使用左右连接
        let (from_edge, to_edge, from_dir, to_dir) = if to_center_x > from_center_x {
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

    /// 根据连接方向计算贝塞尔曲线控制点
    fn calculate_control_points(
        from: Pos2,
        to: Pos2,
        from_dir: i32,
        to_dir: i32,
        zoom: f32,
    ) -> (Pos2, Pos2) {
        let control_distance = 50.0 * zoom;

        // 根据方向计算控制点偏移
        // direction: 0=右, 1=下, 2=左, 3=上
        let from_offset = match from_dir {
            0 => Vec2::new(control_distance, 0.0),  // 右
            1 => Vec2::new(0.0, control_distance),  // 下
            2 => Vec2::new(-control_distance, 0.0), // 左
            3 => Vec2::new(0.0, -control_distance), // 上
            _ => Vec2::ZERO,
        };

        let to_offset = match to_dir {
            0 => Vec2::new(control_distance, 0.0),  // 右
            1 => Vec2::new(0.0, control_distance),  // 下
            2 => Vec2::new(-control_distance, 0.0), // 左
            3 => Vec2::new(0.0, -control_distance), // 上
            _ => Vec2::ZERO,
        };

        (from + from_offset, to + to_offset)
    }

    /// 绘制关系线
    fn draw_relationships(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        colors: &RenderColors,
    ) {
        for rel in &self.relationships {
            let from_table = self.tables.iter().find(|t| t.name == rel.from_table);
            let to_table = self.tables.iter().find(|t| t.name == rel.to_table);

            if let (Some(from), Some(to)) = (from_table, to_table) {
                // 计算连接点（在外键列位置，只使用左右连接）
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

                // 计算控制点
                let (ctrl1, ctrl2) = Self::calculate_control_points(
                    from_screen,
                    to_screen,
                    from_dir,
                    to_dir,
                    self.zoom,
                );

                // 绘制贝塞尔曲线
                let points: Vec<Pos2> = (0..=20)
                    .map(|i| {
                        let t = i as f32 / 20.0;
                        let t2 = t * t;
                        let t3 = t2 * t;
                        let mt = 1.0 - t;
                        let mt2 = mt * mt;
                        let mt3 = mt2 * mt;

                        Pos2::new(
                            mt3 * from_screen.x
                                + 3.0 * mt2 * t * ctrl1.x
                                + 3.0 * mt * t2 * ctrl2.x
                                + t3 * to_screen.x,
                            mt3 * from_screen.y
                                + 3.0 * mt2 * t * ctrl1.y
                                + 3.0 * mt * t2 * ctrl2.y
                                + t3 * to_screen.y,
                        )
                    })
                    .collect();

                for window in points.windows(2) {
                    painter.line_segment(
                        [window[0], window[1]],
                        Stroke::new(2.0, colors.relation_line),
                    );
                }

                // 绘制箭头（在 to 端）
                let arrow_size = 8.0 * self.zoom;
                let angle = (to_screen.y - ctrl2.y).atan2(to_screen.x - ctrl2.x);
                let arrow_p1 = Pos2::new(
                    to_screen.x - arrow_size * (angle - 0.4).cos(),
                    to_screen.y - arrow_size * (angle - 0.4).sin(),
                );
                let arrow_p2 = Pos2::new(
                    to_screen.x - arrow_size * (angle + 0.4).cos(),
                    to_screen.y - arrow_size * (angle + 0.4).sin(),
                );
                painter.line_segment(
                    [to_screen, arrow_p1],
                    Stroke::new(2.0, colors.relation_line),
                );
                painter.line_segment(
                    [to_screen, arrow_p2],
                    Stroke::new(2.0, colors.relation_line),
                );

                // 绘制关系类型标记
                let mid_point = Pos2::new(
                    (from_screen.x + to_screen.x) / 2.0,
                    (from_screen.y + to_screen.y) / 2.0 - 10.0 * self.zoom,
                );
                let label = match rel.relation_type {
                    RelationType::OneToOne => "1:1",
                    RelationType::OneToMany => "1:N",
                    RelationType::ManyToMany => "N:M",
                };
                painter.text(
                    mid_point,
                    egui::Align2::CENTER_CENTER,
                    label,
                    FontId::proportional(10.0 * self.zoom),
                    colors.text_secondary,
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
    use super::{ERDiagramInteractionMode, ERDiagramState, RenderColors, er_toolbar_button_chrome};
    use crate::core::ThemePreset;
    use crate::ui::ERTable;
    use crate::ui::styles::{
        theme_accent, theme_muted_text, theme_selection_fill, theme_subtle_stroke, theme_text,
    };
    use eframe::egui::{Area, Context, Event, Id, Key, Modifiers, RawInput};
    use egui::{Color32, Pos2, Stroke, Vec2, Visuals};

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
        assert_ne!(tokyo.relation_line, nord.relation_line);
        assert_ne!(tokyo.text_type, nord.text_type);
    }
}
