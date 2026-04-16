//! 数据导入对话框 - 支持 SQL/CSV/TSV/JSON 格式，提供预览和直接执行功能
//!
//! 支持的快捷键：
//! - `Esc` - 关闭对话框
//! - `Enter` - 执行导入/复制到编辑器
//! - `1/2/3/4` - 快速选择格式 (SQL/CSV/TSV/JSON)
//! - `h/l` - 切换格式
//! - `Ctrl+R` - 刷新预览

mod import_parser;
mod import_types;

pub use import_parser::parse_sql_file;
pub use import_types::*;

use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStatus, DialogStyle, DialogWindow,
    FormDialogShell,
};
use crate::ui::styles::{DANGER, GRAY, MUTED, SPACING_SM};
use crate::ui::{LocalShortcut, local_shortcut_text, local_shortcut_tooltip, local_shortcuts_text};
use egui::{self, Color32, RichText, TextEdit};
use std::cell::RefCell;

pub struct ImportDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImportKeyAction {
    Close,
    Confirm,
    RefreshPreview,
    SetFormat(ImportFormat),
    CycleFormatPrev,
    CycleFormatNext,
}

const CMD_DIALOG_DISMISS: &str = "dialog.common.dismiss";
const CMD_DIALOG_CONFIRM: &str = "dialog.common.confirm";
const CMD_IMPORT_REFRESH: &str = "dialog.import.refresh";
const CMD_IMPORT_FORMAT_SQL: &str = "dialog.import.format_sql";
const CMD_IMPORT_FORMAT_CSV: &str = "dialog.import.format_csv";
const CMD_IMPORT_FORMAT_TSV: &str = "dialog.import.format_tsv";
const CMD_IMPORT_FORMAT_JSON: &str = "dialog.import.format_json";
const CMD_IMPORT_CYCLE_PREV: &str = "dialog.import.cycle_prev";
const CMD_IMPORT_CYCLE_NEXT: &str = "dialog.import.cycle_next";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponsiveRowClass {
    Wide,
    Medium,
    Narrow,
}

struct ImportFooterState<'a> {
    effective_mode: ImportMode,
    has_file: bool,
    can_confirm: bool,
    disabled_reason: Option<&'a str>,
    copy_sql: Option<&'a str>,
}

impl ImportDialog {
    const WIDE_ROW_THRESHOLD: f32 = 720.0;
    const MEDIUM_ROW_THRESHOLD: f32 = 560.0;

    #[inline]
    fn effective_mode_for_format(state: &ImportState) -> ImportMode {
        state.mode
    }

    #[inline]
    fn set_format(state: &mut ImportState, format: ImportFormat) {
        state.format = format;
        state.preview = None;
        state.error = None;
        if let Some(delimiter) = format.default_delimiter() {
            state.csv_config.delimiter = delimiter;
        } else if state.csv_config.delimiter == '\t' {
            state.csv_config.delimiter = ',';
        }
    }

    fn detect_key_action(
        ctx: &egui::Context,
        has_file: bool,
        can_import: bool,
    ) -> Option<ImportKeyAction> {
        let shortcuts = DialogShortcutContext::new(ctx);

        if shortcuts.consume_command(CMD_DIALOG_DISMISS) {
            return Some(ImportKeyAction::Close);
        }
        if can_import && shortcuts.consume_command(CMD_DIALOG_CONFIRM) {
            return Some(ImportKeyAction::Confirm);
        }

        if let Some(action) = shortcuts.resolve_commands(&[
            (
                CMD_IMPORT_FORMAT_SQL,
                ImportKeyAction::SetFormat(ImportFormat::Sql),
            ),
            (
                CMD_IMPORT_FORMAT_CSV,
                ImportKeyAction::SetFormat(ImportFormat::Csv),
            ),
            (
                CMD_IMPORT_FORMAT_TSV,
                ImportKeyAction::SetFormat(ImportFormat::Tsv),
            ),
            (
                CMD_IMPORT_FORMAT_JSON,
                ImportKeyAction::SetFormat(ImportFormat::Json),
            ),
            (CMD_IMPORT_CYCLE_PREV, ImportKeyAction::CycleFormatPrev),
            (CMD_IMPORT_CYCLE_NEXT, ImportKeyAction::CycleFormatNext),
        ]) {
            return Some(action);
        }

        if has_file && shortcuts.consume_command(CMD_IMPORT_REFRESH) {
            return Some(ImportKeyAction::RefreshPreview);
        }

        None
    }

    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        state: &mut ImportState,
        is_mysql: bool,
    ) -> ImportAction {
        if !*show {
            return ImportAction::None;
        }

        let has_file = state.file_path.is_some();
        let has_preview = state.preview.is_some();
        let can_import = has_file && has_preview && state.error.is_none();
        let mut initial_action = ImportAction::None;

        if let Some(key_action) = Self::detect_key_action(ctx, has_file, can_import) {
            match key_action {
                ImportKeyAction::Close => {
                    *show = false;
                    return ImportAction::Close;
                }
                ImportKeyAction::Confirm => {
                    return match Self::effective_mode_for_format(state) {
                        ImportMode::Execute => ImportAction::Execute,
                        ImportMode::CopyToEditor => {
                            if let Some(ref preview) = state.preview {
                                let sql = preview.sql_statements.join("\n\n");
                                ImportAction::CopyToEditor(sql)
                            } else {
                                ImportAction::None
                            }
                        }
                    };
                }
                ImportKeyAction::RefreshPreview => initial_action = ImportAction::RefreshPreview,
                ImportKeyAction::SetFormat(format) => {
                    Self::set_format(state, format);
                }
                ImportKeyAction::CycleFormatPrev => {
                    Self::set_format(state, state.format.previous());
                }
                ImportKeyAction::CycleFormatNext => {
                    Self::set_format(state, state.format.next());
                }
            }
        }

        let style = DialogStyle::LARGE;
        let effective_mode = Self::effective_mode_for_format(state);
        let footer_has_file = state.file_path.is_some();
        let footer_can_confirm =
            footer_has_file && state.preview.is_some() && state.error.is_none();
        let footer_disabled_reason = Self::disabled_reason(state);
        let footer_copy_sql = state
            .preview
            .as_ref()
            .map(|preview| preview.sql_statements.join("\n\n"));
        let footer_state = ImportFooterState {
            effective_mode,
            has_file: footer_has_file,
            can_confirm: footer_can_confirm,
            disabled_reason: footer_disabled_reason,
            copy_sql: footer_copy_sql.as_deref(),
        };
        let pending_action = RefCell::new(initial_action);

        DialogWindow::resizable(ctx, "📥 导入数据", &style).show(ctx, |ui| {
            FormDialogShell::show(
                ui,
                "import_dialog_form_shell",
                |ui| {
                    DialogContent::shortcut_hint(
                        ui,
                        &[
                            (local_shortcut_text(LocalShortcut::Dismiss).as_str(), "关闭"),
                            (
                                local_shortcut_text(LocalShortcut::Confirm).as_str(),
                                "执行 / 复制",
                            ),
                            (
                                local_shortcuts_text(&[
                                    LocalShortcut::ImportCyclePrev,
                                    LocalShortcut::ImportCycleNext,
                                ])
                                .as_str(),
                                "切换格式",
                            ),
                            (
                                local_shortcut_text(LocalShortcut::ImportRefresh).as_str(),
                                "刷新预览",
                            ),
                        ],
                    );
                },
                |ui, _body_ctx| {
                    DialogContent::section_with_description(
                        ui,
                        "导入源",
                        "选择本地文件后会自动推断格式，并将文件路径与元数据固定在顶部。",
                        |ui| {
                            Self::store_action(
                                &pending_action,
                                Self::show_file_selector(ui, state),
                            );
                        },
                    );

                    if state.file_path.is_some() {
                        DialogContent::section_with_description(
                            ui,
                            "导入策略",
                            "格式切换、执行模式和预览共享同一份配置，避免两边状态漂移。",
                            |ui| Self::show_format_mode_selector(ui, state),
                        );

                        DialogContent::section_with_description(
                            ui,
                            Self::options_title(state.format),
                            Self::options_description(state.format),
                            |ui| match state.format {
                                ImportFormat::Sql => Self::show_sql_options(ui, state),
                                ImportFormat::Csv | ImportFormat::Tsv => {
                                    Self::show_csv_options(ui, state, is_mysql)
                                }
                                ImportFormat::Json => Self::show_json_options(ui, state, is_mysql),
                            },
                        );

                        DialogContent::section_with_description(
                            ui,
                            "导入预览",
                            "加载中、错误、无预览和成功预览都在同一个面板里处理。",
                            |ui| {
                                Self::store_action(
                                    &pending_action,
                                    Self::show_preview_panel(ui, state),
                                );
                            },
                        );
                    } else {
                        DialogContent::info_text(
                            ui,
                            "选择文件后会显示格式选项、预览结果和执行动作。",
                        );
                        ui.add_space(SPACING_SM);
                    }
                },
                |ui| {
                    Self::store_action(
                        &pending_action,
                        Self::show_buttons(ui, show, &footer_state, &style),
                    );
                },
            );
        });

        pending_action.into_inner()
    }

    /// 文件选择区域
    fn show_file_selector(ui: &mut egui::Ui, state: &mut ImportState) -> ImportAction {
        let mut action = ImportAction::None;

        let path_display = state
            .file_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "未选择文件".to_string());

        Self::show_responsive_labeled_row(ui, "文件", |ui, row_class| match row_class {
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                ui.horizontal(|ui| {
                    let button_width = 120.0;
                    let field_width = (ui.available_width() - button_width - SPACING_SM).max(160.0);
                    let mut readonly_text = path_display.clone();
                    ui.add_sized(
                        [field_width, 0.0],
                        TextEdit::singleline(&mut readonly_text).interactive(false),
                    );

                    if ui
                        .add_sized([button_width, 0.0], egui::Button::new("📂 浏览..."))
                        .clicked()
                    {
                        action = ImportAction::SelectFile;
                    }
                });
            }
            ResponsiveRowClass::Narrow => {
                let mut readonly_text = path_display.clone();
                ui.add_sized(
                    [ui.available_width(), 0.0],
                    TextEdit::singleline(&mut readonly_text).interactive(false),
                );
                ui.add_space(4.0);
                let button_width = ui.available_width().min(140.0);
                if ui
                    .add_sized([button_width, 0.0], egui::Button::new("📂 浏览..."))
                    .clicked()
                {
                    action = ImportAction::SelectFile;
                }
            }
        });

        // 显示文件信息
        if let Some(ref path) = state.file_path {
            DialogContent::toolbar(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    // 文件大小
                    if let Ok(metadata) = std::fs::metadata(path) {
                        let size = metadata.len();
                        let size_str = if size < 1024 {
                            format!("{} B", size)
                        } else if size < 1024 * 1024 {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        } else {
                            format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
                        };
                        ui.label(
                            RichText::new(format!("大小: {}", size_str))
                                .small()
                                .color(MUTED),
                        );
                    }

                    ui.separator();

                    ui.label(
                        RichText::new(format!(
                            "{} {} 格式",
                            state.format.icon(),
                            state.format.name()
                        ))
                        .small()
                        .color(MUTED),
                    );
                });
            });
        }

        action
    }

    /// 格式和模式选择
    fn show_format_mode_selector(ui: &mut egui::Ui, state: &mut ImportState) {
        Self::show_responsive_labeled_row(ui, "格式", |ui, row_class| {
            let format_choices = [
                ImportFormat::Sql,
                ImportFormat::Csv,
                ImportFormat::Tsv,
                ImportFormat::Json,
            ];

            if matches!(row_class, ResponsiveRowClass::Narrow) {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("h/l").small().color(GRAY));
                });
            }

            ui.horizontal_wrapped(|ui| {
                for (idx, fmt) in format_choices.iter().enumerate() {
                    let is_selected = state.format == *fmt;
                    let text = format!("{} {} [{}]", fmt.icon(), fmt.name(), idx + 1);
                    if ui
                        .selectable_label(is_selected, RichText::new(&text))
                        .on_hover_text(local_shortcut_tooltip(
                            &format!("切换到 {} 导入", fmt.name()),
                            LocalShortcut::FormatSelectionCycle,
                        ))
                        .clicked()
                    {
                        Self::set_format(state, *fmt);
                    }
                }

                if !matches!(row_class, ResponsiveRowClass::Narrow) {
                    ui.separator();
                    ui.label(RichText::new("h/l").small().color(GRAY));
                }
            });
        });

        Self::show_responsive_labeled_row(ui, "模式", |ui, _row_class| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .selectable_label(state.mode == ImportMode::Execute, "🚀 直接执行")
                    .on_hover_text(match state.format {
                        ImportFormat::Sql => "逐条执行 SQL 语句。按 Enter 直接导入。",
                        _ => "先转换为 INSERT 语句，再直接写入数据库。按 Enter 直接导入。",
                    })
                    .clicked()
                {
                    state.mode = ImportMode::Execute;
                }

                if ui
                    .selectable_label(state.mode == ImportMode::CopyToEditor, "📋 复制到编辑器")
                    .on_hover_text(match state.format {
                        ImportFormat::Sql => "将 SQL 复制到编辑器中。按 Enter 复制结果。",
                        _ => "先转换为 INSERT 语句，再复制到编辑器里手动检查与执行。",
                    })
                    .clicked()
                {
                    state.mode = ImportMode::CopyToEditor;
                }
            });
        });
    }

    /// SQL 选项
    fn show_sql_options(ui: &mut egui::Ui, state: &mut ImportState) {
        let mut needs_refresh = false;

        ui.horizontal_wrapped(|ui| {
            if ui
                .checkbox(&mut state.sql_config.strip_comments, "移除注释")
                .changed()
            {
                needs_refresh = true;
            }
            ui.label(RichText::new("(-- 和 /* */)").small().color(MUTED));
        });

        if ui
            .checkbox(&mut state.sql_config.strip_empty_lines, "移除空行")
            .changed()
        {
            needs_refresh = true;
        }

        if needs_refresh {
            state.preview = None;
            state.error = None;
        }

        Self::show_execute_options(ui, state);
    }

    /// CSV 选项
    fn show_csv_options(ui: &mut egui::Ui, state: &mut ImportState, _is_mysql: bool) {
        Self::show_responsive_labeled_row(ui, "目标表", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 220.0);
            ui.add_sized(
                [control_width, 0.0],
                TextEdit::singleline(&mut state.csv_config.table_name).hint_text("表名"),
            );
        });

        ui.add_space(SPACING_SM);

        if state.format == ImportFormat::Tsv {
            state.csv_config.delimiter = '\t';
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("分隔符:").color(GRAY));
                ui.label(RichText::new("Tab").strong());
                ui.label(RichText::new("(TSV 固定为制表符)").small().color(MUTED));
            });
        } else {
            Self::show_responsive_labeled_row(ui, "分隔符", |ui, _row_class| {
                for (label, delim) in [(",", ','), (";", ';'), ("Tab", '\t'), ("|", '|')] {
                    if ui
                        .selectable_label(state.csv_config.delimiter == delim, label)
                        .clicked()
                    {
                        state.csv_config.delimiter = delim;
                        state.preview = None;
                    }
                }
            });
        }

        ui.add_space(SPACING_SM);

        DialogContent::toolbar(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                if ui
                    .checkbox(&mut state.csv_config.has_header, "首行为表头")
                    .changed()
                {
                    state.preview = None;
                    state.error = None;
                }

                ui.label(RichText::new("跳过行:").color(GRAY));
                let mut skip_str = state.csv_config.skip_rows.to_string();
                if ui
                    .add(TextEdit::singleline(&mut skip_str).desired_width(50.0))
                    .changed()
                {
                    state.csv_config.skip_rows = skip_str.parse().unwrap_or(0);
                    state.preview = None;
                    state.error = None;
                }
            });
        });

        Self::show_execute_options(ui, state);
    }

    /// JSON 选项
    fn show_json_options(ui: &mut egui::Ui, state: &mut ImportState, _is_mysql: bool) {
        let mut needs_refresh = false;

        Self::show_responsive_labeled_row(ui, "目标表", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 220.0);
            ui.add_sized(
                [control_width, 0.0],
                TextEdit::singleline(&mut state.json_config.table_name).hint_text("表名"),
            );
        });

        ui.add_space(SPACING_SM);

        Self::show_responsive_labeled_row(ui, "数据路径", |ui, row_class| {
            let control_width = Self::control_width(ui, row_class, 320.0);
            if ui
                .add_sized(
                    [control_width, 0.0],
                    TextEdit::singleline(&mut state.json_config.json_path)
                        .hint_text("例如: data.items (留空表示根数组)"),
                )
                .changed()
            {
                needs_refresh = true;
            }
        });

        ui.add_space(SPACING_SM);

        if ui
            .checkbox(&mut state.json_config.flatten_nested, "展平嵌套对象")
            .changed()
        {
            needs_refresh = true;
        }

        if needs_refresh {
            state.preview = None;
            state.error = None;
        }

        Self::show_execute_options(ui, state);
    }

    /// 执行选项
    fn show_execute_options(ui: &mut egui::Ui, state: &mut ImportState) {
        if state.mode != ImportMode::Execute {
            return;
        }

        ui.add_space(SPACING_SM);
        DialogContent::toolbar(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut state.sql_config.stop_on_error, "遇到错误时停止");
                ui.checkbox(&mut state.sql_config.use_transaction, "使用事务");
                ui.label(RichText::new("(全部成功或全部回滚)").small().color(MUTED));
            });
        });
    }

    /// 预览区域
    fn show_preview(ui: &mut egui::Ui, state: &ImportState, preview: &ImportPreview) {
        let header = match state.format {
            ImportFormat::Sql => format!("预览 ({} 条 SQL 语句)", preview.statement_count),
            _ => format!(
                "预览 ({} 列 × {} 行，可生成 {} 条 INSERT)",
                preview.columns.len(),
                preview.total_rows,
                preview.statement_count
            ),
        };

        ui.label(RichText::new(header).small().color(MUTED));
        ui.add_space(SPACING_SM);

        if !preview.warnings.is_empty() {
            DialogContent::card(ui, Some(Color32::from_rgb(255, 193, 7)), |ui| {
                for warning in &preview.warnings {
                    DialogContent::warning_text(ui, warning);
                }
            });
            ui.add_space(SPACING_SM);
        }

        DialogContent::code_surface(
            ui,
            DialogContent::adaptive_height(ui, 0.5, 160.0, 260.0),
            |ui| match state.format {
                ImportFormat::Sql => Self::show_sql_preview(ui, preview),
                _ => Self::show_table_preview(ui, preview),
            },
        );
    }

    /// SQL 预览
    fn show_sql_preview(ui: &mut egui::Ui, preview: &ImportPreview) {
        for (i, stmt) in preview.sql_statements.iter().take(10).enumerate() {
            let display = if stmt.len() > 100 {
                format!("{}...", &stmt[..100])
            } else {
                stmt.clone()
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{}.", i + 1)).small().color(MUTED));
                ui.label(RichText::new(&display).small().monospace());
            });
        }

        if preview.statement_count > 10 {
            ui.label(
                RichText::new(format!("... 还有 {} 条语句", preview.statement_count - 10))
                    .small()
                    .color(MUTED),
            );
        }
    }

    /// 表格预览（CSV/TSV/JSON）
    fn show_table_preview(ui: &mut egui::Ui, preview: &ImportPreview) {
        use egui_extras::{Column, TableBuilder};

        if preview.columns.is_empty() {
            ui.label(RichText::new("无数据").color(MUTED));
            return;
        }

        let col_count = preview.columns.len();

        TableBuilder::new(ui)
            .striped(true)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .columns(Column::auto().at_least(60.0).clip(true), col_count)
            .header(20.0, |mut header| {
                for col_name in &preview.columns {
                    header.col(|ui| {
                        ui.label(RichText::new(col_name).strong().small());
                    });
                }
            })
            .body(|body| {
                body.rows(18.0, preview.preview_rows.len(), |mut row| {
                    let row_idx = row.index();
                    if let Some(row_data) = preview.preview_rows.get(row_idx) {
                        for cell in row_data {
                            row.col(|ui| {
                                let display = if cell.len() > 30 {
                                    format!("{}...", &cell[..27])
                                } else {
                                    cell.clone()
                                };
                                ui.label(RichText::new(&display).small());
                            });
                        }
                    }
                });
            });

        if preview.total_rows > preview.preview_rows.len() {
            ui.add_space(SPACING_SM);
            ui.label(
                RichText::new(format!(
                    "... 还有 {} 行数据",
                    preview.total_rows - preview.preview_rows.len()
                ))
                .small()
                .color(MUTED),
            );
        }
    }

    fn options_title(format: ImportFormat) -> &'static str {
        match format {
            ImportFormat::Sql => "SQL 解析选项",
            ImportFormat::Csv => "CSV 解析选项",
            ImportFormat::Tsv => "TSV 解析选项",
            ImportFormat::Json => "JSON 解析选项",
        }
    }

    fn options_description(format: ImportFormat) -> &'static str {
        match format {
            ImportFormat::Sql => "控制注释过滤、空行清理和执行阶段的容错方式。",
            ImportFormat::Csv => "控制目标表、分隔符、表头和跳过行数。",
            ImportFormat::Tsv => "TSV 固定使用制表符，其余流程与 CSV 对齐。",
            ImportFormat::Json => "控制目标表、JSON 路径和嵌套对象展平策略。",
        }
    }

    fn show_preview_panel(ui: &mut egui::Ui, state: &ImportState) -> ImportAction {
        if let Some(ref preview) = state.preview {
            Self::show_preview(ui, state, preview);
            return ImportAction::None;
        }

        if state.loading {
            DialogContent::toolbar(ui, |ui| {
                DialogStatus::show_loading(ui, "正在解析文件并生成预览...");
            });
            return ImportAction::None;
        }

        if let Some(ref err) = state.error {
            DialogContent::card(ui, Some(DANGER), |ui| {
                DialogContent::error_text(ui, err);
            });
            return ImportAction::None;
        }

        let mut action = ImportAction::None;
        DialogContent::toolbar(ui, |ui| {
            DialogContent::info_text(ui, "尚未生成预览，先加载一次再执行导入或复制。 ");
            ui.add_space(SPACING_SM);
            if ui
                .button(format!(
                    "🔍 加载预览 [{}]",
                    local_shortcut_text(LocalShortcut::ImportRefresh)
                ))
                .on_hover_text(local_shortcut_tooltip(
                    "加载或刷新预览",
                    LocalShortcut::ImportRefresh,
                ))
                .clicked()
            {
                action = ImportAction::RefreshPreview;
            }
        });
        action
    }

    fn disabled_reason(state: &ImportState) -> Option<&'static str> {
        if state.file_path.is_none() {
            Some("请先选择要导入的文件。")
        } else if state.preview.is_none() {
            Some("请先生成预览，再执行导入或复制。")
        } else if state.error.is_some() {
            Some("当前预览存在错误，请修正配置后再继续。")
        } else {
            None
        }
    }

    /// 底部按钮
    fn show_buttons(
        ui: &mut egui::Ui,
        show: &mut bool,
        footer_state: &ImportFooterState<'_>,
        style: &DialogStyle,
    ) -> ImportAction {
        let mut action = ImportAction::None;

        if footer_state.has_file {
            DialogContent::toolbar(ui, |ui| {
                if ui
                    .button(format!(
                        "🔄 刷新预览 [{}]",
                        local_shortcut_text(LocalShortcut::ImportRefresh)
                    ))
                    .on_hover_text(local_shortcut_tooltip(
                        "刷新导入预览",
                        LocalShortcut::ImportRefresh,
                    ))
                    .clicked()
                {
                    action = ImportAction::RefreshPreview;
                }
            });
            ui.add_space(SPACING_SM);
        }

        if let Some(reason) = footer_state.disabled_reason {
            DialogContent::warning_text(ui, reason);
            ui.add_space(SPACING_SM);
        }

        let footer = DialogFooter::show(
            ui,
            &match footer_state.effective_mode {
                ImportMode::Execute => {
                    format!("执行导入 [{}]", local_shortcut_text(LocalShortcut::Confirm))
                }
                ImportMode::CopyToEditor => {
                    format!(
                        "复制到编辑器 [{}]",
                        local_shortcut_text(LocalShortcut::Confirm)
                    )
                }
            },
            &format!("取消 [{}]", local_shortcut_text(LocalShortcut::Dismiss)),
            footer_state.can_confirm,
            style,
        );

        if footer.cancelled {
            *show = false;
            action = ImportAction::Close;
        }
        if footer.confirmed {
            match footer_state.effective_mode {
                ImportMode::Execute => action = ImportAction::Execute,
                ImportMode::CopyToEditor => {
                    if let Some(sql) = footer_state.copy_sql {
                        action = ImportAction::CopyToEditor(sql.to_string());
                    }
                }
            }
        }

        action
    }

    fn row_width_class(available_width: f32) -> ResponsiveRowClass {
        if available_width >= Self::WIDE_ROW_THRESHOLD {
            ResponsiveRowClass::Wide
        } else if available_width >= Self::MEDIUM_ROW_THRESHOLD {
            ResponsiveRowClass::Medium
        } else {
            ResponsiveRowClass::Narrow
        }
    }

    fn label_width(row_class: ResponsiveRowClass) -> f32 {
        match row_class {
            ResponsiveRowClass::Wide => 84.0,
            ResponsiveRowClass::Medium => 76.0,
            ResponsiveRowClass::Narrow => 0.0,
        }
    }

    fn control_width(ui: &egui::Ui, row_class: ResponsiveRowClass, preferred_width: f32) -> f32 {
        match row_class {
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                ui.available_width().min(preferred_width)
            }
            ResponsiveRowClass::Narrow => ui.available_width(),
        }
    }

    fn show_responsive_labeled_row(
        ui: &mut egui::Ui,
        label: &str,
        body: impl FnOnce(&mut egui::Ui, ResponsiveRowClass),
    ) {
        let row_class = Self::row_width_class(ui.available_width());

        match row_class {
            ResponsiveRowClass::Narrow => {
                ui.label(RichText::new(label).color(GRAY));
                ui.add_space(4.0);
                body(ui, row_class);
            }
            ResponsiveRowClass::Wide | ResponsiveRowClass::Medium => {
                let label_width = Self::label_width(row_class);
                ui.horizontal_top(|ui| {
                    ui.add_sized(
                        [label_width, 0.0],
                        egui::Label::new(RichText::new(label).color(GRAY)),
                    );
                    ui.add_space(SPACING_SM);
                    ui.vertical(|ui| {
                        body(ui, row_class);
                    });
                });
            }
        }

        ui.add_space(SPACING_SM);
    }

    fn store_action(slot: &RefCell<ImportAction>, next_action: ImportAction) {
        if !matches!(next_action, ImportAction::None) {
            *slot.borrow_mut() = next_action;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key, Modifiers, RawInput};

    fn key_event(key: Key, modifiers: Modifiers) -> Event {
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        begin_key_pass_with_modifiers(ctx, key, Modifiers::NONE);
    }

    fn begin_key_pass_with_modifiers(ctx: &egui::Context, key: Key, modifiers: Modifiers) {
        ctx.begin_pass(RawInput {
            events: vec![key_event(key, modifiers)],
            modifiers,
            ..Default::default()
        });
    }

    fn ctrl_modifiers() -> Modifiers {
        Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        }
    }

    fn focus_text_input(ctx: &egui::Context) {
        let mut text = String::new();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("import dialog shortcut test input").show(ctx, |ui| {
            let response =
                ui.add(egui::TextEdit::singleline(&mut text).id_salt("import_shortcut_text_input"));
            response.request_focus();
        });
        let _ = ctx.end_pass();
    }

    #[test]
    fn test_set_format_preserves_copy_mode_for_non_sql() {
        let mut state = ImportState {
            mode: ImportMode::CopyToEditor,
            preview: Some(ImportPreview::default()),
            error: Some("old error".to_string()),
            ..Default::default()
        };

        ImportDialog::set_format(&mut state, ImportFormat::Csv);

        assert_eq!(state.format, ImportFormat::Csv);
        assert_eq!(state.mode, ImportMode::CopyToEditor);
        assert!(state.preview.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_effective_mode_for_non_sql_keeps_selected_mode() {
        let state = ImportState {
            format: ImportFormat::Json,
            mode: ImportMode::CopyToEditor,
            ..Default::default()
        };

        let effective = ImportDialog::effective_mode_for_format(&state);
        assert_eq!(effective, ImportMode::CopyToEditor);
    }

    #[test]
    fn test_set_format_syncs_tsv_delimiter() {
        let mut state = ImportState::default();
        state.csv_config.delimiter = ',';

        ImportDialog::set_format(&mut state, ImportFormat::Tsv);

        assert_eq!(state.csv_config.delimiter, '\t');
    }

    #[test]
    fn import_dialog_detects_format_shortcut_through_scoped_command_id() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::Num3);

        let action = ImportDialog::detect_key_action(&ctx, false, false);

        assert_eq!(action, Some(ImportKeyAction::SetFormat(ImportFormat::Tsv)));

        let _ = ctx.end_pass();
    }

    #[test]
    fn import_dialog_refresh_requires_selected_file() {
        let ctx = egui::Context::default();
        begin_key_pass_with_modifiers(&ctx, Key::R, ctrl_modifiers());

        let action = ImportDialog::detect_key_action(&ctx, false, false);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn import_dialog_refresh_consumes_scoped_command_when_file_exists() {
        let ctx = egui::Context::default();
        begin_key_pass_with_modifiers(&ctx, Key::R, ctrl_modifiers());

        let action = ImportDialog::detect_key_action(&ctx, true, false);

        assert_eq!(action, Some(ImportKeyAction::RefreshPreview));

        let _ = ctx.end_pass();
    }

    #[test]
    fn import_dialog_blocks_cycle_text_conflicts_when_text_input_is_focused() {
        let ctx = egui::Context::default();
        focus_text_input(&ctx);
        begin_key_pass(&ctx, Key::H);

        let action = ImportDialog::detect_key_action(&ctx, true, true);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn import_dialog_row_width_classes_follow_shared_thresholds() {
        assert_eq!(
            ImportDialog::row_width_class(ImportDialog::WIDE_ROW_THRESHOLD),
            ResponsiveRowClass::Wide
        );
        assert_eq!(
            ImportDialog::row_width_class(680.0),
            ResponsiveRowClass::Medium
        );
        assert_eq!(
            ImportDialog::row_width_class(ImportDialog::MEDIUM_ROW_THRESHOLD - 1.0),
            ResponsiveRowClass::Narrow
        );
    }
}
