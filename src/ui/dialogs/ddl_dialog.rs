//! DDL 操作对话框
//!
//! 提供创建表、修改表结构等 DDL 操作的 UI。
//! 支持 Helix 风格的键盘导航。

use super::common::{
    DialogContent, DialogFooter, DialogShortcutContext, DialogStyle, DialogWindow, FormDialogShell,
};
use crate::database::DatabaseType;
use crate::ui::{LocalShortcut, local_shortcut_text, local_shortcut_tooltip, local_shortcuts_text};
use egui::{self, Color32, RichText, TextEdit};

// ============================================================================
// 列定义
// ============================================================================

/// 列数据类型（支持多种数据库的常用类型）
#[allow(dead_code)] // 公开 API，完整的列类型定义
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnType {
    // 整数类型
    Integer,
    BigInt,
    SmallInt,
    TinyInt,

    // 浮点类型
    Float,
    Double,
    Decimal { precision: u8, scale: u8 },

    // 字符串类型
    Varchar(u16),
    Char(u16),
    Text,

    // 日期时间类型
    Date,
    Time,
    DateTime,
    Timestamp,

    // 二进制类型
    Blob,
    Binary(u16),

    // 其他类型
    Boolean,
    Json,
    Uuid,

    // 自定义类型（原始 SQL 类型字符串）
    Custom(String),
}

impl ColumnType {
    /// 获取显示名称
    pub fn display_name(&self) -> String {
        match self {
            Self::Integer => "INTEGER".to_string(),
            Self::BigInt => "BIGINT".to_string(),
            Self::SmallInt => "SMALLINT".to_string(),
            Self::TinyInt => "TINYINT".to_string(),
            Self::Float => "FLOAT".to_string(),
            Self::Double => "DOUBLE".to_string(),
            Self::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
            Self::Varchar(len) => format!("VARCHAR({})", len),
            Self::Char(len) => format!("CHAR({})", len),
            Self::Text => "TEXT".to_string(),
            Self::Date => "DATE".to_string(),
            Self::Time => "TIME".to_string(),
            Self::DateTime => "DATETIME".to_string(),
            Self::Timestamp => "TIMESTAMP".to_string(),
            Self::Blob => "BLOB".to_string(),
            Self::Binary(len) => format!("BINARY({})", len),
            Self::Boolean => "BOOLEAN".to_string(),
            Self::Json => "JSON".to_string(),
            Self::Uuid => "UUID".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// 转换为特定数据库的 SQL 类型
    pub fn to_sql(&self, db_type: &DatabaseType) -> String {
        match db_type {
            DatabaseType::SQLite => self.to_sqlite_sql(),
            DatabaseType::MySQL => self.to_mysql_sql(),
            DatabaseType::PostgreSQL => self.to_postgres_sql(),
        }
    }

    fn to_sqlite_sql(&self) -> String {
        match self {
            Self::Integer | Self::BigInt | Self::SmallInt | Self::TinyInt => "INTEGER".to_string(),
            Self::Float | Self::Double => "REAL".to_string(),
            Self::Decimal { .. } => "REAL".to_string(),
            Self::Varchar(_) | Self::Char(_) | Self::Text => "TEXT".to_string(),
            Self::Date | Self::Time | Self::DateTime | Self::Timestamp => "TEXT".to_string(),
            Self::Blob | Self::Binary(_) => "BLOB".to_string(),
            Self::Boolean => "INTEGER".to_string(),
            Self::Json => "TEXT".to_string(),
            Self::Uuid => "TEXT".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    fn to_mysql_sql(&self) -> String {
        match self {
            Self::Integer => "INT".to_string(),
            Self::BigInt => "BIGINT".to_string(),
            Self::SmallInt => "SMALLINT".to_string(),
            Self::TinyInt => "TINYINT".to_string(),
            Self::Float => "FLOAT".to_string(),
            Self::Double => "DOUBLE".to_string(),
            Self::Decimal { precision, scale } => format!("DECIMAL({},{})", precision, scale),
            Self::Varchar(len) => format!("VARCHAR({})", len),
            Self::Char(len) => format!("CHAR({})", len),
            Self::Text => "TEXT".to_string(),
            Self::Date => "DATE".to_string(),
            Self::Time => "TIME".to_string(),
            Self::DateTime => "DATETIME".to_string(),
            Self::Timestamp => "TIMESTAMP".to_string(),
            Self::Blob => "BLOB".to_string(),
            Self::Binary(len) => format!("BINARY({})", len),
            Self::Boolean => "TINYINT(1)".to_string(),
            Self::Json => "JSON".to_string(),
            Self::Uuid => "CHAR(36)".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    fn to_postgres_sql(&self) -> String {
        match self {
            Self::Integer => "INTEGER".to_string(),
            Self::BigInt => "BIGINT".to_string(),
            Self::SmallInt => "SMALLINT".to_string(),
            Self::TinyInt => "SMALLINT".to_string(),
            Self::Float => "REAL".to_string(),
            Self::Double => "DOUBLE PRECISION".to_string(),
            Self::Decimal { precision, scale } => format!("NUMERIC({},{})", precision, scale),
            Self::Varchar(len) => format!("VARCHAR({})", len),
            Self::Char(len) => format!("CHAR({})", len),
            Self::Text => "TEXT".to_string(),
            Self::Date => "DATE".to_string(),
            Self::Time => "TIME".to_string(),
            Self::DateTime => "TIMESTAMP".to_string(),
            Self::Timestamp => "TIMESTAMPTZ".to_string(),
            Self::Blob => "BYTEA".to_string(),
            Self::Binary(_) => "BYTEA".to_string(),
            Self::Boolean => "BOOLEAN".to_string(),
            Self::Json => "JSONB".to_string(),
            Self::Uuid => "UUID".to_string(),
            Self::Custom(s) => s.clone(),
        }
    }

    /// 常用类型列表
    pub fn common_types() -> Vec<Self> {
        vec![
            Self::Integer,
            Self::BigInt,
            Self::Varchar(255),
            Self::Text,
            Self::Boolean,
            Self::Date,
            Self::DateTime,
            Self::Decimal {
                precision: 10,
                scale: 2,
            },
            Self::Float,
            Self::Json,
        ]
    }
}

impl Default for ColumnType {
    fn default() -> Self {
        Self::Varchar(255)
    }
}

/// 列定义
#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    /// 列名
    pub name: String,
    /// 数据类型
    pub data_type: ColumnType,
    /// 是否允许 NULL
    pub nullable: bool,
    /// 是否是主键
    pub primary_key: bool,
    /// 是否自增
    pub auto_increment: bool,
    /// 是否唯一
    pub unique: bool,
    /// 默认值
    pub default_value: String,
    /// 注释
    pub comment: String,
}

impl Default for ColumnDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            data_type: ColumnType::default(),
            nullable: true,
            primary_key: false,
            auto_increment: false,
            unique: false,
            default_value: String::new(),
            comment: String::new(),
        }
    }
}

impl ColumnDefinition {
    /// 生成列的 SQL 定义
    pub fn to_sql(&self, db_type: &DatabaseType) -> String {
        let mut parts = vec![
            quote_identifier(&self.name, db_type),
            self.data_type.to_sql(db_type),
        ];

        if self.primary_key {
            parts.push("PRIMARY KEY".to_string());
        }

        if self.auto_increment {
            match db_type {
                DatabaseType::SQLite => {
                    // SQLite 使用 AUTOINCREMENT 关键字，但只能用于 INTEGER PRIMARY KEY
                    if self.primary_key {
                        parts.push("AUTOINCREMENT".to_string());
                    }
                }
                DatabaseType::MySQL => parts.push("AUTO_INCREMENT".to_string()),
                DatabaseType::PostgreSQL => {
                    // PostgreSQL 使用 SERIAL 类型，这里假设已经设置了正确的类型
                }
            }
        }

        if !self.nullable && !self.primary_key {
            parts.push("NOT NULL".to_string());
        }

        if self.unique && !self.primary_key {
            parts.push("UNIQUE".to_string());
        }

        if !self.default_value.is_empty() {
            parts.push(format!("DEFAULT {}", self.default_value));
        }

        if !self.comment.is_empty() && matches!(db_type, DatabaseType::MySQL) {
            parts.push(format!("COMMENT '{}'", self.comment.replace('\'', "''")));
        }

        parts.join(" ")
    }
}

// ============================================================================
// 表定义
// ============================================================================

/// 表定义
#[derive(Debug, Clone, Default)]
pub struct TableDefinition {
    /// 表名
    pub name: String,
    /// 列定义
    pub columns: Vec<ColumnDefinition>,
    /// 表注释
    pub comment: String,
    /// 数据库类型
    pub db_type: DatabaseType,
}

impl TableDefinition {
    /// 创建新的表定义
    pub fn new(db_type: DatabaseType) -> Self {
        Self {
            db_type,
            ..Default::default()
        }
    }

    /// 生成 CREATE TABLE SQL
    pub fn to_create_sql(&self) -> String {
        if self.name.is_empty() || self.columns.is_empty() {
            return String::new();
        }

        let table_name = quote_identifier(&self.name, &self.db_type);
        let columns: Vec<String> = self
            .columns
            .iter()
            .map(|c| format!("    {}", c.to_sql(&self.db_type)))
            .collect();

        let mut sql = format!("CREATE TABLE {} (\n{}\n)", table_name, columns.join(",\n"));

        // MySQL 表注释
        if !self.comment.is_empty() && matches!(self.db_type, DatabaseType::MySQL) {
            sql.push_str(&format!(" COMMENT='{}'", self.comment.replace('\'', "''")));
        }

        sql.push(';');

        // PostgreSQL 表注释需要单独的语句
        if !self.comment.is_empty() && matches!(self.db_type, DatabaseType::PostgreSQL) {
            sql.push_str(&format!(
                "\nCOMMENT ON TABLE {} IS '{}';",
                table_name,
                self.comment.replace('\'', "''")
            ));
        }

        sql
    }

    /// 验证表定义
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("表名不能为空".to_string());
        }

        if self.columns.is_empty() {
            return Err("至少需要一个列".to_string());
        }

        // 检查列名是否有重复
        let mut names = std::collections::HashSet::new();
        for col in &self.columns {
            if col.name.is_empty() {
                return Err("列名不能为空".to_string());
            }
            if !names.insert(col.name.to_lowercase()) {
                return Err(format!("列名 '{}' 重复", col.name));
            }
        }

        // 检查主键数量
        let pk_count = self.columns.iter().filter(|c| c.primary_key).count();
        if pk_count > 1 {
            return Err("只能有一个主键列".to_string());
        }

        Ok(())
    }
}

// ============================================================================
// DDL 对话框状态
// ============================================================================

/// DDL 对话框状态
#[allow(dead_code)] // type_dropdown_open 预留用于类型选择 UI
#[derive(Default)]
pub struct DdlDialogState {
    /// 当前正在编辑的表定义
    pub table: TableDefinition,
    /// 是否显示对话框
    pub show: bool,
    /// 类型选择下拉框打开状态
    pub type_dropdown_open: Option<usize>,
    /// 错误信息
    pub error: Option<String>,
    /// 生成的 SQL
    pub generated_sql: String,
    /// 当前选中的列索引（用于键盘导航）
    pub selected_column: usize,
}

#[allow(dead_code)] // 公开 API，供外部使用
impl DdlDialogState {
    /// 创建新的 DDL 对话框状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 打开创建表对话框
    pub fn open_create_table(&mut self, db_type: DatabaseType) {
        self.table = TableDefinition::new(db_type);
        // 添加一个默认的 id 列
        self.table.columns.push(ColumnDefinition {
            name: "id".to_string(),
            data_type: ColumnType::Integer,
            primary_key: true,
            auto_increment: true,
            nullable: false,
            ..Default::default()
        });
        self.show = true;
        self.error = None;
        self.generated_sql.clear();
        self.selected_column = 0;
    }

    /// 关闭对话框
    pub fn close(&mut self) {
        self.show = false;
        self.table = TableDefinition::default();
        self.error = None;
        self.generated_sql.clear();
    }
}

// ============================================================================
// DDL 对话框 UI
// ============================================================================

/// DDL 对话框
pub struct DdlDialog;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DdlKeyAction {
    Confirm,
    Close,
    ColumnPrev,
    ColumnNext,
    ColumnStart,
    ColumnEnd,
    ColumnDelete,
    ColumnAddBelow,
    ColumnAddAbove,
    ColumnTogglePrimaryKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponsiveRowClass {
    Wide,
    Medium,
    Narrow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ColumnRowLayout {
    row_class: ResponsiveRowClass,
    is_selected: bool,
    idx: usize,
    col_count: usize,
}

impl DdlDialog {
    const WINDOW_WIDTH: f32 = 920.0;
    const WINDOW_HEIGHT: f32 = 500.0;
    const WIDE_ROW_THRESHOLD: f32 = 720.0;
    const MEDIUM_ROW_THRESHOLD: f32 = 560.0;
    const COLUMN_LIST_MIN_HEIGHT: f32 = 140.0;
    const COLUMN_LIST_MAX_HEIGHT: f32 = 260.0;
    const SQL_PREVIEW_MIN_HEIGHT: f32 = 120.0;
    const SQL_PREVIEW_MAX_HEIGHT: f32 = 180.0;

    fn try_create_table(state: &mut DdlDialogState) -> Result<String, String> {
        match state.table.validate() {
            Ok(()) => {
                let sql = state.table.to_create_sql();
                state.error = None;
                Ok(sql)
            }
            Err(error) => {
                state.error = Some(error.clone());
                Err(error)
            }
        }
    }

    fn toggle_primary_key(state: &mut DdlDialogState) {
        if let Some(col) = state.table.columns.get_mut(state.selected_column) {
            let new_primary_key = !col.primary_key;
            col.primary_key = new_primary_key;
            if new_primary_key {
                col.nullable = false;
                for (idx, other_col) in state.table.columns.iter_mut().enumerate() {
                    if idx != state.selected_column {
                        other_col.primary_key = false;
                    }
                }
            }
        }
    }

    fn detect_key_action(ctx: &egui::Context) -> Option<DdlKeyAction> {
        DialogShortcutContext::new(ctx).resolve_commands(&[
            (LocalShortcut::Dismiss.config_key(), DdlKeyAction::Close),
            (LocalShortcut::Confirm.config_key(), DdlKeyAction::Confirm),
            (
                LocalShortcut::DdlColumnPrev.config_key(),
                DdlKeyAction::ColumnPrev,
            ),
            (
                LocalShortcut::DdlColumnNext.config_key(),
                DdlKeyAction::ColumnNext,
            ),
            (
                LocalShortcut::DdlColumnStart.config_key(),
                DdlKeyAction::ColumnStart,
            ),
            (
                LocalShortcut::DdlColumnEnd.config_key(),
                DdlKeyAction::ColumnEnd,
            ),
            (
                LocalShortcut::DdlColumnDelete.config_key(),
                DdlKeyAction::ColumnDelete,
            ),
            (
                LocalShortcut::DdlColumnAddBelow.config_key(),
                DdlKeyAction::ColumnAddBelow,
            ),
            (
                LocalShortcut::DdlColumnAddAbove.config_key(),
                DdlKeyAction::ColumnAddAbove,
            ),
            (
                LocalShortcut::DdlColumnTogglePrimaryKey.config_key(),
                DdlKeyAction::ColumnTogglePrimaryKey,
            ),
        ])
    }

    /// 显示创建表对话框
    pub fn show_create_table(ctx: &egui::Context, state: &mut DdlDialogState) -> Option<String> {
        if !state.show {
            return None;
        }

        let mut result: Option<String> = None;
        let mut should_close = false;
        let mut footer_confirmed = false;
        let mut footer_cancelled = false;

        // 键盘快捷键处理（文本输入优先于普通命令键）
        let col_count = state.table.columns.len();
        match Self::detect_key_action(ctx) {
            Some(DdlKeyAction::Close) => {
                state.close();
                return None;
            }
            Some(DdlKeyAction::Confirm) => {
                if let Ok(sql) = Self::try_create_table(state) {
                    result = Some(sql);
                    state.close();
                    return result;
                }
            }
            Some(DdlKeyAction::ColumnPrev) => {
                if state.selected_column > 0 {
                    state.selected_column -= 1;
                }
            }
            Some(DdlKeyAction::ColumnNext) => {
                if state.selected_column < col_count.saturating_sub(1) {
                    state.selected_column += 1;
                }
            }
            Some(DdlKeyAction::ColumnStart) => {
                state.selected_column = 0;
            }
            Some(DdlKeyAction::ColumnEnd) => {
                state.selected_column = col_count.saturating_sub(1);
            }
            Some(DdlKeyAction::ColumnDelete) => {
                if col_count > 1 {
                    state.table.columns.remove(state.selected_column);
                    if state.selected_column >= state.table.columns.len() {
                        state.selected_column = state.table.columns.len().saturating_sub(1);
                    }
                }
            }
            Some(DdlKeyAction::ColumnAddBelow) => {
                let insert_pos = (state.selected_column + 1).min(col_count);
                state
                    .table
                    .columns
                    .insert(insert_pos, ColumnDefinition::default());
                state.selected_column = insert_pos;
            }
            Some(DdlKeyAction::ColumnAddAbove) => {
                state
                    .table
                    .columns
                    .insert(state.selected_column, ColumnDefinition::default());
            }
            Some(DdlKeyAction::ColumnTogglePrimaryKey) => Self::toggle_primary_key(state),
            None => {}
        }

        let style = DialogStyle::WORKSPACE;
        let can_create = !state.table.name.trim().is_empty()
            && state
                .table
                .columns
                .iter()
                .any(|column| !column.name.trim().is_empty());
        DialogWindow::workspace(
            ctx,
            "创建表",
            &style,
            Self::WINDOW_WIDTH,
            Self::WINDOW_HEIGHT,
        )
        .show(ctx, |ui| {
            FormDialogShell::show(
                ui,
                "ddl_form_shell",
                |ui| {
                    DialogContent::shortcut_hint(
                        ui,
                        &[
                            (local_shortcut_text(LocalShortcut::Dismiss).as_str(), "关闭"),
                            (local_shortcut_text(LocalShortcut::Confirm).as_str(), "创建"),
                        ],
                    );
                },
                |ui| {
                    DialogContent::section_with_description(
                        ui,
                        "表信息",
                        "先确定表名和注释，再进入列设计与 SQL 预览。",
                        |ui| Self::show_table_info_fields(ui, &mut state.table),
                    );

                    DialogContent::section_with_description(
                        ui,
                        "列定义",
                        "保留原有键盘导航与列级操作，但统一到工作台面板里。",
                        |ui| {
                            DialogContent::toolbar(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label(RichText::new("列定义").strong());
                                    ui.label(
                                        RichText::new(format!(
                                            "[{} 移动 | {} / {} 添加 | {} 删除 | {} 切换主键]",
                                            local_shortcuts_text(&[
                                                LocalShortcut::DdlColumnPrev,
                                                LocalShortcut::DdlColumnNext,
                                            ]),
                                            local_shortcut_text(LocalShortcut::DdlColumnAddBelow),
                                            local_shortcut_text(LocalShortcut::DdlColumnAddAbove),
                                            local_shortcut_text(LocalShortcut::DdlColumnDelete),
                                            local_shortcut_text(
                                                LocalShortcut::DdlColumnTogglePrimaryKey
                                            ),
                                        ))
                                        .small()
                                        .color(Color32::from_rgb(120, 120, 120)),
                                    );
                                    if ui
                                        .button(format!(
                                            "+ 添加列 [{}]",
                                            local_shortcut_text(LocalShortcut::DdlColumnAddBelow)
                                        ))
                                        .on_hover_text(local_shortcut_tooltip(
                                            "在当前列下方添加新列",
                                            LocalShortcut::DdlColumnAddBelow,
                                        ))
                                        .clicked()
                                    {
                                        let insert_pos = (state.selected_column + 1)
                                            .min(state.table.columns.len());
                                        state
                                            .table
                                            .columns
                                            .insert(insert_pos, ColumnDefinition::default());
                                        state.selected_column = insert_pos;
                                    }
                                });
                            });

                            ui.add_space(6.0);
                            let dense_row_class = Self::row_width_class(ui.available_width());
                            Self::show_column_headers(ui, dense_row_class);

                            let mut col_to_remove: Option<usize> = None;
                            let mut new_pk_idx: Option<usize> = None;
                            let col_count = state.table.columns.len();

                            egui::ScrollArea::vertical()
                                .max_height(Self::column_list_max_height(ui))
                                .show(ui, |ui| {
                                    for idx in 0..col_count {
                                        let layout = ColumnRowLayout {
                                            row_class: Self::row_width_class(ui.available_width()),
                                            is_selected: idx == state.selected_column,
                                            idx,
                                            col_count,
                                        };
                                        let col = &mut state.table.columns[idx];

                                        let row_response = Self::show_column_row(
                                            ui,
                                            col,
                                            layout,
                                            &mut new_pk_idx,
                                            &mut col_to_remove,
                                        );

                                        if row_response.response.clicked() {
                                            state.selected_column = idx;
                                        }
                                    }
                                });

                            if let Some(pk_idx) = new_pk_idx {
                                for (i, col) in state.table.columns.iter_mut().enumerate() {
                                    if i != pk_idx {
                                        col.primary_key = false;
                                    }
                                }
                            }

                            if let Some(idx) = col_to_remove {
                                state.table.columns.remove(idx);
                            }
                        },
                    );

                    DialogContent::section_with_description(
                        ui,
                        "预览 SQL",
                        "根据当前表结构实时生成 `CREATE TABLE` 语句。",
                        |ui| {
                            let sql = state.table.to_create_sql();
                            DialogContent::code_block(ui, &sql, Self::sql_preview_max_height(ui));
                        },
                    );

                    if let Some(err) = &state.error {
                        DialogContent::warning_text(ui, err);
                        ui.add_space(8.0);
                    }
                },
                |ui| {
                    let footer = DialogFooter::show(
                        ui,
                        &format!("创建表 [{}]", local_shortcut_text(LocalShortcut::Confirm)),
                        &format!("取消 [{}]", local_shortcut_text(LocalShortcut::Dismiss)),
                        can_create,
                        &style,
                    );
                    footer_confirmed = footer.confirmed;
                    footer_cancelled = footer.cancelled;
                },
            );
        });

        if footer_confirmed && let Ok(sql) = Self::try_create_table(state) {
            result = Some(sql);
            should_close = true;
        }
        if footer_cancelled {
            should_close = true;
        }

        if should_close {
            state.close();
        }

        result
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

    fn column_list_max_height(ui: &egui::Ui) -> f32 {
        DialogContent::adaptive_height(
            ui,
            0.34,
            Self::COLUMN_LIST_MIN_HEIGHT,
            Self::COLUMN_LIST_MAX_HEIGHT,
        )
    }

    fn sql_preview_max_height(ui: &egui::Ui) -> f32 {
        DialogContent::adaptive_height(
            ui,
            0.28,
            Self::SQL_PREVIEW_MIN_HEIGHT,
            Self::SQL_PREVIEW_MAX_HEIGHT,
        )
    }

    fn show_table_info_fields(ui: &mut egui::Ui, table: &mut TableDefinition) {
        match Self::row_width_class(ui.available_width()) {
            ResponsiveRowClass::Wide => {
                let pair_width = ((ui.available_width() - 12.0) / 2.0).max(200.0);
                ui.horizontal_top(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(pair_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| Self::show_table_info_input(ui, "表名", &mut table.name, "输入表名"),
                    );
                    ui.add_space(12.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(pair_width, 0.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| Self::show_table_info_input(ui, "注释", &mut table.comment, "可选"),
                    );
                });
            }
            ResponsiveRowClass::Medium | ResponsiveRowClass::Narrow => {
                Self::show_table_info_input(ui, "表名", &mut table.name, "输入表名");
                ui.add_space(8.0);
                Self::show_table_info_input(ui, "注释", &mut table.comment, "可选");
            }
        }
    }

    fn show_table_info_input(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str) {
        ui.label(RichText::new(format!("{}:", label)).color(Color32::from_gray(180)));
        ui.add_sized(
            [ui.available_width().min(280.0), 0.0],
            TextEdit::singleline(value).hint_text(hint),
        );
    }

    fn show_column_headers(ui: &mut egui::Ui, row_class: ResponsiveRowClass) {
        if !matches!(row_class, ResponsiveRowClass::Wide) {
            return;
        }

        ui.horizontal(|ui| {
            ui.label(RichText::new("列名").small().strong());
            ui.add_space(80.0);
            ui.label(RichText::new("类型").small().strong());
            ui.add_space(80.0);
            ui.label(RichText::new("PK").small().strong());
            ui.add_space(10.0);
            ui.label(RichText::new("AI").small().strong());
            ui.add_space(10.0);
            ui.label(RichText::new("NN").small().strong());
            ui.add_space(10.0);
            ui.label(RichText::new("UQ").small().strong());
            ui.add_space(10.0);
            ui.label(RichText::new("默认值").small().strong());
        });

        ui.add_space(4.0);
    }

    fn show_column_row(
        ui: &mut egui::Ui,
        col: &mut ColumnDefinition,
        layout: ColumnRowLayout,
        new_pk_idx: &mut Option<usize>,
        col_to_remove: &mut Option<usize>,
    ) -> egui::InnerResponse<()> {
        let fill = if layout.is_selected {
            Color32::from_rgba_unmultiplied(100, 180, 255, 18)
        } else {
            Color32::TRANSPARENT
        };
        let stroke = if layout.is_selected {
            egui::Stroke::new(1.0, Color32::from_rgb(100, 180, 255))
        } else {
            egui::Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color)
        };

        egui::Frame::NONE
            .fill(fill)
            .stroke(stroke)
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(8, 6))
            .show(ui, |ui| match layout.row_class {
                ResponsiveRowClass::Wide => {
                    Self::show_wide_column_row(ui, col, layout, new_pk_idx, col_to_remove);
                }
                ResponsiveRowClass::Medium => {
                    Self::show_medium_column_row(ui, col, layout, new_pk_idx, col_to_remove);
                }
                ResponsiveRowClass::Narrow => {
                    Self::show_narrow_column_row(ui, col, layout, new_pk_idx, col_to_remove);
                }
            })
    }

    fn show_wide_column_row(
        ui: &mut egui::Ui,
        col: &mut ColumnDefinition,
        layout: ColumnRowLayout,
        new_pk_idx: &mut Option<usize>,
        col_to_remove: &mut Option<usize>,
    ) {
        ui.horizontal(|ui| {
            Self::show_row_indicator(ui, layout.is_selected);
            ui.add_sized(
                [100.0, 0.0],
                TextEdit::singleline(&mut col.name).hint_text("列名"),
            );

            Self::show_type_combo(ui, layout.idx, col, 120.0);

            Self::show_flag_checkboxes(ui, col, layout.idx, false, new_pk_idx);

            ui.add_sized(
                [80.0, 0.0],
                TextEdit::singleline(&mut col.default_value).hint_text("默认值"),
            );

            Self::show_delete_button(ui, layout, col_to_remove);
        });
    }

    fn show_medium_column_row(
        ui: &mut egui::Ui,
        col: &mut ColumnDefinition,
        layout: ColumnRowLayout,
        new_pk_idx: &mut Option<usize>,
        col_to_remove: &mut Option<usize>,
    ) {
        ui.horizontal(|ui| {
            Self::show_row_indicator(ui, layout.is_selected);
            let field_width = (ui.available_width() * 0.38).clamp(110.0, 180.0);
            ui.add_sized(
                [field_width, 0.0],
                TextEdit::singleline(&mut col.name).hint_text("列名"),
            );

            let combo_width = (ui.available_width() * 0.42).clamp(130.0, 190.0);
            Self::show_type_combo(ui, layout.idx, col, combo_width);

            Self::show_delete_button(ui, layout, col_to_remove);
        });

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            Self::show_flag_checkboxes(ui, col, layout.idx, true, new_pk_idx);
            ui.label(
                RichText::new("默认值")
                    .small()
                    .color(Color32::from_gray(170)),
            );
            let width = ui.available_width().clamp(110.0, 180.0);
            ui.add_sized(
                [width, 0.0],
                TextEdit::singleline(&mut col.default_value).hint_text("默认值"),
            );
        });
    }

    fn show_narrow_column_row(
        ui: &mut egui::Ui,
        col: &mut ColumnDefinition,
        layout: ColumnRowLayout,
        new_pk_idx: &mut Option<usize>,
        col_to_remove: &mut Option<usize>,
    ) {
        ui.horizontal(|ui| {
            Self::show_row_indicator(ui, layout.is_selected);
            ui.label(
                RichText::new(format!("第 {} 列", layout.idx + 1))
                    .small()
                    .strong(),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                Self::show_delete_button(ui, layout, col_to_remove);
            });
        });

        ui.add_space(4.0);
        ui.label(RichText::new("列名").small().color(Color32::from_gray(170)));
        ui.add_sized(
            [ui.available_width(), 0.0],
            TextEdit::singleline(&mut col.name).hint_text("列名"),
        );

        ui.add_space(4.0);
        ui.label(RichText::new("类型").small().color(Color32::from_gray(170)));
        Self::show_type_combo(ui, layout.idx, col, ui.available_width());

        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            Self::show_flag_checkboxes(ui, col, layout.idx, true, new_pk_idx);
        });

        ui.add_space(4.0);
        ui.label(
            RichText::new("默认值")
                .small()
                .color(Color32::from_gray(170)),
        );
        ui.add_sized(
            [ui.available_width(), 0.0],
            TextEdit::singleline(&mut col.default_value).hint_text("默认值"),
        );
    }

    fn show_row_indicator(ui: &mut egui::Ui, is_selected: bool) {
        if is_selected {
            ui.label(RichText::new(">").color(Color32::from_rgb(100, 180, 255)));
        } else {
            ui.label(RichText::new(" ").monospace());
        }
    }

    fn show_type_combo(ui: &mut egui::Ui, idx: usize, col: &mut ColumnDefinition, width: f32) {
        egui::ComboBox::from_id_salt(format!("col_type_{}", idx))
            .selected_text(col.data_type.display_name())
            .width(width)
            .show_ui(ui, |ui| {
                for t in ColumnType::common_types() {
                    let name = t.display_name();
                    ui.selectable_value(&mut col.data_type, t, name);
                }
            });
    }

    fn show_flag_checkboxes(
        ui: &mut egui::Ui,
        col: &mut ColumnDefinition,
        idx: usize,
        labeled: bool,
        new_pk_idx: &mut Option<usize>,
    ) {
        let mut pk = col.primary_key;
        if ui
            .checkbox(&mut pk, if labeled { "PK" } else { "" })
            .on_hover_text("主键")
            .changed()
        {
            col.primary_key = pk;
            if pk {
                col.nullable = false;
                *new_pk_idx = Some(idx);
            }
        }

        ui.checkbox(&mut col.auto_increment, if labeled { "AI" } else { "" })
            .on_hover_text("自增");

        let mut not_null = !col.nullable;
        if ui
            .checkbox(&mut not_null, if labeled { "NN" } else { "" })
            .on_hover_text("非空")
            .changed()
        {
            col.nullable = !not_null;
        }

        ui.checkbox(&mut col.unique, if labeled { "UQ" } else { "" })
            .on_hover_text("唯一");
    }

    fn show_delete_button(
        ui: &mut egui::Ui,
        layout: ColumnRowLayout,
        col_to_remove: &mut Option<usize>,
    ) {
        if layout.col_count > 1
            && ui
                .small_button("×")
                .on_hover_text(local_shortcut_tooltip(
                    "删除当前列",
                    LocalShortcut::DdlColumnDelete,
                ))
                .clicked()
        {
            *col_to_remove = Some(layout.idx);
        }
    }
}

// ============================================================================
// 工具函数
// ============================================================================

/// 引用标识符
fn quote_identifier(name: &str, db_type: &DatabaseType) -> String {
    match db_type {
        DatabaseType::MySQL => format!("`{}`", name.replace('`', "``")),
        DatabaseType::PostgreSQL | DatabaseType::SQLite => {
            format!("\"{}\"", name.replace('"', "\"\""))
        }
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Key, Modifiers, RawInput};

    fn begin_key_pass(ctx: &egui::Context, key: Key) {
        ctx.begin_pass(RawInput {
            events: vec![Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            modifiers: Modifiers::NONE,
            ..Default::default()
        });
    }

    fn focus_text_input(ctx: &egui::Context) {
        let mut text = String::new();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("ddl dialog shortcut test input").show(ctx, |ui| {
            let response =
                ui.add(egui::TextEdit::singleline(&mut text).id_salt("ddl_shortcut_text_input"));
            response.request_focus();
        });
        let _ = ctx.end_pass();
    }

    #[test]
    fn ddl_dialog_detects_column_next_shortcut_through_scoped_command_id() {
        let ctx = egui::Context::default();
        begin_key_pass(&ctx, Key::J);

        let action = DdlDialog::detect_key_action(&ctx);

        assert_eq!(action, Some(DdlKeyAction::ColumnNext));

        let _ = ctx.end_pass();
    }

    #[test]
    fn ddl_dialog_blocks_column_navigation_text_conflicts_when_text_input_is_focused() {
        let ctx = egui::Context::default();
        focus_text_input(&ctx);
        begin_key_pass(&ctx, Key::J);

        let action = DdlDialog::detect_key_action(&ctx);

        assert_eq!(action, None);

        let _ = ctx.end_pass();
    }

    #[test]
    fn ddl_dialog_row_width_classes_follow_shared_thresholds() {
        assert_eq!(
            DdlDialog::row_width_class(DdlDialog::WIDE_ROW_THRESHOLD),
            ResponsiveRowClass::Wide
        );
        assert_eq!(
            DdlDialog::row_width_class(680.0),
            ResponsiveRowClass::Medium
        );
        assert_eq!(
            DdlDialog::row_width_class(DdlDialog::MEDIUM_ROW_THRESHOLD - 1.0),
            ResponsiveRowClass::Narrow
        );
    }

    #[test]
    fn ddl_dialog_uses_compact_workspace_window_profile() {
        assert_eq!(DdlDialog::WINDOW_WIDTH, 920.0);
        assert_eq!(DdlDialog::WINDOW_HEIGHT, 500.0);
    }

    #[test]
    fn ddl_dialog_compact_heights_stay_below_previous_workspace_defaults() {
        let ctx = egui::Context::default();
        ctx.begin_pass(RawInput::default());
        egui::Window::new("ddl_dialog_compact_height_test").show(&ctx, |ui| {
            let column_height = DdlDialog::column_list_max_height(ui);
            let preview_height = DdlDialog::sql_preview_max_height(ui);

            assert!(column_height >= DdlDialog::COLUMN_LIST_MIN_HEIGHT);
            assert!(column_height <= DdlDialog::COLUMN_LIST_MAX_HEIGHT);
            assert!(preview_height >= DdlDialog::SQL_PREVIEW_MIN_HEIGHT);
            assert!(preview_height <= DdlDialog::SQL_PREVIEW_MAX_HEIGHT);
            assert!(column_height < 320.0);
            assert!(preview_height <= 180.0);
        });
        let _ = ctx.end_pass();
    }
}
