//! 对话框窄宽度响应式行布局
//!
//! 统一的"标签 + 控件"行:可用宽度足够时标签在左、控件在右;窄宽度时标签独占一行,
//! 控件占满可用宽度。固定宽度的控件会把内容顶出对话框(窄视口下溢出/裁切),
//! 因此连接、新建数据库、新建用户三个对话框共用本模块,而不是各自维护一份实现。

use crate::ui::styles::{GRAY, SPACING_SM};
use egui::{self, RichText};

/// 行宽档位。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResponsiveRowClass {
    /// 宽档:标签列最宽,控件用首选宽度。
    Wide,
    /// 中档:标签列略窄,其余同宽档。
    Medium,
    /// 窄档:单列布局,标签在控件上方。
    Narrow,
}

/// 进入宽档的可用宽度阈值。
pub const WIDE_ROW_THRESHOLD: f32 = 720.0;
/// 进入中档的可用宽度阈值;低于此值使用单列布局。
pub const MEDIUM_ROW_THRESHOLD: f32 = 560.0;

/// 单个对话框的标签列宽:宽档与中档各一个,窄档固定为 0。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RowMetrics {
    wide_label_width: f32,
    medium_label_width: f32,
}

impl RowMetrics {
    /// 构造标签列宽,宽档不小于中档。
    pub const fn new(wide_label_width: f32, medium_label_width: f32) -> Self {
        Self {
            wide_label_width,
            medium_label_width,
        }
    }

    /// 标签列宽度;窄档返回 0 表示标签独占一行。
    pub fn label_width(self, row_class: ResponsiveRowClass) -> f32 {
        match row_class {
            ResponsiveRowClass::Wide => self.wide_label_width,
            ResponsiveRowClass::Medium => self.medium_label_width,
            ResponsiveRowClass::Narrow => 0.0,
        }
    }
}

/// 按可用宽度选择行宽档位。
pub fn row_width_class(available_width: f32) -> ResponsiveRowClass {
    if available_width >= WIDE_ROW_THRESHOLD {
        ResponsiveRowClass::Wide
    } else if available_width >= MEDIUM_ROW_THRESHOLD {
        ResponsiveRowClass::Medium
    } else {
        ResponsiveRowClass::Narrow
    }
}

/// 控件宽度:宽档/中档取首选宽度与可用宽度的较小值,窄档占满可用宽度。
pub fn control_width(ui: &egui::Ui, row_class: ResponsiveRowClass, preferred_width: f32) -> f32 {
    match row_class {
        ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
            ui.available_width().min(preferred_width)
        }
        ResponsiveRowClass::Narrow => ui.available_width(),
    }
}

/// 渲染一行"标签 + 控件";控件由 `body` 绘制,并收到本行的档位,返回值透传给调用方。
pub fn responsive_row<T>(
    ui: &mut egui::Ui,
    label: &str,
    metrics: RowMetrics,
    body: impl FnOnce(&mut egui::Ui, ResponsiveRowClass) -> T,
) -> T {
    let row_class = row_width_class(ui.available_width());

    let value = match row_class {
        ResponsiveRowClass::Narrow => {
            ui.label(RichText::new(label).color(GRAY));
            ui.add_space(SPACING_SM);
            body(ui, row_class)
        }
        ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
            let label_width = metrics.label_width(row_class);
            ui.horizontal_top(|ui| {
                // 横向子 Ui 里的 `add_sized` 会给标签 `wrap.max_width = INFINITY`，
                // 超长标签会撑宽标签列并右推控件；显式截断把标签锁在列宽内。
                ui.add_sized(
                    [label_width, 0.0],
                    egui::Label::new(RichText::new(label).color(GRAY)).truncate(),
                );
                ui.add_space(SPACING_SM);
                ui.vertical(|ui| body(ui, row_class)).inner
            })
            .inner
        }
    };

    ui.add_space(SPACING_SM);
    value
}

/// 渲染一行"标签 + 控件",忽略 `body` 的返回值。
pub fn show_responsive_labeled_row(
    ui: &mut egui::Ui,
    label: &str,
    metrics: RowMetrics,
    body: impl FnOnce(&mut egui::Ui, ResponsiveRowClass),
) {
    responsive_row(ui, label, metrics, |ui, row_class| body(ui, row_class));
}

/// 渲染一行单行文本输入;宽度随档位收敛到可用宽度。
pub fn show_responsive_text_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut String,
    hint: &str,
    preferred_width: f32,
    metrics: RowMetrics,
) {
    show_responsive_labeled_row(ui, label, metrics, |ui, row_class| {
        let width = control_width(ui, row_class, preferred_width);
        ui.add_sized(
            [width, 0.0],
            egui::TextEdit::singleline(value).hint_text(hint),
        );
    });
}

/// 渲染一行下拉框;`body` 收到收敛后的控件宽度。
///
/// `empty_selection` 为真且处于窄档时补一点间距,避免空选择文本与下一行贴在一起。
pub fn show_responsive_combo_row(
    ui: &mut egui::Ui,
    label: &str,
    preferred_width: f32,
    empty_selection: bool,
    metrics: RowMetrics,
    body: impl FnOnce(&mut egui::Ui, f32),
) {
    show_responsive_labeled_row(ui, label, metrics, |ui, row_class| {
        let width = control_width(ui, row_class, preferred_width);
        ui.scope(|ui| {
            ui.set_min_width(width);
            ui.set_width(width);
            body(ui, width);
        });

        if matches!(row_class, ResponsiveRowClass::Narrow) && empty_selection {
            ui.add_space(2.0);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Pos2, RawInput, Rect, Vec2};

    const METRICS: RowMetrics = RowMetrics::new(88.0, 80.0);

    /// 在指定可用宽度下渲染一行,返回
    /// (内容宽度, 档位, `control_width` 给出的控件宽度, 行内可用宽度)。
    ///
    /// 渲染同时校验控件 rect 不超过请求宽度,避免布局挤压被静默忽略。
    fn rendered_row(
        viewport_width: f32,
        label: &str,
        metrics: RowMetrics,
    ) -> (f32, ResponsiveRowClass, f32, f32) {
        let ctx = egui::Context::default();
        ctx.begin_pass(RawInput {
            screen_rect: Some(Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(viewport_width, 400.0),
            )),
            ..Default::default()
        });

        let mut row_class = ResponsiveRowClass::Wide;
        let mut content_width = 0.0_f32;
        let mut requested_width = 0.0_f32;
        let mut row_available_width = 0.0_f32;
        egui::Area::new(egui::Id::new("responsive_row_test_area")).show(&ctx, |ui| {
            ui.set_max_width(viewport_width);
            let available = ui.available_width();
            row_class = row_width_class(available);
            show_responsive_labeled_row(ui, label, metrics, |ui, class| {
                row_available_width = ui.available_width();
                let width = control_width(ui, class, 200.0);
                requested_width = width;
                let rect = ui
                    .add_sized([width, 0.0], egui::TextEdit::singleline(&mut String::new()))
                    .rect;
                assert!(
                    rect.width() <= width + 1.0,
                    "控件渲染宽度 {} 超过请求宽度 {width}",
                    rect.width()
                );
            });
            content_width = ui.min_rect().width();
        });
        let _ = ctx.end_pass();

        (
            content_width,
            row_class,
            requested_width,
            row_available_width,
        )
    }

    #[test]
    fn row_width_class_classifies_threshold_boundaries() {
        assert_eq!(
            row_width_class(WIDE_ROW_THRESHOLD),
            ResponsiveRowClass::Wide
        );
        assert_eq!(
            row_width_class(WIDE_ROW_THRESHOLD - 0.5),
            ResponsiveRowClass::Medium
        );
        assert_eq!(
            row_width_class(MEDIUM_ROW_THRESHOLD),
            ResponsiveRowClass::Medium
        );
        assert_eq!(
            row_width_class(MEDIUM_ROW_THRESHOLD - 0.5),
            ResponsiveRowClass::Narrow
        );
    }

    #[test]
    fn label_width_is_zero_only_in_narrow_rows() {
        assert_eq!(METRICS.label_width(ResponsiveRowClass::Wide), 88.0);
        assert_eq!(METRICS.label_width(ResponsiveRowClass::Medium), 80.0);
        assert_eq!(METRICS.label_width(ResponsiveRowClass::Narrow), 0.0);
    }

    #[test]
    fn labeled_row_never_exceeds_narrow_viewport() {
        // 260 是旧实现（标签列 88 + 间距 + 固定 200 输入框 ≈ 296）必然溢出的宽度，
        // 因此这条断言在修复前会失败；320/420 用于覆盖常见窄窗口。
        for width in [260.0_f32, 320.0, 359.0, 420.0] {
            let (content_width, row_class, requested_width, row_available) =
                rendered_row(width, "用户名", METRICS);
            assert_eq!(
                row_class,
                ResponsiveRowClass::Narrow,
                "宽度 {width} 应为单列布局"
            );
            assert!(
                content_width <= width,
                "宽度 {width} 下内容宽度 {content_width} 溢出"
            );
            assert!(
                requested_width >= row_available - 0.5,
                "窄档控件应占满行可用宽度: {requested_width} < {row_available}"
            );
        }
    }

    #[test]
    fn labeled_row_keeps_preferred_width_when_room_allows() {
        let (content_width, row_class, requested_width, _) = rendered_row(900.0, "用户名", METRICS);
        assert_eq!(row_class, ResponsiveRowClass::Wide);
        assert!(
            content_width <= 900.0,
            "宽档下内容宽度 {content_width} 不应超过视口"
        );
        assert!(
            (requested_width - 200.0).abs() < 1.0,
            "宽档控件应保持首选宽度 200,实际 {requested_width}"
        );
    }

    #[test]
    fn overlong_label_is_truncated_to_its_column() {
        let long_label = "一个远超标签列宽度的字段说明文本用于验证截断行为".repeat(3);
        for width in [900.0_f32, 600.0] {
            let (content_width, row_class, requested_width, _) =
                rendered_row(width, &long_label, METRICS);
            assert_ne!(row_class, ResponsiveRowClass::Narrow, "应为宽/中档布局");
            assert!(
                content_width <= width,
                "宽度 {width} 下超长标签使内容宽度达到 {content_width}"
            );
            assert!(
                requested_width <= 200.5,
                "标签不得挤压控件: {requested_width}"
            );
        }
    }
}
