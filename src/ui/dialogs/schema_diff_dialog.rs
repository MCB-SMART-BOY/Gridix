//! Schema 对比对话框：比较活动连接内两张表的 schema，并预览迁移 SQL。
//!
//! 只读能力：仅使用已加载的 schema 目录构造快照，不触发新的目录加载，
//! 也不对数据库执行任何写入。差异计算与 SQL 预览由
//! [`crate::domain::schema_diff`] 提供，本模块只负责选择、呈现与错误提示。

use super::common::{DialogStyle, DialogWindow};
use crate::domain::identifier::IdentifierDialect;
use crate::domain::metadata::{ColumnMetadata, ForeignKeyMetadata, KeyMetadata, SchemaCatalog};
use crate::domain::schema_diff::{ColumnChange, SchemaDiff, SchemaSnapshot};
use crate::types::DatabaseType;
use crate::ui::styles::{ACCENT_BLUE, DANGER, GRAY, MUTED, SUCCESS};
use crate::ui::{LocalShortcut, local_shortcut_text};
use egui::{Context, RichText, ScrollArea, TextEdit, Ui};

/// Schema 对比无法进行的原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchemaDiffError {
    /// 源表或目标表尚未选择。
    TableNotSelected,
    /// 目录中没有该表（目录未加载或表名过期）。
    TableNotLoaded(String),
}

impl std::fmt::Display for SchemaDiffError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TableNotSelected => write!(formatter, "请选择源表与目标表"),
            Self::TableNotLoaded(table) => write!(
                formatter,
                "已加载的 schema 目录中没有表 {table}：请重新加载目录后再试"
            ),
        }
    }
}

impl std::error::Error for SchemaDiffError {}

/// 对话框状态：选择两张表并保存计算出的差异。
#[derive(Debug, Clone, Default)]
pub struct SchemaDiffDialogState {
    pub show: bool,
    /// 源表（当前结构）
    pub source_table: Option<String>,
    /// 目标表（期望结构）
    pub target_table: Option<String>,
    /// 最近一次对比结果
    pub diff: Option<SchemaDiff>,
    /// 最近一次对比失败的原因
    pub error: Option<String>,
}

impl SchemaDiffDialogState {
    /// 打开对话框，并把给定表作为源表。
    pub fn open_for(&mut self, source_table: Option<String>) {
        self.show = true;
        self.source_table = source_table;
        self.target_table = None;
        self.diff = None;
        self.error = None;
    }

    /// 关闭对话框并清空对比结果。
    pub fn close(&mut self) {
        self.show = false;
        self.diff = None;
        self.error = None;
    }

    /// 按当前选择重新计算差异。
    pub fn compare(&mut self, catalog: &SchemaCatalog) {
        self.diff = None;
        self.error = None;
        match compare_tables(
            catalog,
            self.source_table.as_deref(),
            self.target_table.as_deref(),
        ) {
            Ok(diff) => self.diff = Some(diff),
            Err(error) => self.error = Some(error.to_string()),
        }
    }
}

/// 从目录构造两张表的快照并求差；`target` 表示期望结构。
///
/// 本对话框比较的是两张真实存在的表，因此把期望快照的表身份对齐到源表：
/// 预览表达的是“把源表改造成目标表的结构”。否则领域层会按“重命名 + 在目标表上
/// 增删列”的语义生成语句（那套语义面向“同一张表的期望结构”），对两张已存在的
/// 表既不成立也有破坏性。
pub fn compare_tables(
    catalog: &SchemaCatalog,
    source: Option<&str>,
    target: Option<&str>,
) -> Result<SchemaDiff, SchemaDiffError> {
    let (Some(source), Some(target)) = (source, target) else {
        return Err(SchemaDiffError::TableNotSelected);
    };
    let current = snapshot_for_table(catalog, source)
        .ok_or_else(|| SchemaDiffError::TableNotLoaded(source.to_string()))?;
    let mut desired = snapshot_for_table(catalog, target)
        .ok_or_else(|| SchemaDiffError::TableNotLoaded(target.to_string()))?;
    desired.table.name = current.table.name.clone();
    desired.table.schema = current.table.schema.clone();
    Ok(SchemaDiff::between(&current, &desired))
}

/// 按名字取快照，优先精确匹配。
///
/// 下拉框的选项来自目录本身，而领域层的 [`SchemaCatalog::table`] 是大小写不敏感
/// 查找并取首个命中：PostgreSQL 允许 `"Users"` 与 `users` 共存，只做不敏感查找
/// 会静默比较另一张表。
fn snapshot_for_table(catalog: &SchemaCatalog, table_name: &str) -> Option<SchemaSnapshot> {
    catalog
        .tables
        .iter()
        .find(|table| table.name == table_name)
        .map(SchemaSnapshot::from_table)
        .or_else(|| SchemaSnapshot::from_catalog(catalog, table_name))
}

/// 数据库类型对应的 SQL 预览方言。
pub(crate) const fn preview_dialect(db_type: DatabaseType) -> IdentifierDialect {
    match db_type {
        DatabaseType::SQLite => IdentifierDialect::SQLite,
        DatabaseType::PostgreSQL => IdentifierDialect::PostgreSql,
        DatabaseType::MySQL => IdentifierDialect::MySql,
    }
}

/// 单列定义的可读描述。
fn describe_column(column: &ColumnMetadata) -> String {
    let nullability = if column.is_nullable {
        "NULL"
    } else {
        "NOT NULL"
    };
    match column.default_value.as_deref() {
        Some(default) => format!(
            "{} {} {} DEFAULT {}",
            column.name, column.type_info.native_name, nullability, default
        ),
        None => format!(
            "{} {} {}",
            column.name, column.type_info.native_name, nullability
        ),
    }
}

/// 键定义的可读描述。
fn describe_key(key: &KeyMetadata) -> String {
    let name = key.name.as_deref().unwrap_or("(未命名)");
    format!("{} ({})", name, key.columns.join(", "))
}

/// 外键定义的可读描述。
fn describe_foreign_key(foreign_key: &ForeignKeyMetadata) -> String {
    let name = foreign_key.name.as_deref().unwrap_or("(未命名)");
    format!(
        "{} ({}) → {}({})",
        name,
        foreign_key.from_columns.join(", "),
        foreign_key.ref_table,
        foreign_key.ref_columns.join(", ")
    )
}

fn describe_column_change(change: &ColumnChange) -> String {
    format!(
        "~ {} {} → {}",
        change.before.name,
        describe_column(&change.before),
        describe_column(&change.after)
    )
}

fn section_header(ui: &mut Ui, title: &str, count: usize) {
    ui.add_space(crate::ui::styles::SPACING_SM);
    ui.label(
        RichText::new(format!("{title} ({count})"))
            .strong()
            .color(ACCENT_BLUE),
    );
}

fn render_column_sections(ui: &mut Ui, diff: &SchemaDiff) {
    section_header(ui, "新增列", diff.added_columns.len());
    for column in &diff.added_columns {
        ui.colored_label(SUCCESS, format!("+ {}", describe_column(column)));
    }
    section_header(ui, "删除列", diff.removed_columns.len());
    for column in &diff.removed_columns {
        ui.colored_label(DANGER, format!("- {}", describe_column(column)));
    }
    section_header(ui, "修改列", diff.changed_columns.len());
    for change in &diff.changed_columns {
        ui.colored_label(ACCENT_BLUE, describe_column_change(change));
    }
}

fn render_primary_key_section(ui: &mut Ui, diff: &SchemaDiff) {
    section_header(ui, "主键", usize::from(diff.primary_key.is_some()));
    let Some(change) = &diff.primary_key else {
        return;
    };
    let before = change
        .before
        .as_ref()
        .map_or_else(|| "(无主键)".to_string(), describe_key);
    let after = change
        .after
        .as_ref()
        .map_or_else(|| "(无主键)".to_string(), describe_key);
    ui.colored_label(DANGER, format!("~ {before} → {after}"));
}

fn render_key_sections(ui: &mut Ui, diff: &SchemaDiff) {
    render_primary_key_section(ui, diff);
    section_header(
        ui,
        "唯一键",
        diff.unique_keys.added.len() + diff.unique_keys.removed.len(),
    );
    for key in &diff.unique_keys.added {
        ui.colored_label(SUCCESS, format!("+ {}", describe_key(key)));
    }
    for key in &diff.unique_keys.removed {
        ui.colored_label(DANGER, format!("- {}", describe_key(key)));
    }
    section_header(
        ui,
        "外键",
        diff.foreign_keys.added.len() + diff.foreign_keys.removed.len(),
    );
    for foreign_key in &diff.foreign_keys.added {
        ui.colored_label(SUCCESS, format!("+ {}", describe_foreign_key(foreign_key)));
    }
    for foreign_key in &diff.foreign_keys.removed {
        ui.colored_label(DANGER, format!("- {}", describe_foreign_key(foreign_key)));
    }
}

fn render_sql_preview(ui: &mut Ui, diff: &SchemaDiff, db_type: DatabaseType) {
    let preview = diff.sql_preview(preview_dialect(db_type));
    if preview.is_empty() {
        return;
    }
    section_header(ui, "迁移 SQL 预览", preview.statements.len());
    let mut text = preview.render();
    ui.add(
        TextEdit::multiline(&mut text)
            .code_editor()
            .desired_width(f32::INFINITY)
            .interactive(false),
    );
}

fn render_unavailable(ui: &mut Ui, message: &str) {
    ui.colored_label(DANGER, message);
}

/// 表选择下拉框。
fn table_picker(
    ui: &mut Ui,
    id: &str,
    label: &str,
    catalog: &SchemaCatalog,
    selected: &mut Option<String>,
) {
    ui.label(RichText::new(label).color(MUTED));
    let text = selected.clone().unwrap_or_else(|| "选择表…".to_string());
    egui::ComboBox::from_id_salt(id)
        .selected_text(text)
        .show_ui(ui, |ui| {
            for table in &catalog.tables {
                let is_selected = selected.as_deref() == Some(table.name.as_str());
                if ui.selectable_label(is_selected, &table.name).clicked() {
                    *selected = Some(table.name.clone());
                }
            }
        });
}

fn render_footer(ui: &mut Ui) {
    ui.add_space(crate::ui::styles::SPACING_MD);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{} 关闭",
                local_shortcut_text(LocalShortcut::Dismiss)
            ))
            .small()
            .color(MUTED),
        );
    });
}

/// Schema 对比对话框渲染入口。
pub struct SchemaDiffDialog;

impl SchemaDiffDialog {
    /// 渲染对话框；`catalog` 为活动连接已加载的 schema 目录。
    pub fn show(
        ctx: &Context,
        state: &mut SchemaDiffDialogState,
        catalog: Option<&SchemaCatalog>,
        db_type: DatabaseType,
    ) {
        if !state.show {
            return;
        }

        let style = DialogStyle::LARGE;
        DialogWindow::resizable(ctx, "dialog.schema_diff", "Schema 对比", &style).show_blocking(
            ctx,
            |ui| {
                let Some(catalog) = catalog else {
                    render_unavailable(
                        ui,
                        "当前连接的 schema 目录尚未加载：请先在侧边栏选中该表以触发目录加载",
                    );
                    render_footer(ui);
                    return;
                };

                ui.horizontal(|ui| {
                    table_picker(
                        ui,
                        "schema_diff_source_picker",
                        "源表",
                        catalog,
                        &mut state.source_table,
                    );
                    table_picker(
                        ui,
                        "schema_diff_target_picker",
                        "目标表",
                        catalog,
                        &mut state.target_table,
                    );
                });
                // 每帧按当前目录重算：目录可能被重新加载（重连、切换数据库、手动刷新），
                // 缓存下来的差异会在那时过期；两张表的比较成本很小。
                state.compare(catalog);

                ui.separator();
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let Some(diff) = state.diff.as_ref() else {
                            let message = state
                                .error
                                .clone()
                                .unwrap_or_else(|| "选择源表与目标表后显示差异".to_string());
                            ui.colored_label(GRAY, message);
                            return;
                        };
                        if diff.is_empty() {
                            ui.colored_label(SUCCESS, "两张表结构一致");
                        }
                        render_column_sections(ui, diff);
                        render_key_sections(ui, diff);
                        render_sql_preview(ui, diff, db_type);
                    });
                render_footer(ui);
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ids::SchemaRevision;
    use crate::domain::value::{DbTypeFamily, DbTypeInfo};

    fn column(name: &str, native: &str, nullable: bool) -> ColumnMetadata {
        ColumnMetadata {
            name: name.to_string(),
            position: 1,
            type_info: DbTypeInfo {
                family: DbTypeFamily::Integer,
                native_name: native.to_string(),
                nullable: Some(nullable),
            },
            is_nullable: nullable,
            is_primary_key: false,
            default_value: None,
        }
    }

    fn catalog() -> SchemaCatalog {
        SchemaCatalog {
            revision: SchemaRevision(1),
            tables: vec![
                crate::domain::metadata::TableMetadata {
                    name: "orders".to_string(),
                    schema: None,
                    columns: vec![
                        column("id", "INTEGER", false),
                        column("total", "INTEGER", true),
                    ],
                    primary_key: Some(KeyMetadata {
                        name: Some("orders_pk".to_string()),
                        columns: vec!["id".to_string()],
                    }),
                    unique_keys: Vec::new(),
                    foreign_keys: Vec::new(),
                },
                crate::domain::metadata::TableMetadata {
                    name: "orders_archive".to_string(),
                    schema: None,
                    columns: vec![
                        column("id", "INTEGER", false),
                        column("total", "BIGINT", false),
                    ],
                    primary_key: Some(KeyMetadata {
                        name: Some("orders_archive_pk".to_string()),
                        columns: vec!["id".to_string()],
                    }),
                    unique_keys: Vec::new(),
                    foreign_keys: Vec::new(),
                },
            ],
        }
    }

    #[test]
    fn compare_tables_reports_column_nullability_change() {
        let diff = compare_tables(&catalog(), Some("orders"), Some("orders_archive"))
            .expect("both tables exist in the catalog");

        assert_eq!(diff.changed_columns.len(), 1);
        assert_eq!(diff.changed_columns[0].before.name, "total");
        assert!(diff.changed_columns[0].before.is_nullable);
        assert!(!diff.changed_columns[0].after.is_nullable);
        assert!(diff.added_columns.is_empty());
        assert!(diff.removed_columns.is_empty());
    }

    /// 目标表比源表多一列，使迁移预览必然包含一条真实语句。
    fn catalog_with_target_only_column() -> SchemaCatalog {
        let mut catalog = catalog();
        let target = catalog
            .tables
            .iter_mut()
            .find(|table| table.name == "orders_archive")
            .expect("fixture must contain orders_archive");
        target.columns.push(column("archived_at", "TEXT", true));
        catalog
    }

    #[test]
    fn compare_tables_previews_changes_on_the_source_table_only() {
        let diff = compare_tables(
            &catalog_with_target_only_column(),
            Some("orders"),
            Some("orders_archive"),
        )
        .expect("both tables exist in the catalog");

        assert!(!diff.table_name_changed);
        assert!(!diff.schema_changed);
        assert_eq!(diff.added_columns.len(), 1);

        let preview = diff.sql_preview(preview_dialect(DatabaseType::SQLite));
        assert_eq!(
            preview.statements.len(),
            1,
            "expected one statement: {:?}",
            preview.statements
        );
        assert!(
            preview.statements[0].starts_with("ALTER TABLE \"orders\" ADD COLUMN \"archived_at\""),
            "预览必须改造源表而不是目标表: {:?}",
            preview.statements
        );
        assert!(
            !preview
                .statements
                .iter()
                .any(|statement| statement.contains("RENAME")),
            "比较两张已存在的表不应生成 RENAME: {:?}",
            preview.statements
        );
    }

    #[test]
    fn compare_tables_requires_both_selections() {
        assert_eq!(
            compare_tables(&catalog(), Some("orders"), None),
            Err(SchemaDiffError::TableNotSelected)
        );
        assert_eq!(
            compare_tables(&catalog(), None, None),
            Err(SchemaDiffError::TableNotSelected)
        );
    }

    #[test]
    fn compare_tables_resolves_exact_names_before_case_insensitive_lookup() {
        let mut catalog = catalog();
        catalog.tables.push(crate::domain::metadata::TableMetadata {
            name: "Users".to_string(),
            schema: None,
            columns: vec![column("id", "INTEGER", false)],
            primary_key: None,
            unique_keys: Vec::new(),
            foreign_keys: Vec::new(),
        });
        catalog.tables.push(crate::domain::metadata::TableMetadata {
            name: "users".to_string(),
            schema: None,
            columns: vec![
                column("id", "INTEGER", false),
                column("email", "TEXT", true),
            ],
            primary_key: None,
            unique_keys: Vec::new(),
            foreign_keys: Vec::new(),
        });

        // 大小写不敏感查找会先命中 "Users"；精确匹配必须选中 "users"。
        let diff = compare_tables(&catalog, Some("users"), Some("Users"))
            .expect("both tables exist in the catalog");

        assert_eq!(diff.current_table_name, "users");
        assert_eq!(diff.removed_columns.len(), 1);
        assert_eq!(diff.removed_columns[0].name, "email");
    }

    #[test]
    fn compare_tables_reports_missing_table() {
        assert_eq!(
            compare_tables(&catalog(), Some("orders"), Some("missing")),
            Err(SchemaDiffError::TableNotLoaded("missing".to_string()))
        );
    }

    #[test]
    fn state_compare_sets_diff_and_clears_on_close() {
        let mut state = SchemaDiffDialogState::default();
        state.open_for(Some("orders".to_string()));
        state.target_table = Some("orders_archive".to_string());
        state.compare(&catalog());
        assert!(state.diff.is_some());
        assert!(state.error.is_none());

        state.close();
        assert!(!state.show);
        assert!(state.diff.is_none());
    }

    #[test]
    fn state_compare_records_error_for_unknown_target() {
        let mut state = SchemaDiffDialogState::default();
        state.open_for(Some("orders".to_string()));
        state.target_table = Some("missing".to_string());
        state.compare(&catalog());

        assert!(state.diff.is_none());
        assert_eq!(
            state.error,
            Some(SchemaDiffError::TableNotLoaded("missing".to_string()).to_string())
        );
    }

    #[test]
    fn preview_dialect_matches_database_type() {
        assert!(matches!(
            preview_dialect(DatabaseType::SQLite),
            IdentifierDialect::SQLite
        ));
        assert!(matches!(
            preview_dialect(DatabaseType::PostgreSQL),
            IdentifierDialect::PostgreSql
        ));
        assert!(matches!(
            preview_dialect(DatabaseType::MySQL),
            IdentifierDialect::MySql
        ));
    }
}
