//! 数据导出对话框 - 支持多种格式和自定义选项
//!
//! 支持的快捷键：
//! - `Esc` / `q` - 关闭对话框
//! - `Enter` - 导出（当配置有效时）
//! - `1/2/3/4` - 快速选择格式 (CSV/TSV/SQL/JSON)
//! - `h/l` - 切换格式
//! - `j/k` - 在列选择中导航
//! - `gg/G` - 跳转到首/末列
//! - `Space` - 切换当前列的选中状态
//! - `a` - 全选/取消全选列

use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStatus, DialogStyle, DialogWindow,
};
use crate::core::{
    ExportFormat, SqlDialect, TransferDelimitedOptions, TransferDirection, TransferFormatOptions,
    TransferJsonOptions, TransferMapping, TransferRowWindow, TransferSchema, TransferSession,
    TransferSqlOptions, preview_export_transfer,
};
use crate::database::{DatabaseType, QueryResult};
use crate::ui::styles::{GRAY, MUTED, SPACING_SM};
use crate::ui::{
    LocalShortcut, local_shortcut_text, local_shortcut_tooltip, local_shortcuts_text,
    local_shortcuts_tooltip,
};
use egui::{self, Color32, CornerRadius, RichText, ScrollArea, TextEdit};

/// 导出配置
#[derive(Clone)]
pub struct ExportConfig {
    /// 导出格式
    pub format: ExportFormat,
    /// 选中的列索引
    pub selected_columns: Vec<bool>,
    /// 行数限制 (0 = 全部)
    pub row_limit: usize,
    /// 起始行 (0-based)
    pub start_row: usize,
    /// CSV: 分隔符
    pub csv_delimiter: char,
    /// CSV: 是否包含表头
    pub csv_include_header: bool,
    /// CSV: 引用字符
    pub csv_quote_char: char,
    /// SQL: 是否使用事务
    pub sql_use_transaction: bool,
    /// SQL: 批量插入大小 (0 = 单行插入)
    pub sql_batch_size: usize,
    /// JSON: 是否美化输出
    pub json_pretty: bool,
    /// 键盘导航: 当前选中的列索引
    #[doc(hidden)]
    pub nav_column_index: usize,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            format: ExportFormat::Csv,
            selected_columns: Vec::new(),
            row_limit: 0,
            start_row: 0,
            csv_delimiter: ',',
            csv_include_header: true,
            csv_quote_char: '"',
            sql_use_transaction: true,
            sql_batch_size: 100,
            json_pretty: true,
            nav_column_index: 0,
        }
    }
}

impl ExportConfig {
    /// 初始化列选择（全选）
    pub fn init_columns(&mut self, column_count: usize) {
        if self.selected_columns.len() != column_count {
            self.selected_columns = vec![true; column_count];
        }
    }

    /// 获取选中的列索引
    pub fn get_selected_column_indices(&self) -> Vec<usize> {
        self.selected_columns
            .iter()
            .enumerate()
            .filter(|&(_, selected)| *selected)
            .map(|(i, _)| i)
            .collect()
    }

    /// 是否全选
    pub fn all_columns_selected(&self) -> bool {
        self.selected_columns.iter().all(|s| *s)
    }

    /// 选中的列数
    pub fn selected_column_count(&self) -> usize {
        self.selected_columns.iter().filter(|&&s| s).count()
    }

    /// 转换为统一传输会话，缺失的列选择按“选中”处理，避免旧配置静默丢列。
    pub fn to_transfer_session(
        &self,
        result: &QueryResult,
        table_name: &str,
        db_type: DatabaseType,
    ) -> TransferSession {
        let selected_columns: Vec<usize> = if self.selected_columns.is_empty() {
            (0..result.columns.len()).collect()
        } else {
            (0..result.columns.len())
                .filter(|&idx| self.selected_columns.get(idx).copied().unwrap_or(true))
                .collect()
        };

        TransferSession {
            direction: TransferDirection::Export,
            format: self.format.into(),
            schema: TransferSchema::from_columns(
                Some(table_name.to_string()),
                Some(table_name.to_string()),
                &result.columns,
                Some(result.rows.len()),
            ),
            mapping: TransferMapping::from_selection(&result.columns, &selected_columns),
            row_window: TransferRowWindow {
                start_row: self.start_row,
                row_limit: self.row_limit,
                preview_rows: 10,
            },
            options: match self.format {
                ExportFormat::Csv | ExportFormat::Tsv => {
                    TransferFormatOptions::Delimited(TransferDelimitedOptions {
                        delimiter: self.csv_delimiter,
                        quote_char: self.csv_quote_char,
                        include_header: self.csv_include_header,
                        ..Default::default()
                    })
                }
                ExportFormat::Sql => TransferFormatOptions::Sql(TransferSqlOptions {
                    use_transaction: self.sql_use_transaction,
                    batch_size: self.sql_batch_size,
                    dialect: SqlDialect::from(db_type),
                    ..Default::default()
                }),
                ExportFormat::Json => TransferFormatOptions::Json(TransferJsonOptions {
                    pretty: self.json_pretty,
                    ..Default::default()
                }),
            },
        }
    }
}

pub struct ExportDialog;

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExportKeyAction {
    Close,
    Confirm,
    SetFormat(ExportFormat),
    CycleFormatPrev,
    CycleFormatNext,
    SelectPreviousColumn,
    SelectNextColumn,
    SelectFirstColumn,
    SelectLastColumn,
    ToggleCurrentColumn,
    ToggleAllColumns,
}

const CMD_DIALOG_DISMISS: &str = "dialog.common.dismiss";
const CMD_DIALOG_CONFIRM: &str = "dialog.common.confirm";
const CMD_EXPORT_FORMAT_CSV: &str = "dialog.export.format_csv";
const CMD_EXPORT_FORMAT_TSV: &str = "dialog.export.format_tsv";
const CMD_EXPORT_FORMAT_SQL: &str = "dialog.export.format_sql";
const CMD_EXPORT_FORMAT_JSON: &str = "dialog.export.format_json";
const CMD_EXPORT_CYCLE_PREV: &str = "dialog.export.cycle_prev";
const CMD_EXPORT_CYCLE_NEXT: &str = "dialog.export.cycle_next";
const CMD_EXPORT_COLUMN_PREV: &str = "dialog.export.column_prev";
const CMD_EXPORT_COLUMN_NEXT: &str = "dialog.export.column_next";
const CMD_EXPORT_COLUMN_START: &str = "dialog.export.column_start";
const CMD_EXPORT_COLUMN_END: &str = "dialog.export.column_end";
const CMD_EXPORT_COLUMN_TOGGLE: &str = "dialog.export.column_toggle";
const CMD_EXPORT_COLUMNS_TOGGLE_ALL: &str = "dialog.export.columns_toggle_all";

impl ExportDialog {
    fn set_format(config: &mut ExportConfig, format: ExportFormat) {
        config.format = format;
        if format == ExportFormat::Tsv {
            config.csv_delimiter = '\t';
        } else if config.csv_delimiter == '\t' {
            config.csv_delimiter = ',';
        }
    }

    fn previous_format(format: ExportFormat) -> ExportFormat {
        match format {
            ExportFormat::Csv => ExportFormat::Json,
            ExportFormat::Tsv => ExportFormat::Csv,
            ExportFormat::Sql => ExportFormat::Tsv,
            ExportFormat::Json => ExportFormat::Sql,
        }
    }

    fn next_format(format: ExportFormat) -> ExportFormat {
        match format {
            ExportFormat::Csv => ExportFormat::Tsv,
            ExportFormat::Tsv => ExportFormat::Sql,
            ExportFormat::Sql => ExportFormat::Json,
            ExportFormat::Json => ExportFormat::Csv,
        }
    }

    fn detect_key_action(
        ctx: &egui::Context,
        has_columns: bool,
        can_export: bool,
    ) -> Option<ExportKeyAction> {
        let shortcuts = DialogShortcutContext::new(ctx);

        if shortcuts.consume_command(CMD_DIALOG_DISMISS) {
            return Some(ExportKeyAction::Close);
        }
        if can_export && shortcuts.consume_command(CMD_DIALOG_CONFIRM) {
            return Some(ExportKeyAction::Confirm);
        }

        if let Some(action) = shortcuts.resolve_commands(&[
            (
                CMD_EXPORT_FORMAT_CSV,
                ExportKeyAction::SetFormat(ExportFormat::Csv),
            ),
            (
                CMD_EXPORT_FORMAT_TSV,
                ExportKeyAction::SetFormat(ExportFormat::Tsv),
            ),
            (
                CMD_EXPORT_FORMAT_SQL,
                ExportKeyAction::SetFormat(ExportFormat::Sql),
            ),
            (
                CMD_EXPORT_FORMAT_JSON,
                ExportKeyAction::SetFormat(ExportFormat::Json),
            ),
            (CMD_EXPORT_CYCLE_PREV, ExportKeyAction::CycleFormatPrev),
            (CMD_EXPORT_CYCLE_NEXT, ExportKeyAction::CycleFormatNext),
        ]) {
            return Some(action);
        }

        if has_columns {
            return shortcuts.resolve_commands(&[
                (
                    CMD_EXPORT_COLUMN_PREV,
                    ExportKeyAction::SelectPreviousColumn,
                ),
                (CMD_EXPORT_COLUMN_NEXT, ExportKeyAction::SelectNextColumn),
                (CMD_EXPORT_COLUMN_START, ExportKeyAction::SelectFirstColumn),
                (CMD_EXPORT_COLUMN_END, ExportKeyAction::SelectLastColumn),
                (
                    CMD_EXPORT_COLUMN_TOGGLE,
                    ExportKeyAction::ToggleCurrentColumn,
                ),
                (
                    CMD_EXPORT_COLUMNS_TOGGLE_ALL,
                    ExportKeyAction::ToggleAllColumns,
                ),
            ]);
        }

        None
    }

    #[allow(clippy::too_many_arguments)]
    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        config: &mut ExportConfig,
        table_name: &str,
        data: Option<&QueryResult>,
        db_type: DatabaseType,
        on_export: &mut Option<ExportConfig>,
        status_message: &Option<Result<String, String>>,
    ) {
        if !*show {
            return;
        }

        if let Some(result) = data {
            config.init_columns(result.columns.len());
        }

        let row_count = data.map(|d| d.rows.len()).unwrap_or(0);
        let col_count = data.map(|d| d.columns.len()).unwrap_or(0);
        let can_export = config.selected_column_count() > 0 && row_count > 0;

        if let Some(key_action) = Self::detect_key_action(ctx, col_count > 0, can_export) {
            match key_action {
                ExportKeyAction::Close => {
                    *show = false;
                    return;
                }
                ExportKeyAction::Confirm => {
                    *on_export = Some(config.clone());
                    return;
                }
                ExportKeyAction::SetFormat(format) => Self::set_format(config, format),
                ExportKeyAction::CycleFormatPrev => {
                    Self::set_format(config, Self::previous_format(config.format));
                }
                ExportKeyAction::CycleFormatNext => {
                    Self::set_format(config, Self::next_format(config.format));
                }
                ExportKeyAction::SelectPreviousColumn => {
                    if col_count > 0 {
                        config.nav_column_index = config.nav_column_index.saturating_sub(1);
                    }
                }
                ExportKeyAction::SelectNextColumn => {
                    if col_count > 0 {
                        config.nav_column_index = (config.nav_column_index + 1).min(col_count - 1);
                    }
                }
                ExportKeyAction::SelectFirstColumn => {
                    config.nav_column_index = 0;
                }
                ExportKeyAction::SelectLastColumn => {
                    if col_count > 0 {
                        config.nav_column_index = col_count.saturating_sub(1);
                    }
                }
                ExportKeyAction::ToggleCurrentColumn => {
                    if let Some(selected) = config.selected_columns.get_mut(config.nav_column_index)
                    {
                        *selected = !*selected;
                    }
                }
                ExportKeyAction::ToggleAllColumns => {
                    let all_selected = config.all_columns_selected();
                    for s in &mut config.selected_columns {
                        *s = !all_selected;
                    }
                }
            }
        }

        let style = DialogStyle::MEDIUM;
        DialogWindow::standard(ctx, "📤 导出数据", &style).show(ctx, |ui| {
            ui.add_space(SPACING_SM);

            DialogContent::toolbar(ui, |ui| {
                Self::show_info_bar(ui, table_name, row_count, col_count, config);
                ui.add_space(SPACING_SM);
                DialogContent::shortcut_hint(
                    ui,
                    &[
                        (local_shortcut_text(LocalShortcut::Dismiss).as_str(), "关闭"),
                        (local_shortcut_text(LocalShortcut::Confirm).as_str(), "导出"),
                        (
                            local_shortcuts_text(&[
                                LocalShortcut::ExportCyclePrev,
                                LocalShortcut::ExportCycleNext,
                            ])
                            .as_str(),
                            "切换格式",
                        ),
                    ],
                );
            });

            DialogContent::section_with_description(
                ui,
                "导出格式",
                "格式切换、快捷键提示和真实导出逻辑保持一致。",
                |ui| Self::show_format_selector(ui, config),
            );

            ScrollArea::vertical()
                .id_salt("export_main_scroll")
                .max_height(DialogContent::adaptive_height(ui, 0.74, 220.0, 430.0))
                .show(ui, |ui| {
                    DialogContent::section_with_description(
                        ui,
                        "导出范围",
                        "控制起始行与数量，0 表示导出全部。",
                        |ui| Self::show_row_range(ui, config, row_count),
                    );

                    if let Some(result) = data {
                        DialogContent::section_with_description(
                            ui,
                            "列选择",
                            "保留导航高亮，避免列很多时失去上下文。",
                            |ui| Self::show_column_selector(ui, config, &result.columns),
                        );
                    }

                    DialogContent::section_with_description(
                        ui,
                        "格式选项",
                        Self::format_options_description(config.format),
                        |ui| Self::show_format_options(ui, config),
                    );

                    if let Some(result) = data {
                        DialogContent::section_with_description(
                            ui,
                            "导出预览",
                            "预览与实际导出共用核心渲染，不再出现样式和方言偏差。",
                            |ui| Self::show_preview(ui, config, result, table_name, db_type),
                        );
                    }
                });

            if let Some(result) = status_message {
                Self::show_status_message(ui, result);
                ui.add_space(SPACING_SM);
            }

            if let Some(reason) = Self::disabled_reason(config, row_count) {
                DialogContent::warning_text(ui, reason);
                ui.add_space(SPACING_SM);
            }

            let footer = DialogFooter::show(
                ui,
                &format!(
                    "导出 {} [{}]",
                    config.format.display_name(),
                    local_shortcut_text(LocalShortcut::Confirm)
                ),
                &format!("取消 [{}]", local_shortcut_text(LocalShortcut::Dismiss)),
                can_export,
                &style,
            );

            if footer.confirmed {
                *on_export = Some(config.clone());
            }
            if footer.cancelled {
                *show = false;
            }
        });
    }

    /// 信息栏（紧凑版）
    fn show_info_bar(
        ui: &mut egui::Ui,
        table_name: &str,
        row_count: usize,
        col_count: usize,
        config: &ExportConfig,
    ) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.label(RichText::new("表").small().color(GRAY));
            ui.label(RichText::new(table_name).strong());

            let selected_cols = config.selected_column_count();
            let export_rows = if config.row_limit > 0 {
                config
                    .row_limit
                    .min(row_count.saturating_sub(config.start_row))
            } else {
                row_count.saturating_sub(config.start_row)
            };

            ui.label(
                RichText::new(format!("{} 列 × {} 行", selected_cols, export_rows))
                    .small()
                    .color(MUTED),
            );
            ui.label(
                RichText::new(format!("共 {} 列 × {} 行", col_count, row_count))
                    .small()
                    .color(MUTED),
            );
            ui.label(
                RichText::new(format!("当前格式: {}", config.format.display_name()))
                    .small()
                    .color(MUTED),
            );
        });
    }

    /// 格式选择器（紧凑版）
    fn show_format_selector(ui: &mut egui::Ui, config: &mut ExportConfig) {
        let format_cycle_text = local_shortcuts_text(&[
            LocalShortcut::ExportCyclePrev,
            LocalShortcut::ExportCycleNext,
        ]);

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);
            ui.label(RichText::new("格式:").color(GRAY));

            for (fmt, icon, name, shortcut) in [
                (
                    ExportFormat::Csv,
                    "📊",
                    "CSV",
                    LocalShortcut::ExportFormatCsv,
                ),
                (
                    ExportFormat::Tsv,
                    "↹",
                    "TSV",
                    LocalShortcut::ExportFormatTsv,
                ),
                (
                    ExportFormat::Sql,
                    "📝",
                    "SQL",
                    LocalShortcut::ExportFormatSql,
                ),
                (
                    ExportFormat::Json,
                    "🔧",
                    "JSON",
                    LocalShortcut::ExportFormatJson,
                ),
            ]
            .iter()
            {
                let is_selected = config.format == *fmt;
                let text = format!("{} {} [{}]", icon, name, local_shortcut_text(*shortcut));
                let clicked = ui
                    .push_id(format!("export_format::{fmt:?}"), |ui| {
                        ui.selectable_label(is_selected, RichText::new(&text).strong())
                            .on_hover_text(local_shortcuts_tooltip(
                                &format!("切换到 {} 导出", name),
                                &[
                                    *shortcut,
                                    LocalShortcut::ExportCyclePrev,
                                    LocalShortcut::ExportCycleNext,
                                ],
                            ))
                            .clicked()
                    })
                    .inner;

                if clicked {
                    Self::set_format(config, *fmt);
                }
            }

            ui.label(
                RichText::new(format!("{} 切换", format_cycle_text))
                    .small()
                    .color(GRAY),
            );
        });
    }

    /// 导出范围
    fn show_row_range(ui: &mut egui::Ui, config: &mut ExportConfig, total_rows: usize) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(8.0, 6.0);
            ui.label(RichText::new("行数:").color(GRAY));

            for (label, limit) in [("全部", 0), ("100", 100), ("1000", 1000)] {
                if ui
                    .selectable_label(config.row_limit == limit && config.start_row == 0, label)
                    .clicked()
                {
                    config.row_limit = limit;
                    config.start_row = 0;
                }
            }

            ui.label(RichText::new("自定义:").small().color(GRAY));
            let mut limit_str = if config.row_limit == 0 {
                String::new()
            } else {
                config.row_limit.to_string()
            };
            if ui
                .add(
                    TextEdit::singleline(&mut limit_str)
                        .id_salt("export_row_limit")
                        .desired_width(60.0)
                        .hint_text("全部"),
                )
                .changed()
            {
                config.row_limit = limit_str.parse().unwrap_or(0);
            }

            ui.label(
                RichText::new(format!("/ {} 行", total_rows))
                    .small()
                    .color(MUTED),
            );
        });
    }

    /// 列选择器（折叠面板）
    fn show_column_selector(ui: &mut egui::Ui, config: &mut ExportConfig, columns: &[String]) {
        let navigation_text = local_shortcuts_text(&[
            LocalShortcut::ExportColumnPrev,
            LocalShortcut::ExportColumnNext,
        ]);
        let range_text = local_shortcuts_text(&[
            LocalShortcut::ExportColumnStart,
            LocalShortcut::ExportColumnEnd,
        ]);
        let toggle_text = local_shortcut_text(LocalShortcut::ExportColumnToggle);
        let toggle_all_text = local_shortcut_text(LocalShortcut::ExportColumnsToggleAll);

        ui.horizontal_wrapped(|ui| {
            let all_selected = config.all_columns_selected();
            ui.label(
                RichText::new(format!(
                    "已选 {}/{} 列",
                    config.selected_column_count(),
                    columns.len()
                ))
                .small()
                .color(MUTED),
            );

            if ui
                .button(if all_selected {
                    format!("取消全选 [{}]", toggle_all_text)
                } else {
                    format!("全选 [{}]", toggle_all_text)
                })
                .on_hover_text(local_shortcut_tooltip(
                    "切换全部列的选择状态",
                    LocalShortcut::ExportColumnsToggleAll,
                ))
                .clicked()
            {
                let new_state = !all_selected;
                for selected in &mut config.selected_columns {
                    *selected = new_state;
                }
            }

            ui.label(
                RichText::new(format!(
                    "{} 导航 · {} 跳首末 · {} 切换",
                    navigation_text, range_text, toggle_text
                ))
                .small()
                .color(GRAY),
            );
        });

        ui.add_space(4.0);

        DialogContent::card(ui, None, |ui| {
            ScrollArea::vertical()
                .id_salt("export_columns_scroll")
                .max_height(DialogContent::adaptive_height(ui, 0.5, 120.0, 220.0))
                .show(ui, |ui| {
                    for (i, col_name) in columns.iter().enumerate() {
                        if i >= config.selected_columns.len() {
                            continue;
                        }

                        let is_nav_selected = i == config.nav_column_index;
                        let display_name = if col_name.len() > 28 {
                            format!("{}…", &col_name[..26])
                        } else {
                            col_name.clone()
                        };
                        let bg_color = if is_nav_selected {
                            Color32::from_rgba_unmultiplied(100, 150, 255, 60)
                        } else {
                            Color32::TRANSPARENT
                        };

                        ui.push_id(format!("export_column::{i}"), |ui| {
                            egui::Frame::NONE
                                .fill(bg_color)
                                .corner_radius(CornerRadius::same(4))
                                .inner_margin(egui::Margin::symmetric(6, 3))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        if is_nav_selected {
                                            ui.label(
                                                RichText::new("▶")
                                                    .small()
                                                    .color(Color32::from_rgb(100, 180, 255)),
                                            );
                                        } else {
                                            ui.label(RichText::new(" ").small());
                                        }

                                        if ui
                                            .checkbox(
                                                &mut config.selected_columns[i],
                                                &display_name,
                                            )
                                            .clicked()
                                        {
                                            config.nav_column_index = i;
                                        }
                                    });
                                });
                        });
                    }
                });
        });
    }

    /// 格式特定选项（折叠面板）
    fn show_format_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        match config.format {
            ExportFormat::Csv => Self::show_csv_options(ui, config),
            ExportFormat::Tsv => Self::show_tsv_options(ui, config),
            ExportFormat::Sql => Self::show_sql_options(ui, config),
            ExportFormat::Json => Self::show_json_options(ui, config),
        }
    }

    /// CSV 选项
    fn show_csv_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("分隔符:").small().color(GRAY));
            for (label, delim) in [(",", ','), (";", ';'), ("Tab", '\t'), ("|", '|')] {
                if ui
                    .selectable_label(config.csv_delimiter == delim, label)
                    .clicked()
                {
                    config.csv_delimiter = delim;
                }
            }
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut config.csv_include_header, "包含表头");
        });
    }

    /// TSV 选项
    fn show_tsv_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        config.csv_delimiter = '\t';

        ui.horizontal(|ui| {
            ui.label(RichText::new("分隔符:").small().color(GRAY));
            ui.label(RichText::new("Tab").strong());
            ui.label(RichText::new("(TSV 固定为制表符)").small().color(MUTED));
        });

        ui.horizontal(|ui| {
            ui.checkbox(&mut config.csv_include_header, "包含表头");
        });
    }

    /// SQL 选项
    fn show_sql_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut config.sql_use_transaction, "事务包装");

            ui.separator();

            ui.label(RichText::new("批量:").small().color(GRAY));
            for (label, size) in [("单行", 0), ("100", 100), ("500", 500)] {
                if ui
                    .selectable_label(config.sql_batch_size == size, label)
                    .clicked()
                {
                    config.sql_batch_size = size;
                }
            }
        });
    }

    /// JSON 选项
    fn show_json_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        ui.horizontal(|ui| {
            ui.checkbox(&mut config.json_pretty, "美化输出");
            if config.json_pretty {
                ui.label(RichText::new("(带缩进)").small().color(MUTED));
            } else {
                ui.label(RichText::new("(紧凑)").small().color(MUTED));
            }
        });
    }

    /// 导出预览（折叠面板）
    fn show_preview(
        ui: &mut egui::Ui,
        config: &ExportConfig,
        data: &QueryResult,
        table_name: &str,
        db_type: DatabaseType,
    ) {
        let mut session = config.to_transfer_session(data, table_name, db_type);
        session.row_window.preview_rows = 3;
        let preview_text = preview_export_transfer(data, &session)
            .ok()
            .and_then(|preview| preview.rendered_text)
            .unwrap_or_else(|| "（预览失败）".to_string());

        DialogContent::code_block_with_id(
            ui,
            "export_preview_scroll",
            &preview_text,
            DialogContent::adaptive_height(ui, 0.45, 120.0, 220.0),
        );
    }

    /// 状态消息
    fn show_status_message(ui: &mut egui::Ui, result: &Result<String, String>) {
        DialogStatus::show(ui, result);
    }

    fn format_options_description(format: ExportFormat) -> &'static str {
        match format {
            ExportFormat::Csv => "控制分隔符和表头输出。",
            ExportFormat::Tsv => "TSV 固定使用制表符，仍可控制表头输出。",
            ExportFormat::Sql => "控制事务包裹和批量插入策略。",
            ExportFormat::Json => "控制 JSON 是否美化输出。",
        }
    }

    fn disabled_reason(config: &ExportConfig, row_count: usize) -> Option<&'static str> {
        if row_count == 0 {
            Some("当前结果集没有可导出的行。")
        } else if config.selected_column_count() == 0 {
            Some("请至少选择一列再执行导出。")
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExportDialog, ExportKeyAction};
    use crate::core::ExportFormat;
    use egui::{Event, Key, Modifiers, RawInput};

    fn key_event(key: Key) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }
    }

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        ctx.begin_pass(RawInput {
            events: vec![key_event(key)],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
    }

    fn focus_text_input(ctx: &egui::Context) {
        let mut text = String::new();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("export dialog shortcut test input").show(ctx, |ui| {
            let response =
                ui.add(egui::TextEdit::singleline(&mut text).id_salt("export_shortcut_text_input"));
            response.request_focus();
        });
        let _ = ctx.end_pass();
    }

    #[test]
    fn export_dialog_detects_format_shortcut_through_scoped_command_id() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Num4);

        let action = ExportDialog::detect_key_action(&ctx, false, false);

        assert_eq!(action, Some(ExportKeyAction::SetFormat(ExportFormat::Json)));

        let _ = ctx.end_pass();
    }

    #[test]
    fn export_dialog_confirm_requires_exportable_selection() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Enter);

        let action = ExportDialog::detect_key_action(&ctx, true, false);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn export_dialog_blocks_column_text_conflicts_when_text_input_is_focused() {
        let ctx = egui::Context::default();
        focus_text_input(&ctx);
        begin_key_pass(&ctx, Key::J);

        let action = ExportDialog::detect_key_action(&ctx, true, true);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }
}
