//! ER 图渲染

use super::state::{ERDiagramState, ERTable, RelationType};
use crate::core::ThemePreset;
use egui::{self, Color32, FontId, Pos2, Rect, RichText, CornerRadius, Sense, Stroke, Vec2};

/// ER 图渲染响应
pub struct ERDiagramResponse {
    /// 是否需要刷新数据
    pub refresh_requested: bool,
    /// 是否需要重新布局
    pub layout_requested: bool,
    /// 是否需要适应视图
    pub fit_view_requested: bool,
}

impl Default for ERDiagramResponse {
    fn default() -> Self {
        Self {
            refresh_requested: false,
            layout_requested: false,
            fit_view_requested: false,
        }
    }
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
    fn from_theme(theme: &ThemePreset) -> Self {
        let is_dark = theme.is_dark();
        
        if is_dark {
            Self {
                background: Color32::from_rgb(32, 33, 36),
                grid_line: Color32::from_rgba_unmultiplied(255, 255, 255, 8),
                table_bg: Color32::from_rgb(48, 49, 54),
                table_header_bg: Color32::from_rgb(66, 66, 77),
                table_border: Color32::from_rgb(88, 88, 100),
                table_selected_border: Color32::from_rgb(100, 150, 255),
                table_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 60),
                text_primary: Color32::from_rgb(230, 230, 235),
                text_secondary: Color32::from_rgb(160, 160, 175),
                text_type: Color32::from_rgb(130, 140, 160),
                pk_icon: Color32::from_rgb(255, 193, 7),  // 金黄色
                fk_icon: Color32::from_rgb(33, 150, 243), // 蓝色
                relation_line: Color32::from_rgb(100, 120, 160),
                row_separator: Color32::from_rgba_unmultiplied(255, 255, 255, 15),
            }
        } else {
            Self {
                background: Color32::from_rgb(250, 250, 252),
                grid_line: Color32::from_rgba_unmultiplied(0, 0, 0, 8),
                table_bg: Color32::from_rgb(255, 255, 255),
                table_header_bg: Color32::from_rgb(248, 249, 252),
                table_border: Color32::from_rgb(218, 220, 228),
                table_selected_border: Color32::from_rgb(66, 133, 244),
                table_shadow: Color32::from_rgba_unmultiplied(0, 0, 0, 25),
                text_primary: Color32::from_rgb(32, 33, 36),
                text_secondary: Color32::from_rgb(95, 99, 104),
                text_type: Color32::from_rgb(128, 134, 145),
                pk_icon: Color32::from_rgb(251, 188, 4),  // 金黄色
                fk_icon: Color32::from_rgb(26, 115, 232), // 蓝色
                relation_line: Color32::from_rgb(130, 140, 170),
                row_separator: Color32::from_rgba_unmultiplied(0, 0, 0, 8),
            }
        }
    }
}

impl ERDiagramState {
    /// 渲染 ER 图
    pub fn show(&mut self, ui: &mut egui::Ui, theme: &ThemePreset) -> ERDiagramResponse {
        let mut response = ERDiagramResponse::default();
        let colors = RenderColors::from_theme(theme);

        // 工具栏
        ui.horizontal(|ui| {
            if ui.button("刷新 [R]").clicked() {
                response.refresh_requested = true;
            }
            if ui.button("布局 [L]").clicked() {
                response.layout_requested = true;
            }
            if ui.button("适应 [F]").clicked() {
                response.fit_view_requested = true;
            }
            
            ui.separator();
            
            if ui.button("+").on_hover_text("放大").clicked() {
                self.zoom_by(1.2);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("-").on_hover_text("缩小").clicked() {
                self.zoom_by(0.8);
            }
            if ui.button("重置").on_hover_text("重置视图").clicked() {
                self.reset_view();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("{} 张表", self.tables.len()))
                        .small()
                        .color(colors.text_secondary),
                );
                
                ui.separator();
                
                // 图例说明
                ui.label(
                    RichText::new("图例:")
                        .small()
                        .color(colors.text_secondary),
                ).on_hover_text("! = NOT NULL\n? = 可空\n= = 有默认值\n● = 主键\n○ = 外键");
            });
        });

        ui.separator();

        // 画布区域
        let available = ui.available_rect_before_wrap();
        let (canvas_response, painter) = ui.allocate_painter(
            available.size(),
            Sense::click_and_drag(),
        );
        let canvas_rect = canvas_response.rect;

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
            // 绘制关系线
            self.draw_relationships(&painter, canvas_rect, &colors);

            // 绘制表格 - 先计算尺寸
            for table in &mut self.tables {
                Self::calculate_table_size(table);
            }
            // 再绘制
            for table in &self.tables {
                Self::draw_table_static(&painter, table, canvas_rect, &colors, self.pan_offset, self.zoom);
            }
        }

        // 处理交互
        self.handle_interaction(ui, &canvas_response, canvas_rect);

        // 键盘快捷键
        if canvas_response.has_focus() || canvas_response.hovered() {
            ui.input(|i| {
                if i.key_pressed(egui::Key::R) {
                    response.refresh_requested = true;
                }
                if i.key_pressed(egui::Key::L) {
                    response.layout_requested = true;
                }
                if i.key_pressed(egui::Key::F) {
                    response.fit_view_requested = true;
                }
                if i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals) {
                    self.zoom_by(1.2);
                }
                if i.key_pressed(egui::Key::Minus) {
                    self.zoom_by(0.8);
                }
            });
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
                painter.circle_filled(
                    Pos2::new(x, y),
                    dot_size,
                    colors.grid_line,
                );
                y += grid_size;
            }
            x += grid_size;
        }
    }

    /// 计算表格尺寸（根据内容自适应宽度）
    fn calculate_table_size(table: &mut ERTable) {
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
        let max_column_width = table.columns.iter().map(|col| {
            let icons = if col.is_primary_key { icon_width } else { 0.0 }
                      + if col.is_foreign_key { icon_width } else { 0.0 };
            let name_width = col.name.len() as f32 * char_width;
            let type_width = col.data_type.len() as f32 * char_width * 0.8;
            icons + name_width + type_gap + type_width + null_marker_width + padding * 2.0
        }).fold(0.0_f32, |a, b| a.max(b));
        
        // 取表名和列中的最大宽度
        let content_width = header_width.max(max_column_width).clamp(min_width, max_width);

        let num_columns = table.columns.len();
        let content_height = header_height + (num_columns as f32 * row_height) + padding;
        table.size = Vec2::new(content_width, content_height.max(min_height));
    }

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
        let header_rect = Rect::from_min_size(
            screen_pos,
            Vec2::new(screen_size.x, header_height * zoom),
        );
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
                Pos2::new(screen_pos.x + screen_size.x, screen_pos.y + header_height * zoom),
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

    /// 绘制关系线
    fn draw_relationships(&self, painter: &egui::Painter, canvas_rect: Rect, colors: &RenderColors) {
        for rel in &self.relationships {
            let from_table = self.tables.iter().find(|t| t.name == rel.from_table);
            let to_table = self.tables.iter().find(|t| t.name == rel.to_table);

            if let (Some(from), Some(to)) = (from_table, to_table) {
                // 计算连接点
                let from_screen = Pos2::new(
                    canvas_rect.left() + (from.position.x + from.size.x + self.pan_offset.x) * self.zoom,
                    canvas_rect.top() + (from.position.y + from.size.y / 2.0 + self.pan_offset.y) * self.zoom,
                );
                let to_screen = Pos2::new(
                    canvas_rect.left() + (to.position.x + self.pan_offset.x) * self.zoom,
                    canvas_rect.top() + (to.position.y + to.size.y / 2.0 + self.pan_offset.y) * self.zoom,
                );

                // 计算控制点（贝塞尔曲线）
                let dx = (to_screen.x - from_screen.x).abs() / 2.0;
                let ctrl1 = Pos2::new(from_screen.x + dx, from_screen.y);
                let ctrl2 = Pos2::new(to_screen.x - dx, to_screen.y);

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

                // 绘制箭头
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
                    (from_screen.y + to_screen.y) / 2.0 - 10.0,
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
    fn handle_interaction(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        canvas_rect: Rect,
    ) {
        // 滚轮缩放
        let scroll_delta = ui.input(|i| i.raw_scroll_delta);
        if response.hovered() && scroll_delta.y != 0.0 {
            let factor = if scroll_delta.y > 0.0 { 1.1 } else { 0.9 };
            self.zoom_by(factor);
        }

        // 点击选择表格
        if response.clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
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
        }

        // 拖动
        if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                if self.dragging_table.is_some() {
                    // 拖动表格
                    let delta = response.drag_delta() / self.zoom;
                    if let Some(idx) = self.dragging_table {
                        if let Some(table) = self.tables.get_mut(idx) {
                            table.position.x += delta.x;
                            table.position.y += delta.y;
                        }
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
        }

        // 拖动结束
        if response.drag_stopped() {
            self.end_drag();
        }
    }
}
