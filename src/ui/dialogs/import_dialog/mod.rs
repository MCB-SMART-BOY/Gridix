//! 数据导入对话框 - 支持 SQL/CSV/JSON 格式，提供预览和直接执行功能
//!
//! 支持的快捷键：
//! - `Esc` - 关闭对话框
//! - `Enter` - 执行导入/复制到编辑器
//! - `1/2/3` - 快速选择格式 (SQL/CSV/JSON)
//! - `h/l` - 切换格式
//! - `Ctrl+R` - 刷新预览

mod import_parser;
mod import_types;

pub use import_parser::parse_sql_file;
pub use import_types::*;

use super::keyboard;
use crate::ui::styles::{DANGER, GRAY, MUTED, SPACING_SM};
use egui::{self, Color32, CornerRadius, Key, RichText, ScrollArea, TextEdit, Vec2};

pub struct ImportDialog;

impl ImportDialog {
    #[inline]
    fn effective_mode_for_format(state: &ImportState) -> ImportMode {
        if state.format == ImportFormat::Sql {
            state.mode
        } else {
            ImportMode::Execute
        }
    }

    #[inline]
    fn set_format(state: &mut ImportState, format: ImportFormat) {
        state.format = format;
        state.preview = None;
        state.error = None;
        if format != ImportFormat::Sql {
            state.mode = ImportMode::Execute;
        }
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

        let mut action = ImportAction::None;
        let has_file = state.file_path.is_some();
        let has_preview = state.preview.is_some();
        let can_import = has_file && has_preview && state.error.is_none();

        // 处理键盘快捷键（仅当没有文本输入焦点时）
        if !keyboard::has_text_focus(ctx) {
            // Esc 关闭
            if keyboard::handle_close_keys(ctx) {
                *show = false;
                return ImportAction::Close;
            }

            // Enter 执行导入
            if can_import && let keyboard::DialogAction::Confirm = keyboard::handle_dialog_keys(ctx)
            {
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

            ctx.input(|i| {
                // 数字键快速选择格式: 1=SQL, 2=CSV, 3=JSON
                if i.key_pressed(Key::Num1) {
                    Self::set_format(state, ImportFormat::Sql);
                }
                if i.key_pressed(Key::Num2) {
                    Self::set_format(state, ImportFormat::Csv);
                }
                if i.key_pressed(Key::Num3) {
                    Self::set_format(state, ImportFormat::Json);
                }

                // h/l 切换格式
                if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
                    let new_format = match state.format {
                        ImportFormat::Sql => ImportFormat::Json,
                        ImportFormat::Csv => ImportFormat::Sql,
                        ImportFormat::Json => ImportFormat::Csv,
                    };
                    Self::set_format(state, new_format);
                }
                if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
                    let new_format = match state.format {
                        ImportFormat::Sql => ImportFormat::Csv,
                        ImportFormat::Csv => ImportFormat::Json,
                        ImportFormat::Json => ImportFormat::Sql,
                    };
                    Self::set_format(state, new_format);
                }

                // Ctrl+R 刷新预览
                if i.modifiers.ctrl && i.key_pressed(Key::R) && has_file {
                    action = ImportAction::RefreshPreview;
                }
            });
        }

        egui::Window::new("📥 导入数据")
            .collapsible(false)
            .resizable(false)
            .fixed_size(Vec2::new(600.0, 500.0))
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                // 限制内容高度
                ui.set_max_height(480.0);
                ui.add_space(SPACING_SM);

                // 文件选择区域
                action = Self::show_file_selector(ui, state);

                if state.file_path.is_some() {
                    ui.add_space(SPACING_SM);
                    ui.separator();
                    ui.add_space(SPACING_SM);

                    // 格式和模式选择
                    Self::show_format_mode_selector(ui, state);

                    ui.add_space(SPACING_SM);
                    ui.separator();
                    ui.add_space(SPACING_SM);

                    // 格式特定选项
                    ScrollArea::vertical()
                        .max_height(120.0)
                        .show(ui, |ui| match state.format {
                            ImportFormat::Sql => Self::show_sql_options(ui, state),
                            ImportFormat::Csv => Self::show_csv_options(ui, state, is_mysql),
                            ImportFormat::Json => Self::show_json_options(ui, state, is_mysql),
                        });

                    ui.add_space(SPACING_SM);
                    ui.separator();
                    ui.add_space(SPACING_SM);

                    // 预览区域
                    if let Some(ref preview) = state.preview {
                        Self::show_preview(ui, state, preview);
                    } else if state.loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("正在加载...");
                        });
                    } else if let Some(ref err) = state.error {
                        ui.label(RichText::new(format!("❌ {}", err)).color(DANGER));
                    } else {
                        ui.horizontal(|ui| {
                            if ui.button("🔍 加载预览").clicked() {
                                action = ImportAction::RefreshPreview;
                            }
                        });
                    }
                }

                ui.add_space(SPACING_SM);
                ui.separator();
                ui.add_space(SPACING_SM);

                // 底部按钮
                let btn_action = Self::show_buttons(ui, show, state);
                if !matches!(btn_action, ImportAction::None) {
                    action = btn_action;
                }

                ui.add_space(SPACING_SM);
            });

        action
    }

    /// 文件选择区域
    fn show_file_selector(ui: &mut egui::Ui, state: &mut ImportState) -> ImportAction {
        let mut action = ImportAction::None;

        ui.horizontal(|ui| {
            ui.label(RichText::new("文件:").color(GRAY));

            // 显示当前文件路径
            let path_text = state
                .file_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "未选择文件".to_string());

            let path_display = if path_text.len() > 60 {
                format!("...{}", &path_text[path_text.len() - 57..])
            } else {
                path_text.clone()
            };

            ui.add(
                TextEdit::singleline(&mut path_display.clone())
                    .desired_width(ui.available_width() - 80.0)
                    .interactive(false),
            );

            if ui.button("📂 浏览...").clicked() {
                action = ImportAction::SelectFile;
            }
        });

        // 显示文件信息
        if let Some(ref path) = state.file_path {
            ui.horizontal(|ui| {
                ui.add_space(40.0);

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

                // 格式图标
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
        }

        action
    }

    /// 格式和模式选择
    fn show_format_mode_selector(ui: &mut egui::Ui, state: &mut ImportState) {
        ui.horizontal(|ui| {
            // 格式选择
            ui.label(RichText::new("格式:").color(GRAY));
            for (idx, fmt) in [ImportFormat::Sql, ImportFormat::Csv, ImportFormat::Json]
                .iter()
                .enumerate()
            {
                let is_selected = state.format == *fmt;
                let text = format!("{} {} [{}]", fmt.icon(), fmt.name(), idx + 1);
                if ui
                    .selectable_label(is_selected, RichText::new(&text))
                    .clicked()
                {
                    Self::set_format(state, *fmt);
                }
            }

            ui.separator();
            ui.label(RichText::new("h/l").small().color(GRAY));
        });

        // 模式选择（仅 SQL 格式显示）
        if state.format == ImportFormat::Sql {
            ui.horizontal(|ui| {
                ui.label(RichText::new("模式:").color(GRAY));

                if ui
                    .selectable_label(state.mode == ImportMode::Execute, "🚀 直接执行")
                    .on_hover_text("逐条执行 SQL 语句")
                    .clicked()
                {
                    state.mode = ImportMode::Execute;
                }

                if ui
                    .selectable_label(state.mode == ImportMode::CopyToEditor, "📋 复制到编辑器")
                    .on_hover_text("将 SQL 复制到编辑器中")
                    .clicked()
                {
                    state.mode = ImportMode::CopyToEditor;
                }
            });
        }
    }

    /// SQL 选项
    fn show_sql_options(ui: &mut egui::Ui, state: &mut ImportState) {
        egui::CollapsingHeader::new("SQL 导入选项")
            .default_open(true)
            .show(ui, |ui| {
                let mut needs_refresh = false;

                ui.horizontal(|ui| {
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

                if state.mode == ImportMode::Execute {
                    ui.add_space(SPACING_SM);
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.sql_config.stop_on_error, "遇到错误时停止");
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.sql_config.use_transaction, "使用事务");
                        ui.label(RichText::new("(全部成功或全部回滚)").small().color(MUTED));
                    });
                }

                if needs_refresh {
                    state.preview = None;
                    state.error = None;
                }
            });
    }

    /// CSV 选项
    fn show_csv_options(ui: &mut egui::Ui, state: &mut ImportState, _is_mysql: bool) {
        egui::CollapsingHeader::new("CSV 导入选项")
            .default_open(true)
            .show(ui, |ui| {
                // 表名
                ui.horizontal(|ui| {
                    ui.label(RichText::new("目标表:").color(GRAY));
                    ui.add(
                        TextEdit::singleline(&mut state.csv_config.table_name)
                            .desired_width(150.0)
                            .hint_text("表名"),
                    );
                });

                ui.add_space(SPACING_SM);

                // 分隔符
                ui.horizontal(|ui| {
                    ui.label(RichText::new("分隔符:").color(GRAY));
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

                ui.add_space(SPACING_SM);

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(&mut state.csv_config.has_header, "首行为表头")
                        .changed()
                    {
                        state.preview = None;
                        state.error = None;
                    }

                    ui.separator();

                    ui.label(RichText::new("跳过行:").color(GRAY));
                    let mut skip_str = state.csv_config.skip_rows.to_string();
                    if ui
                        .add(TextEdit::singleline(&mut skip_str).desired_width(40.0))
                        .changed()
                    {
                        state.csv_config.skip_rows = skip_str.parse().unwrap_or(0);
                        state.preview = None;
                        state.error = None;
                    }
                });
            });
    }

    /// JSON 选项
    fn show_json_options(ui: &mut egui::Ui, state: &mut ImportState, _is_mysql: bool) {
        egui::CollapsingHeader::new("JSON 导入选项")
            .default_open(true)
            .show(ui, |ui| {
                let mut needs_refresh = false;

                // 表名
                ui.horizontal(|ui| {
                    ui.label(RichText::new("目标表:").color(GRAY));
                    ui.add(
                        TextEdit::singleline(&mut state.json_config.table_name)
                            .desired_width(150.0)
                            .hint_text("表名"),
                    );
                });

                ui.add_space(SPACING_SM);

                // JSON 路径
                ui.horizontal(|ui| {
                    ui.label(RichText::new("数据路径:").color(GRAY));
                    if ui
                        .add(
                            TextEdit::singleline(&mut state.json_config.json_path)
                                .desired_width(200.0)
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
            });
    }

    /// 预览区域
    fn show_preview(ui: &mut egui::Ui, state: &ImportState, preview: &ImportPreview) {
        let header = match state.format {
            ImportFormat::Sql => format!("预览 ({} 条 SQL 语句)", preview.statement_count),
            _ => format!(
                "预览 ({} 列 × {} 行)",
                preview.columns.len(),
                preview.total_rows
            ),
        };

        egui::CollapsingHeader::new(header)
            .default_open(true)
            .show(ui, |ui| {
                // 警告信息
                if !preview.warnings.is_empty() {
                    for warning in &preview.warnings {
                        ui.label(
                            RichText::new(format!("⚠ {}", warning))
                                .small()
                                .color(Color32::YELLOW),
                        );
                    }
                    ui.add_space(SPACING_SM);
                }

                // 预览内容
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 50, 200))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ScrollArea::both()
                            .max_height(180.0)
                            .show(ui, |ui| match state.format {
                                ImportFormat::Sql => {
                                    Self::show_sql_preview(ui, preview);
                                }
                                _ => {
                                    Self::show_table_preview(ui, preview);
                                }
                            });
                    });
            });
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

    /// 表格预览（CSV/JSON）
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

    /// 底部按钮
    fn show_buttons(ui: &mut egui::Ui, show: &mut bool, state: &ImportState) -> ImportAction {
        let mut action = ImportAction::None;
        let effective_mode = Self::effective_mode_for_format(state);

        ui.horizontal(|ui| {
            let has_file = state.file_path.is_some();
            let has_preview = state.preview.is_some();

            // 刷新预览按钮
            if has_file && ui.button("🔄 刷新预览 [Ctrl+R]").clicked() {
                action = ImportAction::RefreshPreview;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // 取消按钮
                if ui.button("取消 [Esc]").clicked() {
                    *show = false;
                    action = ImportAction::Close;
                }

                // 导入按钮
                let can_import = has_file && has_preview && state.error.is_none();

                ui.add_enabled_ui(can_import, |ui| {
                    let btn_text = match effective_mode {
                        ImportMode::Execute => "🚀 执行导入 [Enter]",
                        ImportMode::CopyToEditor => "📋 复制到编辑器 [Enter]",
                    };

                    if ui.button(RichText::new(btn_text).strong()).clicked() {
                        match effective_mode {
                            ImportMode::Execute => action = ImportAction::Execute,
                            ImportMode::CopyToEditor => {
                                if let Some(ref preview) = state.preview {
                                    let sql = preview.sql_statements.join("\n\n");
                                    action = ImportAction::CopyToEditor(sql);
                                }
                            }
                        }
                    }
                });
            });
        });

        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_format_resets_hidden_copy_mode_for_non_sql() {
        let mut state = ImportState {
            mode: ImportMode::CopyToEditor,
            preview: Some(ImportPreview::default()),
            error: Some("old error".to_string()),
            ..Default::default()
        };

        ImportDialog::set_format(&mut state, ImportFormat::Csv);

        assert_eq!(state.format, ImportFormat::Csv);
        assert_eq!(state.mode, ImportMode::Execute);
        assert!(state.preview.is_none());
        assert!(state.error.is_none());
    }

    #[test]
    fn test_effective_mode_for_non_sql_forces_execute() {
        let state = ImportState {
            format: ImportFormat::Json,
            mode: ImportMode::CopyToEditor,
            ..Default::default()
        };

        let effective = ImportDialog::effective_mode_for_format(&state);
        assert_eq!(effective, ImportMode::Execute);
    }
}
