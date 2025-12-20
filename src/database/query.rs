//! 数据库查询执行模块
//!
//! 提供对 SQLite、PostgreSQL、MySQL 的统一查询接口。
//! PostgreSQL 和 MySQL 使用连接池优化性能。

#![allow(dead_code)] // 公开 API，部分功能预留

use super::ssh_tunnel::{SshTunnel, SSH_TUNNEL_MANAGER};
use super::*;
use crate::core::constants;
use mysql_async::prelude::*;
use rusqlite::{types::ValueRef, Connection as SqliteConn};
use std::sync::Arc;
use std::time::Duration;
use tokio::task;

// ============================================================================
// 公共入口函数
// ============================================================================

/// 连接结果类型
pub enum ConnectResult {
    /// SQLite: 直接返回表列表
    Tables(Vec<String>),
    /// MySQL/PostgreSQL: 返回数据库列表
    Databases(Vec<String>),
}

/// 连接数据库
///
/// - SQLite: 返回表列表
/// - MySQL/PostgreSQL: 返回数据库列表
///
/// 如果配置了 SSH 隧道，会自动建立隧道连接
pub async fn connect_database(config: &ConnectionConfig) -> Result<ConnectResult, DbError> {
    // 如果启用了 SSH 隧道，先建立隧道并修改连接配置
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;

    match effective_config.db_type {
        DatabaseType::SQLite => {
            let tables = task::spawn_blocking(move || connect_sqlite(&effective_config))
                .await
                .map_err(|e| DbError::Connection(format!("任务执行失败: {}", e)))??;
            Ok(ConnectResult::Tables(tables))
        }
        DatabaseType::PostgreSQL => {
            let databases = get_postgres_databases(&effective_config).await?;
            Ok(ConnectResult::Databases(databases))
        }
        DatabaseType::MySQL => {
            let databases = get_mysql_databases(&effective_config).await?;
            Ok(ConnectResult::Databases(databases))
        }
    }
}

/// 获取指定数据库的表列表
pub async fn get_tables_for_database(
    config: &ConnectionConfig,
    database: &str,
) -> Result<Vec<String>, DbError> {
    // 如果启用了 SSH 隧道，先建立隧道并修改连接配置
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;
    let database = database.to_string();

    match effective_config.db_type {
        DatabaseType::SQLite => task::spawn_blocking(move || connect_sqlite(&effective_config))
            .await
            .map_err(|e| DbError::Connection(format!("任务执行失败: {}", e)))?,
        DatabaseType::PostgreSQL => get_postgres_tables(&effective_config, &database).await,
        DatabaseType::MySQL => get_mysql_tables(&effective_config, &database).await,
    }
}

/// 获取表的主键列名
/// 
/// 从数据库元数据中查询主键信息，返回主键列名（如果存在）
pub async fn get_primary_key_column(
    config: &ConnectionConfig,
    table: &str,
) -> Result<Option<String>, DbError> {
    // 如果启用了 SSH 隧道，先建立隧道并修改连接配置
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;
    let table = table.to_string();

    match effective_config.db_type {
        DatabaseType::SQLite => {
            task::spawn_blocking(move || get_sqlite_primary_key(&effective_config, &table))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => get_postgres_primary_key(&effective_config, &table).await,
        DatabaseType::MySQL => get_mysql_primary_key(&effective_config, &table).await,
    }
}



/// 执行 SQL 查询或命令
///
/// # Arguments
/// * `config` - 数据库连接配置
/// * `sql` - SQL 语句
///
/// # Returns
/// 成功返回查询结果，失败返回错误
pub async fn execute_query(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    // 如果启用了 SSH 隧道，先建立隧道并修改连接配置
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;
    let sql = sql.to_string();

    match effective_config.db_type {
        DatabaseType::SQLite => task::spawn_blocking(move || execute_sqlite(&effective_config, &sql))
            .await
            .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?,
        DatabaseType::PostgreSQL => execute_postgres(&effective_config, &sql).await,
        DatabaseType::MySQL => execute_mysql(&effective_config, &sql).await,
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 如果启用了 SSH 隧道，建立隧道并返回修改后的连接配置
///
/// 返回 (有效配置, 可选的隧道引用)
/// 隧道引用需要在连接期间保持存活
async fn setup_ssh_tunnel_if_enabled(
    config: &ConnectionConfig,
) -> Result<(ConnectionConfig, Option<Arc<SshTunnel>>), DbError> {
    // SQLite 不需要 SSH 隧道
    if matches!(config.db_type, DatabaseType::SQLite) {
        return Ok((config.clone(), None));
    }

    // 检查是否启用了 SSH 隧道
    if !config.ssh_config.enabled {
        return Ok((config.clone(), None));
    }

    // 验证 SSH 配置
    config
        .ssh_config
        .validate()
        .map_err(|e| DbError::Connection(format!("SSH 配置无效: {}", e)))?;

    // 创建隧道标识符（基于配置生成唯一名称）
    let tunnel_name = format!(
        "{}:{}->{}:{}",
        config.ssh_config.ssh_host,
        config.ssh_config.ssh_port,
        config.ssh_config.remote_host,
        config.ssh_config.remote_port
    );

    // 获取或创建隧道（带超时）
    let timeout_duration = Duration::from_secs(constants::database::SSH_TUNNEL_TIMEOUT_SECS);
    let tunnel = tokio::time::timeout(
        timeout_duration,
        SSH_TUNNEL_MANAGER.get_or_create(&tunnel_name, &config.ssh_config),
    )
    .await
    .map_err(|_| {
        DbError::Connection(format!(
            "SSH 隧道建立超时 ({}秒)。请检查:\n\
             • SSH 服务器地址和端口是否正确\n\
             • 网络连接是否正常\n\
             • 防火墙是否允许连接",
            constants::database::SSH_TUNNEL_TIMEOUT_SECS
        ))
    })?
    .map_err(|e| DbError::Connection(format!("SSH 隧道建立失败: {}", e)))?;

    // 修改连接配置，使用隧道的本地端口
    let mut effective_config = config.clone();
    effective_config.host = "127.0.0.1".to_string();
    effective_config.port = tunnel.local_port();

    Ok((effective_config, Some(tunnel)))
}

/// 判断 SQL 是否为查询语句（返回结果集）
#[inline]
fn is_query_statement(sql: &str, db_type: &DatabaseType) -> bool {
    let sql_lower = sql.trim().to_lowercase();

    let common = sql_lower.starts_with("select")
        || sql_lower.starts_with("with")
        || sql_lower.starts_with("explain");

    match db_type {
        DatabaseType::SQLite => common || sql_lower.starts_with("pragma"),
        DatabaseType::PostgreSQL => common || sql_lower.starts_with("show"),
        DatabaseType::MySQL => {
            common || sql_lower.starts_with("show") || sql_lower.starts_with("describe")
        }
    }
}

/// 构建查询成功的结果
#[inline]
fn query_result(columns: Vec<String>, rows: Vec<Vec<String>>) -> QueryResult {
    QueryResult {
        columns,
        rows,
        affected_rows: 0,
        truncated: false,
        original_row_count: None,
    }
}

/// 构建执行成功的结果
#[inline]
fn exec_result(affected: u64) -> QueryResult {
    QueryResult {
        columns: vec![],
        rows: vec![],
        affected_rows: affected,
        truncated: false,
        original_row_count: None,
    }
}

/// 构建空结果
#[inline]
fn empty_result() -> QueryResult {
    QueryResult {
        columns: vec![],
        rows: vec![],
        affected_rows: 0,
        truncated: false,
        original_row_count: None,
    }
}

// ============================================================================
// SQLite 实现
// ============================================================================

fn connect_sqlite(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
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

/// 获取 SQLite 表的主键列名
fn get_sqlite_primary_key(config: &ConnectionConfig, table: &str) -> Result<Option<String>, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;
    
    // 使用 PRAGMA table_info 查询主键列（pk 字段 > 0 表示是主键）
    let escaped_table = table.replace('\'', "''");
    let sql = format!("PRAGMA table_info('{}')", escaped_table);
    
    let mut stmt = conn.prepare(&sql)
        .map_err(|e| DbError::Query(e.to_string()))?;
    
    // table_info 返回: cid, name, type, notnull, dflt_value, pk
    let pk_columns: Vec<String> = stmt
        .query_map([], |row| {
            let pk: i32 = row.get(5)?;  // pk 列
            let name: String = row.get(1)?;  // name 列
            Ok((name, pk))
        })
        .map_err(|e| DbError::Query(e.to_string()))?
        .filter_map(|r| r.ok())
        .filter(|(_, pk)| *pk > 0)
        .map(|(name, _)| name)
        .collect();
    
    // 返回第一个主键列（通常只有一个）
    Ok(pk_columns.into_iter().next())
}

fn execute_sqlite(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    if is_query_statement(sql, &DatabaseType::SQLite) {
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DbError::Query(e.to_string()))?;

        let columns: Vec<String> = stmt.column_names().into_iter().map(String::from).collect();

        let rows: Result<Vec<Vec<String>>, _> = stmt
            .query_map([], |row| {
                (0..columns.len())
                    .map(|i| sqlite_value_to_string(row.get_ref(i)))
                    .collect::<Result<Vec<_>, _>>()
            })
            .map_err(|e| DbError::Query(e.to_string()))?
            .collect();

        let rows = rows.map_err(|e| DbError::Query(e.to_string()))?;
        Ok(query_result(columns, rows))
    } else {
        let affected = conn
            .execute(sql, [])
            .map_err(|e| DbError::Query(e.to_string()))? as u64;
        Ok(exec_result(affected))
    }
}

/// 将 SQLite 值转换为字符串
fn sqlite_value_to_string(
    val: Result<ValueRef<'_>, rusqlite::Error>,
) -> Result<String, rusqlite::Error> {
    Ok(match val? {
        ValueRef::Null => String::from("NULL"),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => format!("<Blob {} bytes>", b.len()),
    })
}

// ============================================================================
// PostgreSQL 实现（使用连接池）
// ============================================================================

/// 获取 PostgreSQL 数据库列表
async fn get_postgres_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
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
async fn get_postgres_tables(config: &ConnectionConfig, database: &str) -> Result<Vec<String>, DbError> {
    // 创建一个临时配置，连接到指定数据库
    let mut db_config = config.clone();
    db_config.database = database.to_string();
    
    let client = POOL_MANAGER.get_pg_client(&db_config).await?;

    let rows = client
        .query(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
            &[],
        )
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get(0)).collect())
}

/// 获取 PostgreSQL 表的主键列名
async fn get_postgres_primary_key(config: &ConnectionConfig, table: &str) -> Result<Option<String>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    
    // 查询 information_schema 获取主键列
    let rows = client
        .query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             WHERE i.indrelid = $1::regclass
             AND i.indisprimary
             LIMIT 1",
            &[&table],
        )
        .await
        .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;
    
    Ok(rows.first().map(|r| r.get(0)))
}



async fn execute_postgres(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    if is_query_statement(sql, &DatabaseType::PostgreSQL) {
        let rows = client
            .query(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        if rows.is_empty() {
            return Ok(empty_result());
        }

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_owned())
            .collect();

        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|row| postgres_row_to_strings(row, columns.len()))
            .collect();

        Ok(query_result(columns, data))
    } else {
        let affected = client
            .execute(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(exec_result(affected))
    }
}

/// 将 PostgreSQL 行转换为字符串向量
fn postgres_row_to_strings(row: &tokio_postgres::Row, col_count: usize) -> Vec<String> {
    (0..col_count)
        .map(|i| {
            // 尝试多种类型转换
            row.try_get::<_, String>(i)
                .or_else(|_| row.try_get::<_, i64>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, i32>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, i16>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, f64>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, f32>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, bool>(i).map(|v| v.to_string()))
                .or_else(|_| {
                    row.try_get::<_, chrono::NaiveDateTime>(i)
                        .map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string())
                })
                .or_else(|_| {
                    row.try_get::<_, chrono::NaiveDate>(i)
                        .map(|v| v.format("%Y-%m-%d").to_string())
                })
                .or_else(|_| {
                    row.try_get::<_, chrono::NaiveTime>(i)
                        .map(|v| v.format("%H:%M:%S").to_string())
                })
                .unwrap_or_else(|_| String::from("NULL"))
        })
        .collect()
}

// ============================================================================
// MySQL 实现（使用连接池）
// ============================================================================

/// 获取 MySQL 数据库列表
async fn get_mysql_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
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
        .filter(|db| !matches!(db.as_str(), "information_schema" | "mysql" | "performance_schema" | "sys"))
        .collect())
}

/// 获取 MySQL 指定数据库的表列表
async fn get_mysql_tables(config: &ConnectionConfig, database: &str) -> Result<Vec<String>, DbError> {
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
async fn get_mysql_primary_key(config: &ConnectionConfig, table: &str) -> Result<Option<String>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;
    
    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;
    
    // 使用 SHOW KEYS 查询主键列
    let escaped_table = table.replace('`', "``").replace('.', "_");
    let sql = format!(
        "SHOW KEYS FROM `{}` WHERE Key_name = 'PRIMARY'",
        escaped_table
    );
    
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



async fn execute_mysql(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    if is_query_statement(sql, &DatabaseType::MySQL) {
        let result: Vec<mysql_async::Row> = conn
            .query(sql)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        if result.is_empty() {
            return Ok(empty_result());
        }

        let columns: Vec<String> = result[0]
            .columns_ref()
            .iter()
            .map(|c| c.name_str().into_owned())
            .collect();

        let data: Vec<Vec<String>> = result
            .iter()
            .map(|row| mysql_row_to_strings(row, columns.len()))
            .collect();

        Ok(query_result(columns, data))
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

/// 将 MySQL 行转换为字符串向量
fn mysql_row_to_strings(row: &mysql_async::Row, col_count: usize) -> Vec<String> {
    (0..col_count)
        .map(|i| {
            row.get::<mysql_async::Value, _>(i)
                .map(mysql_value_to_string)
                .unwrap_or_else(|| String::from("NULL"))
        })
        .collect()
}

/// 将 MySQL Value 转换为字符串
fn mysql_value_to_string(val: mysql_async::Value) -> String {
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

// ============================================================================
// 触发器查询
// ============================================================================

/// 触发器信息
#[derive(Debug, Clone)]
pub struct TriggerInfo {
    pub name: String,
    pub table_name: String,
    pub event: String,      // INSERT/UPDATE/DELETE
    pub timing: String,     // BEFORE/AFTER
    pub definition: String, // SQL 定义
}

/// 获取数据库的触发器列表
pub async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;

    match effective_config.db_type {
        DatabaseType::SQLite => {
            task::spawn_blocking(move || get_sqlite_triggers(&effective_config))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => get_postgres_triggers(&effective_config).await,
        DatabaseType::MySQL => get_mysql_triggers(&effective_config).await,
    }
}

/// 获取 SQLite 触发器
fn get_sqlite_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
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
            }.to_string();
            
            let event = if sql_upper.contains("INSERT") {
                "INSERT"
            } else if sql_upper.contains("UPDATE") {
                "UPDATE"
            } else if sql_upper.contains("DELETE") {
                "DELETE"
            } else {
                "UNKNOWN"
            }.to_string();
            
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

/// 获取 PostgreSQL 触发器
async fn get_postgres_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

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
          AND n.nspname = 'public'
        ORDER BY t.tgname
    "#;

    let rows = client
        .query(sql, &[])
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

/// 获取 MySQL 触发器
async fn get_mysql_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
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

// ============================================================================
// 外键查询（用于 ER 图）
// ============================================================================

/// 外键信息
#[derive(Debug, Clone)]
pub struct ForeignKeyInfo {
    pub from_table: String,
    pub from_column: String,
    pub to_table: String,
    pub to_column: String,
}

/// 列信息
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub is_primary_key: bool,
    pub is_nullable: bool,
    /// 默认值（如有）
    pub default_value: Option<String>,
}

/// 获取数据库的所有外键关系
pub async fn get_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;

    match effective_config.db_type {
        DatabaseType::SQLite => {
            task::spawn_blocking(move || get_sqlite_foreign_keys(&effective_config))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => get_postgres_foreign_keys(&effective_config).await,
        DatabaseType::MySQL => get_mysql_foreign_keys(&effective_config).await,
    }
}

/// 获取指定表的列信息
pub async fn get_table_columns(
    config: &ConnectionConfig,
    table: &str,
) -> Result<Vec<ColumnInfo>, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;
    let table = table.to_string();

    match effective_config.db_type {
        DatabaseType::SQLite => {
            task::spawn_blocking(move || get_sqlite_columns(&effective_config, &table))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => get_postgres_columns(&effective_config, &table).await,
        DatabaseType::MySQL => get_mysql_columns(&effective_config, &table).await,
    }
}

/// 获取 SQLite 外键
fn get_sqlite_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    // 首先获取所有表
    let mut tables_stmt = conn
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")
        .map_err(|e| DbError::Query(e.to_string()))?;

    let tables: Vec<String> = tables_stmt
        .query_map([], |row| row.get(0))
        .map_err(|e| DbError::Query(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    let mut foreign_keys = Vec::new();

    // 对每个表查询外键
    for table in tables {
        let sql = format!("PRAGMA foreign_key_list('{}')", table.replace('\'', "''"));
        let mut fk_stmt = conn
            .prepare(&sql)
            .map_err(|e| DbError::Query(e.to_string()))?;

        // foreign_key_list 返回: id, seq, table, from, to, on_update, on_delete, match
        let fks: Vec<ForeignKeyInfo> = fk_stmt
            .query_map([], |row| {
                let to_table: String = row.get(2)?;
                let from_column: String = row.get(3)?;
                let to_column: String = row.get(4)?;
                Ok(ForeignKeyInfo {
                    from_table: table.clone(),
                    from_column,
                    to_table,
                    to_column,
                })
            })
            .map_err(|e| DbError::Query(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        foreign_keys.extend(fks);
    }

    Ok(foreign_keys)
}

/// 获取 PostgreSQL 外键
async fn get_postgres_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

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
        WHERE kcu.table_schema = 'public'
        ORDER BY kcu.table_name, kcu.column_name
    "#;

    let rows = client
        .query(sql, &[])
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

/// 获取 MySQL 外键
async fn get_mysql_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
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

/// 获取 SQLite 表的列信息
fn get_sqlite_columns(config: &ConnectionConfig, table: &str) -> Result<Vec<ColumnInfo>, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    let sql = format!("PRAGMA table_info('{}')", table.replace('\'', "''"));
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| DbError::Query(e.to_string()))?;

    // table_info 返回: cid, name, type, notnull, dflt_value, pk
    let columns: Vec<ColumnInfo> = stmt
        .query_map([], |row| {
            let name: String = row.get(1)?;
            let data_type: String = row.get(2)?;
            let notnull: i32 = row.get(3)?;
            let default_value: Option<String> = row.get(4)?;
            let pk: i32 = row.get(5)?;
            Ok(ColumnInfo {
                name,
                data_type,
                is_primary_key: pk > 0,
                is_nullable: notnull == 0,
                default_value,
            })
        })
        .map_err(|e| DbError::Query(e.to_string()))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(columns)
}

/// 获取 PostgreSQL 表的列信息
async fn get_postgres_columns(config: &ConnectionConfig, table: &str) -> Result<Vec<ColumnInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

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
              AND tc.table_name = $1
              AND tc.table_schema = 'public'
        ) pk ON c.column_name = pk.column_name
        WHERE c.table_name = $1
          AND c.table_schema = 'public'
        ORDER BY c.ordinal_position
    "#;

    let rows = client
        .query(sql, &[&table])
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

/// 获取 MySQL 表的列信息
async fn get_mysql_columns(config: &ConnectionConfig, table: &str) -> Result<Vec<ColumnInfo>, DbError> {
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
