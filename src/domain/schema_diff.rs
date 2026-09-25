//! 单表 schema 差异与只读 SQL 预览。
//!
//! 本模块只消费 [`TableMetadata`]，不会连接数据库或执行迁移。生成的 SQL
//! 仅用于人工审核；SQLite 不支持直接修改约束/列定义的差异会以警告注释
//! 保留，而不是伪装成可安全执行的迁移。

use super::identifier::IdentifierDialect;
use super::metadata::{
    ColumnMetadata, ForeignKeyMetadata, KeyMetadata, SchemaCatalog, TableMetadata,
};
use super::value::{DbTypeFamily, DbTypeInfo};

/// 单表 schema 的不可变快照。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSnapshot {
    /// 快照对应的表元数据。
    pub table: TableMetadata,
}

impl SchemaSnapshot {
    /// 从现有的领域元数据创建快照。
    pub fn new(table: TableMetadata) -> Self {
        Self { table }
    }

    /// 克隆单表元数据，避免快照与可变 catalog 共享所有权。
    pub fn from_table(table: &TableMetadata) -> Self {
        Self::new(table.clone())
    }

    /// 从统一 schema catalog 中选取一张表。
    pub fn from_catalog(catalog: &SchemaCatalog, table_name: &str) -> Option<Self> {
        catalog.table(table_name).map(Self::from_table)
    }

    /// 返回快照中的表元数据。
    pub fn table_metadata(&self) -> &TableMetadata {
        &self.table
    }

    /// 返回表名。
    pub fn table_name(&self) -> &str {
        &self.table.name
    }

    /// 比较当前快照与目标快照。
    pub fn diff(&self, target: &Self) -> SchemaDiff {
        SchemaDiff::between(self, target)
    }
}

impl From<TableMetadata> for SchemaSnapshot {
    fn from(table: TableMetadata) -> Self {
        Self::new(table)
    }
}

impl From<&TableMetadata> for SchemaSnapshot {
    fn from(table: &TableMetadata) -> Self {
        Self::from_table(table)
    }
}

/// 单列定义的修改前后值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnChange {
    pub before: ColumnMetadata,
    pub after: ColumnMetadata,
}

/// 键定义的修改前后值。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyChange {
    pub before: Option<KeyMetadata>,
    pub after: Option<KeyMetadata>,
}

/// 集合型 schema 对象的新增/删除差异。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionDiff<T> {
    pub added: Vec<T>,
    pub removed: Vec<T>,
}

impl<T> CollectionDiff<T> {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty()
    }
}

/// 只读的单表 schema 差异。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDiff {
    /// 目标表名，供 SQL preview 使用。
    pub table_name: String,
    /// 当前快照中的表名。
    pub current_table_name: String,
    pub current_schema: Option<String>,
    /// 目标快照中的 schema。
    pub schema: Option<String>,
    pub table_name_changed: bool,
    pub schema_changed: bool,
    pub added_columns: Vec<ColumnMetadata>,
    pub removed_columns: Vec<ColumnMetadata>,
    pub changed_columns: Vec<ColumnChange>,
    /// `None` 表示主键定义没有变化。
    pub primary_key: Option<KeyChange>,
    pub unique_keys: CollectionDiff<KeyMetadata>,
    pub foreign_keys: CollectionDiff<ForeignKeyMetadata>,
}

impl SchemaDiff {
    /// 比较两个单表快照；`target` 表示期望的结构。
    pub fn between(current: &SchemaSnapshot, target: &SchemaSnapshot) -> Self {
        let (added_columns, removed_columns, changed_columns) =
            diff_columns(&current.table.columns, &target.table.columns);
        let primary_key = (!keys_equal(
            current.table.primary_key.as_ref(),
            target.table.primary_key.as_ref(),
        ))
        .then(|| KeyChange {
            before: current.table.primary_key.clone(),
            after: target.table.primary_key.clone(),
        });

        Self {
            table_name: target.table.name.clone(),
            current_table_name: current.table.name.clone(),
            current_schema: current.table.schema.clone(),
            schema: target.table.schema.clone(),
            table_name_changed: !identifiers_equal(&current.table.name, &target.table.name),
            schema_changed: !optional_identifiers_equal(
                current.table.schema.as_deref(),
                target.table.schema.as_deref(),
            ),
            added_columns,
            removed_columns,
            changed_columns,
            primary_key,
            unique_keys: diff_collection(
                &current.table.unique_keys,
                &target.table.unique_keys,
                key_values_equal,
            ),
            foreign_keys: diff_collection(
                &current.table.foreign_keys,
                &target.table.foreign_keys,
                foreign_keys_equal,
            ),
        }
    }

    /// 是否没有任何差异。
    pub fn is_empty(&self) -> bool {
        !self.table_name_changed
            && !self.schema_changed
            && self.added_columns.is_empty()
            && self.removed_columns.is_empty()
            && self.changed_columns.is_empty()
            && self.primary_key.is_none()
            && self.unique_keys.is_empty()
            && self.foreign_keys.is_empty()
    }

    /// 主键是否发生变化。
    pub fn primary_key_changed(&self) -> bool {
        self.primary_key.is_some()
    }

    /// 生成只读 SQL 预览结果；不会执行任何语句。
    pub fn sql_preview(&self, dialect: IdentifierDialect) -> SqlPreview {
        let mut preview = SqlPreview::default();
        self.add_rename_preview(&mut preview, dialect);
        self.add_column_previews(&mut preview, dialect);
        self.add_unsupported_change_warnings(&mut preview);
        preview
    }

    /// 将 SQL 预览渲染成可直接放入代码面板的文本。
    pub fn to_sql_preview(&self, dialect: IdentifierDialect) -> String {
        self.sql_preview(dialect).render()
    }

    /// 以 SQLite 方言生成 SQL preview。
    pub fn sqlite_sql_preview(&self) -> String {
        self.to_sql_preview(IdentifierDialect::SQLite)
    }

    fn add_rename_preview(&self, preview: &mut SqlPreview, dialect: IdentifierDialect) {
        if !self.table_name_changed || self.schema_changed {
            return;
        }
        preview.statements.push(format!(
            "ALTER TABLE {} RENAME TO {};",
            quote_table(
                dialect,
                self.current_schema.as_deref(),
                &self.current_table_name
            ),
            dialect.quote(&self.table_name),
        ));
    }

    fn add_column_previews(&self, preview: &mut SqlPreview, dialect: IdentifierDialect) {
        let table = quote_table(dialect, self.schema.as_deref(), &self.table_name);
        preview
            .statements
            .extend(self.added_columns.iter().map(|column| {
                format!(
                    "ALTER TABLE {} ADD COLUMN {};",
                    table,
                    column_definition(dialect, column),
                )
            }));
        preview
            .statements
            .extend(self.removed_columns.iter().map(|column| {
                format!(
                    "ALTER TABLE {} DROP COLUMN {};",
                    table,
                    dialect.quote(&column.name),
                )
            }));
    }

    fn add_unsupported_change_warnings(&self, preview: &mut SqlPreview) {
        if self.schema_changed {
            preview.warnings.push(
                "schema 作用域发生变化；未生成跨 schema 的迁移语句，请人工审核。".to_string(),
            );
        }
        if !self.changed_columns.is_empty() {
            let names = self
                .changed_columns
                .iter()
                .map(|change| change.after.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            preview.warnings.push(format!(
                "列定义发生变化（{}）；未生成修改列语句，请人工审核并考虑 SQLite 表重建。",
                names
            ));
        }
        if self.primary_key_changed() {
            preview
                .warnings
                .push("主键定义发生变化；未生成约束迁移语句，请人工审核。".to_string());
        }
        if !self.unique_keys.is_empty() {
            preview
                .warnings
                .push("唯一键定义发生变化；未生成约束迁移语句，请人工审核。".to_string());
        }
        if !self.foreign_keys.is_empty() {
            preview
                .warnings
                .push("外键定义发生变化；未生成约束迁移语句，请人工审核。".to_string());
        }
    }
}

/// SQL preview 的语句和人工审核警告。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SqlPreview {
    pub statements: Vec<String>,
    pub warnings: Vec<String>,
}

impl SqlPreview {
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty() && self.warnings.is_empty()
    }

    pub fn render(&self) -> String {
        let mut lines = self.statements.clone();
        lines.extend(
            self.warnings
                .iter()
                .map(|warning| format!("-- {}", warning)),
        );
        if lines.is_empty() {
            "-- No schema changes detected.".to_string()
        } else {
            lines.join("\n")
        }
    }
}

fn diff_columns(
    current: &[ColumnMetadata],
    target: &[ColumnMetadata],
) -> (Vec<ColumnMetadata>, Vec<ColumnMetadata>, Vec<ColumnChange>) {
    let added = target
        .iter()
        .filter(|column| find_column(current, &column.name).is_none())
        .cloned()
        .collect();
    let removed = current
        .iter()
        .filter(|column| find_column(target, &column.name).is_none())
        .cloned()
        .collect();
    let changed = target
        .iter()
        .filter_map(|after| {
            let before = find_column(current, &after.name)?;
            (!columns_equal(before, after)).then(|| ColumnChange {
                before: before.clone(),
                after: after.clone(),
            })
        })
        .collect();
    (added, removed, changed)
}

fn diff_collection<T, F>(current: &[T], target: &[T], equal: F) -> CollectionDiff<T>
where
    T: Clone,
    F: Fn(&T, &T) -> bool + Copy,
{
    let added = target
        .iter()
        .filter(|item| !current.iter().any(|existing| equal(existing, item)))
        .cloned()
        .collect();
    let removed = current
        .iter()
        .filter(|item| !target.iter().any(|existing| equal(existing, item)))
        .cloned()
        .collect();
    CollectionDiff { added, removed }
}

fn find_column<'a>(columns: &'a [ColumnMetadata], name: &str) -> Option<&'a ColumnMetadata> {
    columns
        .iter()
        .find(|column| identifiers_equal(&column.name, name))
}

fn columns_equal(current: &ColumnMetadata, target: &ColumnMetadata) -> bool {
    identifiers_equal(&current.name, &target.name)
        && current.position == target.position
        && current.type_info == target.type_info
        && current.is_nullable == target.is_nullable
        && current.is_primary_key == target.is_primary_key
        && current.default_value == target.default_value
}

fn keys_equal(current: Option<&KeyMetadata>, target: Option<&KeyMetadata>) -> bool {
    match (current, target) {
        (None, None) => true,
        (Some(current), Some(target)) => {
            optional_identifiers_equal(current.name.as_deref(), target.name.as_deref())
                && identifier_lists_equal(&current.columns, &target.columns)
        }
        _ => false,
    }
}

fn key_values_equal(current: &KeyMetadata, target: &KeyMetadata) -> bool {
    optional_identifiers_equal(current.name.as_deref(), target.name.as_deref())
        && identifier_lists_equal(&current.columns, &target.columns)
}

fn foreign_keys_equal(current: &ForeignKeyMetadata, target: &ForeignKeyMetadata) -> bool {
    optional_identifiers_equal(current.name.as_deref(), target.name.as_deref())
        && identifier_lists_equal(&current.from_columns, &target.from_columns)
        && identifiers_equal(&current.ref_table, &target.ref_table)
        && identifier_lists_equal(&current.ref_columns, &target.ref_columns)
}

fn identifier_lists_equal(current: &[String], target: &[String]) -> bool {
    current.len() == target.len()
        && current
            .iter()
            .zip(target)
            .all(|(current, target)| identifiers_equal(current, target))
}

fn identifiers_equal(current: &str, target: &str) -> bool {
    current.eq_ignore_ascii_case(target)
}

fn optional_identifiers_equal(current: Option<&str>, target: Option<&str>) -> bool {
    match (current, target) {
        (None, None) => true,
        (Some(current), Some(target)) => identifiers_equal(current, target),
        _ => false,
    }
}

fn quote_table(dialect: IdentifierDialect, schema: Option<&str>, table: &str) -> String {
    schema
        .map(|schema| format!("{}.{}", dialect.quote(schema), dialect.quote(table)))
        .unwrap_or_else(|| dialect.quote(table))
}

fn column_definition(dialect: IdentifierDialect, column: &ColumnMetadata) -> String {
    let mut definition = format!(
        "{} {}",
        dialect.quote(&column.name),
        sql_type_name(&column.type_info)
    );
    if !column.is_nullable {
        definition.push_str(" NOT NULL");
    }
    if let Some(default) = column
        .default_value
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        definition.push_str(" DEFAULT ");
        definition.push_str(default);
    }
    definition
}

fn sql_type_name(type_info: &DbTypeInfo) -> String {
    let native_name = type_info.native_name.trim();
    if !native_name.is_empty() {
        return native_name.to_string();
    }
    match type_info.family {
        DbTypeFamily::Bool => "BOOLEAN",
        DbTypeFamily::Integer => "INTEGER",
        DbTypeFamily::Float => "REAL",
        DbTypeFamily::Decimal => "DECIMAL",
        DbTypeFamily::Bytes => "BLOB",
        DbTypeFamily::Date => "DATE",
        DbTypeFamily::Time => "TIME",
        DbTypeFamily::DateTime => "DATETIME",
        DbTypeFamily::Json => "JSON",
        DbTypeFamily::Uuid => "UUID",
        DbTypeFamily::Array => "TEXT",
        DbTypeFamily::Null | DbTypeFamily::Text | DbTypeFamily::Other => "TEXT",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(name: &str, position: usize, native_name: &str, nullable: bool) -> ColumnMetadata {
        ColumnMetadata {
            name: name.to_string(),
            position,
            type_info: DbTypeInfo {
                family: DbTypeFamily::Text,
                native_name: native_name.to_string(),
                nullable: Some(nullable),
            },
            is_nullable: nullable,
            is_primary_key: false,
            default_value: None,
        }
    }

    fn table(columns: Vec<ColumnMetadata>) -> TableMetadata {
        TableMetadata {
            name: "users".to_string(),
            schema: None,
            columns,
            primary_key: None,
            unique_keys: Vec::new(),
            foreign_keys: Vec::new(),
        }
    }

    #[test]
    fn schema_diff_tracks_added_removed_and_changed_columns() {
        let current = SchemaSnapshot::from_table(&table(vec![
            column("id", 0, "INTEGER", false),
            column("name", 1, "TEXT", true),
            column("legacy", 2, "TEXT", true),
        ]));
        let target = SchemaSnapshot::from_table(&table(vec![
            column("id", 0, "INTEGER", false),
            column("name", 1, "VARCHAR(80)", false),
            column("email", 2, "TEXT", true),
        ]));

        let diff = current.diff(&target);

        assert_eq!(diff.added_columns[0].name, "email");
        assert_eq!(diff.removed_columns[0].name, "legacy");
        assert_eq!(diff.changed_columns[0].after.name, "name");
        assert!(!diff.is_empty());
    }

    #[test]
    fn sqlite_sql_preview_quotes_identifiers_and_never_executes() {
        let current =
            SchemaSnapshot::from_table(&table(vec![column("display name", 0, "TEXT", true)]));
        let target = SchemaSnapshot::from_table(&table(vec![
            column("display name", 0, "TEXT", true),
            ColumnMetadata {
                name: "email address".to_string(),
                position: 1,
                type_info: DbTypeInfo {
                    family: DbTypeFamily::Text,
                    native_name: "TEXT".to_string(),
                    nullable: Some(false),
                },
                is_nullable: false,
                is_primary_key: false,
                default_value: Some("'unknown'".to_string()),
            },
        ]));

        let sql = current.diff(&target).sqlite_sql_preview();

        assert!(sql.contains(
            "ALTER TABLE \"users\" ADD COLUMN \"email address\" TEXT NOT NULL DEFAULT 'unknown';"
        ));
        assert!(!sql.contains("execute"));
    }

    #[test]
    fn schema_diff_tracks_constraint_changes_as_review_warnings() {
        let mut current_table = table(vec![column("id", 0, "INTEGER", false)]);
        current_table.primary_key = Some(KeyMetadata {
            name: None,
            columns: vec!["id".to_string()],
        });
        current_table.unique_keys.push(KeyMetadata {
            name: Some("users_email_key".to_string()),
            columns: vec!["email".to_string()],
        });
        current_table.foreign_keys.push(ForeignKeyMetadata {
            name: None,
            from_columns: vec!["role_id".to_string()],
            ref_table: "roles".to_string(),
            ref_columns: vec!["id".to_string()],
        });
        let current = SchemaSnapshot::from_table(&current_table);
        let target = SchemaSnapshot::from_table(&table(vec![column("id", 0, "INTEGER", false)]));

        let diff = current.diff(&target);
        let preview = diff.sqlite_sql_preview();

        assert!(diff.primary_key_changed());
        assert_eq!(diff.unique_keys.removed.len(), 1);
        assert_eq!(diff.foreign_keys.removed.len(), 1);
        assert!(preview.contains("主键定义发生变化"));
        assert!(preview.contains("外键定义发生变化"));
    }

    #[test]
    fn empty_schema_diff_renders_explicit_no_change_preview() {
        let snapshot = SchemaSnapshot::from_table(&table(vec![column("id", 0, "INTEGER", false)]));
        let diff = snapshot.diff(&snapshot);

        assert!(diff.is_empty());
        assert_eq!(diff.sqlite_sql_preview(), "-- No schema changes detected.");
    }
}
