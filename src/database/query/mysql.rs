//! MySQL 查询实现

use super::{
    ColumnInfo, ForeignKeyInfo, ImportExecutionReport, RoutineInfo, RoutineType, TriggerInfo,
    empty_result, exec_result, is_query_statement, query_result,
};
use crate::core::constants;
use crate::database::{ConnectionConfig, DatabaseType, DbError, POOL_MANAGER, QueryResult};
use mysql_async::prelude::*;
use std::future::Future;
use std::time::Duration;
use tokio::sync::oneshot;

const MYSQL_CANCEL_GRACE_PERIOD: Duration = Duration::from_millis(500);
const MYSQL_CONN_ID_WAIT_PERIOD: Duration = Duration::from_millis(200);
const MYSQL_CANCEL_RACE_WAIT_PERIOD: Duration = Duration::from_millis(50);

struct AbortOnDrop(Option<tokio::task::AbortHandle>);

impl AbortOnDrop {
    fn new(handle: tokio::task::AbortHandle) -> Self {
        Self(Some(handle))
    }

    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}

/// 获取 MySQL 数据库列表
pub async fn get_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
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
pub async fn get_tables(config: &ConnectionConfig, database: &str) -> Result<Vec<String>, DbError> {
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

/// 获取 MySQL 表的主键列名
pub async fn get_primary_key(
    config: &ConnectionConfig,
    table: &str,
) -> Result<Option<String>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    // 使用 SHOW KEYS 查询主键列
    let quoted_table = quote_mysql_identifier(table);
    let sql = format!("SHOW KEYS FROM {} WHERE Key_name = 'PRIMARY'", quoted_table);

    let result: Vec<mysql_async::Row> = conn
        .query(&sql)
        .await
        .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;

    // Column_name 是第 5 列（索引 4）
    if let Some(row) = result.first() {
        let col_name: Option<String> = row.get(4);
        return Ok(col_name);
    }

    Ok(None)
}

fn quote_mysql_identifier(name: &str) -> String {
    name.split('.')
        .map(|part| format!("`{}`", part.replace('`', "``")))
        .collect::<Vec<_>>()
        .join(".")
}

#[cfg(test)]
mod tests {
    use super::{
        DbError, await_cancellable_query, build_kill_query_sql, format_cancel_message,
        query_result, quote_mysql_identifier,
    };
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::Duration;
    use tokio::sync::oneshot;

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
    fn test_build_kill_query_sql() {
        assert_eq!(build_kill_query_sql(123), "KILL QUERY 123");
    }

    #[test]
    fn test_format_cancel_message_without_detail() {
        assert_eq!(format_cancel_message(None), "查询已取消");
    }

    #[test]
    fn test_format_cancel_message_with_detail() {
        let msg = format_cancel_message(Some("权限不足".to_string()));
        assert_eq!(msg, "查询已取消（权限不足）");
    }

    #[tokio::test]
    async fn test_await_cancellable_query_returns_result_when_query_completes_first() {
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let (_conn_id_tx, conn_id_rx) = oneshot::channel::<u32>();

        let query_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok(query_result(
                vec!["id".to_string()],
                vec![vec!["1".to_string()]],
            ))
        });

        let result = await_cancellable_query(
            query_task,
            cancel_rx,
            conn_id_rx,
            Duration::from_millis(5),
            Duration::from_millis(5),
            Duration::from_millis(10),
            |_| async { Ok(()) },
        )
        .await
        .expect("query should complete before cancellation");

        assert_eq!(result.columns, vec!["id".to_string()]);
        assert_eq!(result.rows, vec![vec!["1".to_string()]]);

        drop(cancel_tx);
    }

    #[tokio::test]
    async fn test_await_cancellable_query_sends_kill_signal_on_cancel() {
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let (conn_id_tx, conn_id_rx) = oneshot::channel::<u32>();
        let cancelled_conn_id = Arc::new(AtomicU32::new(0));
        let cancelled_conn_id_clone = Arc::clone(&cancelled_conn_id);

        let query_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            Ok(query_result(Vec::new(), Vec::new()))
        });

        conn_id_tx.send(88).expect("connection id should be sent");
        cancel_tx.send(()).expect("cancel signal should be sent");

        let err = await_cancellable_query(
            query_task,
            cancel_rx,
            conn_id_rx,
            Duration::from_millis(5),
            Duration::from_millis(5),
            Duration::from_millis(5),
            move |conn_id| {
                let cancelled_conn_id = Arc::clone(&cancelled_conn_id_clone);
                async move {
                    cancelled_conn_id.store(conn_id, Ordering::Relaxed);
                    Ok(())
                }
            },
        )
        .await
        .expect_err("query should be cancelled");

        match err {
            DbError::Query(msg) => assert!(msg.starts_with("查询已取消")),
            other => panic!("unexpected error type: {}", other),
        }
        assert_eq!(cancelled_conn_id.load(Ordering::Relaxed), 88);
    }

    #[tokio::test]
    async fn test_await_cancellable_query_skips_kill_when_query_finishes_during_race_window() {
        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        let (conn_id_tx, conn_id_rx) = oneshot::channel::<u32>();
        let kill_called = Arc::new(AtomicU32::new(0));
        let kill_called_clone = Arc::clone(&kill_called);

        let query_task = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(5)).await;
            Ok(query_result(
                vec!["ok".to_string()],
                vec![vec!["1".to_string()]],
            ))
        });

        conn_id_tx.send(99).expect("connection id should be sent");
        cancel_tx.send(()).expect("cancel signal should be sent");

        let result = await_cancellable_query(
            query_task,
            cancel_rx,
            conn_id_rx,
            Duration::from_millis(20),
            Duration::from_millis(5),
            Duration::from_millis(5),
            move |_| {
                let kill_called = Arc::clone(&kill_called_clone);
                async move {
                    kill_called.store(1, Ordering::Relaxed);
                    Ok(())
                }
            },
        )
        .await
        .expect("query should win race window and avoid kill");

        assert_eq!(result.columns, vec!["ok".to_string()]);
        assert_eq!(kill_called.load(Ordering::Relaxed), 0);
    }
}

/// 执行 MySQL 查询
pub async fn execute(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    execute_with_connection_id(config, sql, None).await
}

/// 执行可取消的 MySQL 查询
pub async fn execute_cancellable(
    config: &ConnectionConfig,
    sql: &str,
    cancel_rx: oneshot::Receiver<()>,
) -> Result<QueryResult, DbError> {
    let (conn_id_tx, conn_id_rx) = oneshot::channel::<u32>();
    let config_for_query = config.clone();
    let sql = sql.to_string();
    let query_task = tokio::spawn(async move {
        execute_with_connection_id(&config_for_query, &sql, Some(conn_id_tx)).await
    });
    let mut abort_on_drop = AbortOnDrop::new(query_task.abort_handle());
    let result = await_cancellable_query(
        query_task,
        cancel_rx,
        conn_id_rx,
        MYSQL_CANCEL_RACE_WAIT_PERIOD,
        MYSQL_CONN_ID_WAIT_PERIOD,
        MYSQL_CANCEL_GRACE_PERIOD,
        |connection_id| cancel_query_by_id(config, connection_id),
    )
    .await;
    abort_on_drop.disarm();
    result
}

async fn await_cancellable_query<F, K>(
    mut query_task: tokio::task::JoinHandle<Result<QueryResult, DbError>>,
    mut cancel_rx: oneshot::Receiver<()>,
    conn_id_rx: oneshot::Receiver<u32>,
    cancel_race_wait_period: Duration,
    conn_id_wait_period: Duration,
    cancel_grace_period: Duration,
    send_kill_query: F,
) -> Result<QueryResult, DbError>
where
    F: Fn(u32) -> K,
    K: Future<Output = Result<(), DbError>>,
{
    fn join_query_task(
        res: Result<Result<QueryResult, DbError>, tokio::task::JoinError>,
    ) -> Result<QueryResult, DbError> {
        res.map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
    }

    tokio::select! {
        res = &mut query_task => {
            join_query_task(res)
        }
        _ = &mut cancel_rx => {
            // 取消请求与查询完成可能几乎同时发生。先给查询一个很短的收敛窗口，
            // 避免查询已完成却仍发送 KILL QUERY 的竞态。
            if let Ok(res) = tokio::time::timeout(cancel_race_wait_period, &mut query_task).await {
                return join_query_task(res);
            }

            if query_task.is_finished() {
                return join_query_task(query_task.await);
            }

            let cancel_detail = match tokio::time::timeout(conn_id_wait_period, conn_id_rx).await {
                Ok(Ok(connection_id)) => {
                    if query_task.is_finished() {
                        return join_query_task(query_task.await);
                    }

                    send_kill_query(connection_id)
                        .await
                        .err()
                        .map(|e| e.to_string())
                }
                Ok(Err(_)) => Some("未能获取 MySQL 连接 ID，取消请求可能未发送".to_string()),
                Err(_) => Some("取消请求过早，尚未建立 MySQL 会话".to_string()),
            };

            if tokio::time::timeout(cancel_grace_period, &mut query_task)
                .await
                .is_err()
            {
                query_task.abort();
            }

            Err(DbError::Query(format_cancel_message(cancel_detail)))
        }
    }
}

fn format_cancel_message(cancel_detail: Option<String>) -> String {
    match cancel_detail {
        Some(detail) => format!("查询已取消（{}）", detail),
        None => "查询已取消".to_string(),
    }
}

fn build_kill_query_sql(connection_id: u32) -> String {
    format!("KILL QUERY {}", connection_id)
}

async fn cancel_query_by_id(config: &ConnectionConfig, connection_id: u32) -> Result<(), DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;
    let mut killer_conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取取消连接失败: {}", e)))?;

    let kill_sql = build_kill_query_sql(connection_id);
    killer_conn
        .query_drop(kill_sql)
        .await
        .map_err(|e| DbError::Query(format!("MySQL 取消查询失败: {}", e)))
}

async fn execute_with_connection_id(
    config: &ConnectionConfig,
    sql: &str,
    conn_id_sender: Option<oneshot::Sender<u32>>,
) -> Result<QueryResult, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    if let Some(sender) = conn_id_sender {
        let _ = sender.send(conn.id());
    }

    execute_with_connection(&mut conn, sql).await
}

async fn execute_with_connection(
    conn: &mut mysql_async::Conn,
    sql: &str,
) -> Result<QueryResult, DbError> {
    if is_query_statement(sql, &DatabaseType::MySQL) {
        let mut result = conn
            .query_iter(sql)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let columns: Vec<String> = result
            .columns_ref()
            .iter()
            .map(|c| c.name_str().into_owned())
            .collect();

        if columns.is_empty() {
            return Ok(empty_result());
        }

        let max_rows = constants::database::MAX_RESULT_SET_ROWS;
        let mut data: Vec<Vec<String>> = Vec::new();
        let mut total_rows = 0usize;

        while let Some(row) = result
            .next()
            .await
            .map_err(|e| DbError::Query(e.to_string()))?
        {
            total_rows += 1;
            if data.len() < max_rows {
                data.push(row_to_strings(&row, columns.len()));
            }
        }

        let mut query_result = query_result(columns, data);
        if total_rows > max_rows {
            query_result.truncated = true;
            query_result.original_row_count = Some(total_rows);
        }

        Ok(query_result)
    } else {
        // 使用 query_iter 来获取影响行数
        let result = conn
            .query_iter(sql)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let affected = result.affected_rows();
        // 需要消耗结果
        drop(result);

        Ok(exec_result(affected))
    }
}

/// 批量执行 MySQL 语句（用于导入）
pub async fn execute_batch(
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

/// 将 MySQL 行转换为字符串向量
fn row_to_strings(row: &mysql_async::Row, col_count: usize) -> Vec<String> {
    (0..col_count)
        .map(|i| {
            row.get::<mysql_async::Value, _>(i)
                .map(value_to_string)
                .unwrap_or_else(|| String::from("NULL"))
        })
        .collect()
}

/// 将 MySQL Value 转换为字符串
fn value_to_string(val: mysql_async::Value) -> String {
    use mysql_async::Value;
    match val {
        Value::NULL => String::from("NULL"),
        Value::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
        Value::Int(i) => i.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Date(y, m, d, h, mi, s, us) => {
            if h == 0 && mi == 0 && s == 0 && us == 0 {
                format!("{:04}-{:02}-{:02}", y, m, d)
            } else if us == 0 {
                format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
            } else {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                    y, m, d, h, mi, s, us
                )
            }
        }
        Value::Time(neg, d, h, m, s, us) => {
            let sign = if neg { "-" } else { "" };
            if d > 0 {
                format!("{}{}d {:02}:{:02}:{:02}", sign, d, h, m, s)
            } else if us > 0 {
                format!("{}{:02}:{:02}:{:02}.{:06}", sign, h, m, s, us)
            } else {
                format!("{}{:02}:{:02}:{:02}", sign, h, m, s)
            }
        }
    }
}

/// 获取 MySQL 触发器
pub async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
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

/// 获取 MySQL 外键
pub async fn get_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = r#"
        SELECT 
            TABLE_NAME,
            COLUMN_NAME,
            REFERENCED_TABLE_NAME,
            REFERENCED_COLUMN_NAME
        FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND REFERENCED_TABLE_NAME IS NOT NULL
        ORDER BY TABLE_NAME, COLUMN_NAME
    "#;

    let result: Vec<mysql_async::Row> = conn
        .query(sql)
        .await
        .map_err(|e| DbError::Query(format!("查询外键失败: {}", e)))?;

    let foreign_keys: Vec<ForeignKeyInfo> = result
        .iter()
        .map(|row| ForeignKeyInfo {
            from_table: row.get(0).unwrap_or_default(),
            from_column: row.get(1).unwrap_or_default(),
            to_table: row.get(2).unwrap_or_default(),
            to_column: row.get(3).unwrap_or_default(),
        })
        .collect();

    Ok(foreign_keys)
}

/// 获取 MySQL 表的列信息
pub async fn get_columns(
    config: &ConnectionConfig,
    table: &str,
) -> Result<Vec<ColumnInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = format!(
        r#"
        SELECT 
            c.COLUMN_NAME,
            c.DATA_TYPE,
            CASE WHEN c.COLUMN_KEY = 'PRI' THEN 1 ELSE 0 END AS is_primary_key,
            CASE WHEN c.IS_NULLABLE = 'YES' THEN 1 ELSE 0 END AS is_nullable,
            c.COLUMN_DEFAULT
        FROM INFORMATION_SCHEMA.COLUMNS c
        WHERE c.TABLE_SCHEMA = DATABASE()
          AND c.TABLE_NAME = '{}'
        ORDER BY c.ORDINAL_POSITION
        "#,
        table.replace('\'', "''")
    );

    let result: Vec<mysql_async::Row> = conn
        .query(&sql)
        .await
        .map_err(|e| DbError::Query(format!("查询列信息失败: {}", e)))?;

    let columns: Vec<ColumnInfo> = result
        .iter()
        .map(|row| {
            let is_pk: i32 = row.get(2).unwrap_or(0);
            let is_null: i32 = row.get(3).unwrap_or(0);
            let default_val: Option<String> = row.get(4).unwrap_or(None);
            ColumnInfo {
                name: row.get(0).unwrap_or_default(),
                data_type: row.get(1).unwrap_or_default(),
                is_primary_key: is_pk == 1,
                is_nullable: is_null == 1,
                default_value: default_val,
            }
        })
        .collect();

    Ok(columns)
}

/// 获取 MySQL 存储过程和函数
pub async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
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
