//! DDL 操作对话框
//!
//! 提供创建表、修改表结构等 DDL 操作的 UI。
//! 支持 Helix 风格的键盘导航。

use super::keyboard::{self, DialogAction, ListNavigation};
use crate::database::DatabaseType;
use egui::{self, Color32, Key, RichText, TextEdit};

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
            Self::Decimal { precision: 10, scale: 2 },
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

        let mut sql = format!(
            "CREATE TABLE {} (\n{}\n)",
            table_name,
            columns.join(",\n")
        );

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

impl DdlDialog {
    /// 显示创建表对话框
    pub fn show_create_table(
        ctx: &egui::Context,
        state: &mut DdlDialogState,
    ) -> Option<String> {
        if !state.show {
            return None;
        }

        let mut result: Option<String> = None;
        let mut should_close = false;

        // 键盘快捷键处理（仅在没有文本框焦点时）
        if !keyboard::has_text_focus(ctx) {
            // Esc/q 关闭
            if keyboard::handle_close_keys(ctx) {
                state.close();
                return None;
            }

            // Enter 创建表
            if let DialogAction::Confirm = keyboard::handle_dialog_keys(ctx) {
                match state.table.validate() {
                    Ok(()) => {
                        result = Some(state.table.to_create_sql());
                        state.close();
                        return result;
                    }
                    Err(e) => {
                        state.error = Some(e);
                    }
                }
            }

            // 列导航
            let col_count = state.table.columns.len();
            match keyboard::handle_list_navigation(ctx) {
                ListNavigation::Up => {
                    if state.selected_column > 0 {
                        state.selected_column -= 1;
                    }
                }
                ListNavigation::Down => {
                    if state.selected_column < col_count.saturating_sub(1) {
                        state.selected_column += 1;
                    }
                }
                ListNavigation::Start => {
                    state.selected_column = 0;
                }
                ListNavigation::End => {
                    state.selected_column = col_count.saturating_sub(1);
                }
                ListNavigation::Delete => {
                    // dd 删除当前列
                    if col_count > 1 {
                        state.table.columns.remove(state.selected_column);
                        if state.selected_column >= state.table.columns.len() {
                            state.selected_column = state.table.columns.len().saturating_sub(1);
                        }
                    }
                }
                ListNavigation::AddBelow => {
                    // o 在下方添加列
                    let insert_pos = (state.selected_column + 1).min(col_count);
                    state.table.columns.insert(insert_pos, ColumnDefinition::default());
                    state.selected_column = insert_pos;
                }
                ListNavigation::AddAbove => {
                    // O 在上方添加列
                    state.table.columns.insert(state.selected_column, ColumnDefinition::default());
                }
                _ => {}
            }

            // 空格切换主键
            ctx.input(|i| {
                if i.key_pressed(Key::Space) && i.modifiers.is_none() {
                    if let Some(col) = state.table.columns.get_mut(state.selected_column) {
                        let new_pk = !col.primary_key;
                        col.primary_key = new_pk;
                        if new_pk {
                            col.nullable = false;
                            // 取消其他列的主键
                            for (idx, other_col) in state.table.columns.iter_mut().enumerate() {
                                if idx != state.selected_column {
                                    other_col.primary_key = false;
                                }
                            }
                        }
                    }
                }
            });
        }

        egui::Window::new("创建表")
            .collapsible(false)
            .resizable(true)
            .min_width(600.0)
            .min_height(400.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // 表名输入
                    ui.horizontal(|ui| {
                        ui.label("表名:");
                        ui.add(
                            TextEdit::singleline(&mut state.table.name)
                                .desired_width(200.0)
                                .hint_text("输入表名"),
                        );

                        ui.add_space(20.0);

                        ui.label("注释:");
                        ui.add(
                            TextEdit::singleline(&mut state.table.comment)
                                .desired_width(200.0)
                                .hint_text("可选"),
                        );
                    });

                    ui.add_space(8.0);
                    ui.separator();

                    // 列定义区域
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("列定义").strong());
                        ui.label(
                            RichText::new("[j/k 移动 | o/O 添加 | dd 删除 | Space 切换主键]")
                                .small()
                                .color(Color32::from_rgb(120, 120, 120)),
                        );
                        
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("+ 添加列 [o]").clicked() {
                                state.table.columns.push(ColumnDefinition::default());
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // 列表头
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

                    // 列列表
                    let mut col_to_remove: Option<usize> = None;
                    let mut new_pk_idx: Option<usize> = None;
                    let col_count = state.table.columns.len();
                    
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for idx in 0..col_count {
                                let is_selected = idx == state.selected_column;
                                let col = &mut state.table.columns[idx];
                                
                                // 选中行高亮背景
                                let row_response = ui.horizontal(|ui| {
                                    // 选中指示器
                                    if is_selected {
                                        ui.label(RichText::new(">").color(Color32::from_rgb(100, 180, 255)));
                                    } else {
                                        ui.label(RichText::new(" ").monospace());
                                    }
                                    // 列名
                                    ui.add(
                                        TextEdit::singleline(&mut col.name)
                                            .desired_width(100.0)
                                            .hint_text("列名"),
                                    );

                                    // 类型选择
                                    egui::ComboBox::from_id_salt(format!("col_type_{}", idx))
                                        .selected_text(col.data_type.display_name())
                                        .width(120.0)
                                        .show_ui(ui, |ui| {
                                            for t in ColumnType::common_types() {
                                                let name = t.display_name();
                                                ui.selectable_value(&mut col.data_type, t, name);
                                            }
                                        });

                                    // 主键
                                    let mut pk = col.primary_key;
                                    if ui.checkbox(&mut pk, "")
                                        .on_hover_text("主键")
                                        .changed()
                                    {
                                        col.primary_key = pk;
                                        if pk {
                                            col.nullable = false;
                                            new_pk_idx = Some(idx);
                                        }
                                    }

                                    // 自增
                                    ui.checkbox(&mut col.auto_increment, "")
                                        .on_hover_text("自增");

                                    // 非空
                                    let mut not_null = !col.nullable;
                                    if ui.checkbox(&mut not_null, "")
                                        .on_hover_text("非空")
                                        .changed()
                                    {
                                        col.nullable = !not_null;
                                    }

                                    // 唯一
                                    ui.checkbox(&mut col.unique, "")
                                        .on_hover_text("唯一");

                                    // 默认值
                                    ui.add(
                                        TextEdit::singleline(&mut col.default_value)
                                            .desired_width(80.0)
                                            .hint_text("默认值"),
                                    );

                                    // 删除按钮
                                    if col_count > 1
                                        && ui.small_button("×")
                                            .on_hover_text("删除列 [dd]")
                                            .clicked()
                                        {
                                            col_to_remove = Some(idx);
                                        }
                                });

                                // 点击行选中
                                if row_response.response.clicked() {
                                    state.selected_column = idx;
                                }
                            }
                        });

                    // 处理主键唯一性（在循环外）
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

                    ui.add_space(8.0);
                    ui.separator();

                    // 预览 SQL
                    ui.collapsing("预览 SQL", |ui| {
                        let sql = state.table.to_create_sql();
                        ui.add(
                            TextEdit::multiline(&mut sql.as_str())
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(6),
                        );
                    });

                    // 错误信息
                    if let Some(err) = &state.error {
                        ui.add_space(4.0);
                        ui.label(RichText::new(err).color(Color32::from_rgb(255, 100, 100)));
                    }

                    ui.add_space(8.0);

                    // 快捷键提示
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("快捷键: Esc/q 关闭 | Enter 创建")
                                .small()
                                .color(Color32::from_rgb(120, 120, 120)),
                        );
                    });

                    ui.add_space(4.0);

                    // 按钮
                    ui.horizontal(|ui| {
                        if ui.button("创建表 [Enter]").clicked() {
                            match state.table.validate() {
                                Ok(()) => {
                                    result = Some(state.table.to_create_sql());
                                    should_close = true;
                                }
                                Err(e) => {
                                    state.error = Some(e);
                                }
                            }
                        }

                        if ui.button("取消 [Esc]").clicked() {
                            should_close = true;
                        }
                    });
                });
            });

        if should_close {
            state.close();
        }

        result
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

