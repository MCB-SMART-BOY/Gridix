//! MySQL 查询实现

use super::{ImportExecutionReport, RoutineInfo, RoutineType, TriggerInfo, is_query_statement};
use crate::core::constants;

use crate::data::{ConnectionConfig, DatabaseType, DbError, POOL_MANAGER};

/// 获取 MySQL 数据库列表
pub(crate) async fn get_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let databases: Vec<String> = conn
        .query("SHOW DATABASES")
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    // 过滤系统数据库
    Ok(databases
        .into_iter()
        .filter(|db| {
            !matches!(
                db.as_str(),
                "information_schema" | "mysql" | "performance_schema" | "sys"
            )
        })
        .collect())
}

/// 获取 MySQL 指定数据库的表列表
pub(crate) async fn get_tables(
    config: &ConnectionConfig,
    database: &str,
) -> Result<Vec<String>, DbError> {
    // 创建一个临时配置，连接到指定数据库
    let mut db_config = config.clone();
    db_config.database = database.to_string();

    let pool = POOL_MANAGER.get_mysql_pool(&db_config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let tables: Vec<String> = conn
        .query("SHOW TABLES")
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(tables)
}

/// 删除 MySQL 数据库。
pub(crate) async fn drop_database(
    config: &ConnectionConfig,
    database: &str,
) -> Result<(), DbError> {
    let sql = format!("DROP DATABASE {}", quote_mysql_identifier(database));
    let mut last_error = None;

    for maintenance_db in mysql_maintenance_databases(config, database) {
        let mut maintenance_config = config.clone();
        maintenance_config.database = maintenance_db;

        match POOL_MANAGER.get_mysql_pool(&maintenance_config).await {
            Ok(pool) => match pool.get_conn().await {
                Ok(mut conn) => {
                    return conn
                        .query_drop(sql.clone())
                        .await
                        .map_err(|e| DbError::Query(e.to_string()));
                }
                Err(error) => {
                    last_error = Some(format!("MySQL 获取连接失败: {}", error));
                }
            },
            Err(error) => {
                last_error = Some(format!("MySQL 维护连接失败: {}", error));
            }
        }
    }

    Err(DbError::Connection(last_error.unwrap_or_else(|| {
        "未找到可用的 MySQL 维护数据库来执行 DROP DATABASE".to_string()
    })))
}

fn mysql_maintenance_databases(config: &ConnectionConfig, target_database: &str) -> Vec<String> {
    let mut databases = Vec::new();
    for candidate in [
        config.database.as_str(),
        "mysql",
        "sys",
        "information_schema",
        "",
    ] {
        let trimmed = candidate.trim();
        if trimmed == target_database || databases.iter().any(|db| db == trimmed) {
            continue;
        }
        databases.push(trimmed.to_string());
    }
    databases
}

fn quote_mysql_identifier(name: &str) -> String {
    name.split('.')
        .map(|part| format!("`{}`", part.replace('`', "``")))
        .collect::<Vec<_>>()
        .join(".")
}

/// 批量执行 MySQL 语句（用于导入）
pub(crate) async fn execute_batch(
    config: &ConnectionConfig,
    statements: &[String],
    use_transaction: bool,
    stop_on_error: bool,
) -> Result<ImportExecutionReport, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let mut report = ImportExecutionReport::new(statements.len());
    if statements.is_empty() {
        return Ok(report);
    }

    if use_transaction {
        conn.query_drop("START TRANSACTION")
            .await
            .map_err(|e| DbError::Query(format!("开启事务失败: {}", e)))?;
    }

    for (index, statement) in statements.iter().enumerate() {
        let exec_result = conn.query_iter(statement).await;
        match exec_result {
            Ok(result) => {
                drop(result);
                report.succeeded += 1;
            }
            Err(e) => {
                let err_msg = format!("第 {} 条语句执行失败: {}", index + 1, e);

                if use_transaction {
                    if let Err(rollback_err) = conn.query_drop("ROLLBACK").await {
                        return Err(DbError::Query(format!(
                            "事务回滚失败（原错误: {}，回滚错误: {}）",
                            err_msg, rollback_err
                        )));
                    }
                    return Err(DbError::Query(format!("事务已回滚，{}", err_msg)));
                }

                report.failed += 1;
                if report.first_error.is_none() {
                    report.first_error = Some(err_msg.clone());
                }

                if stop_on_error {
                    return Err(DbError::Query(err_msg));
                }
            }
        }
    }

    if use_transaction {
        conn.query_drop("COMMIT")
            .await
            .map_err(|e| DbError::Query(format!("提交事务失败: {}", e)))?;
    }

    Ok(report)
}

/// 获取 MySQL 触发器
pub(crate) async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = r#"
        SELECT 
            TRIGGER_NAME,
            EVENT_OBJECT_TABLE,
            ACTION_TIMING,
            EVENT_MANIPULATION,
            ACTION_STATEMENT
        FROM INFORMATION_SCHEMA.TRIGGERS
        WHERE TRIGGER_SCHEMA = DATABASE()
        ORDER BY TRIGGER_NAME
    "#;

    let result: Vec<mysql_async::Row> = conn
        .query(sql)
        .await
        .map_err(|e| DbError::Query(format!("查询触发器失败: {}", e)))?;

    let triggers: Vec<TriggerInfo> = result
        .iter()
        .map(|row| {
            let name: String = row.get(0).unwrap_or_default();
            let table_name: String = row.get(1).unwrap_or_default();
            let timing: String = row.get(2).unwrap_or_default();
            let event: String = row.get(3).unwrap_or_default();
            let action: String = row.get(4).unwrap_or_default();

            // 构造完整的触发器定义
            let definition = format!(
                "CREATE TRIGGER {} {} {} ON {} FOR EACH ROW {}",
                name, timing, event, table_name, action
            );

            TriggerInfo {
                name,
                table_name,
                event,
                timing,
                definition,
            }
        })
        .collect();

    Ok(triggers)
}

/// 获取 MySQL 存储过程和函数
pub(crate) async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = r#"
        SELECT 
            ROUTINE_NAME,
            ROUTINE_TYPE,
            ROUTINE_DEFINITION,
            DTD_IDENTIFIER
        FROM INFORMATION_SCHEMA.ROUTINES
        WHERE ROUTINE_SCHEMA = DATABASE()
        ORDER BY ROUTINE_TYPE, ROUTINE_NAME
    "#;

    let result: Vec<mysql_async::Row> = conn
        .query(sql)
        .await
        .map_err(|e| DbError::Query(format!("查询存储过程失败: {}", e)))?;

    // 获取参数信息
    let params_sql = r#"
        SELECT 
            SPECIFIC_NAME,
            PARAMETER_MODE,
            PARAMETER_NAME,
            DATA_TYPE
        FROM INFORMATION_SCHEMA.PARAMETERS
        WHERE SPECIFIC_SCHEMA = DATABASE()
        ORDER BY SPECIFIC_NAME, ORDINAL_POSITION
    "#;

    let params_result: Vec<mysql_async::Row> = match conn.query(params_sql).await {
        Ok(rows) => rows,
        Err(e) => {
            tracing::warn!(error = %e, "查询存储过程参数失败，将继续返回基础信息");
            Vec::new()
        }
    };

    // 构建参数映射
    let mut params_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for row in &params_result {
        let routine_name: String = row.get(0).unwrap_or_default();
        let mode: Option<String> = row.get(1).unwrap_or(None);
        let param_name: Option<String> = row.get(2).unwrap_or(None);
        let data_type: String = row.get(3).unwrap_or_default();

        // 跳过返回值参数（PARAMETER_NAME 为 NULL 且 PARAMETER_MODE 为 NULL）
        if let Some(name) = param_name {
            let param_str = if let Some(m) = mode {
                format!("{} {} {}", m, name, data_type)
            } else {
                format!("{} {}", name, data_type)
            };
            params_map.entry(routine_name).or_default().push(param_str);
        }
    }

    let routines: Vec<RoutineInfo> = result
        .iter()
        .map(|row| {
            let name: String = row.get(0).unwrap_or_default();
            let type_str: String = row.get(1).unwrap_or_default();
            let definition: Option<String> = row.get(2).unwrap_or(None);
            let return_type: Option<String> = row.get(3).unwrap_or(None);

            let routine_type = if type_str == "FUNCTION" {
                RoutineType::Function
            } else {
                RoutineType::Procedure
            };

            let parameters = params_map
                .get(&name)
                .map(|p| p.join(", "))
                .unwrap_or_default();

            RoutineInfo {
                name,
                routine_type,
                parameters,
                return_type,
                definition: definition.unwrap_or_else(|| "(定义不可见)".to_string()),
            }
        })
        .collect();

    Ok(routines)
}

// ── SchemaCatalog 加载 (Phase 6 / Sprint 3) ──

use super::infer_type_family;
use crate::domain::ids::SchemaRevision;
use crate::domain::metadata::{
    ColumnMetadata, ForeignKeyMetadata, KeyMetadata, SchemaCatalog, TableMetadata,
};

/// 从 information_schema 加载 MySQL schema 元数据
pub(crate) async fn load_catalog(
    config: &ConnectionConfig,
    revision: SchemaRevision,
) -> Result<SchemaCatalog, DbError> {
    let pool = POOL_MANAGER
        .get_mysql_pool(config)
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 连接池获取失败: {}", e)))?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 连接失败: {}", e)))?;

    // 获取当前连接的数据库名
    let schema_name: String = conn
        .query("SELECT DATABASE()")
        .await
        .map_err(|e| DbError::Query(format!("查询当前数据库失败: {}", e)))?
        .first()
        .map(|row: &mysql_async::Row| row.get::<String, _>(0).unwrap_or_default())
        .unwrap_or_default();
    // 1. 获取表列表
    let table_names: Vec<String> = conn
        .query(
            "SELECT TABLE_NAME FROM information_schema.TABLES \
                WHERE TABLE_SCHEMA = DATABASE() AND TABLE_TYPE = 'BASE TABLE' \
                ORDER BY TABLE_NAME",
        )
        .await
        .map_err(|e| DbError::Query(format!("查询表列表失败: {}", e)))?
        .into_iter()
        .map(|row: mysql_async::Row| row.get::<String, _>(0).unwrap_or_default())
        .collect();

    let mut tables = Vec::with_capacity(table_names.len());

    for table_name in &table_names {
        // 2. 列信息
        let col_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT COLUMN_NAME, ORDINAL_POSITION, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT \
                 FROM information_schema.COLUMNS \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? \
                 ORDER BY ORDINAL_POSITION",
                (table_name.as_str(),),
            )
            .await
            .map_err(|e| DbError::Query(format!("查询列信息失败: {}", e)))?;

        // 3. 主键
        let pk_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT COLUMN_NAME FROM information_schema.STATISTICS \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? \
                   AND INDEX_NAME = 'PRIMARY'",
                (table_name.as_str(),),
            )
            .await
            .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;

        let pk_columns: Vec<String> = pk_rows
            .iter()
            .map(|r| r.get::<String, _>(0).unwrap_or_default())
            .collect();

        let primary_key = if pk_columns.is_empty() {
            None
        } else {
            Some(KeyMetadata {
                name: None,
                columns: pk_columns.clone(),
            })
        };

        // 4. 外键
        let fk_rows: Vec<mysql_async::Row> = conn
            .exec(
                "SELECT COLUMN_NAME, REFERENCED_TABLE_NAME, REFERENCED_COLUMN_NAME \
                 FROM information_schema.KEY_COLUMN_USAGE \
                 WHERE TABLE_SCHEMA = DATABASE() AND TABLE_NAME = ? \
                   AND REFERENCED_TABLE_NAME IS NOT NULL",
                (table_name.as_str(),),
            )
            .await
            .map_err(|e| DbError::Query(format!("查询外键失败: {}", e)))?;

        let foreign_keys: Vec<ForeignKeyMetadata> = fk_rows
            .iter()
            .map(|r| ForeignKeyMetadata {
                name: None,
                from_columns: vec![r.get::<String, _>(0).unwrap_or_default()],
                ref_table: r.get::<String, _>(1).unwrap_or_default(),
                ref_columns: vec![r.get::<String, _>(2).unwrap_or_default()],
            })
            .collect();

        let is_pk = |col_name: &str| pk_columns.iter().any(|pk| pk == col_name);

        let columns: Vec<ColumnMetadata> = col_rows
            .iter()
            .map(|r| -> ColumnMetadata {
                let col_name: String = r.get::<String, _>(0).unwrap_or_default();
                let pos: u32 = r.get::<u32, _>(1).unwrap_or(0);
                let data_type: String = r.get::<String, _>(2).unwrap_or_default();
                let is_nullable_str: String = r.get::<String, _>(3).unwrap_or_default();
                let default: Option<String> = r.get::<Option<String>, _>(4).unwrap_or(None);

                ColumnMetadata {
                    name: col_name.clone(),
                    position: pos as usize,
                    type_info: crate::domain::value::DbTypeInfo {
                        family: infer_type_family(&data_type),
                        native_name: data_type,
                        nullable: Some(is_nullable_str == "YES"),
                    },
                    is_nullable: is_nullable_str == "YES",
                    is_primary_key: is_pk(&col_name),
                    default_value: default,
                }
            })
            .collect();

        tables.push(TableMetadata {
            name: table_name.clone(),
            schema: Some(schema_name.clone()),
            columns,
            primary_key,
            unique_keys: Vec::new(),
            foreign_keys,
        });
    }

    Ok(SchemaCatalog { revision, tables })
}

// ── Typed ResultSet 执行 (Phase 4 / Sprint 2) ──

use crate::domain::execution::ExecutionOutcome;
use crate::domain::result::{ResultColumn, ResultCompleteness, ResultSet};
use crate::domain::value::{DbTypeFamily, DbTypeInfo, DbValue};
use mysql_async::consts::ColumnType;
use mysql_async::prelude::Queryable;

/// 执行 SQL 并返回类型化 ResultSet（MySQL 原生 Value 路径）
pub(crate) async fn execute_typed(
    config: &ConnectionConfig,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    let pool = POOL_MANAGER
        .get_mysql_pool(config)
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 连接池获取失败: {}", e)))?;
    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 连接失败: {}", e)))?;
    execute_typed_with_conn(&mut conn, sql).await
}

/// 使用单独控制连接发送 KILL QUERY，协作取消正在执行的 MySQL 查询。
pub(crate) async fn execute_typed_cancellable(
    config: &ConnectionConfig,
    sql: &str,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<ExecutionOutcome, DbError> {
    if cancellation.is_cancelled() {
        return Err(DbError::Cancelled);
    }

    let pool = POOL_MANAGER
        .get_mysql_pool(config)
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 连接池获取失败: {}", e)))?;
    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 执行连接获取失败: {}", e)))?;
    let connection_id = conn.id();
    let query = execute_typed_with_conn(&mut conn, sql);
    tokio::pin!(query);

    tokio::select! {
        biased;
        result = &mut query => result,
        _ = cancellation.cancelled() => {
            cancel_mysql_query(&pool, connection_id).await?;
            match query.await {
                Err(_) => Err(DbError::Cancelled),
                Ok(outcome) => Ok(outcome),
            }
        }
    }
}

async fn cancel_mysql_query(pool: &mysql_async::Pool, connection_id: u32) -> Result<(), DbError> {
    let mut control = pool.get_conn().await.map_err(|e| {
        DbError::Connection(format!(
            "获取 MySQL 取消控制连接失败（connection_id={}）: {}",
            connection_id, e
        ))
    })?;
    control
        .query_drop(format!("KILL QUERY {}", connection_id))
        .await
        .map_err(|e| {
            DbError::Query(format!(
                "取消 MySQL 查询失败（connection_id={}）: {}",
                connection_id, e
            ))
        })
}

async fn execute_typed_with_conn(
    conn: &mut mysql_async::Conn,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    if !is_query_statement(sql, &DatabaseType::MySQL) {
        conn.exec_drop(sql, ())
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        let affected = conn.affected_rows();
        return Ok(ExecutionOutcome::affected_rows(affected));
    }
    let mut result = conn
        .query_iter(sql)
        .await
        .map_err(|e| DbError::Query(format!("MySQL query_iter 失败: {}", e)))?;

    let columns: Vec<mysql_async::Column> = result.columns_ref().to_vec();
    let col_names: Vec<String> = columns.iter().map(|c| c.name_str().into_owned()).collect();
    let col_types: Vec<ColumnType> = columns.iter().map(|c| c.column_type()).collect();
    let col_count = col_names.len();
    let max_rows = constants::database::MAX_RESULT_SET_ROWS;

    let mut cells: Vec<DbValue> = Vec::new();
    let mut total_rows = 0usize;
    loop {
        let row_opt = result
            .next()
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        let Some(row) = row_opt else { break };
        total_rows += 1;
        if total_rows <= max_rows + 1 && cells.len() / col_count < max_rows {
            for (index, column_type) in col_types.iter().enumerate() {
                let val: mysql_async::Value = row.get(index).unwrap_or(mysql_async::Value::NULL);
                cells.push(mysql_value_to_dbvalue(val, column_type));
            }
        }
        if total_rows > max_rows {
            break;
        }
    }

    let row_count = cells.len() / col_count;
    let completeness = if total_rows > max_rows {
        ResultCompleteness::Truncated {
            displayed: row_count,
        }
    } else {
        ResultCompleteness::Complete
    };

    let typed_columns: std::sync::Arc<[ResultColumn]> = col_names
        .iter()
        .enumerate()
        .map(|(i, name)| ResultColumn {
            name: name.clone(),
            type_info: DbTypeInfo {
                family: mysql_type_to_family(&col_types[i]),
                native_name: format!("{:?}", col_types[i]),
                nullable: None,
            },
        })
        .collect();

    Ok(ExecutionOutcome::single_result(ResultSet {
        columns: typed_columns,
        cells,
        row_count,
        completeness,
    }))
}
fn mysql_value_to_dbvalue(val: mysql_async::Value, col_type: &ColumnType) -> DbValue {
    use mysql_async::Value;
    match val {
        Value::NULL => DbValue::Null,
        Value::Int(i) => DbValue::Int(i),
        Value::UInt(u) => DbValue::UInt(u),
        Value::Float(f) => DbValue::Float(f as f64),
        Value::Double(d) => DbValue::Float(d),
        Value::Bytes(b) => match col_type {
            ColumnType::MYSQL_TYPE_BLOB
            | ColumnType::MYSQL_TYPE_LONG_BLOB
            | ColumnType::MYSQL_TYPE_MEDIUM_BLOB
            | ColumnType::MYSQL_TYPE_TINY_BLOB
            | ColumnType::MYSQL_TYPE_BIT => DbValue::Bytes(std::sync::Arc::from(b)),
            _ => String::from_utf8(b)
                .map(DbValue::Text)
                .unwrap_or(DbValue::Null),
        },
        Value::Date(y, m, d, h, mi, s, us) => {
            if h == 0 && mi == 0 && s == 0 && us == 0 {
                DbValue::Date(crate::domain::value::DbDate {
                    year: y as i32,
                    month: m,
                    day: d,
                })
            } else {
                DbValue::DateTime(crate::domain::value::DbDateTime {
                    date: crate::domain::value::DbDate {
                        year: y as i32,
                        month: m,
                        day: d,
                    },
                    time: crate::domain::value::DbTime {
                        hour: h,
                        minute: mi,
                        second: s,
                        nanos: us * 1000,
                    },
                })
            }
        }
        Value::Time(neg, d, h, mi, s, us) => {
            let total_seconds = if neg { -(s as i64) } else { s as i64 };
            let total_hours = (d as i64) * 24 + (h as i64);
            DbValue::Time(crate::domain::value::DbTime {
                hour: total_hours as u8,
                minute: mi,
                second: total_seconds.unsigned_abs() as u8,
                nanos: us * 1000,
            })
        }
    }
}

fn mysql_type_to_family(ct: &ColumnType) -> DbTypeFamily {
    use mysql_async::consts::ColumnType::*;
    match ct {
        MYSQL_TYPE_NULL | MYSQL_TYPE_DECIMAL | MYSQL_TYPE_NEWDECIMAL => DbTypeFamily::Decimal,
        MYSQL_TYPE_TINY | MYSQL_TYPE_SHORT | MYSQL_TYPE_LONG | MYSQL_TYPE_LONGLONG
        | MYSQL_TYPE_INT24 => DbTypeFamily::Integer,
        MYSQL_TYPE_FLOAT | MYSQL_TYPE_DOUBLE => DbTypeFamily::Float,
        MYSQL_TYPE_VARCHAR
        | MYSQL_TYPE_VAR_STRING
        | MYSQL_TYPE_STRING
        | MYSQL_TYPE_ENUM
        | MYSQL_TYPE_SET
        | MYSQL_TYPE_TINY_BLOB
        | MYSQL_TYPE_MEDIUM_BLOB
        | MYSQL_TYPE_LONG_BLOB
        | MYSQL_TYPE_BLOB
        | MYSQL_TYPE_JSON
        | MYSQL_TYPE_GEOMETRY => DbTypeFamily::Text,
        MYSQL_TYPE_TIMESTAMP | MYSQL_TYPE_TIMESTAMP2 => DbTypeFamily::DateTime,
        MYSQL_TYPE_DATE | MYSQL_TYPE_NEWDATE => DbTypeFamily::Date,
        MYSQL_TYPE_TIME | MYSQL_TYPE_TIME2 => DbTypeFamily::Time,
        MYSQL_TYPE_DATETIME | MYSQL_TYPE_DATETIME2 => DbTypeFamily::DateTime,
        MYSQL_TYPE_YEAR => DbTypeFamily::Integer,
        MYSQL_TYPE_BIT => DbTypeFamily::Bytes,
        _ => DbTypeFamily::Text,
    }
}

use crate::domain::mutation::{
    ExpectedRows, InputValue, Mutation, MutationBatch, MutationBatchResult, RowIdentity,
};

/// DbValue → mysql_async::Value 转换。
fn dbvalue_to_mysql(value: &crate::domain::value::DbValue) -> mysql_async::Value {
    use crate::domain::value::DbValue;
    match value {
        DbValue::Null => mysql_async::Value::NULL,
        DbValue::Bool(b) => mysql_async::Value::Int(if *b { 1 } else { 0 }),
        DbValue::Int(i) => mysql_async::Value::Int(*i),
        DbValue::UInt(u) => mysql_async::Value::UInt(*u),
        DbValue::Float(f) => mysql_async::Value::Double(*f),
        DbValue::Text(s) => mysql_async::Value::Bytes(s.as_bytes().to_vec()),
        DbValue::Bytes(b) => mysql_async::Value::Bytes(b.to_vec()),
        DbValue::Decimal(d) => mysql_async::Value::Bytes(d.as_bytes().to_vec()),
        DbValue::Json(j) => mysql_async::Value::Bytes(j.to_string().as_bytes().to_vec()),
        DbValue::Uuid(u) => mysql_async::Value::Bytes(u.to_string().as_bytes().to_vec()),
        _ => mysql_async::Value::NULL,
    }
}

/// 以参数化方式执行 MutationBatch（MySQL 实现）。
pub(crate) async fn apply_mutations(
    config: &ConnectionConfig,
    batch: &MutationBatch,
) -> Result<MutationBatchResult, DbError> {
    use crate::domain::identifier::IdentifierDialect;
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;
    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL: {e}")))?;
    conn.query_drop("START TRANSACTION")
        .await
        .map_err(|e| DbError::Query(format!("MySQL BEGIN: {e}")))?;

    let result: Result<Vec<u64>, DbError> = async {
        let mut rows_vec = Vec::with_capacity(batch.mutations.len());
        for mutation in batch.mutations.iter() {
            let rows = match mutation {
                Mutation::Insert {
                    table,
                    columns,
                    values,
                } => {
                    let included: Vec<(&str, &InputValue)> = columns
                        .iter()
                        .zip(values.iter())
                        .filter(|(_, v)| {
                            !matches!(v, InputValue::Unspecified | InputValue::Default)
                        })
                        .map(|(c, v)| (c.name.as_str(), v))
                        .collect();
                    if included.is_empty() {
                        let sql = format!(
                            "INSERT INTO {} () VALUES ()",
                            IdentifierDialect::MySql.quote(&table.name)
                        );
                        conn.exec_drop(sql, ())
                            .await
                            .map_err(|e| DbError::Query(format!("MySQL INSERT DEFAULT: {e}")))?;
                        1u64
                    } else {
                        let cols: Vec<String> = included
                            .iter()
                            .map(|(name, _)| IdentifierDialect::MySql.quote(name))
                            .collect();
                        let placeholders = vec!["?"; included.len()].join(", ");
                        let sql = format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            IdentifierDialect::MySql.quote(&table.name),
                            cols.join(", "),
                            placeholders
                        );
                        let params: Vec<mysql_async::Value> =
                            included.iter().map(|(_, v)| mysql_value_param(v)).collect();
                        conn.exec_drop(sql, mysql_async::Params::Positional(params))
                            .await
                            .map_err(|e| DbError::Query(format!("MySQL INSERT: {e}")))?;
                        conn.affected_rows()
                    }
                }
                Mutation::Update {
                    table,
                    identity,
                    changes,
                    expected_rows,
                } => {
                    let pk = extract_pk_mysql(identity)?;
                    let set_sql: Vec<String> = changes
                        .iter()
                        .map(|(col, _)| {
                            format!("{} = ?", IdentifierDialect::MySql.quote(&col.name))
                        })
                        .collect();
                    let where_sql: Vec<String> = pk
                        .iter()
                        .map(|(col, _)| {
                            format!("{} = ?", IdentifierDialect::MySql.quote(&col.name))
                        })
                        .collect();
                    let sql = format!(
                        "UPDATE {} SET {} WHERE {}",
                        IdentifierDialect::MySql.quote(&table.name),
                        set_sql.join(", "),
                        where_sql.join(" AND ")
                    );
                    let mut params: Vec<mysql_async::Value> =
                        changes.iter().map(|(_, v)| mysql_value_param(v)).collect();
                    params.extend(pk.iter().map(|(_, v)| dbvalue_to_mysql(v)));
                    conn.exec_drop(sql, mysql_async::Params::Positional(params))
                        .await
                        .map_err(|e| DbError::Query(format!("MySQL UPDATE: {e}")))?;
                    let n = conn.affected_rows();
                    if let ExpectedRows::Exactly(e) = expected_rows
                        && n != *e
                    {
                        return Err(DbError::Query(format!(
                            "MySQL UPDATE: expected {e} rows, affected {n}"
                        )));
                    }
                    n
                }
                Mutation::Delete {
                    table,
                    identity,
                    expected_rows,
                } => {
                    let pk = extract_pk_mysql(identity)?;
                    let where_sql: Vec<String> = pk
                        .iter()
                        .map(|(col, _)| {
                            format!("{} = ?", IdentifierDialect::MySql.quote(&col.name))
                        })
                        .collect();
                    let sql = format!(
                        "DELETE FROM {} WHERE {}",
                        IdentifierDialect::MySql.quote(&table.name),
                        where_sql.join(" AND ")
                    );
                    let params: Vec<mysql_async::Value> =
                        pk.iter().map(|(_, v)| dbvalue_to_mysql(v)).collect();
                    conn.exec_drop(sql, mysql_async::Params::Positional(params))
                        .await
                        .map_err(|e| DbError::Query(format!("MySQL DELETE: {e}")))?;
                    let n = conn.affected_rows();
                    if let ExpectedRows::Exactly(e) = expected_rows
                        && n != *e
                    {
                        return Err(DbError::Query(format!(
                            "MySQL DELETE: expected {e} rows, affected {n}"
                        )));
                    }
                    n
                }
            };
            rows_vec.push(rows);
        }
        Ok(rows_vec)
    }
    .await;

    match result {
        Ok(r) => {
            conn.query_drop("COMMIT")
                .await
                .map_err(|e| DbError::Query(format!("MySQL COMMIT: {e}")))?;
            Ok(MutationBatchResult {
                affected: r,
                all_success: true,
            })
        }
        Err(e) => {
            let _ = conn.query_drop("ROLLBACK").await;
            Err(e)
        }
    }
}

fn mysql_value_param(v: &InputValue) -> mysql_async::Value {
    match v {
        InputValue::Value(dv) => dbvalue_to_mysql(dv),
        InputValue::Null => mysql_async::Value::NULL,
        _ => mysql_async::Value::NULL,
    }
}

fn extract_pk_mysql(
    identity: &RowIdentity,
) -> Result<
    &Vec<(
        crate::domain::mutation::ColumnRef,
        crate::domain::value::DbValue,
    )>,
    DbError,
> {
    match identity {
        RowIdentity::PrimaryKey(cols) => Ok(cols),
        RowIdentity::UniqueKey { columns, .. } => Ok(columns),
    }
}
#[cfg(test)]
mod tests {
    use super::{mysql_maintenance_databases, quote_mysql_identifier};
    use crate::data::{ConnectionConfig, DatabaseType};

    #[test]
    fn test_quote_mysql_identifier_schema_table() {
        let quoted = quote_mysql_identifier("my_db.user-table");
        assert_eq!(quoted, "`my_db`.`user-table`");
    }

    #[test]
    fn test_quote_mysql_identifier_escapes_backticks() {
        let quoted = quote_mysql_identifier("na`me");
        assert_eq!(quoted, "`na``me`");
    }

    #[test]
    fn test_mysql_drop_database_uses_maintenance_candidates() {
        let mut config = ConnectionConfig {
            db_type: DatabaseType::MySQL,
            ..Default::default()
        };
        config.database = "app_db".to_string();

        let candidates = mysql_maintenance_databases(&config, "app_db");

        assert_eq!(
            candidates,
            vec![
                "mysql".to_string(),
                "sys".to_string(),
                "information_schema".to_string(),
                "".to_string(),
            ]
        );
    }
}
