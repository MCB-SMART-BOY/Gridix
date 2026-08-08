//! PostgreSQL 查询实现

use super::{ImportExecutionReport, RoutineInfo, RoutineType, TriggerInfo, is_query_statement};
use crate::core::constants;
use crate::data::{ConnectionConfig, DatabaseType, DbError, POOL_MANAGER};
use futures_util::StreamExt;
use tokio_postgres::SimpleQueryMessage;

async fn current_schema(client: &tokio_postgres::Client) -> Result<String, DbError> {
    let schema: Option<String> = client
        .query_one("SELECT current_schema()", &[])
        .await
        .map_err(|e| DbError::Query(format!("获取当前 schema 失败: {}", e)))?
        .get(0);
    Ok(schema.unwrap_or_else(|| "public".to_string()))
}

fn normalize_identifier(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.len() >= 2 {
        if trimmed.starts_with('"') && trimmed.ends_with('"') {
            return trimmed[1..trimmed.len() - 1].replace("\"\"", "\"");
        }
        if trimmed.starts_with('`') && trimmed.ends_with('`') {
            return trimmed[1..trimmed.len() - 1].replace("``", "`");
        }
    }
    trimmed.to_string()
}

fn parse_table_ref(table: &str) -> (Option<String>, String) {
    if let Some((schema, table_name)) = table.split_once('.') {
        (
            Some(normalize_identifier(schema)),
            normalize_identifier(table_name),
        )
    } else {
        (None, normalize_identifier(table))
    }
}

fn quote_postgres_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

fn postgres_maintenance_databases(config: &ConnectionConfig, target_database: &str) -> Vec<String> {
    let mut databases = Vec::new();
    for candidate in [
        config.database.as_str(),
        "postgres",
        "template1",
        "defaultdb",
    ] {
        let trimmed = candidate.trim();
        if trimmed.is_empty()
            || trimmed == target_database
            || databases.iter().any(|db| db == trimmed)
        {
            continue;
        }
        databases.push(trimmed.to_string());
    }
    databases
}

/// 获取 PostgreSQL 数据库列表
pub(crate) async fn get_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    let rows = client
        .query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            &[],
        )
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get(0)).collect())
}

/// 获取 PostgreSQL 指定数据库的表列表
pub(crate) async fn get_tables(
    config: &ConnectionConfig,
    database: &str,
) -> Result<Vec<String>, DbError> {
    // 创建一个临时配置，连接到指定数据库
    let mut db_config = config.clone();
    db_config.database = database.to_string();

    let client = POOL_MANAGER.get_pg_client(&db_config).await?;
    let schema = current_schema(&client).await?;

    let rows = client
        .query(
            "SELECT tablename FROM pg_tables WHERE schemaname = $1 ORDER BY tablename",
            &[&schema],
        )
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get(0)).collect())
}

/// 删除 PostgreSQL 数据库。
pub(crate) async fn drop_database(
    config: &ConnectionConfig,
    database: &str,
) -> Result<(), DbError> {
    let quoted_database = quote_postgres_identifier(database);
    let maintenance_dbs = postgres_maintenance_databases(config, database);
    let mut last_error = None;

    for maintenance_db in maintenance_dbs {
        let mut maintenance_config = config.clone();
        maintenance_config.database = maintenance_db.clone();

        match POOL_MANAGER.get_pg_client(&maintenance_config).await {
            Ok(client) => {
                let sql = format!("DROP DATABASE {}", quoted_database);
                return client
                    .execute(sql.as_str(), &[])
                    .await
                    .map(|_| ())
                    .map_err(|e| DbError::Query(format!("删除数据库失败: {}", e)));
            }
            Err(error) => {
                last_error = Some(format!("连接维护数据库 {} 失败: {}", maintenance_db, error));
            }
        }
    }

    Err(DbError::Connection(last_error.unwrap_or_else(|| {
        "未找到可用的 PostgreSQL 维护数据库来执行 DROP DATABASE".to_string()
    })))
}

/// 批量执行 PostgreSQL 语句（用于导入）
pub(crate) async fn execute_batch(
    config: &ConnectionConfig,
    statements: &[String],
    use_transaction: bool,
    stop_on_error: bool,
) -> Result<ImportExecutionReport, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    let mut report = ImportExecutionReport::new(statements.len());
    if statements.is_empty() {
        return Ok(report);
    }

    if use_transaction {
        client
            .batch_execute("BEGIN")
            .await
            .map_err(|e| DbError::Query(format!("开启事务失败: {}", e)))?;
    }

    for (index, statement) in statements.iter().enumerate() {
        if let Err(e) = client.batch_execute(statement).await {
            let err_msg = format!("第 {} 条语句执行失败: {}", index + 1, e);

            if use_transaction {
                if let Err(rollback_err) = client.batch_execute("ROLLBACK").await {
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
        } else {
            report.succeeded += 1;
        }
    }

    if use_transaction {
        client
            .batch_execute("COMMIT")
            .await
            .map_err(|e| DbError::Query(format!("提交事务失败: {}", e)))?;
    }

    Ok(report)
}

/// 获取 PostgreSQL 触发器
pub(crate) async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    let schema = current_schema(&client).await?;

    let sql = r#"
        SELECT 
            t.tgname AS trigger_name,
            c.relname AS table_name,
            CASE 
                WHEN t.tgtype & 2 = 2 THEN 'BEFORE'
                WHEN t.tgtype & 64 = 64 THEN 'INSTEAD OF'
                ELSE 'AFTER'
            END AS timing,
            CASE 
                WHEN t.tgtype & 4 = 4 THEN 'INSERT'
                WHEN t.tgtype & 8 = 8 THEN 'DELETE'
                WHEN t.tgtype & 16 = 16 THEN 'UPDATE'
                ELSE 'UNKNOWN'
            END AS event,
            pg_get_triggerdef(t.oid) AS definition
        FROM pg_trigger t
        JOIN pg_class c ON t.tgrelid = c.oid
        JOIN pg_namespace n ON c.relnamespace = n.oid
        WHERE NOT t.tgisinternal
          AND n.nspname = $1
        ORDER BY t.tgname
    "#;

    let rows = client
        .query(sql, &[&schema])
        .await
        .map_err(|e| DbError::Query(format!("查询触发器失败: {}", e)))?;

    let triggers: Vec<TriggerInfo> = rows
        .iter()
        .map(|row| TriggerInfo {
            name: row.get(0),
            table_name: row.get(1),
            timing: row.get(2),
            event: row.get(3),
            definition: row.get(4),
        })
        .collect();

    Ok(triggers)
}

/// 获取 PostgreSQL 存储过程和函数
pub(crate) async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    let schema = current_schema(&client).await?;

    // 查询用户定义的函数和存储过程
    // prokind: 'f' = function, 'p' = procedure, 'a' = aggregate, 'w' = window
    let sql = r#"
        SELECT 
            p.proname AS name,
            CASE p.prokind 
                WHEN 'p' THEN 'PROCEDURE'
                ELSE 'FUNCTION'
            END AS routine_type,
            pg_get_function_arguments(p.oid) AS parameters,
            CASE WHEN p.prokind != 'p' THEN
                pg_catalog.format_type(p.prorettype, NULL)
            ELSE NULL END AS return_type,
            pg_get_functiondef(p.oid) AS definition
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = $1
          AND p.prokind IN ('f', 'p')
        ORDER BY 
            CASE p.prokind WHEN 'p' THEN 0 ELSE 1 END,
            p.proname
    "#;

    let rows = client
        .query(sql, &[&schema])
        .await
        .map_err(|e| DbError::Query(format!("查询存储过程失败: {}", e)))?;

    let routines: Vec<RoutineInfo> = rows
        .iter()
        .map(|row| {
            let name: String = row.get(0);
            let type_str: String = row.get(1);
            let parameters: String = row.get(2);
            let return_type: Option<String> = row.get(3);
            let definition: String = row.get(4);

            let routine_type = if type_str == "PROCEDURE" {
                RoutineType::Procedure
            } else {
                RoutineType::Function
            };

            RoutineInfo {
                name,
                routine_type,
                parameters,
                return_type,
                definition,
            }
        })
        .collect();

    Ok(routines)
}

// ── SchemaCatalog 加载 ──

use super::infer_type_family;
use crate::domain::ids::SchemaRevision;
use crate::domain::metadata::{
    ColumnMetadata, ForeignKeyMetadata, KeyMetadata, SchemaCatalog, TableMetadata,
};

/// 从 information_schema 加载 PostgreSQL schema 元数据
pub(crate) async fn load_catalog(
    config: &ConnectionConfig,
    revision: SchemaRevision,
) -> Result<SchemaCatalog, DbError> {
    let client = POOL_MANAGER
        .get_pg_client(config)
        .await
        .map_err(|e| DbError::Connection(format!("PG 连接池获取失败: {}", e)))?;

    let schema = current_schema(&client).await?;

    // 1. 获取表列表
    let table_rows = client
        .query(
            "SELECT TABLE_NAME FROM information_schema.TABLES \
             WHERE TABLE_SCHEMA = $1 AND TABLE_TYPE = 'BASE TABLE' \
             ORDER BY TABLE_NAME",
            &[&schema],
        )
        .await
        .map_err(|e| DbError::Query(format!("查询表列表失败: {}", e)))?;

    let table_names: Vec<String> = table_rows
        .iter()
        .map(|row| row.get::<_, String>(0))
        .collect();

    let mut tables = Vec::with_capacity(table_names.len());

    for table_name in &table_names {
        // 2. 列信息
        let col_rows = client
            .query(
                "SELECT COLUMN_NAME, ORDINAL_POSITION, DATA_TYPE, IS_NULLABLE, COLUMN_DEFAULT \
                 FROM information_schema.COLUMNS \
                 WHERE TABLE_SCHEMA = $1 AND TABLE_NAME = $2 \
                 ORDER BY ORDINAL_POSITION",
                &[&schema, &table_name.as_str()],
            )
            .await
            .map_err(|e| DbError::Query(format!("查询列信息失败: {}", e)))?;

        // 3. 主键 — 聚合同一 constraint 的多列（复合主键）
        let pk_rows = client
            .query(
                "SELECT kcu.COLUMN_NAME, kcu.CONSTRAINT_NAME \
                 FROM information_schema.TABLE_CONSTRAINTS tc \
                 JOIN information_schema.KEY_COLUMN_USAGE kcu \
                   ON tc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME \
                  AND tc.TABLE_SCHEMA = kcu.TABLE_SCHEMA \
                  AND tc.TABLE_NAME = kcu.TABLE_NAME \
                 WHERE tc.CONSTRAINT_TYPE = 'PRIMARY KEY' \
                   AND tc.TABLE_SCHEMA = $1 \
                   AND tc.TABLE_NAME = $2 \
                 ORDER BY kcu.ORDINAL_POSITION",
                &[&schema, &table_name.as_str()],
            )
            .await
            .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;

        let pk_columns: Vec<String> = pk_rows.iter().map(|r| r.get::<_, String>(0)).collect();

        let primary_key = if pk_columns.is_empty() {
            None
        } else {
            // PG 约束名用于展示，columns 是实际主键列
            let pk_name: Option<String> = pk_rows.first().map(|r| r.get(1));
            Some(KeyMetadata {
                name: pk_name,
                columns: pk_columns.clone(),
            })
        };

        // 4. 外键 — 聚合同一 constraint 的多列（复合外键）
        let fk_rows = client
            .query(
                "SELECT kcu.COLUMN_NAME, \
                        ccu.TABLE_NAME AS REFERENCED_TABLE_NAME, \
                        ccu.COLUMN_NAME AS REFERENCED_COLUMN_NAME, \
                        kcu.CONSTRAINT_NAME \
                 FROM information_schema.KEY_COLUMN_USAGE kcu \
                 JOIN information_schema.REFERENTIAL_CONSTRAINTS rc \
                   ON kcu.CONSTRAINT_NAME = rc.CONSTRAINT_NAME \
                  AND kcu.TABLE_SCHEMA = rc.CONSTRAINT_SCHEMA \
                 JOIN information_schema.CONSTRAINT_COLUMN_USAGE ccu \
                   ON rc.UNIQUE_CONSTRAINT_NAME = ccu.CONSTRAINT_NAME \
                  AND rc.UNIQUE_CONSTRAINT_SCHEMA = ccu.TABLE_SCHEMA \
                 WHERE kcu.TABLE_SCHEMA = $1 \
                   AND kcu.TABLE_NAME = $2 \
                 ORDER BY kcu.CONSTRAINT_NAME, kcu.ORDINAL_POSITION",
                &[&schema, &table_name.as_str()],
            )
            .await
            .map_err(|e| DbError::Query(format!("查询外键失败: {}", e)))?;

        // 按约束名聚合外键列
        type FkAggregate = (Option<String>, Vec<String>, String, Vec<String>);
        let mut fk_map: std::collections::BTreeMap<String, FkAggregate> =
            std::collections::BTreeMap::new();
        for row in &fk_rows {
            let constraint: String = row.get(3);
            let col: String = row.get(0);
            let ref_table: String = row.get(1);
            let ref_col: String = row.get(2);
            let entry = fk_map
                .entry(constraint.clone())
                .or_insert_with(|| (Some(constraint.clone()), Vec::new(), ref_table, Vec::new()));
            entry.1.push(col);
            entry.3.push(ref_col);
        }
        let foreign_keys: Vec<ForeignKeyMetadata> = fk_map
            .into_values()
            .map(
                |(name, from_cols, ref_table, ref_cols)| ForeignKeyMetadata {
                    name,
                    from_columns: from_cols,
                    ref_table,
                    ref_columns: ref_cols,
                },
            )
            .collect();

        let is_pk = |col_name: &str| pk_columns.iter().any(|pk| pk == col_name);

        let columns: Vec<ColumnMetadata> = col_rows
            .iter()
            .map(|r| -> ColumnMetadata {
                let col_name: String = r.get(0);
                let pos: i32 = r.get(1);
                let data_type: String = r.get(2);
                let is_nullable_str: String = r.get(3);
                let default: Option<String> = r.get(4);

                ColumnMetadata {
                    name: col_name.clone(),
                    position: pos as usize,
                    type_info: DbTypeInfo {
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
            schema: Some(schema.clone()),
            columns,
            primary_key,
            unique_keys: Vec::new(),
            foreign_keys,
        });
    }

    Ok(SchemaCatalog { revision, tables })
}

// ── Typed ResultSet 执行 ──

use crate::domain::execution::ExecutionOutcome;
use crate::domain::result::{ResultColumn, ResultCompleteness, ResultSet};
use crate::domain::value::{DbTypeFamily, DbTypeInfo, DbValue};

/// 执行 SQL 并返回类型化 ResultSet（PostgreSQL 原生路径）
pub(crate) async fn execute_typed(
    config: &ConnectionConfig,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    execute_typed_with_client(client.as_ref(), sql).await
}

/// 使用 PostgreSQL CancelRequest 协作取消正在执行的查询。
pub(crate) async fn execute_typed_cancellable(
    config: &ConnectionConfig,
    sql: &str,
    cancellation: &tokio_util::sync::CancellationToken,
) -> Result<ExecutionOutcome, DbError> {
    if cancellation.is_cancelled() {
        return Err(DbError::Cancelled);
    }

    let client = POOL_MANAGER.get_pg_client(config).await?;
    let cancel_token = client.cancel_token();
    let query = execute_typed_with_client(client.as_ref(), sql);
    tokio::pin!(query);

    tokio::select! {
        biased;
        result = &mut query => result,
        _ = cancellation.cancelled() => {
            cancel_pg_query(config, cancel_token).await?;
            match query.await {
                Err(_) => Err(DbError::Cancelled),
                Ok(outcome) => Ok(outcome),
            }
        }
    }
}

async fn cancel_pg_query(
    config: &ConnectionConfig,
    token: tokio_postgres::CancelToken,
) -> Result<(), DbError> {
    use crate::data::pool::build_pg_tls_connector;
    use crate::types::PostgresSslMode;

    match config.postgres_ssl_mode {
        PostgresSslMode::Disable => token
            .cancel_query(tokio_postgres::NoTls)
            .await
            .map_err(|e| DbError::Connection(format!("发送 PostgreSQL 取消请求失败: {}", e))),
        PostgresSslMode::Prefer => token
            .cancel_query(build_pg_tls_connector(config, true)?)
            .await
            .map_err(|e| DbError::Connection(format!("发送 PostgreSQL 取消请求失败: {}", e))),
        PostgresSslMode::Require | PostgresSslMode::VerifyCa | PostgresSslMode::VerifyFull => token
            .cancel_query(build_pg_tls_connector(config, false)?)
            .await
            .map_err(|e| DbError::Connection(format!("发送 PostgreSQL 取消请求失败: {}", e))),
    }
}

async fn execute_typed_with_client(
    client: &tokio_postgres::Client,
    sql: &str,
) -> Result<ExecutionOutcome, DbError> {
    if !is_query_statement(sql, &DatabaseType::PostgreSQL) {
        let affected = client
            .execute(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        return Ok(ExecutionOutcome::affected_rows(affected));
    }

    let stream = client
        .simple_query_raw(sql)
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;
    futures_util::pin_mut!(stream);

    let mut col_names: Vec<String> = Vec::new();
    let mut cells: Vec<DbValue> = Vec::new();
    let mut total_rows = 0usize;
    let max_rows = constants::database::MAX_RESULT_SET_ROWS;

    while let Some(message) = stream.next().await {
        match message.map_err(|e| DbError::Query(e.to_string()))? {
            SimpleQueryMessage::RowDescription(desc) => {
                if col_names.is_empty() {
                    col_names = desc.iter().map(|c| c.name().to_owned()).collect();
                }
            }
            SimpleQueryMessage::Row(row) => {
                if col_names.is_empty() {
                    col_names = row.columns().iter().map(|c| c.name().to_owned()).collect();
                }
                total_rows += 1;
                if cells.len() / col_names.len().max(1) < max_rows {
                    for i in 0..row.len() {
                        match row.get(i) {
                            Some(value) => cells.push(DbValue::Text(value.to_string())),
                            None => cells.push(DbValue::Null),
                        }
                    }
                }
            }
            SimpleQueryMessage::CommandComplete(_) => {}
            _ => {}
        }
    }

    if col_names.is_empty() && total_rows == 0 {
        return Ok(ExecutionOutcome::single_result(ResultSet::empty()));
    }

    let col_count = col_names.len();
    let row_count = cells.len() / col_count.max(1);
    let completeness = if total_rows > max_rows {
        ResultCompleteness::Truncated {
            displayed: row_count,
        }
    } else {
        ResultCompleteness::Complete
    };

    let typed_columns: std::sync::Arc<[ResultColumn]> = col_names
        .iter()
        .map(|name| ResultColumn {
            name: name.clone(),
            type_info: DbTypeInfo {
                family: DbTypeFamily::Text,
                native_name: "text".to_string(),
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
use crate::domain::mutation::{
    ExpectedRows, InputValue, Mutation, MutationBatch, MutationBatchResult, RowIdentity,
};
use tokio_postgres::types::ToSql;

/// DbValue → tokio_postgres parameter value.
fn pg_param(
    value: &crate::domain::value::DbValue,
) -> Result<Box<dyn ToSql + Sync + Send>, DbError> {
    Ok(match value {
        crate::domain::value::DbValue::Null => Box::new(None::<String>),
        crate::domain::value::DbValue::Bool(b) => Box::new(*b),
        crate::domain::value::DbValue::Int(i) => Box::new(*i),
        crate::domain::value::DbValue::UInt(u) => {
            Box::new(i64::try_from(*u).map_err(|_| DbError::Unsupported {
                capability: "u64 > i64::MAX for PG",
            })?)
        }
        crate::domain::value::DbValue::Float(f) => Box::new(*f),
        crate::domain::value::DbValue::Text(s) => Box::new(s.clone()),
        crate::domain::value::DbValue::Bytes(b) => Box::new(b.to_vec()),
        crate::domain::value::DbValue::Decimal(d) => Box::new(d.clone()),
        _ => {
            return Err(DbError::Unsupported {
                capability: "PG param",
            });
        }
    })
}

fn pg_value(v: &InputValue) -> Result<Box<dyn ToSql + Sync + Send>, DbError> {
    match v {
        InputValue::Value(dv) => pg_param(dv),
        InputValue::Null => Ok(Box::new(None::<String>)),
        InputValue::Default => Err(DbError::Unsupported {
            capability: "PG DEFAULT",
        }),
        InputValue::Unspecified => unreachable!(),
    }
}

fn extract_pk(
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

pub(crate) async fn apply_mutations(
    config: &ConnectionConfig,
    batch: &MutationBatch,
) -> Result<MutationBatchResult, DbError> {
    use crate::domain::identifier::IdentifierDialect;
    let client = POOL_MANAGER.get_pg_client(config).await?;
    client
        .batch_execute("BEGIN")
        .await
        .map_err(|e| DbError::Query(format!("PG BEGIN: {e}")))?;

    let mut affected = Vec::with_capacity(batch.mutations.len());
    for mutation in &batch.mutations {
        let n = match mutation {
            Mutation::Insert {
                table,
                columns,
                values,
            } => {
                let included: Vec<(&str, &InputValue)> = columns
                    .iter()
                    .zip(values.iter())
                    .filter(|(_, v)| !matches!(v, InputValue::Unspecified | InputValue::Default))
                    .map(|(c, v)| (c.name.as_str(), v))
                    .collect();
                if included.is_empty() {
                    let sql = format!(
                        "INSERT INTO {} DEFAULT VALUES",
                        IdentifierDialect::PostgreSql.quote(&table.name)
                    );
                    client
                        .execute(&sql, &[])
                        .await
                        .map_err(|e| DbError::Query(format!("PG INSERT DEFAULT: {e}")))?
                } else {
                    let cols: Vec<String> = included
                        .iter()
                        .map(|(n, _)| IdentifierDialect::PostgreSql.quote(n))
                        .collect();
                    let ph: Vec<String> = (1..=included.len()).map(|i| format!("${i}")).collect();
                    let sql = format!(
                        "INSERT INTO {} ({}) VALUES ({})",
                        IdentifierDialect::PostgreSql.quote(&table.name),
                        cols.join(", "),
                        ph.join(", ")
                    );
                    // scope to drop params before .await
                    let params: Vec<Box<dyn ToSql + Sync + Send>> = included
                        .iter()
                        .map(|(_, v)| pg_value(v))
                        .collect::<Result<_, _>>()?;
                    let refs: Vec<&(dyn ToSql + Sync)> = params
                        .iter()
                        .map(|p| p.as_ref() as &(dyn ToSql + Sync))
                        .collect();
                    client
                        .execute(&sql, &refs)
                        .await
                        .map_err(|e| DbError::Query(format!("PG INSERT: {e}")))?
                }
            }
            Mutation::Update {
                table,
                identity,
                changes,
                expected_rows,
            } => {
                let pk = extract_pk(identity)?;
                let set_sql: Vec<String> = changes
                    .iter()
                    .enumerate()
                    .map(|(i, (c, _))| {
                        format!(
                            "{} = ${}",
                            IdentifierDialect::PostgreSql.quote(&c.name),
                            i + 1
                        )
                    })
                    .collect();
                let where_sql: Vec<String> = pk
                    .iter()
                    .enumerate()
                    .map(|(i, (c, _))| {
                        format!(
                            "{} = ${}",
                            IdentifierDialect::PostgreSql.quote(&c.name),
                            changes.len() + i + 1
                        )
                    })
                    .collect();
                let sql = format!(
                    "UPDATE {} SET {} WHERE {}",
                    IdentifierDialect::PostgreSql.quote(&table.name),
                    set_sql.join(", "),
                    where_sql.join(" AND ")
                );
                let mut params: Vec<Box<dyn ToSql + Sync + Send>> = changes
                    .iter()
                    .map(|(_, v)| pg_value(v))
                    .collect::<Result<_, _>>()?;
                params.extend(
                    pk.iter()
                        .map(|(_, v)| pg_param(v))
                        .collect::<Result<Vec<_>, _>>()?,
                );
                let refs: Vec<&(dyn ToSql + Sync)> = params
                    .iter()
                    .map(|p| p.as_ref() as &(dyn ToSql + Sync))
                    .collect();
                let n = client
                    .execute(&sql, &refs)
                    .await
                    .map_err(|e| DbError::Query(format!("PG UPDATE: {e}")))?
                    as u64;
                if let ExpectedRows::Exactly(e) = expected_rows
                    && n != *e
                {
                    return Err(DbError::Query(format!(
                        "PG UPDATE: expected {e} rows, affected {n}"
                    )));
                }
                n
            }
            Mutation::Delete {
                table,
                identity,
                expected_rows,
            } => {
                let pk = extract_pk(identity)?;
                let where_sql: Vec<String> = pk
                    .iter()
                    .enumerate()
                    .map(|(i, (c, _))| {
                        format!(
                            "{} = ${}",
                            IdentifierDialect::PostgreSql.quote(&c.name),
                            i + 1
                        )
                    })
                    .collect();
                let sql = format!(
                    "DELETE FROM {} WHERE {}",
                    IdentifierDialect::PostgreSql.quote(&table.name),
                    where_sql.join(" AND ")
                );
                let params: Vec<Box<dyn ToSql + Sync + Send>> = pk
                    .iter()
                    .map(|(_, v)| pg_param(v))
                    .collect::<Result<_, _>>()?;
                let refs: Vec<&(dyn ToSql + Sync)> = params
                    .iter()
                    .map(|p| p.as_ref() as &(dyn ToSql + Sync))
                    .collect();
                let n = client
                    .execute(&sql, &refs)
                    .await
                    .map_err(|e| DbError::Query(format!("PG DELETE: {e}")))?
                    as u64;
                if let ExpectedRows::Exactly(e) = expected_rows
                    && n != *e
                {
                    return Err(DbError::Query(format!(
                        "PG DELETE: expected {e} rows, affected {n}"
                    )));
                }
                n
            }
        };
        affected.push(n);
    }
    client
        .batch_execute("COMMIT")
        .await
        .map_err(|e| DbError::Query(format!("PG COMMIT: {e}")))?;
    Ok(MutationBatchResult {
        affected,
        all_success: true,
    })
}
