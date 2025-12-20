//! 数据导出对话框 - 支持多种格式和自定义选项
//!
//! 支持的快捷键：
//! - `Esc` - 关闭对话框
//! - `Enter` - 导出（当配置有效时）
//! - `1/2/3` - 快速选择格式 (CSV/SQL/JSON)
//! - `h/l` - 切换格式
//! - `j/k` - 在列选择中导航
//! - `Space` - 切换当前列的选中状态
//! - `a` - 全选/取消全选列

use super::keyboard;
use crate::core::ExportFormat;
use crate::database::QueryResult;
use crate::ui::styles::{DANGER, GRAY, MUTED, SUCCESS, SPACING_SM, SPACING_MD};
use egui::{self, Color32, Key, RichText, CornerRadius, ScrollArea, TextEdit};

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
            .filter(|(_, &selected)| selected)
            .map(|(i, _)| i)
            .collect()
    }

    /// 是否全选
    pub fn all_columns_selected(&self) -> bool {
        self.selected_columns.iter().all(|&s| s)
    }

    /// 选中的列数
    pub fn selected_column_count(&self) -> usize {
        self.selected_columns.iter().filter(|&&s| s).count()
    }
}

pub struct ExportDialog;

impl ExportDialog {
    pub fn show(
        ctx: &egui::Context,
        show: &mut bool,
        config: &mut ExportConfig,
        table_name: &str,
        data: Option<&QueryResult>,
        on_export: &mut Option<ExportConfig>,
        status_message: &Option<Result<String, String>>,
    ) {
        if !*show {
            return;
        }

        // 初始化列选择
        if let Some(result) = data {
            config.init_columns(result.columns.len());
        }

        let row_count = data.map(|d| d.rows.len()).unwrap_or(0);
        let col_count = data.map(|d| d.columns.len()).unwrap_or(0);
        let can_export = config.selected_column_count() > 0 && row_count > 0;

        // 处理键盘快捷键（仅当没有文本输入焦点时）
        if !keyboard::has_text_focus(ctx) {
            // Esc 关闭
            if keyboard::handle_close_keys(ctx) {
                *show = false;
                return;
            }

            // Enter 导出
            if can_export {
                match keyboard::handle_dialog_keys(ctx) {
                    keyboard::DialogAction::Confirm => {
                        *on_export = Some(config.clone());
                        return;
                    }
                    _ => {}
                }
            }

            ctx.input(|i| {
                // 数字键快速选择格式: 1=CSV, 2=SQL, 3=JSON
                if i.key_pressed(Key::Num1) {
                    config.format = ExportFormat::Csv;
                }
                if i.key_pressed(Key::Num2) {
                    config.format = ExportFormat::Sql;
                }
                if i.key_pressed(Key::Num3) {
                    config.format = ExportFormat::Json;
                }

                // h/l 切换格式
                if i.key_pressed(Key::H) || i.key_pressed(Key::ArrowLeft) {
                    config.format = match config.format {
                        ExportFormat::Csv => ExportFormat::Json,
                        ExportFormat::Sql => ExportFormat::Csv,
                        ExportFormat::Json => ExportFormat::Sql,
                    };
                }
                if i.key_pressed(Key::L) || i.key_pressed(Key::ArrowRight) {
                    config.format = match config.format {
                        ExportFormat::Csv => ExportFormat::Sql,
                        ExportFormat::Sql => ExportFormat::Json,
                        ExportFormat::Json => ExportFormat::Csv,
                    };
                }

                // j/k 列选择导航
                if col_count > 0 {
                    if i.key_pressed(Key::J) || i.key_pressed(Key::ArrowDown) {
                        config.nav_column_index = (config.nav_column_index + 1).min(col_count - 1);
                    }
                    if i.key_pressed(Key::K) || i.key_pressed(Key::ArrowUp) {
                        config.nav_column_index = config.nav_column_index.saturating_sub(1);
                    }
                    if i.key_pressed(Key::G) && !i.modifiers.shift {
                        config.nav_column_index = 0;
                    }
                    if i.key_pressed(Key::G) && i.modifiers.shift {
                        config.nav_column_index = col_count.saturating_sub(1);
                    }

                    // Space 切换当前列
                    if i.key_pressed(Key::Space) {
                        if let Some(selected) = config.selected_columns.get_mut(config.nav_column_index) {
                            *selected = !*selected;
                        }
                    }

                    // a 全选/取消全选
                    if i.key_pressed(Key::A) {
                        let all_selected = config.all_columns_selected();
                        for s in &mut config.selected_columns {
                            *s = !all_selected;
                        }
                    }
                }
            });
        }

        egui::Window::new("📤 导出数据")
            .collapsible(false)
            .resizable(false)
            .min_width(320.0)
            .max_width(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.add_space(SPACING_SM);

                // 顶部信息栏
                Self::show_info_bar(ui, table_name, row_count, col_count, config);

                ui.add_space(SPACING_SM);
                ui.separator();
                ui.add_space(SPACING_SM);

                // 格式选择（紧凑版）
                Self::show_format_selector(ui, config);

                ui.add_space(SPACING_MD);

                // 使用折叠面板组织选项
                ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        // 导出范围
                        Self::show_row_range(ui, config, row_count);
                        
                        ui.add_space(SPACING_SM);

                        // 列选择（折叠）
                        if let Some(result) = data {
                            Self::show_column_selector(ui, config, &result.columns);
                        }

                        ui.add_space(SPACING_SM);

                        // 格式特定选项（折叠）
                        Self::show_format_options(ui, config);

                        ui.add_space(SPACING_SM);

                        // 导出预览（折叠）
                        if let Some(result) = data {
                            Self::show_preview(ui, config, result);
                        }
                    });

                ui.add_space(SPACING_SM);

                // 状态消息
                if let Some(result) = status_message {
                    Self::show_status_message(ui, result);
                    ui.add_space(SPACING_SM);
                }

                ui.separator();
                ui.add_space(SPACING_SM);

                // 底部按钮
                Self::show_buttons(ui, show, config, on_export, row_count);

                ui.add_space(SPACING_SM);
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
        ui.horizontal(|ui| {
            // 表名
            ui.label(RichText::new("表:").small().color(GRAY));
            ui.label(RichText::new(table_name).strong());

            ui.separator();

            // 统计信息
            let selected_cols = config.selected_column_count();
            let export_rows = if config.row_limit > 0 {
                config.row_limit.min(row_count.saturating_sub(config.start_row))
            } else {
                row_count.saturating_sub(config.start_row)
            };

            ui.label(RichText::new(format!(
                "导出: {}列 × {}行",
                selected_cols, export_rows
            )).small().color(MUTED));
            
            ui.label(RichText::new(format!("(共{}×{})", col_count, row_count))
                .small()
                .color(MUTED));
        });
    }

    /// 格式选择器（紧凑版）
    fn show_format_selector(ui: &mut egui::Ui, config: &mut ExportConfig) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("格式:").color(GRAY));
            
            for (idx, (fmt, icon, name)) in [
                (ExportFormat::Csv, "📊", "CSV"),
                (ExportFormat::Sql, "📝", "SQL"),
                (ExportFormat::Json, "🔧", "JSON"),
            ].iter().enumerate() {
                let is_selected = config.format == *fmt;
                let text = format!("{} {} [{}]", icon, name, idx + 1);
                
                if ui.selectable_label(is_selected, RichText::new(&text).strong()).clicked() {
                    config.format = *fmt;
                }
            }
            
            ui.separator();
            ui.label(RichText::new("h/l 切换").small().color(GRAY));
        });
    }

    /// 导出范围
    fn show_row_range(ui: &mut egui::Ui, config: &mut ExportConfig, total_rows: usize) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("行数:").color(GRAY));
            
            // 快捷按钮
            for (label, limit) in [("全部", 0), ("100", 100), ("1000", 1000)] {
                if ui.selectable_label(
                    config.row_limit == limit && config.start_row == 0,
                    label
                ).clicked() {
                    config.row_limit = limit;
                    config.start_row = 0;
                }
            }
            
            ui.separator();
            
            // 自定义行数
            ui.label(RichText::new("自定义:").small().color(GRAY));
            let mut limit_str = if config.row_limit == 0 {
                String::new()
            } else {
                config.row_limit.to_string()
            };
            if ui.add(
                TextEdit::singleline(&mut limit_str)
                    .desired_width(50.0)
                    .hint_text("全部")
            ).changed() {
                config.row_limit = limit_str.parse().unwrap_or(0);
            }
            
            ui.label(RichText::new(format!("/{}", total_rows)).small().color(MUTED));
        });
    }

    /// 列选择器（折叠面板）
    fn show_column_selector(ui: &mut egui::Ui, config: &mut ExportConfig, columns: &[String]) {
        let header = format!(
            "选择列 ({}/{}) [j/k Space a]",
            config.selected_column_count(),
            columns.len()
        );
        
        egui::CollapsingHeader::new(header)
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let all_selected = config.all_columns_selected();
                    if ui.button(if all_selected { "取消全选 [a]" } else { "全选 [a]" }).clicked() {
                        let new_state = !all_selected;
                        for s in &mut config.selected_columns {
                            *s = new_state;
                        }
                    }
                    
                    ui.separator();
                    ui.label(RichText::new("j/k 导航, Space 切换").small().color(GRAY));
                });
                
                ui.add_space(4.0);
                
                // 列复选框（垂直列表，支持键盘导航高亮）
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(60, 60, 70, 30))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ScrollArea::vertical()
                            .max_height(120.0)
                            .show(ui, |ui| {
                                for (i, col_name) in columns.iter().enumerate() {
                                    if i < config.selected_columns.len() {
                                        let is_nav_selected = i == config.nav_column_index;
                                        let display_name = if col_name.len() > 20 {
                                            format!("{}…", &col_name[..18])
                                        } else {
                                            col_name.clone()
                                        };
                                        
                                        // 键盘导航高亮
                                        let bg_color = if is_nav_selected {
                                            Color32::from_rgba_unmultiplied(100, 150, 255, 60)
                                        } else {
                                            Color32::TRANSPARENT
                                        };
                                        
                                        egui::Frame::NONE
                                            .fill(bg_color)
                                            .corner_radius(CornerRadius::same(2))
                                            .inner_margin(egui::Margin::symmetric(4, 1))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    if is_nav_selected {
                                                        ui.label(RichText::new(">").small().color(Color32::from_rgb(100, 180, 255)));
                                                    }
                                                    if ui.checkbox(&mut config.selected_columns[i], &display_name).clicked() {
                                                        config.nav_column_index = i;
                                                    }
                                                });
                                            });
                                    }
                                }
                            });
                    });
            });
    }

    /// 格式特定选项（折叠面板）
    fn show_format_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        let header = match config.format {
            ExportFormat::Csv => "CSV 选项",
            ExportFormat::Sql => "SQL 选项",
            ExportFormat::Json => "JSON 选项",
        };
        
        egui::CollapsingHeader::new(header)
            .default_open(false)
            .show(ui, |ui| {
                match config.format {
                    ExportFormat::Csv => Self::show_csv_options(ui, config),
                    ExportFormat::Sql => Self::show_sql_options(ui, config),
                    ExportFormat::Json => Self::show_json_options(ui, config),
                }
            });
    }

    /// CSV 选项
    fn show_csv_options(ui: &mut egui::Ui, config: &mut ExportConfig) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("分隔符:").small().color(GRAY));
            for (label, delim) in [(",", ','), (";", ';'), ("Tab", '\t'), ("|", '|')] {
                if ui.selectable_label(config.csv_delimiter == delim, label).clicked() {
                    config.csv_delimiter = delim;
                }
            }
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
                if ui.selectable_label(config.sql_batch_size == size, label).clicked() {
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
    fn show_preview(ui: &mut egui::Ui, config: &ExportConfig, data: &QueryResult) {
        egui::CollapsingHeader::new("预览")
            .default_open(false)
            .show(ui, |ui| {
                let preview_text = Self::generate_preview(config, data);
                
                egui::Frame::NONE
                    .fill(Color32::from_rgba_unmultiplied(40, 40, 50, 60))
                    .corner_radius(CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        ScrollArea::horizontal()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&preview_text)
                                        .monospace()
                                        .size(10.0)
                                        .color(Color32::from_rgb(180, 180, 190))
                                );
                            });
                    });
            });
    }

    /// 生成预览文本
    fn generate_preview(config: &ExportConfig, data: &QueryResult) -> String {
        let selected_indices = config.get_selected_column_indices();
        if selected_indices.is_empty() {
            return "（未选择任何列）".to_string();
        }

        let preview_rows = 3.min(data.rows.len());
        let selected_cols: Vec<&String> = selected_indices
            .iter()
            .filter_map(|&i| data.columns.get(i))
            .collect();

        match config.format {
            ExportFormat::Csv => {
                let mut lines = Vec::new();
                if config.csv_include_header {
                    lines.push(selected_cols.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(&config.csv_delimiter.to_string()));
                }
                for row in data.rows.iter().skip(config.start_row).take(preview_rows) {
                    let values: Vec<&str> = selected_indices
                        .iter()
                        .filter_map(|&i| row.get(i).map(|s| s.as_str()))
                        .collect();
                    lines.push(values.join(&config.csv_delimiter.to_string()));
                }
                if data.rows.len() > preview_rows {
                    lines.push(format!("... (+{} 行)", data.rows.len() - preview_rows));
                }
                lines.join("\n")
            }
            ExportFormat::Sql => {
                let mut lines = Vec::new();
                let cols_str = selected_cols.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", ");
                for row in data.rows.iter().skip(config.start_row).take(preview_rows.min(2)) {
                    let values: Vec<String> = selected_indices
                        .iter()
                        .filter_map(|&i| row.get(i))
                        .map(|v| if v == "NULL" { "NULL".to_string() } else { format!("'{}'", v) })
                        .collect();
                    lines.push(format!("INSERT INTO `t` ({}) VALUES ({});", cols_str, values.join(", ")));
                }
                if data.rows.len() > 2 {
                    lines.push(format!("... (+{} 条)", data.rows.len() - 2));
                }
                lines.join("\n")
            }
            ExportFormat::Json => {
                let mut items = Vec::new();
                for row in data.rows.iter().skip(config.start_row).take(2) {
                    let obj: Vec<String> = selected_indices
                        .iter()
                        .zip(selected_cols.iter())
                        .filter_map(|(&i, col)| {
                            row.get(i).map(|v| {
                                if v == "NULL" {
                                    format!("\"{}\": null", col)
                                } else {
                                    format!("\"{}\": \"{}\"", col, v)
                                }
                            })
                        })
                        .collect();
                    items.push(format!("{{ {} }}", obj.join(", ")));
                }
                if data.rows.len() > 2 {
                    items.push(format!("... (+{} 条)", data.rows.len() - 2));
                }
                format!("[{}]", items.join(", "))
            }
        }
    }

    /// 状态消息
    fn show_status_message(ui: &mut egui::Ui, result: &Result<String, String>) {
        let (icon, message, color, bg_color) = match result {
            Ok(msg) => ("[OK]", msg.as_str(), SUCCESS, Color32::from_rgba_unmultiplied(82, 196, 106, 25)),
            Err(msg) => ("[X]", msg.as_str(), DANGER, Color32::from_rgba_unmultiplied(235, 87, 87, 25)),
        };

        egui::Frame::NONE
            .fill(bg_color)
            .corner_radius(CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(icon).color(color));
                    ui.label(RichText::new(message).small().color(color));
                });
            });
    }

    /// 底部按钮
    fn show_buttons(
        ui: &mut egui::Ui,
        show: &mut bool,
        config: &ExportConfig,
        on_export: &mut Option<ExportConfig>,
        row_count: usize,
    ) {
        let can_export = config.selected_column_count() > 0 && row_count > 0;

        ui.horizontal(|ui| {
            if ui.button("取消 [Esc]").clicked() {
                *show = false;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn_text = format!("导出 {} [Enter]", config.format.display_name());
                let export_btn = egui::Button::new(
                    RichText::new(&btn_text)
                        .color(if can_export { Color32::WHITE } else { GRAY })
                )
                .fill(if can_export { SUCCESS } else { Color32::from_rgb(80, 80, 90) });

                if ui.add_enabled(can_export, export_btn).clicked() {
                    *on_export = Some(config.clone());
                }

                if !can_export {
                    ui.label(RichText::new("请选择列").small().color(DANGER));
                }
            });
        });
    }
}
