//! SQLite 查询实现

use super::{ImportExecutionReport, TriggerInfo, is_query_statement};
use crate::core::constants;
use crate::data::{ConnectionConfig, DatabaseType, DbError};
use crate::domain::execution::ExecutionOutcome;
use crate::domain::identifier::IdentifierDialect;
use crate::domain::result::{ResultColumn, ResultCompleteness, ResultSet};
use crate::domain::value::{DbTypeFamily, DbTypeInfo, DbValue};
use rusqlite::{Connection as SqliteConn, types::ValueRef};
use std::sync::Arc;

/// 连接 SQLite 并获取表列表
pub(crate) fn connect(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    let mut stmt = conn.prepare(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"
    ).map_err(|e| DbError::Query(e.to_string()))?;

    let tables: Result<Vec<String>, _> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| DbError::Query(e.to_string()))?
        .collect();

    tables.map_err(|e| DbError::Query(e.to_string()))
}

// ── Typed Mutation execution (Phase 7) ──

use crate::domain::mutation::{
    ExpectedRows, InputValue, Mutation, MutationBatch, MutationBatchResult, RowIdentity,
};
/// 以参数化方式执行 MutationBatch（SQLite 实现）
pub(crate) fn apply_mutations(
    config: &ConnectionConfig,
    batch: &MutationBatch,
) -> Result<MutationBatchResult, DbError> {
    let mut conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    let mut affected = Vec::with_capacity(batch.mutations.len());

    if batch.atomic {
        let tx = conn
            .transaction()
            .map_err(|e| DbError::Query(format!("BEGIN 失败: {}", e)))?;

        for mutation in &batch.mutations {
            affected.push(execute_one(&tx, mutation)?);
        }

        tx.commit()
            .map_err(|e| DbError::Query(format!("COMMIT 失败: {}", e)))?;
    } else {
        for mutation in &batch.mutations {
            affected.push(execute_one(&conn, mutation)?);
        }
    }

    Ok(MutationBatchResult {
        affected,
        all_success: true,
    })
}
fn execute_one(conn: &SqliteConn, mutation: &Mutation) -> Result<u64, DbError> {
    match mutation {
        Mutation::Insert {
            table,
            columns,
            values,
        } => {
            // 过滤掉 Unspecified 和 Default 列（让 DB 使用默认值/自增）
            let mut included_cols: Vec<&str> = Vec::new();
            let mut included_vals: Vec<rusqlite::types::Value> = Vec::new();
            for (col, val) in columns.iter().zip(values.iter()) {
                match val {
                    InputValue::Unspecified | InputValue::Default => {
                        // 跳过：让 DB 使用 DEFAULT 或 auto-increment
                    }
                    InputValue::Null => {
                        included_cols.push(&col.name);
                        included_vals.push(rusqlite::types::Value::Null);
                    }
                    InputValue::Value(v) => {
                        included_cols.push(&col.name);
                        included_vals.push(dbvalue_to_rusqlite(v)?);
                    }
                }
            }

            if included_cols.is_empty() {
                // 所有列都是 Unspecified/Default → 使用 DEFAULT VALUES 语法
                let sql = format!(
                    "INSERT INTO {} DEFAULT VALUES",
                    IdentifierDialect::SQLite.quote(&table.name)
                );
                let rows = conn
                    .execute(&sql, [])
                    .map_err(|e| DbError::Query(format!("INSERT DEFAULT VALUES 失败: {}", e)))?
                    as u64;
                return Ok(rows);
            }

            let quoted_cols: Vec<String> = included_cols
                .iter()
                .map(|name| IdentifierDialect::SQLite.quote(name))
                .collect();
            let placeholders: Vec<&str> = vec!["?"; included_cols.len()];

            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({})",
                IdentifierDialect::SQLite.quote(&table.name),
                quoted_cols.join(", "),
                placeholders.join(", ")
            );

            let rows = conn
                .execute(&sql, rusqlite::params_from_iter(included_vals.iter()))
                .map_err(|e| DbError::Query(format!("INSERT 失败: {}", e)))?
                as u64;
            Ok(rows)
        }

        Mutation::Update {
            table,
            identity,
            changes,
            expected_rows,
        } => {
            let mut set_clauses: Vec<String> = Vec::new();
            let mut set_values: Vec<rusqlite::types::Value> = Vec::new();
            for (col, val) in changes.iter() {
                match val {
                    InputValue::Unspecified => {
                        return Err(DbError::Query("UPDATE 中不允许 Unspecified 值".to_string()));
                    }
                    InputValue::Default => {
                        // SQLite 不支持 SET col = DEFAULT
                        return Err(DbError::Unsupported {
                            capability: "SQLite UPDATE SET DEFAULT",
                        });
                    }
                    InputValue::Null => {
                        set_clauses.push(format!(
                            "{} = NULL",
                            IdentifierDialect::SQLite.quote(&col.name)
                        ));
                    }
                    InputValue::Value(v) => {
                        set_clauses.push(format!(
                            "{} = ?",
                            IdentifierDialect::SQLite.quote(&col.name)
                        ));
                        set_values.push(dbvalue_to_rusqlite(v)?);
                    }
                }
            }

            if set_clauses.is_empty() {
                return Ok(0); // 没有实际变更，不执行
            }

            let id_cols = get_id_columns(identity);
            let where_clauses: Vec<String> =
                id_cols.iter().map(|col| format!("{} = ?", col)).collect();

            let sql = format!(
                "UPDATE {} SET {} WHERE {}",
                IdentifierDialect::SQLite.quote(&table.name),
                set_clauses.join(", "),
                where_clauses.join(" AND ")
            );

            let mut params = set_values;
            for val in get_id_dbvalues(identity) {
                params.push(dbvalue_to_rusqlite(val)?);
            }

            let rows = conn
                .execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| DbError::Query(format!("UPDATE 失败: {}", e)))?
                as u64;

            check_expected(rows, *expected_rows, "UPDATE")?;
            Ok(rows)
        }

        Mutation::Delete {
            table,
            identity,
            expected_rows,
        } => {
            let id_cols = get_id_columns(identity);
            let where_clauses: Vec<String> =
                id_cols.iter().map(|col| format!("{} = ?", col)).collect();

            let sql = format!(
                "DELETE FROM {} WHERE {}",
                IdentifierDialect::SQLite.quote(&table.name),
                where_clauses.join(" AND ")
            );

            let params: Vec<rusqlite::types::Value> = get_id_dbvalues(identity)
                .iter()
                .map(|v| dbvalue_to_rusqlite(v))
                .collect::<Result<Vec<_>, _>>()?;

            let rows = conn
                .execute(&sql, rusqlite::params_from_iter(params.iter()))
                .map_err(|e| DbError::Query(format!("DELETE 失败: {}", e)))?
                as u64;

            check_expected(rows, *expected_rows, "DELETE")?;
            Ok(rows)
        }
    }
}

fn get_id_columns(id: &RowIdentity) -> Vec<String> {
    match id {
        RowIdentity::PrimaryKey(cols) => cols
            .iter()
            .map(|(c, _)| IdentifierDialect::SQLite.quote(&c.name))
            .collect(),
        RowIdentity::UniqueKey { columns, .. } => columns
            .iter()
            .map(|(c, _)| IdentifierDialect::SQLite.quote(&c.name))
            .collect(),
    }
}

fn get_id_dbvalues(id: &RowIdentity) -> Vec<&DbValue> {
    match id {
        RowIdentity::PrimaryKey(cols) => cols.iter().map(|(_, v)| v).collect(),
        RowIdentity::UniqueKey { columns, .. } => columns.iter().map(|(_, v)| v).collect(),
    }
}

fn check_expected(actual: u64, expected: ExpectedRows, op: &str) -> Result<(), DbError> {
    match expected {
        ExpectedRows::Exactly(n) if actual != n => Err(DbError::Query(format!(
            "{} 预期 {} 行，实际 {} 行",
            op, n, actual
        ))),
        ExpectedRows::AtLeast(n) if actual < n => Err(DbError::Query(format!(
            "{} 预期至少 {} 行，实际 {} 行",
            op, n, actual
        ))),
        _ => Ok(()),
    }
}

/// 将 DbValue 转换为 rusqlite 可绑定的值（带溢出检查和类型支持）
fn dbvalue_to_rusqlite(v: &DbValue) -> Result<rusqlite::types::Value, DbError> {
    match v {
        DbValue::Null => Ok(rusqlite::types::Value::Null),
        DbValue::Bool(b) => Ok(rusqlite::types::Value::Integer(if *b { 1 } else { 0 })),
        DbValue::Int(i) => Ok(rusqlite::types::Value::Integer(*i)),
        DbValue::UInt(u) => {
            let v = i64::try_from(*u).map_err(|_| DbError::ValueOutOfRange {
                value: u.to_string(),
                reason: "unsigned integer exceeds i64 range",
            })?;
            Ok(rusqlite::types::Value::Integer(v))
        }
        DbValue::Float(f) => Ok(rusqlite::types::Value::Real(*f)),
        DbValue::Decimal(s) | DbValue::Text(s) => Ok(rusqlite::types::Value::Text(s.clone())),
        DbValue::Date(d) => Ok(rusqlite::types::Value::Text(format!(
            "{:04}-{:02}-{:02}",
            d.year, d.month, d.day
        ))),
        DbValue::Time(t) => Ok(rusqlite::types::Value::Text(format!(
            "{:02}:{:02}:{:02}",
            t.hour, t.minute, t.second
        ))),
        DbValue::DateTime(dt) => Ok(rusqlite::types::Value::Text(format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            dt.date.year, dt.date.month, dt.date.day, dt.time.hour, dt.time.minute, dt.time.second
        ))),
        DbValue::Bytes(b) => Ok(rusqlite::types::Value::Blob(b.to_vec())),
        DbValue::Json(j) => Ok(rusqlite::types::Value::Text(j.to_string())),
        DbValue::Uuid(u) => Ok(rusqlite::types::Value::Text(u.to_string())),
        DbValue::Array(_) | DbValue::Other { .. } => Err(DbError::Unsupported {
            capability: "array/other type not supported for SQLite binding",
        }),
    }
}
/// 批量执行 SQLite 语句（用于导入）
pub(crate) fn execute_batch(
    config: &ConnectionConfig,
    statements: &[String],
    use_transaction: bool,
    stop_on_error: bool,
) -> Result<ImportExecutionReport, DbError> {
    let mut conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    let mut report = ImportExecutionReport::new(statements.len());
    if statements.is_empty() {
        return Ok(report);
    }

    if use_transaction {
        let tx = conn
            .transaction()
            .map_err(|e| DbError::Query(format!("开启事务失败: {}", e)))?;

        for (index, statement) in statements.iter().enumerate() {
            if let Err(e) = tx.execute_batch(statement) {
                return Err(DbError::Query(format!(
                    "事务已回滚，第 {} 条语句执行失败: {}",
                    index + 1,
                    e
                )));
            }
            report.succeeded += 1;
        }

        tx.commit()
            .map_err(|e| DbError::Query(format!("提交事务失败: {}", e)))?;
        return Ok(report);
    }

    for (index, statement) in statements.iter().enumerate() {
        if let Err(e) = conn.execute_batch(statement) {
            report.failed += 1;
            if report.first_error.is_none() {
                report.first_error = Some(format!("第 {} 条语句执行失败: {}", index + 1, e));
            }

            if stop_on_error {
                return Err(DbError::Query(
                    report
                        .first_error
                        .clone()
                        .unwrap_or_else(|| format!("第 {} 条语句执行失败", index + 1)),
                ));
            }
        } else {
            report.succeeded += 1;
        }
    }

    Ok(report)
}

// ── Typed ResultSet execution (Phase 4) ──

/// 执行 SQL 并返回类型化 ResultSet（SQLite 原生路径）
pub(crate) fn execute_typed(
    config: &ConnectionConfig,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;
    execute_typed_with_connection(&conn, sql)
}

fn execute_typed_with_connection(
    conn: &SqliteConn,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    if !is_query_statement(sql, &DatabaseType::SQLite) {
        // 非 SELECT 语句 → 返回空的 ResultSet（通过 affected_rows 传达结果）
        let affected = conn
            .execute(sql, [])
            .map_err(|e| DbError::Query(e.to_string()))? as u64;
        return Ok(ExecutionOutcome::affected_rows(affected));
    }

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| DbError::Query(e.to_string()))?;

    // 从 column_names 获取列名（rusqlite 0.39 的 columns() API 不可用）
    let col_names: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();

    let col_count = col_names.len();
    let mut cells: Vec<DbValue> = Vec::new();
    let _max_rows = constants::database::MAX_RESULT_SET_ROWS;
    let mut total_rows = 0usize;

    let row_iter = stmt
        .query_map([], |row| {
            (0..col_count)
                .map(|i| row.get_ref(i).map(value_ref_to_dbvalue))
                .collect::<Result<Vec<_>, _>>()
        })
        .map_err(|e| DbError::Query(e.to_string()))?;

    const FETCH_LIMIT: usize = constants::database::MAX_RESULT_SET_ROWS;
    const PROBE_EXTRA: usize = 1;

    for row in row_iter {
        let row = row.map_err(|e| DbError::Query(e.to_string()))?;
        total_rows += 1;
        if total_rows <= FETCH_LIMIT + PROBE_EXTRA && cells.len() / col_count < FETCH_LIMIT {
            cells.extend(row);
        }
        if total_rows > FETCH_LIMIT {
            // 停止拉取：drop statement 结束 cursor，不再处理剩余数据
            break;
        }
    }

    let row_count = cells.len() / col_count;
    let completeness = if total_rows > FETCH_LIMIT {
        ResultCompleteness::Truncated {
            displayed: row_count,
        }
    } else {
        ResultCompleteness::Complete
    };

    // 从数据推断列类型（第一行非 NULL 值）
    let columns: Arc<[ResultColumn]> = col_names
        .iter()
        .enumerate()
        .map(|(col_idx, name)| {
            let sample = (0..row_count)
                .find(|&r| !matches!(cells.get(r * col_count + col_idx), Some(DbValue::Null)))
                .and_then(|r| cells.get(r * col_count + col_idx));
            let family = sample.map(dbvalue_to_family).unwrap_or(DbTypeFamily::Text);
            ResultColumn {
                name: name.clone(),
                type_info: DbTypeInfo {
                    family,
                    native_name: String::new(),
                    nullable: None,
                },
            }
        })
        .collect();

    Ok(ExecutionOutcome::single_result(ResultSet {
        columns,
        cells,
        row_count,
        completeness,
    }))
}
/// 将 rusqlite ValueRef 转换为 DbValue
fn value_ref_to_dbvalue(val: ValueRef<'_>) -> DbValue {
    match val {
        ValueRef::Null => DbValue::Null,
        ValueRef::Integer(i) => DbValue::Int(i),
        ValueRef::Real(f) => DbValue::Float(f),
        ValueRef::Text(t) => match std::str::from_utf8(t) {
            Ok(s) => DbValue::Text(s.to_string()),
            Err(_) => DbValue::Bytes(Arc::from(t)),
        },
        ValueRef::Blob(b) => DbValue::Bytes(Arc::from(b)),
    }
}

// ── MetadataCatalog 统一加载 (Phase 6) ──

use super::infer_type_family;
use crate::domain::ids::SchemaRevision;
use crate::domain::metadata::{
    ColumnMetadata as CatalogColumn, ForeignKeyMetadata, KeyMetadata, SchemaCatalog, TableMetadata,
};

/// 一次性加载 SQLite 数据库的完整 schema catalog。
///
/// 在单个连接中执行：
/// 1. `sqlite_master` → 表列表
/// 2. 每表 `PRAGMA table_info` → 列信息（含 PK）
/// 3. 每表 `PRAGMA foreign_key_list` → 外键
///
/// 返回的 SchemaCatalog 可直接用于 autocomplete、grid PK、ER 图。
pub(crate) fn load_catalog(
    config: &ConnectionConfig,
    revision: SchemaRevision,
) -> Result<SchemaCatalog, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    // 1. 获取表列表
    let mut stmt = conn
        .prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        )
        .map_err(|e| DbError::Query(e.to_string()))?;

    let table_names: Vec<String> = stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| DbError::Query(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut tables = Vec::with_capacity(table_names.len());

    for table_name in &table_names {
        // 2. PRAGMA table_info → 列信息
        let pragma_sql = format!("PRAGMA table_info('{}')", table_name.replace('\'', "''"));
        let mut col_stmt = conn
            .prepare(&pragma_sql)
            .map_err(|e| DbError::Query(e.to_string()))?;

        let col_rows: Vec<(usize, String, String, bool, Option<String>, bool)> = col_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)? as usize,   // cid
                    row.get(1)?,                      // name
                    row.get::<_, String>(2)?,         // type
                    row.get::<_, bool>(3)?,           // notnull
                    row.get::<_, Option<String>>(4)?, // dflt_value
                    row.get::<_, i64>(5)? > 0,        // pk
                ))
            })
            .map_err(|e| DbError::Query(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let columns: Vec<CatalogColumn> = col_rows
            .iter()
            .map(
                |(pos, name, data_type, notnull, default, is_pk)| CatalogColumn {
                    name: name.clone(),
                    position: *pos,
                    type_info: crate::domain::value::DbTypeInfo {
                        family: infer_type_family(data_type),
                        native_name: data_type.clone(),
                        nullable: Some(!notnull),
                    },
                    is_nullable: !notnull,
                    is_primary_key: *is_pk,
                    default_value: default.clone(),
                },
            )
            .collect();

        let pk_columns: Vec<String> = columns
            .iter()
            .filter(|c| c.is_primary_key)
            .map(|c| c.name.clone())
            .collect();
        let primary_key = if pk_columns.is_empty() {
            None
        } else {
            Some(KeyMetadata {
                name: None,
                columns: pk_columns,
            })
        };

        // 3. PRAGMA foreign_key_list → 外键
        let fk_sql = format!(
            "PRAGMA foreign_key_list('{}')",
            table_name.replace('\'', "''")
        );
        let mut fk_stmt = conn
            .prepare(&fk_sql)
            .map_err(|e| DbError::Query(e.to_string()))?;

        let fk_rows: Vec<(String, String, String)> = fk_stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(2)?, // table (referenced)
                    row.get::<_, String>(3)?, // from
                    row.get::<_, String>(4)?, // to
                ))
            })
            .map_err(|e| DbError::Query(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let foreign_keys: Vec<ForeignKeyMetadata> = fk_rows
            .into_iter()
            .map(|(ref_table, from_col, to_col)| ForeignKeyMetadata {
                name: None,
                from_columns: vec![from_col],
                ref_table,
                ref_columns: vec![to_col],
            })
            .collect();

        tables.push(TableMetadata {
            name: table_name.clone(),
            schema: None,
            columns,
            primary_key,
            unique_keys: Vec::new(),
            foreign_keys,
        });
    }

    Ok(SchemaCatalog { revision, tables })
}
/// SQLite 声明类型 → DbTypeFamily
fn sqlite_decl_type_to_family(decl_type: &str) -> DbTypeFamily {
    let t = decl_type.to_ascii_lowercase();
    if t.contains("int") {
        return DbTypeFamily::Integer;
    }
    if t.contains("char") || t.contains("clob") || t.contains("text") {
        return DbTypeFamily::Text;
    }
    if t.contains("blob") {
        return DbTypeFamily::Bytes;
    }
    if t.contains("real") || t.contains("floa") || t.contains("doub") {
        return DbTypeFamily::Float;
    }
    if t.contains("numeric") || t.contains("decimal") {
        return DbTypeFamily::Decimal;
    }
    if t.contains("bool") {
        return DbTypeFamily::Bool;
    }
    if t.contains("date") || t.contains("time") {
        return DbTypeFamily::DateTime;
    }
    DbTypeFamily::Text // SQLite 默认亲和性
}

/// 从 DbValue 推断类型族
fn dbvalue_to_family(val: &DbValue) -> DbTypeFamily {
    match val {
        DbValue::Null => DbTypeFamily::Null,
        DbValue::Bool(_) => DbTypeFamily::Bool,
        DbValue::Int(_) => DbTypeFamily::Integer,
        DbValue::UInt(_) => DbTypeFamily::Integer,
        DbValue::Float(_) => DbTypeFamily::Float,
        DbValue::Decimal(_) => DbTypeFamily::Decimal,
        DbValue::Text(_) => DbTypeFamily::Text,
        DbValue::Bytes(_) => DbTypeFamily::Bytes,
        DbValue::Date(_) => DbTypeFamily::Date,
        DbValue::Time(_) => DbTypeFamily::Time,
        DbValue::DateTime(_) => DbTypeFamily::DateTime,
        DbValue::Json(_) => DbTypeFamily::Json,
        DbValue::Uuid(_) => DbTypeFamily::Uuid,
        DbValue::Array(_) => DbTypeFamily::Array,
        DbValue::Other { .. } => DbTypeFamily::Other,
    }
}
/// 获取 SQLite 触发器
pub(crate) fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    let mut stmt = conn
        .prepare("SELECT name, tbl_name, sql FROM sqlite_master WHERE type='trigger' ORDER BY name")
        .map_err(|e| DbError::Query(e.to_string()))?;

    let triggers: Result<Vec<TriggerInfo>, _> = stmt
        .query_map([], |row| {
            let name: String = row.get(0)?;
            let table_name: String = row.get(1)?;
            let sql: String = row.get(2)?;

            // 从 SQL 中解析 timing 和 event
            let sql_upper = sql.to_uppercase();
            let timing = if sql_upper.contains("BEFORE") {
                "BEFORE"
            } else if sql_upper.contains("AFTER") {
                "AFTER"
            } else if sql_upper.contains("INSTEAD OF") {
                "INSTEAD OF"
            } else {
                "UNKNOWN"
            }
            .to_string();

            let event = if sql_upper.contains("INSERT") {
                "INSERT"
            } else if sql_upper.contains("UPDATE") {
                "UPDATE"
            } else if sql_upper.contains("DELETE") {
                "DELETE"
            } else {
                "UNKNOWN"
            }
            .to_string();

            Ok(TriggerInfo {
                name,
                table_name,
                event,
                timing,
                definition: sql,
            })
        })
        .map_err(|e| DbError::Query(e.to_string()))?
        .collect();

    triggers.map_err(|e| DbError::Query(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ConnectionConfig;
    use tempfile::NamedTempFile;

    fn test_config_for_path(path: impl Into<String>) -> ConnectionConfig {
        ConnectionConfig {
            db_type: DatabaseType::SQLite,
            database: path.into(),
            ..Default::default()
        }
    }

    fn test_config() -> ConnectionConfig {
        test_config_for_path(":memory:")
    }

    #[test]
    fn connect_returns_empty_tables_for_new_db() {
        let config = test_config();
        let tables = connect(&config).unwrap();
        assert!(tables.is_empty());
    }

    #[test]
    fn connect_creates_and_lists_tables() {
        let config = test_config();
        let conn = rusqlite::Connection::open(":memory:").unwrap();
        conn.execute("CREATE TABLE users (id INTEGER, name TEXT)", [])
            .unwrap();
        let tables = connect(&config).unwrap();
        assert_eq!(tables, Vec::<String>::new()); // different connection, no tables visible
        // Verify the original connection sees the table
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table'")
            .unwrap();
        let names: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();
        assert!(names.contains(&"users".to_string()));
    }

    #[test]
    fn get_triggers_returns_empty_for_no_triggers() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute("CREATE TABLE t (x INTEGER)", []).unwrap();
        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let triggers = get_triggers(&config).unwrap();
        assert!(triggers.is_empty());
    }

    #[test]
    fn connect_rejects_missing_database() {
        let config = ConnectionConfig {
            db_type: DatabaseType::SQLite,
            database: "/nonexistent/path/db.sqlite".to_string(),
            ..Default::default()
        };
        let result = connect(&config);
        assert!(result.is_err()); // can't create dirs in test
    }

    #[test]
    fn get_triggers_returns_trigger_info() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute("CREATE TABLE t (x INTEGER)", []).unwrap();
        conn.execute(
            "CREATE TRIGGER trg AFTER INSERT ON t BEGIN UPDATE t SET x = x + 1; END",
            [],
        )
        .unwrap();
        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let triggers = get_triggers(&config).unwrap();
        assert_eq!(triggers.len(), 1);
        assert_eq!(triggers[0].name, "trg");
        assert!(triggers[0].definition.contains("INSERT ON t"));
    }

    #[test]
    fn execute_batch_rolls_back_whole_batch_on_failure() {
        // 验证 B2 原子性：事务批次中任一语句失败，整批回滚，表数据不变。
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute(
            "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO users (id, name) VALUES (1, 'alice')", [])
            .unwrap();

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let statements = vec![
            "UPDATE \"users\" SET \"name\" = 'bob' WHERE \"id\" = 1;".to_string(),
            // 第二条违反 NOT NULL，必然失败 → 整批应回滚
            "UPDATE \"users\" SET \"name\" = NULL WHERE \"id\" = 1;".to_string(),
        ];

        let result = execute_batch(&config, &statements, true, true);
        assert!(result.is_err(), "batch with a failing statement must error");

        // 第一条 UPDATE 也必须被回滚：name 仍为原值 alice。
        let name: String = conn
            .query_row("SELECT name FROM users WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "alice", "transaction must roll back the whole batch");
    }
    // ── Mutation contract tests (Phase 7 Convergence) ──

    use crate::domain::mutation::{
        ColumnRef, ExpectedRows, InputValue, Mutation, MutationBatch, RowIdentity,
    };
    use crate::domain::value::DbValue;

    fn col(name: &str) -> ColumnRef {
        ColumnRef {
            name: name.to_string(),
        }
    }

    fn pk(cols: Vec<(&str, DbValue)>) -> RowIdentity {
        RowIdentity::PrimaryKey(cols.into_iter().map(|(n, v)| (col(n), v)).collect())
    }

    fn apply(
        config: &ConnectionConfig,
        batch: &MutationBatch,
    ) -> Result<super::MutationBatchResult, super::DbError> {
        super::apply_mutations(config, batch)
    }

    fn setup_table(conn: &rusqlite::Connection, ddl: &str) {
        conn.execute_batch(ddl).unwrap();
    }

    fn query_cell(conn: &rusqlite::Connection, sql: &str) -> String {
        conn.query_row(sql, [], |r| -> rusqlite::Result<String> {
            // Try as string first, then handle NULL
            match r.get::<_, Option<String>>(0) {
                Ok(Some(s)) => Ok(s),
                Ok(None) => Ok("NULL".to_string()),
                Err(e) => Err(e),
            }
        })
        .unwrap_or_else(|_| "ERROR".to_string())
    }

    #[test]
    fn apply_insert_unspecified_omits_column() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT DEFAULT 'hello', age INT DEFAULT 0)",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Insert {
                table: col("t"),
                columns: vec![col("id"), col("age")],
                values: vec![
                    InputValue::Value(DbValue::Int(1)),
                    InputValue::Value(DbValue::Int(42)),
                ],
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);
        assert_eq!(result.affected, vec![1]);

        // val column omitted → should get DEFAULT 'hello'
        let val = query_cell(&conn, "SELECT val FROM t WHERE id = 1");
        assert_eq!(val, "hello");
        let age: i64 = conn
            .query_row("SELECT age FROM t WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(age, 42);
    }

    #[test]
    fn apply_insert_null_stores_null() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(&conn, "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)");

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Insert {
                table: col("t"),
                columns: vec![col("id"), col("val")],
                values: vec![InputValue::Value(DbValue::Int(1)), InputValue::Null],
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);

        let val: Option<String> = conn
            .query_row("SELECT val FROM t WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert!(val.is_none(), "val should be NULL");
    }

    #[test]
    fn apply_insert_default_uses_database_default() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT DEFAULT 'hello')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Insert {
                table: col("t"),
                columns: vec![col("id"), col("val")],
                values: vec![InputValue::Value(DbValue::Int(1)), InputValue::Default],
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);

        let val = query_cell(&conn, "SELECT val FROM t WHERE id = 1");
        assert_eq!(val, "hello");
    }

    #[test]
    fn apply_insert_empty_string_is_not_null() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT NOT NULL)",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Insert {
                table: col("t"),
                columns: vec![col("id"), col("val")],
                values: vec![
                    InputValue::Value(DbValue::Int(1)),
                    InputValue::Value(DbValue::Text(String::new())),
                ],
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);

        let val: Option<String> = conn
            .query_row("SELECT val FROM t WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert!(val.is_some(), "val should not be NULL");
        assert_eq!(val.unwrap(), "", "val should be empty string, not NULL");
    }

    #[test]
    fn apply_update_one_row_succeeds() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT); INSERT INTO t VALUES (1, 'old')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Update {
                table: col("t"),
                identity: pk(vec![("id", DbValue::Int(1))]),
                changes: vec![(
                    col("val"),
                    InputValue::Value(DbValue::Text("new".to_string())),
                )],
                expected_rows: ExpectedRows::Exactly(1),
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);
        assert_eq!(result.affected, vec![1]);

        let val = query_cell(&conn, "SELECT val FROM t WHERE id = 1");
        assert_eq!(val, "new");
    }

    #[test]
    fn apply_update_zero_rows_is_error() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT); INSERT INTO t VALUES (1, 'old')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Update {
                table: col("t"),
                identity: pk(vec![("id", DbValue::Int(999))]),
                changes: vec![(
                    col("val"),
                    InputValue::Value(DbValue::Text("new".to_string())),
                )],
                expected_rows: ExpectedRows::Exactly(1),
            }],
            atomic: true,
        };

        let result = apply(&config, &batch);
        assert!(result.is_err(), "update of non-existent row should error");
    }

    #[test]
    fn apply_update_primary_key_succeeds() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT); INSERT INTO t VALUES (1, 'old')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Update {
                table: col("t"),
                identity: pk(vec![("id", DbValue::Int(1))]),
                changes: vec![
                    (col("id"), InputValue::Value(DbValue::Int(2))),
                    (
                        col("val"),
                        InputValue::Value(DbValue::Text("new".to_string())),
                    ),
                ],
                expected_rows: ExpectedRows::Exactly(1),
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);

        // Old id=1 should be gone
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
        // New id=2 should exist with 'new'
        let val = query_cell(&conn, "SELECT val FROM t WHERE id = 2");
        assert_eq!(val, "new");
    }

    #[test]
    fn apply_delete_one_row_succeeds() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT); INSERT INTO t VALUES (1, 'a')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Delete {
                table: col("t"),
                identity: pk(vec![("id", DbValue::Int(1))]),
                expected_rows: ExpectedRows::Exactly(1),
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);
        assert_eq!(result.affected, vec![1]);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_mutation_failure_rolls_back_batch() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT NOT NULL); INSERT INTO t VALUES (1, 'alice')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![
                Mutation::Update {
                    table: col("t"),
                    identity: pk(vec![("id", DbValue::Int(1))]),
                    changes: vec![(
                        col("val"),
                        InputValue::Value(DbValue::Text("bob".to_string())),
                    )],
                    expected_rows: ExpectedRows::Exactly(1),
                },
                Mutation::Update {
                    table: col("t"),
                    identity: pk(vec![("id", DbValue::Int(1))]),
                    changes: vec![(col("val"), InputValue::Null)], // NOT NULL violation
                    expected_rows: ExpectedRows::Exactly(1),
                },
            ],
            atomic: true,
        };

        let result = apply(&config, &batch);
        assert!(
            result.is_err(),
            "batch with NOT NULL violation should error"
        );

        // First UPDATE must be rolled back
        let val = query_cell(&conn, "SELECT val FROM t WHERE id = 1");
        assert_eq!(val, "alice", "transaction must roll back on error");
    }

    #[test]
    fn apply_composite_primary_key_update() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (a INT, b INT, val TEXT, PRIMARY KEY (a, b)); INSERT INTO t VALUES (1, 2, 'old')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Update {
                table: col("t"),
                identity: pk(vec![("a", DbValue::Int(1)), ("b", DbValue::Int(2))]),
                changes: vec![(
                    col("val"),
                    InputValue::Value(DbValue::Text("new".to_string())),
                )],
                expected_rows: ExpectedRows::Exactly(1),
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);

        let val = query_cell(&conn, "SELECT val FROM t WHERE a = 1 AND b = 2");
        assert_eq!(val, "new");
    }

    #[test]
    fn apply_composite_primary_key_delete() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (a INT, b INT, val TEXT, PRIMARY KEY (a, b)); INSERT INTO t VALUES (1, 2, 'old')",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Delete {
                table: col("t"),
                identity: pk(vec![("a", DbValue::Int(1)), ("b", DbValue::Int(2))]),
                expected_rows: ExpectedRows::Exactly(1),
            }],
            atomic: true,
        };

        let result = apply(&config, &batch).unwrap();
        assert!(result.all_success);

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM t", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn apply_unsigned_overflow_is_rejected() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(
            &conn,
            "CREATE TABLE t (id INTEGER PRIMARY KEY, val INTEGER)",
        );

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Insert {
                table: col("t"),
                columns: vec![col("id"), col("val")],
                values: vec![
                    InputValue::Value(DbValue::Int(1)),
                    InputValue::Value(DbValue::UInt(u64::MAX)),
                ],
            }],
            atomic: true,
        };

        let result = apply(&config, &batch);
        assert!(result.is_err(), "u64::MAX should be rejected");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exceeds"),
            "error should mention range overflow"
        );
    }

    #[test]
    fn apply_unsupported_value_is_rejected() {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        setup_table(&conn, "CREATE TABLE t (id INTEGER PRIMARY KEY, val TEXT)");

        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        let batch = MutationBatch {
            mutations: vec![Mutation::Insert {
                table: col("t"),
                columns: vec![col("id"), col("val")],
                values: vec![
                    InputValue::Value(DbValue::Int(1)),
                    InputValue::Value(DbValue::Array(vec![])),
                ],
            }],
            atomic: true,
        };

        let result = apply(&config, &batch);
        assert!(result.is_err(), "DbValue::Array should be rejected");
    }

    // ─── Sprint 3.1: SchemaCatalog 合约测试 ───

    /// 辅助：创建临时文件 SQLite 数据库并返回 config
    fn temp_db(ddl: &str) -> (NamedTempFile, ConnectionConfig) {
        let db = NamedTempFile::new().unwrap();
        let conn = rusqlite::Connection::open(db.path()).unwrap();
        conn.execute_batch(ddl).unwrap();
        let config = test_config_for_path(db.path().to_string_lossy().into_owned());
        (db, config)
    }

    #[test]
    fn catalog_loads_primary_key() {
        let (_db, config) = temp_db("CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT);");
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        let t = catalog.table("t").expect("table t should exist");
        let pk = t.primary_key.as_ref().expect("should have a PK");
        assert_eq!(pk.columns, vec!["id"]);
    }

    #[test]
    fn catalog_loads_composite_primary_key() {
        let (_db, config) = temp_db("CREATE TABLE t (a INT, b INT, val TEXT, PRIMARY KEY (a, b));");
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        let t = catalog.table("t").expect("table t should exist");
        let pk = t.primary_key.as_ref().expect("should have composite PK");
        assert_eq!(pk.columns.len(), 2);
        assert!(pk.columns.contains(&"a".to_string()));
        assert!(pk.columns.contains(&"b".to_string()));
    }

    #[test]
    fn catalog_loads_foreign_key() {
        let (_db, config) = temp_db(
            "CREATE TABLE parent (id INTEGER PRIMARY KEY);
             CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER REFERENCES parent(id));",
        );
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        let child = catalog.table("child").expect("child should exist");
        assert!(!child.foreign_keys.is_empty(), "child should have FK");
        let fk = &child.foreign_keys[0];
        assert_eq!(fk.from_columns, vec!["parent_id"]);
        assert_eq!(fk.ref_table, "parent");
        assert_eq!(fk.ref_columns, vec!["id"]);
    }

    #[test]
    fn catalog_preserves_column_order() {
        let (_db, config) = temp_db("CREATE TABLE t (z INT, a INT, m INT, b INT);");
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        let t = catalog.table("t").expect("table t should exist");
        let names: Vec<&str> = t.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["z", "a", "m", "b"]);
    }

    #[test]
    fn catalog_loads_default_value() {
        let (_db, config) = temp_db(
            "CREATE TABLE t (id INTEGER PRIMARY KEY, name TEXT DEFAULT 'hello', age INT DEFAULT 0);",
        );
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        let t = catalog.table("t").expect("table t should exist");
        let name_col = t.columns.iter().find(|c| c.name == "name").unwrap();
        assert_eq!(name_col.default_value.as_deref(), Some("'hello'"));
        let age_col = t.columns.iter().find(|c| c.name == "age").unwrap();
        assert_eq!(age_col.default_value.as_deref(), Some("0"));
    }

    #[test]
    fn catalog_empty_database_is_valid() {
        let (_db, config) = temp_db(""); // empty DDL
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        assert!(catalog.tables.is_empty());
        assert_eq!(catalog.revision, SchemaRevision(1));
    }

    #[test]
    fn catalog_handles_special_table_name() {
        // SQLite allows unusual but valid table names
        let (_db, config) =
            temp_db("CREATE TABLE \"my-table\" (id INTEGER PRIMARY KEY, \"col name\" TEXT);");
        let catalog = super::load_catalog(&config, SchemaRevision(1)).unwrap();
        let t = catalog
            .table("my-table")
            .expect("table with hyphen should exist");
        assert_eq!(t.columns.len(), 2);
    }
}
