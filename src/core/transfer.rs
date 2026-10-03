//! 统一数据传输管线
//!
//! 将导入和导出统一到同一套 session / preview / plan / execution 模型中，
//! 让 UI 只负责收集配置与展示结果，核心逻辑留在 core 层。

use super::export::{
    CsvImportConfig as LegacyCsvImportConfig, ExportFormat as LegacyExportFormat, ExportOptions,
    JsonImportConfig as LegacyJsonImportConfig, SqlDialect, filter_result_for_export,
    import_csv_to_sql, import_json_to_sql, preview_csv, preview_export, preview_json,
    render_export_content_for_transfer,
};
use crate::domain::result::ResultSet;
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferDirection {
    Import,
    #[default]
    Export,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransferFormat {
    #[default]
    Csv,
    Tsv,
    Sql,
    Json,
}

impl TransferFormat {
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Tsv => "tsv",
            Self::Sql => "sql",
            Self::Json => "json",
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Csv => "CSV",
            Self::Tsv => "TSV",
            Self::Sql => "SQL",
            Self::Json => "JSON",
        }
    }
}

impl From<LegacyExportFormat> for TransferFormat {
    fn from(value: LegacyExportFormat) -> Self {
        match value {
            LegacyExportFormat::Csv => Self::Csv,
            LegacyExportFormat::Tsv => Self::Tsv,
            LegacyExportFormat::Sql => Self::Sql,
            LegacyExportFormat::Json => Self::Json,
        }
    }
}

impl From<TransferFormat> for LegacyExportFormat {
    fn from(value: TransferFormat) -> Self {
        match value {
            TransferFormat::Csv => Self::Csv,
            TransferFormat::Tsv => Self::Tsv,
            TransferFormat::Sql => Self::Sql,
            TransferFormat::Json => Self::Json,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferField {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransferSchema {
    pub source_name: Option<String>,
    pub target_name: Option<String>,
    pub fields: Vec<TransferField>,
    pub total_rows: Option<usize>,
}

impl TransferSchema {
    pub fn from_columns(
        source_name: Option<String>,
        target_name: Option<String>,
        columns: &[String],
        total_rows: Option<usize>,
    ) -> Self {
        Self {
            source_name,
            target_name,
            fields: columns
                .iter()
                .cloned()
                .map(|name| TransferField { name })
                .collect(),
            total_rows,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferFieldMapping {
    pub source_index: usize,
    pub target_name: String,
    pub included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransferMapping {
    pub fields: Vec<TransferFieldMapping>,
}

impl TransferMapping {
    pub fn from_columns(columns: &[String]) -> Self {
        Self {
            fields: columns
                .iter()
                .enumerate()
                .map(|(idx, name)| TransferFieldMapping {
                    source_index: idx,
                    target_name: name.clone(),
                    included: true,
                })
                .collect(),
        }
    }

    pub fn from_selection(columns: &[String], selected_indices: &[usize]) -> Self {
        let selected: HashSet<usize> = selected_indices.iter().copied().collect();
        Self {
            fields: columns
                .iter()
                .enumerate()
                .map(|(idx, name)| TransferFieldMapping {
                    source_index: idx,
                    target_name: name.clone(),
                    included: selected.is_empty() || selected.contains(&idx),
                })
                .collect(),
        }
    }

    pub fn selected_indices(&self) -> Vec<usize> {
        self.fields
            .iter()
            .filter(|field| field.included)
            .map(|field| field.source_index)
            .collect()
    }

    pub fn selected_count(&self) -> usize {
        self.fields.iter().filter(|field| field.included).count()
    }

    pub fn target_columns(&self) -> Vec<String> {
        self.fields
            .iter()
            .filter(|field| field.included)
            .map(|field| field.target_name.clone())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferRowWindow {
    pub start_row: usize,
    pub row_limit: usize,
    pub preview_rows: usize,
}

impl Default for TransferRowWindow {
    fn default() -> Self {
        Self {
            start_row: 0,
            row_limit: 0,
            preview_rows: 10,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferDelimitedOptions {
    pub delimiter: char,
    pub quote_char: char,
    pub include_header: bool,
    pub has_header: bool,
    pub skip_rows: usize,
    pub max_rows: usize,
}

impl Default for TransferDelimitedOptions {
    fn default() -> Self {
        Self {
            delimiter: ',',
            quote_char: '"',
            include_header: true,
            has_header: true,
            skip_rows: 0,
            max_rows: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferJsonOptions {
    pub pretty: bool,
    pub json_path: Option<String>,
    pub flatten_nested: bool,
    pub max_rows: usize,
}

impl Default for TransferJsonOptions {
    fn default() -> Self {
        Self {
            pretty: true,
            json_path: None,
            flatten_nested: false,
            max_rows: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferSqlOptions {
    pub use_transaction: bool,
    pub batch_size: usize,
    pub strip_comments: bool,
    pub strip_empty_lines: bool,
    pub stop_on_error: bool,
}

impl Default for TransferSqlOptions {
    fn default() -> Self {
        Self {
            use_transaction: true,
            batch_size: 100,
            strip_comments: true,
            strip_empty_lines: true,
            stop_on_error: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferFormatOptions {
    Delimited(TransferDelimitedOptions),
    Json(TransferJsonOptions),
    Sql(TransferSqlOptions),
}

impl Default for TransferFormatOptions {
    fn default() -> Self {
        Self::Delimited(TransferDelimitedOptions::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransferSession {
    pub direction: TransferDirection,
    pub format: TransferFormat,
    /// SQL generation dialect for the target database, independent of file format.
    pub sql_dialect: SqlDialect,
    pub schema: TransferSchema,
    pub mapping: TransferMapping,
    pub row_window: TransferRowWindow,
    pub options: TransferFormatOptions,
}

impl TransferSession {
    pub fn delimited_options(&self) -> Option<&TransferDelimitedOptions> {
        match &self.options {
            TransferFormatOptions::Delimited(options) => Some(options),
            _ => None,
        }
    }

    pub fn json_options(&self) -> Option<&TransferJsonOptions> {
        match &self.options {
            TransferFormatOptions::Json(options) => Some(options),
            _ => None,
        }
    }

    pub fn sql_options(&self) -> Option<&TransferSqlOptions> {
        match &self.options {
            TransferFormatOptions::Sql(options) => Some(options),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransferPreview {
    pub schema: TransferSchema,
    pub mapping: TransferMapping,
    pub sample_rows: Vec<Vec<String>>,
    pub total_rows: usize,
    pub warnings: Vec<String>,
    pub statement_count: usize,
    pub rendered_text: Option<String>,
    pub sql_statements: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferExecutionPayload {
    FileContent(String),
    SqlStatements(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferExecutionPlan {
    pub session: TransferSession,
    pub schema: TransferSchema,
    pub mapping: TransferMapping,
    pub warnings: Vec<String>,
    pub total_rows: usize,
    pub statement_count: usize,
    pub payload: TransferExecutionPayload,
}

impl TransferExecutionPlan {
    pub fn rendered_text(&self) -> Option<&str> {
        match &self.payload {
            TransferExecutionPayload::FileContent(content) => Some(content.as_str()),
            TransferExecutionPayload::SqlStatements(_) => None,
        }
    }

    pub fn into_rendered_text(self) -> Result<String, String> {
        match self.payload {
            TransferExecutionPayload::FileContent(content) => Ok(content),
            TransferExecutionPayload::SqlStatements(_) => {
                Err("当前传输计划不是文件内容导出".to_string())
            }
        }
    }

    pub fn sql_statements(&self) -> Option<&[String]> {
        match &self.payload {
            TransferExecutionPayload::SqlStatements(statements) => Some(statements.as_slice()),
            TransferExecutionPayload::FileContent(_) => None,
        }
    }

    pub fn into_sql_statements(self) -> Result<Vec<String>, String> {
        match self.payload {
            TransferExecutionPayload::SqlStatements(statements) => Ok(statements),
            TransferExecutionPayload::FileContent(_) => {
                Err("当前传输计划不是 SQL 语句执行计划".to_string())
            }
        }
    }
}

pub fn preview_export_transfer(
    result: &ResultSet,
    session: &TransferSession,
) -> Result<TransferPreview, String> {
    ensure_direction(session, TransferDirection::Export)?;
    let options = export_options_from_session(session)?;
    let table_name = export_target_name(session);
    let filtered = filter_result_for_export(result, &options);

    if filtered.columns.is_empty() {
        return Err("未选择任何列".to_string());
    }

    let preview_rows = session.row_window.preview_rows.max(1);
    let column_names = filtered.column_names();
    let schema = TransferSchema::from_columns(
        session.schema.source_name.clone(),
        Some(table_name.clone()),
        &column_names,
        Some(filtered.row_count),
    );

    let sample_rows: Vec<Vec<String>> = (0..filtered.row_count.min(preview_rows))
        .map(|ri| filtered.row(ri).iter().map(|v| v.display()).collect())
        .collect();

    Ok(TransferPreview {
        schema: schema.clone(),
        mapping: TransferMapping::from_columns(&column_names),
        sample_rows,
        total_rows: filtered.row_count,
        warnings: Vec::new(),
        statement_count: export_statement_count(&filtered, &options),
        rendered_text: Some(preview_export(result, &table_name, &options, preview_rows)),
        sql_statements: Vec::new(),
    })
}

pub fn plan_export_transfer(
    result: &ResultSet,
    session: &TransferSession,
) -> Result<TransferExecutionPlan, String> {
    ensure_direction(session, TransferDirection::Export)?;
    let options = export_options_from_session(session)?;
    let table_name = export_target_name(session);
    let filtered = filter_result_for_export(result, &options);

    if filtered.columns.is_empty() {
        return Err("未选择任何列".to_string());
    }

    let column_names = filtered.column_names();
    let schema = TransferSchema::from_columns(
        session.schema.source_name.clone(),
        Some(table_name.clone()),
        &column_names,
        Some(filtered.row_count),
    );
    let content = render_export_content_for_transfer(&filtered, &table_name, &options)?;

    Ok(TransferExecutionPlan {
        session: session.clone(),
        schema: schema.clone(),
        mapping: TransferMapping::from_columns(&column_names),
        warnings: Vec::new(),
        total_rows: filtered.row_count,
        statement_count: export_statement_count(&filtered, &options),
        payload: TransferExecutionPayload::FileContent(content),
    })
}

pub fn preview_import_transfer(
    path: &Path,
    session: &TransferSession,
) -> Result<TransferPreview, String> {
    ensure_direction(session, TransferDirection::Import)?;
    match session.format {
        TransferFormat::Csv | TransferFormat::Tsv => preview_delimited_import(path, session),
        TransferFormat::Json => preview_json_import(path, session),
        TransferFormat::Sql => {
            let content =
                std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?;
            preview_sql_transfer_content(&content, session)
        }
    }
}

pub fn plan_import_transfer(
    path: &Path,
    session: &TransferSession,
) -> Result<TransferExecutionPlan, String> {
    ensure_direction(session, TransferDirection::Import)?;
    match session.format {
        TransferFormat::Csv | TransferFormat::Tsv => plan_delimited_import(path, session),
        TransferFormat::Json => plan_json_import(path, session),
        TransferFormat::Sql => {
            let content =
                std::fs::read_to_string(path).map_err(|e| format!("读取文件失败: {}", e))?;
            plan_sql_transfer_content(&content, session)
        }
    }
}

pub fn preview_sql_transfer_content(
    content: &str,
    session: &TransferSession,
) -> Result<TransferPreview, String> {
    ensure_direction(session, TransferDirection::Import)?;
    if session.format != TransferFormat::Sql {
        return Err("SQL 内容预览只适用于 SQL 传输会话".to_string());
    }

    let sql_options = session
        .sql_options()
        .ok_or_else(|| "SQL 传输缺少 SQL 配置".to_string())?;
    let (statements, warnings) = parse_sql_statements(content, sql_options, session.sql_dialect);
    let columns = vec!["SQL 语句".to_string()];
    let schema = TransferSchema::from_columns(
        session.schema.source_name.clone(),
        session.schema.target_name.clone(),
        &columns,
        Some(statements.len()),
    );

    Ok(TransferPreview {
        schema,
        mapping: TransferMapping::from_columns(&columns),
        sample_rows: statements
            .iter()
            .take(session.row_window.preview_rows.max(1))
            .cloned()
            .map(|statement| vec![statement])
            .collect(),
        total_rows: statements.len(),
        warnings,
        statement_count: statements.len(),
        rendered_text: Some(
            statements
                .iter()
                .take(session.row_window.preview_rows.max(1))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n\n"),
        ),
        sql_statements: statements,
    })
}

pub fn plan_sql_transfer_content(
    content: &str,
    session: &TransferSession,
) -> Result<TransferExecutionPlan, String> {
    if session
        .sql_options()
        .is_some_and(|options| options.use_transaction)
    {
        // Validate the original text as well: preview options may remove
        // comments or split a backend-specific literal differently.
        validate_wrapped_import_statements(&[content.to_owned()], session.sql_dialect)?;
    }
    let preview = preview_sql_transfer_content(content, session)?;
    if session
        .sql_options()
        .is_some_and(|options| options.use_transaction)
    {
        validate_wrapped_import_statements(&preview.sql_statements, session.sql_dialect)?;
    }

    Ok(TransferExecutionPlan {
        session: session.clone(),
        schema: preview.schema.clone(),
        mapping: preview.mapping.clone(),
        warnings: preview.warnings.clone(),
        total_rows: preview.total_rows,
        statement_count: preview.statement_count,
        payload: TransferExecutionPayload::SqlStatements(preview.sql_statements),
    })
}

pub fn write_transfer_plan(path: &Path, plan: &TransferExecutionPlan) -> Result<(), String> {
    let content = match &plan.payload {
        TransferExecutionPayload::FileContent(content) => content.clone(),
        TransferExecutionPayload::SqlStatements(statements) => statements.join("\n\n"),
    };

    std::fs::write(path, content).map_err(|e| e.to_string())
}

fn preview_delimited_import(
    path: &Path,
    session: &TransferSession,
) -> Result<TransferPreview, String> {
    let config = legacy_csv_config_from_session(session)?;
    let preview = preview_csv(path, &config)?;
    let statements =
        import_csv_to_sql(path, &config, import_uses_mysql_syntax(session))?.sql_statements;
    Ok(build_import_preview(
        session,
        config.table_name,
        preview.columns,
        preview.preview_rows,
        preview.total_rows,
        preview.warnings,
        statements,
    ))
}

fn plan_delimited_import(
    path: &Path,
    session: &TransferSession,
) -> Result<TransferExecutionPlan, String> {
    let config = legacy_csv_config_from_session(session)?;
    let preview = preview_csv(path, &config)?;
    let statements =
        import_csv_to_sql(path, &config, import_uses_mysql_syntax(session))?.sql_statements;
    Ok(build_import_plan(
        session,
        config.table_name,
        preview.columns,
        preview.total_rows,
        preview.warnings,
        statements,
    ))
}

fn preview_json_import(path: &Path, session: &TransferSession) -> Result<TransferPreview, String> {
    let config = legacy_json_config_from_session(session)?;
    let preview = preview_json(path, &config)?;
    let statements =
        import_json_to_sql(path, &config, import_uses_mysql_syntax(session))?.sql_statements;
    Ok(build_import_preview(
        session,
        config.table_name,
        preview.columns,
        preview.preview_rows,
        preview.total_rows,
        preview.warnings,
        statements,
    ))
}

fn plan_json_import(
    path: &Path,
    session: &TransferSession,
) -> Result<TransferExecutionPlan, String> {
    let config = legacy_json_config_from_session(session)?;
    let preview = preview_json(path, &config)?;
    let statements =
        import_json_to_sql(path, &config, import_uses_mysql_syntax(session))?.sql_statements;
    Ok(build_import_plan(
        session,
        config.table_name,
        preview.columns,
        preview.total_rows,
        preview.warnings,
        statements,
    ))
}

fn build_import_preview(
    session: &TransferSession,
    target_name: String,
    columns: Vec<String>,
    preview_rows: Vec<Vec<String>>,
    total_rows: usize,
    warnings: Vec<String>,
    statements: Vec<String>,
) -> TransferPreview {
    let schema = TransferSchema::from_columns(
        session.schema.source_name.clone(),
        Some(target_name),
        &columns,
        Some(total_rows),
    );
    let mapping = TransferMapping::from_columns(&columns);
    let statement_count = statements.len();

    TransferPreview {
        schema,
        mapping,
        sample_rows: preview_rows,
        total_rows,
        warnings,
        statement_count,
        rendered_text: None,
        sql_statements: statements,
    }
}

fn build_import_plan(
    session: &TransferSession,
    target_name: String,
    columns: Vec<String>,
    total_rows: usize,
    warnings: Vec<String>,
    statements: Vec<String>,
) -> TransferExecutionPlan {
    let schema = TransferSchema::from_columns(
        session.schema.source_name.clone(),
        Some(target_name),
        &columns,
        Some(total_rows),
    );
    let mapping = TransferMapping::from_columns(&columns);
    let statement_count = statements.len();

    TransferExecutionPlan {
        session: session.clone(),
        schema,
        mapping,
        warnings,
        total_rows,
        statement_count,
        payload: TransferExecutionPayload::SqlStatements(statements),
    }
}

fn export_options_from_session(session: &TransferSession) -> Result<ExportOptions, String> {
    let sql_options = session.sql_options().cloned().unwrap_or_default();
    let delimited_options = session.delimited_options().cloned().unwrap_or_default();
    let json_options = session.json_options().cloned().unwrap_or_default();

    Ok(ExportOptions {
        format: session.format.into(),
        selected_columns: session.mapping.selected_indices(),
        row_limit: session.row_window.row_limit,
        start_row: session.row_window.start_row,
        csv_delimiter: match session.format {
            TransferFormat::Tsv => '\t',
            _ => delimited_options.delimiter,
        },
        csv_include_header: delimited_options.include_header,
        csv_quote_char: delimited_options.quote_char,
        sql_use_transaction: sql_options.use_transaction,
        sql_batch_size: sql_options.batch_size,
        json_pretty: json_options.pretty,
        sql_dialect: session.sql_dialect,
    })
}

fn legacy_csv_config_from_session(
    session: &TransferSession,
) -> Result<LegacyCsvImportConfig, String> {
    let options = session
        .delimited_options()
        .ok_or_else(|| "分隔文本传输缺少 CSV/TSV 配置".to_string())?;

    Ok(LegacyCsvImportConfig {
        has_header: options.has_header,
        delimiter: match session.format {
            TransferFormat::Tsv => '\t',
            _ => options.delimiter,
        },
        quote_char: options.quote_char,
        skip_rows: options.skip_rows,
        max_rows: options.max_rows,
        table_name: import_target_name(session)?,
        column_names: session.mapping.target_columns(),
    })
}

fn legacy_json_config_from_session(
    session: &TransferSession,
) -> Result<LegacyJsonImportConfig, String> {
    let options = session
        .json_options()
        .ok_or_else(|| "JSON 传输缺少 JSON 配置".to_string())?;

    Ok(LegacyJsonImportConfig {
        table_name: import_target_name(session)?,
        max_rows: options.max_rows,
        json_path: options.json_path.clone(),
        flatten_nested: options.flatten_nested,
    })
}

fn import_target_name(session: &TransferSession) -> Result<String, String> {
    session
        .schema
        .target_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .ok_or_else(|| "未指定目标表名".to_string())
}

fn export_target_name(session: &TransferSession) -> String {
    session
        .schema
        .target_name
        .clone()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| "query_result".to_string())
}

fn import_uses_mysql_syntax(session: &TransferSession) -> bool {
    session.sql_dialect == SqlDialect::MySql
}

fn ensure_direction(session: &TransferSession, expected: TransferDirection) -> Result<(), String> {
    if session.direction == expected {
        Ok(())
    } else {
        Err(format!(
            "传输会话方向不匹配：期望 {:?}，实际 {:?}",
            expected, session.direction
        ))
    }
}

fn export_statement_count(result: &ResultSet, options: &ExportOptions) -> usize {
    match options.format {
        LegacyExportFormat::Sql => {
            if result.row_count == 0 {
                0
            } else if options.sql_batch_size > 0 {
                result.row_count.div_ceil(options.sql_batch_size)
            } else {
                result.row_count
            }
        }
        _ => usize::from(result.row_count > 0 || !result.columns.is_empty()),
    }
}

fn reject_ambiguous_wrapped_import_syntax(
    statement: &str,
    dialect: SqlDialect,
    index: usize,
) -> Result<(), String> {
    // MySQL versioned comments are executable, unlike regular SQL comments.
    if dialect == SqlDialect::MySql && statement.contains("/*!") {
        return Err(format!(
            "第 {} 条 SQL 含 MySQL 可执行注释；事务包装下不能保证回滚",
            index + 1
        ));
    }
    // Backslash-escaped quotes depend on backend/session SQL modes. A false
    // split can hide COMMIT, so reject wrapped imports instead of guessing.
    if matches!(dialect, SqlDialect::Postgres | SqlDialect::MySql) && statement.contains('\\') {
        return Err(format!(
            "第 {} 条 SQL 含反斜杠转义，无法安全预检事务边界；请关闭事务包装后导入",
            index + 1
        ));
    }
    Ok(())
}

/// Reject SQL that can escape an outer import transaction before any writes start.
pub(crate) fn validate_wrapped_import_statements(
    statements: &[String],
    dialect: SqlDialect,
) -> Result<(), String> {
    let options = TransferSqlOptions::default();
    for (index, statement) in statements.iter().enumerate() {
        reject_ambiguous_wrapped_import_syntax(statement, dialect, index)?;
        let normalized;
        let statement = if dialect == SqlDialect::Postgres && statement.contains('\r') {
            normalized = statement.replace('\r', "\n");
            normalized.as_str()
        } else {
            statement.as_str()
        };
        let (parts, _) = parse_sql_statements(statement, &options, dialect);
        for part in parts {
            // SQLite tokenizes a UTF-8 BOM as whitespace, including before COMMIT.
            let keyword = part
                .trim_start_matches(|ch: char| {
                    ch == '('
                        || ch.is_whitespace()
                        || (dialect == SqlDialect::Standard && ch == '\u{feff}')
                })
                .split(|ch: char| {
                    ch.is_whitespace() || (dialect == SqlDialect::Standard && ch == '\u{feff}')
                })
                .next()
                .unwrap_or_default();
            let uppercase = keyword.to_ascii_uppercase();
            let is_control = matches!(
                uppercase.as_str(),
                "BEGIN"
                    | "START"
                    | "COMMIT"
                    | "END"
                    | "ROLLBACK"
                    | "ABORT"
                    | "SAVEPOINT"
                    | "RELEASE"
                    | "XA"
                    | "PREPARE"
            );
            let is_mysql_unsafe = dialect == SqlDialect::MySql
                && !matches!(
                    uppercase.as_str(),
                    "INSERT" | "UPDATE" | "DELETE" | "REPLACE" | "SELECT"
                );
            if is_control || is_mysql_unsafe {
                return Err(format!(
                    "第 {} 条 SQL 含事务控制或无法保证回滚的操作 ({keyword})；请关闭事务包装后导入",
                    index + 1
                ));
            }
        }
    }
    Ok(())
}

fn resume_after_cr_comment(
    chars: &[char],
    position: &mut usize,
    processed_line: &mut String,
    should_strip_comments: bool,
) -> bool {
    let Some(offset) = chars[*position..].iter().position(|ch| *ch == '\r') else {
        return false;
    };
    if should_strip_comments {
        processed_line.push(' ');
    } else {
        processed_line.extend(chars[*position..=*position + offset].iter().copied());
    }
    *position += offset + 1;
    true
}

fn parse_sql_statements(
    content: &str,
    options: &TransferSqlOptions,
    dialect: SqlDialect,
) -> (Vec<String>, Vec<String>) {
    let mut statements = Vec::new();
    let mut warnings = Vec::new();
    let mut current_statement = String::new();
    // Comments retained for execution are not SQL tokens; client directives
    // remain legal after a comment-only preamble, but never inside a statement.
    let mut has_sql_content = false;
    let mut block_comment_depth = 0usize;
    let mut in_string = false;
    let mut string_char = '"';
    let mut dollar_quote_tag: Option<String> = None;
    let mut delimiter = ";".to_string();
    let mut delimiter_chars: Vec<char> = delimiter.chars().collect();

    // Preserve input bytes inside literals. PostgreSQL bare CR comments are
    // normalized only in the security preflight above, not in imported SQL.
    for line in content.split_terminator('\n') {
        if dialect == SqlDialect::MySql
            && !has_sql_content
            && block_comment_depth == 0
            && !in_string
            && dollar_quote_tag.is_none()
            && let Some(new_delimiter) = parse_delimiter_command(line)
        {
            if new_delimiter.is_empty() {
                warnings.push("检测到 DELIMITER 语句但未提供分隔符，已保持原配置".to_string());
            } else {
                delimiter = new_delimiter.to_string();
                delimiter_chars = delimiter.chars().collect();
            }
            continue;
        }

        let mut processed_line = String::new();
        let chars: Vec<char> = line.chars().collect();
        let mut started_inside_literal = in_string || dollar_quote_tag.is_some();
        let mut i = 0;

        while i < chars.len() {
            if block_comment_depth > 0 {
                if dialect == SqlDialect::Postgres && matches_token(&chars, i, "/*") {
                    block_comment_depth += 1;
                    if !options.strip_comments {
                        processed_line.push_str("/*");
                    }
                    i += 2;
                } else if matches_token(&chars, i, "*/") {
                    block_comment_depth -= 1;
                    if !options.strip_comments {
                        processed_line.push_str("*/");
                    }
                    i += 2;
                } else {
                    if !options.strip_comments {
                        processed_line.push(chars[i]);
                    }
                    i += 1;
                }
                continue;
            }

            if let Some(tag) = &dollar_quote_tag {
                if matches_token(&chars, i, tag) {
                    for ch in tag.chars() {
                        processed_line.push(ch);
                    }
                    i += tag.chars().count();
                    dollar_quote_tag = None;
                } else {
                    processed_line.push(chars[i]);
                    i += 1;
                }
                continue;
            }

            if in_string {
                processed_line.push(chars[i]);
                if chars[i] == string_char {
                    if i + 1 < chars.len() && chars[i + 1] == string_char {
                        processed_line.push(chars[i + 1]);
                        i += 2;
                        continue;
                    }
                    in_string = false;
                }
                i += 1;
                continue;
            }

            if dialect == SqlDialect::Postgres
                && chars[i] == '$'
                && (i == 0
                    || !(chars[i - 1].is_ascii_alphanumeric()
                        || chars[i - 1] == '_'
                        || chars[i - 1] == '$'
                        || !chars[i - 1].is_ascii()))
                && let Some(tag) = parse_dollar_quote_tag(&chars, i)
            {
                has_sql_content = true;
                for ch in tag.chars() {
                    processed_line.push(ch);
                }
                i += tag.chars().count();
                dollar_quote_tag = Some(tag);
                continue;
            }

            if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
                if options.strip_comments {
                    processed_line.push(' ');
                } else {
                    processed_line.push_str("/*");
                }
                block_comment_depth = 1;
                i += 2;
                continue;
            }

            // MySQL checks the byte after --: UTF-8 non-ASCII whitespace is
            // not a comment introducer, but ASCII controls (including DEL) are.
            let is_dash_comment = i + 1 < chars.len()
                && chars[i] == '-'
                && chars[i + 1] == '-'
                && (dialect != SqlDialect::MySql
                    || chars
                        .get(i + 2)
                        .is_none_or(|ch| *ch == ' ' || ch.is_ascii_control()));
            if is_dash_comment || (dialect == SqlDialect::MySql && chars[i] == '#') {
                // PostgreSQL ends -- comments at bare CR; MySQL and SQLite
                // only resume after LF (handled by split_terminator above).
                if dialect == SqlDialect::Postgres
                    && resume_after_cr_comment(
                        &chars,
                        &mut i,
                        &mut processed_line,
                        options.strip_comments,
                    )
                {
                    continue;
                }
                if !options.strip_comments {
                    processed_line.extend(chars[i..].iter().copied());
                }
                break;
            }

            if chars[i] == '\''
                || chars[i] == '"'
                || (dialect != SqlDialect::Postgres && chars[i] == '`')
                || (dialect == SqlDialect::Standard && chars[i] == '[')
            {
                in_string = true;
                has_sql_content = true;
                string_char = if chars[i] == '[' { ']' } else { chars[i] };
                processed_line.push(chars[i]);
                i += 1;
                continue;
            }

            if matches_delimiter(&chars, i, &delimiter_chars) {
                let segment = trim_sql_segment(&processed_line, started_inside_literal, false);
                if !segment.is_empty() {
                    if !current_statement.is_empty() {
                        current_statement.push('\n');
                    }
                    current_statement.push_str(segment);
                }

                let statement = current_statement.trim().to_string();
                if !statement.is_empty() {
                    statements.push(statement);
                }
                current_statement.clear();
                has_sql_content = false;
                processed_line.clear();
                i += delimiter_chars.len();
                started_inside_literal = false;
                continue;
            }

            if !chars[i].is_whitespace() {
                has_sql_content = true;
            }
            processed_line.push(chars[i]);
            i += 1;
        }

        let segment = trim_sql_segment(
            &processed_line,
            started_inside_literal,
            in_string || dollar_quote_tag.is_some(),
        );
        if !options.strip_empty_lines
            || !segment.is_empty()
            || in_string
            || dollar_quote_tag.is_some()
        {
            if !current_statement.is_empty() {
                current_statement.push('\n');
            }
            current_statement.push_str(segment);
        }
    }

    let final_statement = current_statement.trim().to_string();
    if !final_statement.is_empty() {
        statements.push(final_statement);
        warnings.push(format!("最后一条语句没有结束分隔符 '{}'", delimiter));
    }

    statements.retain(|statement| !statement.trim().is_empty());
    (statements, warnings)
}

fn trim_sql_segment(
    segment: &str,
    started_inside_literal: bool,
    ended_inside_literal: bool,
) -> &str {
    let segment = if started_inside_literal {
        segment
    } else {
        segment.trim_start()
    };
    if ended_inside_literal {
        segment
    } else {
        segment.trim_end()
    }
}

fn parse_delimiter_command(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let mut parts = trimmed.split_whitespace();
    let command = parts.next()?;
    if !command.eq_ignore_ascii_case("delimiter") {
        return None;
    }
    parts.next()
}

fn matches_delimiter(chars: &[char], idx: usize, delimiter_chars: &[char]) -> bool {
    if delimiter_chars.is_empty() || idx + delimiter_chars.len() > chars.len() {
        return false;
    }

    chars[idx..idx + delimiter_chars.len()] == *delimiter_chars
}

fn parse_dollar_quote_tag(chars: &[char], idx: usize) -> Option<String> {
    if chars.get(idx) != Some(&'$') {
        return None;
    }

    let mut j = idx + 1;
    if j >= chars.len() {
        return None;
    }

    if chars[j] == '$' {
        return Some("$$".to_string());
    }

    // PostgreSQL scans identifiers bytewise: every UTF-8 high byte is valid
    // in a dollar-quote tag, including non-alphabetic Unicode characters.
    if !(chars[j].is_ascii_alphabetic() || chars[j] == '_' || !chars[j].is_ascii()) {
        return None;
    }
    j += 1;
    while j < chars.len()
        && (chars[j].is_ascii_alphanumeric() || chars[j] == '_' || !chars[j].is_ascii())
    {
        j += 1;
    }

    if j < chars.len() && chars[j] == '$' {
        Some(chars[idx..=j].iter().collect())
    } else {
        None
    }
}

fn matches_token(chars: &[char], idx: usize, token: &str) -> bool {
    let token_len = token.chars().count();
    if idx + token_len > chars.len() {
        return false;
    }

    for (offset, ch) in token.chars().enumerate() {
        if chars[idx + offset] != ch {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::result::{ResultColumn, ResultCompleteness, ResultSet};
    use crate::domain::value::{DbTypeFamily, DbTypeInfo, DbValue};
    use std::sync::Arc;
    use tempfile::NamedTempFile;

    fn make_result_set(column_names: &[&str], rows: Vec<Vec<DbValue>>) -> ResultSet {
        let columns: Arc<[ResultColumn]> = Arc::from(
            column_names
                .iter()
                .map(|&name| ResultColumn {
                    name: name.to_string(),
                    type_info: DbTypeInfo {
                        family: DbTypeFamily::Other,
                        native_name: String::new(),
                        nullable: None,
                    },
                })
                .collect::<Vec<_>>(),
        );
        let row_count = rows.len();
        let mut cells = Vec::new();
        for row in &rows {
            for cell in row {
                cells.push(cell.clone());
            }
        }
        ResultSet {
            columns,
            cells,
            row_count,
            completeness: ResultCompleteness::Complete,
        }
    }

    #[test]
    fn sql_transfer_preview_dialect_delimiters_preserve_statements() {
        let mysql_content = r#"
DELIMITER //
CREATE PROCEDURE test_proc()
BEGIN
  INSERT INTO t VALUES (1);
  INSERT INTO t VALUES (2);
END//
DELIMITER ;
SELECT 1;
"#;
        let postgres_content = r#"
CREATE FUNCTION f_test() RETURNS void AS $$
BEGIN
  PERFORM 1;
  PERFORM 2;
END;
$$ LANGUAGE plpgsql;
SELECT 1;
"#;

        let session = TransferSession {
            direction: TransferDirection::Import,
            sql_dialect: SqlDialect::MySql,
            format: TransferFormat::Sql,
            schema: TransferSchema::default(),
            mapping: TransferMapping::default(),
            row_window: TransferRowWindow {
                preview_rows: 10,
                ..Default::default()
            },
            options: TransferFormatOptions::Sql(TransferSqlOptions {
                use_transaction: false,
                batch_size: 0,
                strip_comments: true,
                strip_empty_lines: true,
                stop_on_error: false,
            }),
        };

        let preview = preview_sql_transfer_content(mysql_content, &session).expect("preview");
        assert_eq!(preview.statement_count, 2);
        assert!(preview.sql_statements[0].starts_with("CREATE PROCEDURE test_proc()"));
        assert!(preview.sql_statements[1].starts_with("SELECT 1"));

        let postgres_session = TransferSession {
            sql_dialect: SqlDialect::Postgres,
            ..session
        };
        let preview = preview_sql_transfer_content(postgres_content, &postgres_session)
            .expect("PostgreSQL preview");
        assert_eq!(preview.statement_count, 2);
        assert!(preview.sql_statements[0].starts_with("CREATE FUNCTION f_test()"));
        assert!(preview.sql_statements[0].contains("PERFORM 2;"));
        assert!(preview.sql_statements[1].starts_with("SELECT 1"));
    }

    #[test]
    fn export_transfer_plan_preserves_literal_null_string() {
        let result = make_result_set(
            &["value"],
            vec![vec![DbValue::Text("NULL".into())], vec![DbValue::Null]],
        );
        let column_names = result.column_names();
        let session = TransferSession {
            direction: TransferDirection::Export,
            sql_dialect: SqlDialect::Standard,
            format: TransferFormat::Sql,
            schema: TransferSchema::from_columns(
                None,
                Some("items".to_string()),
                &column_names,
                Some(result.row_count),
            ),
            mapping: TransferMapping::from_columns(&column_names),
            row_window: TransferRowWindow::default(),
            options: TransferFormatOptions::Sql(TransferSqlOptions {
                use_transaction: false,
                batch_size: 0,
                strip_comments: true,
                strip_empty_lines: true,
                stop_on_error: false,
            }),
        };

        let plan = plan_export_transfer(&result, &session).expect("plan");
        let content = plan.into_rendered_text().expect("content");
        assert!(content.contains("VALUES ('NULL');"));
        assert!(content.contains("VALUES (NULL);"));
    }

    #[test]
    fn csv_transfer_preview_and_plan_share_schema() {
        let file = NamedTempFile::new().expect("temp file");
        std::fs::write(file.path(), "id,name\n1,Alice\n2,Bob\n").expect("write csv");

        let session = TransferSession {
            direction: TransferDirection::Import,
            sql_dialect: SqlDialect::Standard,
            format: TransferFormat::Csv,
            schema: TransferSchema {
                source_name: Some("people.csv".to_string()),
                target_name: Some("people".to_string()),
                fields: Vec::new(),
                total_rows: None,
            },
            mapping: TransferMapping::default(),
            row_window: TransferRowWindow::default(),
            options: TransferFormatOptions::Delimited(TransferDelimitedOptions::default()),
        };

        let preview = preview_import_transfer(file.path(), &session).expect("preview");
        let plan = plan_import_transfer(file.path(), &session).expect("plan");

        assert_eq!(preview.schema.fields.len(), 2);
        assert_eq!(preview.schema.fields[0].name, "id");
        assert_eq!(plan.schema.fields[1].name, "name");
        assert_eq!(preview.statement_count, plan.statement_count);
        assert!(preview.statement_count > 0);
    }
    #[test]
    fn sql_import_preserves_multiline_literal_whitespace() {
        let session = TransferSession {
            direction: TransferDirection::Import,
            format: TransferFormat::Sql,
            options: TransferFormatOptions::Sql(TransferSqlOptions {
                use_transaction: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        let plan =
            plan_sql_transfer_content("INSERT INTO t(v) VALUES ('a  \n  b\n\n c');", &session)
                .expect("SQL plan");
        let statements = plan.into_sql_statements().expect("SQL statements");
        assert_eq!(statements, ["INSERT INTO t(v) VALUES ('a  \n  b\n\n c')"]);
    }

    #[test]
    fn wrapped_import_rejects_nested_control_without_rejecting_quoted_word() {
        let session = TransferSession {
            direction: TransferDirection::Import,
            format: TransferFormat::Sql,
            options: TransferFormatOptions::Sql(TransferSqlOptions::default()),
            ..Default::default()
        };
        assert!(
            plan_sql_transfer_content("INSERT INTO t VALUES ('COMMIT'); COMMIT;", &session)
                .is_err()
        );
        assert!(plan_sql_transfer_content("INSERT INTO t VALUES ('COMMIT');", &session).is_ok());
        assert!(
            validate_wrapped_import_statements(
                &["CREATE TABLE t(id INT)".into()],
                SqlDialect::MySql
            )
            .is_err()
        );
    }

    #[test]
    fn wrapped_import_mid_statement_delimiter_cannot_hide_commit() {
        let sql = "SELECT 1 AS\nDELIMITER ;\nCOMMIT;";
        for dialect in [
            SqlDialect::Standard,
            SqlDialect::Postgres,
            SqlDialect::MySql,
        ] {
            let (statements, _) =
                parse_sql_statements(sql, &TransferSqlOptions::default(), dialect);
            assert_eq!(statements, ["SELECT 1 AS\nDELIMITER", "COMMIT"]);
            assert!(validate_wrapped_import_statements(&[sql.into()], dialect).is_err());
        }
    }

    #[test]
    fn sql_import_preserved_comments_do_not_change_statement_boundaries() {
        let sql = "INSERT INTO t VALUES (1);\nSELECT 1 AS -- '\nDELIMITER ;\nCOMMIT; -- '\nINSERT INTO missing_table_xyz VALUES (2);";
        let options = TransferSqlOptions {
            strip_comments: false,
            ..Default::default()
        };
        for dialect in [
            SqlDialect::Standard,
            SqlDialect::Postgres,
            SqlDialect::MySql,
        ] {
            let (statements, _) = parse_sql_statements(sql, &options, dialect);
            assert_eq!(
                statements,
                [
                    "INSERT INTO t VALUES (1)",
                    "SELECT 1 AS -- '\nDELIMITER",
                    "COMMIT",
                    "-- '\nINSERT INTO missing_table_xyz VALUES (2)"
                ],
                "{dialect:?}"
            );
            assert!(validate_wrapped_import_statements(&[sql.into()], dialect).is_err());
        }
    }

    #[test]
    fn sql_import_postgres_delimiter_identifier_is_not_client_command() {
        let options = TransferSqlOptions {
            strip_comments: false,
            ..Default::default()
        };
        let (statements, _) =
            parse_sql_statements("SELECT 1 AS\nDELIMITER;", &options, SqlDialect::Postgres);
        assert_eq!(statements, ["SELECT 1 AS\nDELIMITER"]);
        assert!(validate_wrapped_import_statements(&statements, SqlDialect::Postgres).is_ok());
    }

    #[test]
    fn sql_import_mysql_delimiter_after_preserved_comment_stays_supported() {
        let options = TransferSqlOptions {
            strip_comments: false,
            ..Default::default()
        };
        let (statements, _) = parse_sql_statements(
            "-- header\nDELIMITER //\nSELECT 1//\nDELIMITER ;\nSELECT 2;",
            &options,
            SqlDialect::MySql,
        );
        assert_eq!(statements, ["-- header\nSELECT 1", "SELECT 2"]);
    }

    #[test]
    fn sql_import_postgres_preserves_cr_comment_without_swallowing_control() {
        let options = TransferSqlOptions {
            strip_comments: false,
            ..Default::default()
        };
        let sql = "SELECT 1 -- '\r + 2; COMMIT; -- '\nSELECT 2;";
        let (statements, _) = parse_sql_statements(sql, &options, SqlDialect::Postgres);
        assert_eq!(
            statements,
            ["SELECT 1 -- '\r + 2", "COMMIT", "-- '\nSELECT 2"]
        );
        assert!(validate_wrapped_import_statements(&[sql.into()], SqlDialect::Postgres).is_err());
    }

    #[test]
    fn wrapped_mysql_import_rejects_executable_comment_that_commits() {
        assert!(
            validate_wrapped_import_statements(
                &["INSERT INTO t VALUES (1); /*! COMMIT */; INSERT INTO t VALUES (2)".into()],
                SqlDialect::MySql,
            )
            .is_err()
        );
    }

    #[test]
    fn wrapped_postgres_import_cr_comment_exposes_commit() {
        for ending in ["\r", "\r\n", "\n"] {
            let sql = format!("-- note{ending}COMMIT; INSERT INTO t VALUES (1)");
            let (statements, _) =
                parse_sql_statements(&sql, &TransferSqlOptions::default(), SqlDialect::Postgres);
            assert_eq!(statements, ["COMMIT", "INSERT INTO t VALUES (1)"]);
            assert!(
                validate_wrapped_import_statements(&[sql], SqlDialect::Postgres).is_err(),
                "{ending:?}"
            );
        }

        // SQLite's -- comment only ends at LF; a bare CR cannot expose COMMIT.
        let (sqlite, _) = parse_sql_statements(
            "-- note\rCOMMIT; INSERT INTO t VALUES (1)",
            &TransferSqlOptions::default(),
            SqlDialect::Standard,
        );
        assert!(sqlite.is_empty());
    }

    #[test]
    fn wrapped_postgres_import_backtick_operator_does_not_hide_commit() {
        let sql = "INSERT INTO t VALUES (1); SELECT 1 `; COMMIT; -- `\nSELECT 2;";
        let (statements, _) =
            parse_sql_statements(sql, &TransferSqlOptions::default(), SqlDialect::Postgres);
        assert_eq!(
            statements,
            [
                "INSERT INTO t VALUES (1)",
                "SELECT 1 `",
                "COMMIT",
                "SELECT 2"
            ]
        );
        assert!(validate_wrapped_import_statements(&[sql.into()], SqlDialect::Postgres).is_err());
    }

    #[test]
    fn wrapped_mysql_import_non_ascii_whitespace_after_dashes_is_not_a_comment() {
        let sql = "INSERT INTO t VALUES (1); SELECT 1--\u{00a0}2; COMMIT;";
        let (statements, _) =
            parse_sql_statements(sql, &TransferSqlOptions::default(), SqlDialect::MySql);
        assert_eq!(
            statements,
            ["INSERT INTO t VALUES (1)", "SELECT 1--\u{00a0}2", "COMMIT"]
        );
        assert!(validate_wrapped_import_statements(&[sql.into()], SqlDialect::MySql).is_err());

        let (ascii_control, _) = parse_sql_statements(
            "SELECT 1; --\u{0001}ignored\rDELETE FROM t;\nSELECT 2;",
            &TransferSqlOptions::default(),
            SqlDialect::MySql,
        );
        assert_eq!(ascii_control, ["SELECT 1", "SELECT 2"]);
    }

    #[test]
    fn wrapped_mysql_import_line_comments_end_at_lf_not_bare_cr() {
        for comment in ["-- note", "# note"] {
            let sql = format!(
                "INSERT INTO t VALUES (1); {comment}\rDELETE FROM t;\nINSERT INTO t VALUES (2);"
            );
            let (statements, _) =
                parse_sql_statements(&sql, &TransferSqlOptions::default(), SqlDialect::MySql);
            assert_eq!(
                statements,
                ["INSERT INTO t VALUES (1)", "INSERT INTO t VALUES (2)"],
                "{comment}"
            );
            assert!(validate_wrapped_import_statements(&[sql], SqlDialect::MySql).is_ok());
        }
    }

    #[test]
    fn wrapped_import_backend_escapes_and_comments_cannot_hide_commit() {
        let postgres = TransferSession {
            direction: TransferDirection::Import,
            format: TransferFormat::Sql,
            sql_dialect: SqlDialect::Postgres,
            options: TransferFormatOptions::Sql(TransferSqlOptions {
                strip_comments: false,
                ..Default::default()
            }),
            ..Default::default()
        };
        for sql in [
            "INSERT INTO t VALUES (1); SELECT E'\\''; COMMIT; -- '; SELECT * FROM missing_table;",
            "INSERT INTO t VALUES (1); SELECT 1 /* outer /* inner */ ' */; COMMIT; -- '; SELECT * FROM missing_table;",
            "INSERT INTO t VALUES (1); SELECT $é$'$é$; COMMIT; -- '; SELECT * FROM missing_table;",
            "INSERT INTO t VALUES (1); SELECT $💥$'$💥$; COMMIT; -- '; SELECT * FROM missing_table;",
        ] {
            assert!(plan_sql_transfer_content(sql, &postgres).is_err(), "{sql}");
        }
        let mysql = TransferSession {
            sql_dialect: SqlDialect::MySql,
            ..postgres
        };
        let sql = "INSERT INTO t VALUES (1); SELECT 1 AS `a'`; COMMIT; -- '; SELECT * FROM missing_table;";
        assert!(plan_sql_transfer_content(sql, &mysql).is_err());
        assert!(
            plan_sql_transfer_content(
                "INSERT INTO t VALUES (1); SELECT 1 --\u{0001}'\n; COMMIT; -- '\nSELECT * FROM missing_table;",
                &mysql,
            )
            .is_err()
        );
    }

    #[test]
    fn wrapped_import_identifier_dollar_sign_does_not_hide_control() {
        let sql = "SELECT 1 AS x$tag$; COMMIT; SELECT 1 AS x$tag$";
        for dialect in [
            SqlDialect::Postgres,
            SqlDialect::Standard,
            SqlDialect::MySql,
        ] {
            let (statements, _) =
                parse_sql_statements(sql, &TransferSqlOptions::default(), dialect);
            assert_eq!(
                statements,
                ["SELECT 1 AS x$tag$", "COMMIT", "SELECT 1 AS x$tag$"],
                "{dialect:?}"
            );
            assert!(validate_wrapped_import_statements(&[sql.into()], dialect).is_err());
        }

        let quoted = "SELECT $tag$; COMMIT$tag$";
        let (postgres, _) =
            parse_sql_statements(quoted, &TransferSqlOptions::default(), SqlDialect::Postgres);
        assert_eq!(postgres, ["SELECT $tag$; COMMIT$tag$"]);
    }

    #[test]
    fn binary_export_requires_sql_while_temporal_text_keeps_fraction() {
        use crate::domain::value::DbTime;
        let result = make_result_set(&["b"], vec![vec![DbValue::Bytes(Arc::from([0u8, 255]))]]);
        for format in [
            TransferFormat::Csv,
            TransferFormat::Tsv,
            TransferFormat::Json,
        ] {
            let session = TransferSession {
                direction: TransferDirection::Export,
                format,
                ..Default::default()
            };
            assert!(
                plan_export_transfer(&result, &session)
                    .unwrap_err()
                    .contains("不支持二进制")
            );
        }
        let times = make_result_set(
            &["t"],
            vec![vec![DbValue::Time(DbTime {
                hour: 1,
                minute: 2,
                second: 3,
                nanos: 123_456_000,
            })]],
        );
        for format in [
            TransferFormat::Csv,
            TransferFormat::Json,
            TransferFormat::Sql,
        ] {
            let session = TransferSession {
                direction: TransferDirection::Export,
                format,
                ..Default::default()
            };
            let content = plan_export_transfer(&times, &session)
                .unwrap()
                .into_rendered_text()
                .unwrap();
            assert!(content.contains("01:02:03.123456"));
        }
    }
    #[test]
    fn csv_export_rejects_ambiguous_text_instead_of_changing_it_on_import() {
        for text in ["", "001", "true", " a ", "NaN"] {
            let result = make_result_set(&["v"], vec![vec![DbValue::Text(text.into())]]);
            let session = TransferSession {
                direction: TransferDirection::Export,
                format: TransferFormat::Csv,
                ..Default::default()
            };
            assert!(
                plan_export_transfer(&result, &session)
                    .unwrap_err()
                    .contains("无法无损导入")
            );
        }
    }
}
