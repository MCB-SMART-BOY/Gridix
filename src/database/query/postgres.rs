//! PostgreSQL 查询实现

use super::{
    ColumnInfo, ForeignKeyInfo, ImportExecutionReport, RoutineInfo, RoutineType, TriggerInfo,
    empty_result, exec_result, is_query_statement, query_result_with_null_flags,
};
use crate::core::constants;
use crate::database::{
    ConnectionConfig, DatabaseType, DbError, POOL_MANAGER, PostgresSslMode, QueryResult,
};
use futures_util::StreamExt;
use tokio::sync::oneshot;
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
pub async fn get_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
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
pub async fn get_tables(config: &ConnectionConfig, database: &str) -> Result<Vec<String>, DbError> {
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
pub async fn drop_database(config: &ConnectionConfig, database: &str) -> Result<(), DbError> {
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

/// 获取 PostgreSQL 表的主键列名
pub async fn get_primary_key(
    config: &ConnectionConfig,
    table: &str,
) -> Result<Option<String>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    let (schema_opt, table_name) = parse_table_ref(table);
    let schema = if let Some(schema) = schema_opt {
        schema
    } else {
        current_schema(&client).await?
    };

    let rows = client
        .query(
            "SELECT kcu.column_name
             FROM information_schema.table_constraints tc
             JOIN information_schema.key_column_usage kcu
               ON tc.constraint_name = kcu.constraint_name
              AND tc.table_schema = kcu.table_schema
             WHERE tc.constraint_type = 'PRIMARY KEY'
               AND tc.table_schema = $1
               AND tc.table_name = $2
             ORDER BY kcu.ordinal_position
             LIMIT 1",
            &[&schema, &table_name],
        )
        .await
        .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;

    Ok(rows.first().map(|r| r.get(0)))
}

/// 执行 PostgreSQL 查询
pub async fn execute(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    execute_with_client(client.as_ref(), sql).await
}

/// 执行可取消的 PostgreSQL 查询
pub async fn execute_cancellable(
    config: &ConnectionConfig,
    sql: &str,
    mut cancel_rx: oneshot::Receiver<()>,
) -> Result<QueryResult, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    let cancel_token = client.cancel_token();
    let exec_fut = execute_with_client(client.as_ref(), sql);
    tokio::pin!(exec_fut);

    tokio::select! {
        res = &mut exec_fut => res,
        _ = &mut cancel_rx => {
            if let Err(e) = try_cancel_query(config, cancel_token).await {
                tracing::warn!(error = %e, "PostgreSQL 查询取消请求发送失败");
            }
            Err(DbError::Query("查询已取消".to_string()))
        }
    }
}

async fn execute_with_client(
    client: &tokio_postgres::Client,
    sql: &str,
) -> Result<QueryResult, DbError> {
    if is_query_statement(sql, &DatabaseType::PostgreSQL) {
        let stream = client
            .simple_query_raw(sql)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        futures_util::pin_mut!(stream);

        let mut columns: Vec<String> = Vec::new();
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut null_flags: Vec<Vec<bool>> = Vec::new();
        let mut total_rows = 0usize;
        let max_rows = constants::database::MAX_RESULT_SET_ROWS;

        while let Some(message) = stream.next().await {
            match message.map_err(|e| DbError::Query(e.to_string()))? {
                SimpleQueryMessage::RowDescription(desc) => {
                    if columns.is_empty() {
                        columns = desc.iter().map(|c| c.name().to_owned()).collect();
                    }
                }
                SimpleQueryMessage::Row(row) => {
                    if columns.is_empty() {
                        columns = row.columns().iter().map(|c| c.name().to_owned()).collect();
                    }

                    total_rows += 1;
                    if rows.len() < max_rows {
                        let mut row_values = Vec::with_capacity(row.len());
                        let mut row_nulls = Vec::with_capacity(row.len());
                        for i in 0..row.len() {
                            match row.get(i) {
                                Some(value) => {
                                    row_values.push(value.to_string());
                                    row_nulls.push(false);
                                }
                                None => {
                                    row_values.push(String::new());
                                    row_nulls.push(true);
                                }
                            }
                        }
                        rows.push(row_values);
                        null_flags.push(row_nulls);
                    }
                }
                SimpleQueryMessage::CommandComplete(_) => {}
                _ => {}
            }
        }

        if columns.is_empty() && total_rows == 0 {
            return Ok(empty_result());
        }

        let mut query_result = query_result_with_null_flags(columns, rows, null_flags);
        if total_rows > max_rows {
            query_result.truncated = true;
            query_result.original_row_count = Some(total_rows);
        }

        Ok(query_result)
    } else {
        let affected = client
            .execute(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(exec_result(affected))
    }
}

async fn try_cancel_query(
    config: &ConnectionConfig,
    cancel_token: tokio_postgres::CancelToken,
) -> Result<(), DbError> {
    match config.postgres_ssl_mode {
        PostgresSslMode::Disable => cancel_token
            .cancel_query(tokio_postgres::NoTls)
            .await
            .map_err(|e| DbError::Query(format!("发送取消请求失败: {}", e))),
        PostgresSslMode::Prefer => {
            // Prefer 可能回退到非 TLS，因此先尝试 TLS 再尝试 NoTls
            if let Ok(tls) = build_cancel_tls_connector(config, true)
                && cancel_token.cancel_query(tls).await.is_ok()
            {
                return Ok(());
            }
            cancel_token
                .cancel_query(tokio_postgres::NoTls)
                .await
                .map_err(|e| DbError::Query(format!("发送取消请求失败: {}", e)))
        }
        PostgresSslMode::Require => {
            let tls = build_cancel_tls_connector(config, true)?;
            cancel_token
                .cancel_query(tls)
                .await
                .map_err(|e| DbError::Query(format!("发送取消请求失败: {}", e)))
        }
        PostgresSslMode::VerifyCa | PostgresSslMode::VerifyFull => {
            let tls = build_cancel_tls_connector(config, false)?;
            cancel_token
                .cancel_query(tls)
                .await
                .map_err(|e| DbError::Query(format!("发送取消请求失败: {}", e)))
        }
    }
}

fn build_cancel_tls_connector(
    config: &ConnectionConfig,
    accept_invalid_certs: bool,
) -> Result<postgres_native_tls::MakeTlsConnector, DbError> {
    use native_tls::TlsConnector as NativeTlsConnector;
    use std::path::Path;

    let mut builder = NativeTlsConnector::builder();

    if accept_invalid_certs {
        builder.danger_accept_invalid_certs(true);
        builder.danger_accept_invalid_hostnames(true);
    }

    if !config.ssl_ca_cert.is_empty() {
        let ca_path = Path::new(&config.ssl_ca_cert);
        if !ca_path.exists() {
            return Err(DbError::Query(format!(
                "取消查询失败，CA 证书不存在: {}",
                config.ssl_ca_cert
            )));
        }

        let ca_data = std::fs::read(&config.ssl_ca_cert)
            .map_err(|e| DbError::Query(format!("取消查询读取 CA 证书失败: {}", e)))?;
        let cert = native_tls::Certificate::from_pem(&ca_data)
            .map_err(|e| DbError::Query(format!("取消查询解析 CA 证书失败: {}", e)))?;
        builder.add_root_certificate(cert);
    }

    if config.postgres_ssl_mode != PostgresSslMode::VerifyFull {
        builder.danger_accept_invalid_hostnames(true);
    }

    let connector = builder
        .build()
        .map_err(|e| DbError::Query(format!("取消查询 TLS 构建失败: {}", e)))?;
    Ok(postgres_native_tls::MakeTlsConnector::new(connector))
}

/// 批量执行 PostgreSQL 语句（用于导入）
pub async fn execute_batch(
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
pub async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
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

/// 获取 PostgreSQL 外键
pub async fn get_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    let schema = current_schema(&client).await?;

    let sql = r#"
        SELECT 
            kcu.table_name AS from_table,
            kcu.column_name AS from_column,
            ccu.table_name AS to_table,
            ccu.column_name AS to_column
        FROM information_schema.key_column_usage kcu
        JOIN information_schema.referential_constraints rc 
            ON kcu.constraint_name = rc.constraint_name
            AND kcu.table_schema = rc.constraint_schema
        JOIN information_schema.constraint_column_usage ccu 
            ON rc.unique_constraint_name = ccu.constraint_name
            AND rc.unique_constraint_schema = ccu.table_schema
        WHERE kcu.table_schema = $1
        ORDER BY kcu.table_name, kcu.column_name
    "#;

    let rows = client
        .query(sql, &[&schema])
        .await
        .map_err(|e| DbError::Query(format!("查询外键失败: {}", e)))?;

    let foreign_keys: Vec<ForeignKeyInfo> = rows
        .iter()
        .map(|row| ForeignKeyInfo {
            from_table: row.get(0),
            from_column: row.get(1),
            to_table: row.get(2),
            to_column: row.get(3),
        })
        .collect();

    Ok(foreign_keys)
}

/// 获取 PostgreSQL 表的列信息
pub async fn get_columns(
    config: &ConnectionConfig,
    table: &str,
) -> Result<Vec<ColumnInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    let (schema_opt, table_name) = parse_table_ref(table);
    let schema = if let Some(schema) = schema_opt {
        schema
    } else {
        current_schema(&client).await?
    };

    let sql = r#"
        SELECT 
            c.column_name,
            c.data_type,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END AS is_primary_key,
            c.is_nullable = 'YES' AS is_nullable,
            c.column_default
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu 
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY'
              AND tc.table_name = $2
              AND tc.table_schema = $1
        ) pk ON c.column_name = pk.column_name
        WHERE c.table_name = $2
          AND c.table_schema = $1
        ORDER BY c.ordinal_position
    "#;

    let rows = client
        .query(sql, &[&schema, &table_name])
        .await
        .map_err(|e| DbError::Query(format!("查询列信息失败: {}", e)))?;

    let columns: Vec<ColumnInfo> = rows
        .iter()
        .map(|row| ColumnInfo {
            name: row.get(0),
            data_type: row.get(1),
            is_primary_key: row.get(2),
            is_nullable: row.get(3),
            default_value: row.get(4),
        })
        .collect();

    Ok(columns)
}

/// 获取 PostgreSQL 存储过程和函数
pub async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
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
