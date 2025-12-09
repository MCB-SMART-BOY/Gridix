//! 数据库查询执行模块
//!
//! 提供对 SQLite、PostgreSQL、MySQL 的统一查询接口。
//! PostgreSQL 和 MySQL 使用连接池优化性能。

use super::*;
use mysql_async::prelude::*;
use rusqlite::{types::ValueRef, Connection as SqliteConn};
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
pub async fn connect_database(config: &ConnectionConfig) -> Result<ConnectResult, DbError> {
    let config = config.clone();

    match config.db_type {
        DatabaseType::SQLite => {
            let tables = task::spawn_blocking(move || connect_sqlite(&config))
                .await
                .map_err(|e| DbError::Connection(format!("任务执行失败: {}", e)))??;
            Ok(ConnectResult::Tables(tables))
        }
        DatabaseType::PostgreSQL => {
            let databases = get_postgres_databases(&config).await?;
            Ok(ConnectResult::Databases(databases))
        }
        DatabaseType::MySQL => {
            let databases = get_mysql_databases(&config).await?;
            Ok(ConnectResult::Databases(databases))
        }
    }
}

/// 获取指定数据库的表列表
pub async fn get_tables_for_database(
    config: &ConnectionConfig,
    database: &str,
) -> Result<Vec<String>, DbError> {
    let config = config.clone();
    let database = database.to_string();

    match config.db_type {
        DatabaseType::SQLite => task::spawn_blocking(move || connect_sqlite(&config))
            .await
            .map_err(|e| DbError::Connection(format!("任务执行失败: {}", e)))?,
        DatabaseType::PostgreSQL => get_postgres_tables(&config, &database).await,
        DatabaseType::MySQL => get_mysql_tables(&config, &database).await,
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
    let config = config.clone();
    let sql = sql.to_string();

    match config.db_type {
        DatabaseType::SQLite => task::spawn_blocking(move || execute_sqlite(&config, &sql))
            .await
            .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?,
        DatabaseType::PostgreSQL => execute_postgres(&config, &sql).await,
        DatabaseType::MySQL => execute_mysql(&config, &sql).await,
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

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
    let msg = format!("返回 {} 行", rows.len());
    QueryResult {
        columns,
        rows,
        message: msg,
        affected_rows: 0,
    }
}

/// 构建执行成功的结果
#[inline]
fn exec_result(affected: u64) -> QueryResult {
    let msg = format!("影响 {} 行", affected);
    QueryResult {
        columns: vec![],
        rows: vec![],
        message: msg,
        affected_rows: affected,
    }
}

/// 构建空结果
#[inline]
fn empty_result() -> QueryResult {
    QueryResult {
        columns: vec![],
        rows: vec![],
        message: "无结果".to_string(),
        affected_rows: 0,
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

fn execute_sqlite(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let conn = SqliteConn::open(&config.database)
        .map_err(|e| DbError::Connection(format!("SQLite 连接失败: {}", e)))?;

    if is_query_statement(sql, &DatabaseType::SQLite) {
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| DbError::Query(e.to_string()))?;

        let columns: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

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
        ValueRef::Null => "NULL".to_string(),
        ValueRef::Integer(i) => i.to_string(),
        ValueRef::Real(f) => f.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).to_string(),
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
            .map(|c| c.name().to_string())
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
                .unwrap_or_else(|_| "NULL".to_string())
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
            return Ok(QueryResult {
                columns: vec![],
                rows: vec![],
                message: "查询完成，无数据返回".to_string(),
                affected_rows: 0,
            });
        }

        let columns: Vec<String> = result[0]
            .columns_ref()
            .iter()
            .map(|c| c.name_str().to_string())
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
                .unwrap_or_else(|| "NULL".to_string())
        })
        .collect()
}

/// 将 MySQL Value 转换为字符串
fn mysql_value_to_string(val: mysql_async::Value) -> String {
    use mysql_async::Value;
    match val {
        Value::NULL => "NULL".to_string(),
        Value::Bytes(b) => String::from_utf8_lossy(&b).to_string(),
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
