//! 数据库查询执行模块
//!
//! 提供对 SQLite、PostgreSQL、MySQL 的统一查询接口。
//! PostgreSQL 和 MySQL 使用连接池优化性能。

#![allow(dead_code)] // 公开 API，部分功能预留

mod mysql;
mod postgres;
mod sqlite;

use super::ssh_tunnel::{SshTunnel, SSH_TUNNEL_MANAGER};
use super::*;
use crate::core::constants;
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
            let tables = task::spawn_blocking(move || sqlite::connect(&effective_config))
                .await
                .map_err(|e| DbError::Connection(format!("任务执行失败: {}", e)))??;
            Ok(ConnectResult::Tables(tables))
        }
        DatabaseType::PostgreSQL => {
            let databases = postgres::get_databases(&effective_config).await?;
            Ok(ConnectResult::Databases(databases))
        }
        DatabaseType::MySQL => {
            let databases = mysql::get_databases(&effective_config).await?;
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
        DatabaseType::SQLite => task::spawn_blocking(move || sqlite::connect(&effective_config))
            .await
            .map_err(|e| DbError::Connection(format!("任务执行失败: {}", e)))?,
        DatabaseType::PostgreSQL => postgres::get_tables(&effective_config, &database).await,
        DatabaseType::MySQL => mysql::get_tables(&effective_config, &database).await,
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
            task::spawn_blocking(move || sqlite::get_primary_key(&effective_config, &table))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => postgres::get_primary_key(&effective_config, &table).await,
        DatabaseType::MySQL => mysql::get_primary_key(&effective_config, &table).await,
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
        DatabaseType::SQLite => {
            task::spawn_blocking(move || sqlite::execute(&effective_config, &sql))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => postgres::execute(&effective_config, &sql).await,
        DatabaseType::MySQL => mysql::execute(&effective_config, &sql).await,
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
pub(crate) fn is_query_statement(sql: &str, db_type: &DatabaseType) -> bool {
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
pub(crate) fn query_result(columns: Vec<String>, rows: Vec<Vec<String>>) -> QueryResult {
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
pub(crate) fn exec_result(affected: u64) -> QueryResult {
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
pub(crate) fn empty_result() -> QueryResult {
    QueryResult {
        columns: vec![],
        rows: vec![],
        affected_rows: 0,
        truncated: false,
        original_row_count: None,
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

/// 存储过程/函数类型
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutineType {
    Procedure,
    Function,
}

impl std::fmt::Display for RoutineType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutineType::Procedure => write!(f, "存储过程"),
            RoutineType::Function => write!(f, "函数"),
        }
    }
}

/// 存储过程/函数信息
#[derive(Debug, Clone)]
pub struct RoutineInfo {
    pub name: String,
    pub routine_type: RoutineType,
    pub parameters: String,      // 参数列表
    pub return_type: Option<String>, // 返回类型（仅函数）
    pub definition: String,      // SQL 定义
}

/// 获取数据库的触发器列表
pub async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;

    match effective_config.db_type {
        DatabaseType::SQLite => {
            task::spawn_blocking(move || sqlite::get_triggers(&effective_config))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => postgres::get_triggers(&effective_config).await,
        DatabaseType::MySQL => mysql::get_triggers(&effective_config).await,
    }
}

/// 获取数据库的存储过程和函数列表
pub async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;

    match effective_config.db_type {
        // SQLite 不支持存储过程
        DatabaseType::SQLite => Ok(Vec::new()),
        DatabaseType::PostgreSQL => postgres::get_routines(&effective_config).await,
        DatabaseType::MySQL => mysql::get_routines(&effective_config).await,
    }
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
            task::spawn_blocking(move || sqlite::get_foreign_keys(&effective_config))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => postgres::get_foreign_keys(&effective_config).await,
        DatabaseType::MySQL => mysql::get_foreign_keys(&effective_config).await,
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
            task::spawn_blocking(move || sqlite::get_columns(&effective_config, &table))
                .await
                .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
        }
        DatabaseType::PostgreSQL => postgres::get_columns(&effective_config, &table).await,
        DatabaseType::MySQL => mysql::get_columns(&effective_config, &table).await,
    }
}
