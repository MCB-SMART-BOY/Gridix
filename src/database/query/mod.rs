//! 数据库查询执行模块
//!
//! 提供对 SQLite、PostgreSQL、MySQL 的统一查询接口。
//! PostgreSQL 和 MySQL 使用连接池优化性能。

#![allow(dead_code)] // 公开 API，部分功能预留

mod mysql;
mod postgres;
mod sqlite;

use super::ssh_tunnel::{SSH_TUNNEL_MANAGER, SshTunnel};
use super::*;
use crate::core::constants;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::oneshot;
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

/// 批量导入执行报告
#[derive(Debug, Clone, Default)]
pub struct ImportExecutionReport {
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub first_error: Option<String>,
}

impl ImportExecutionReport {
    pub const fn new(total: usize) -> Self {
        Self {
            total,
            succeeded: 0,
            failed: 0,
            first_error: None,
        }
    }
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

/// 执行可取消的 SQL 查询
///
/// `cancel_rx` 收到信号后会尝试在数据库侧取消正在执行的查询。
pub async fn execute_query_cancellable(
    config: &ConnectionConfig,
    sql: &str,
    mut cancel_rx: oneshot::Receiver<()>,
) -> Result<QueryResult, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;
    let sql = sql.to_string();

    match effective_config.db_type {
        DatabaseType::SQLite => {
            let (interrupt_tx, interrupt_rx) = oneshot::channel::<rusqlite::InterruptHandle>();
            let mut query_task = task::spawn_blocking(move || {
                sqlite::execute_with_interrupt_handle(&effective_config, &sql, Some(interrupt_tx))
            });

            tokio::select! {
                res = &mut query_task => {
                    res.map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?
                }
                _ = &mut cancel_rx => {
                    if let Ok(handle) = interrupt_rx.await {
                        handle.interrupt();
                    }
                    let _ = (&mut query_task).await;
                    Err(DbError::Query("查询已取消".to_string()))
                }
            }
        }
        DatabaseType::PostgreSQL => {
            postgres::execute_cancellable(&effective_config, &sql, cancel_rx).await
        }
        DatabaseType::MySQL => mysql::execute_cancellable(&effective_config, &sql, cancel_rx).await,
    }
}

/// 批量执行 SQL（用于导入）
pub async fn execute_import_batch(
    config: &ConnectionConfig,
    statements: Vec<String>,
    use_transaction: bool,
    stop_on_error: bool,
) -> Result<ImportExecutionReport, DbError> {
    let (effective_config, _tunnel) = setup_ssh_tunnel_if_enabled(config).await?;
    let statements: Vec<String> = statements
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .collect();

    match effective_config.db_type {
        DatabaseType::SQLite => task::spawn_blocking(move || {
            sqlite::execute_batch(
                &effective_config,
                &statements,
                use_transaction,
                stop_on_error,
            )
        })
        .await
        .map_err(|e| DbError::Query(format!("任务执行失败: {}", e)))?,
        DatabaseType::PostgreSQL => {
            postgres::execute_batch(
                &effective_config,
                &statements,
                use_transaction,
                stop_on_error,
            )
            .await
        }
        DatabaseType::MySQL => {
            mysql::execute_batch(
                &effective_config,
                &statements,
                use_transaction,
                stop_on_error,
            )
            .await
        }
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
    let tunnel_name = config.ssh_config.tunnel_name();

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
    let mut i = skip_sql_ws_and_comments(sql, 0);
    if let Some(keyword) = read_sql_keyword(sql, &mut i) {
        if keyword == "with" {
            if let Some(main_keyword) = with_main_keyword(sql) {
                if is_dml_keyword(&main_keyword) && has_returning_clause(sql) {
                    return true;
                }
                return is_query_keyword(&main_keyword, db_type);
            }

            // 解析失败时，保守地按常见读查询模式兜底，避免误判大多数 WITH SELECT 场景。
            let lower = sql.to_ascii_lowercase();
            return lower.contains(") select")
                || lower.contains(") values")
                || lower.contains(") explain");
        }
        if is_dml_keyword(&keyword) && has_returning_clause(sql) {
            return true;
        }
        return is_query_keyword(&keyword, db_type);
    }

    false
}

/// 供 UI 使用的 SQL 动作提示
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct SqlUiHints {
    pub is_update_or_delete: bool,
    pub is_insert: bool,
    pub is_drop_table: bool,
}

/// 分析 SQL 的主动作（忽略前导注释/空白，支持 WITH 主语句）
pub(crate) fn analyze_sql_for_ui(sql: &str) -> SqlUiHints {
    let mut i = skip_sql_ws_and_comments(sql, 0);
    let Some(keyword) = read_sql_keyword(sql, &mut i) else {
        return SqlUiHints::default();
    };

    let main_keyword = if keyword == "with" {
        with_main_keyword(sql).unwrap_or_default()
    } else {
        keyword
    };

    let mut hints = SqlUiHints::default();
    match main_keyword.as_str() {
        "update" | "delete" => hints.is_update_or_delete = true,
        "insert" => hints.is_insert = true,
        "drop" => {
            i = skip_sql_ws_and_comments(sql, i);
            if let Some(next_keyword) = read_sql_keyword(sql, &mut i)
                && next_keyword == "table"
            {
                hints.is_drop_table = true;
            }
        }
        _ => {}
    }

    hints
}

#[inline]
fn is_query_keyword(keyword: &str, db_type: &DatabaseType) -> bool {
    match db_type {
        DatabaseType::SQLite => {
            matches!(keyword, "select" | "values" | "explain" | "pragma")
        }
        DatabaseType::PostgreSQL => {
            matches!(keyword, "select" | "values" | "explain" | "show" | "table")
        }
        DatabaseType::MySQL => {
            matches!(
                keyword,
                "select" | "values" | "explain" | "show" | "describe" | "desc"
            )
        }
    }
}

#[inline]
fn is_dml_keyword(keyword: &str) -> bool {
    matches!(keyword, "insert" | "update" | "delete")
}

/// 跳过 SQL 前导空白和注释，返回下一个字符位置
fn skip_sql_ws_and_comments(sql: &str, mut i: usize) -> usize {
    let bytes = sql.as_bytes();
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }
        break;
    }
    i
}

/// 读取当前位置的关键字（小写），并推进索引
fn read_sql_keyword(sql: &str, i: &mut usize) -> Option<String> {
    let bytes = sql.as_bytes();
    let start = *i;
    while *i < bytes.len() && bytes[*i].is_ascii_alphabetic() {
        *i += 1;
    }
    if *i == start {
        return None;
    }
    Some(sql[start..*i].to_ascii_lowercase())
}

/// 在 SQL 中查找指定关键字（忽略大小写），跳过字符串和注释
fn contains_keyword_outside_literals(sql: &str, keyword: &str) -> bool {
    let bytes = sql.as_bytes();
    let mut i = 0usize;

    while i < bytes.len() {
        // 跳过空白
        if bytes[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }

        // 跳过行注释
        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        // 跳过块注释
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        // 跳过普通引号/反引号内容
        if matches!(bytes[i], b'\'' | b'"' | b'`') {
            if let Some(next_i) = skip_quoted(sql, i, bytes[i]) {
                i = next_i;
                continue;
            }
            return false;
        }

        // 跳过 PostgreSQL dollar-quoted 内容
        if bytes[i] == b'$'
            && let Some(tag) = parse_dollar_quote_tag_bytes(bytes, i)
        {
            i += tag.len();
            while i < bytes.len() {
                if i + tag.len() <= bytes.len() && &bytes[i..i + tag.len()] == tag.as_slice() {
                    i += tag.len();
                    break;
                }
                i += 1;
            }
            continue;
        }

        // 读取 token 并比对关键字
        if bytes[i].is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            if sql[start..i].eq_ignore_ascii_case(keyword) {
                return true;
            }
            continue;
        }

        i += 1;
    }

    false
}

#[inline]
fn has_returning_clause(sql: &str) -> bool {
    contains_keyword_outside_literals(sql, "returning")
}

fn parse_dollar_quote_tag_bytes(bytes: &[u8], idx: usize) -> Option<Vec<u8>> {
    if bytes.get(idx) != Some(&b'$') {
        return None;
    }

    let mut j = idx + 1;
    if j >= bytes.len() {
        return None;
    }

    if bytes[j] == b'$' {
        return Some(vec![b'$', b'$']);
    }

    if !(bytes[j].is_ascii_alphabetic() || bytes[j] == b'_') {
        return None;
    }
    j += 1;
    while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
        j += 1;
    }

    if j < bytes.len() && bytes[j] == b'$' {
        Some(bytes[idx..=j].to_vec())
    } else {
        None
    }
}

/// 判断当前位置是否是指定关键字（忽略大小写），并推进索引
fn consume_keyword_ci(sql: &str, i: &mut usize, keyword: &str) -> bool {
    let bytes = sql.as_bytes();
    let end = *i + keyword.len();
    if end > bytes.len() {
        return false;
    }

    if !sql[*i..end].eq_ignore_ascii_case(keyword) {
        return false;
    }

    // 关键字边界检查：后面不能直接连标识符字符
    if end < bytes.len() && is_ident_char(bytes[end]) {
        return false;
    }

    *i = end;
    true
}

#[inline]
fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$' | b'.')
}

/// 识别 WITH 语句的主关键字（如 SELECT/UPDATE/DELETE）
fn with_main_keyword(sql: &str) -> Option<String> {
    let bytes = sql.as_bytes();
    let mut i = skip_sql_ws_and_comments(sql, 0);
    if !consume_keyword_ci(sql, &mut i, "with") {
        return None;
    }

    i = skip_sql_ws_and_comments(sql, i);
    if consume_keyword_ci(sql, &mut i, "recursive") {
        i = skip_sql_ws_and_comments(sql, i);
    }

    loop {
        i = skip_sql_ws_and_comments(sql, i);
        i = skip_cte_name(sql, i)?;
        i = skip_sql_ws_and_comments(sql, i);

        // 可选列名列表：cte_name(col1, col2)
        if i < bytes.len() && bytes[i] == b'(' {
            i = skip_balanced_parens(sql, i)?;
            i = skip_sql_ws_and_comments(sql, i);
        }

        if !consume_keyword_ci(sql, &mut i, "as") {
            return None;
        }
        i = skip_sql_ws_and_comments(sql, i);

        if i >= bytes.len() || bytes[i] != b'(' {
            return None;
        }
        i = skip_balanced_parens(sql, i)?;
        i = skip_sql_ws_and_comments(sql, i);

        if i < bytes.len() && bytes[i] == b',' {
            i += 1;
            continue;
        }
        break;
    }

    i = skip_sql_ws_and_comments(sql, i);
    read_sql_keyword(sql, &mut i)
}

/// 跳过 CTE 名称（支持普通标识符和引号标识符）
fn skip_cte_name(sql: &str, mut i: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    if i >= bytes.len() {
        return None;
    }

    match bytes[i] {
        b'"' | b'`' => skip_quoted(sql, i, bytes[i]),
        _ => {
            if !bytes[i].is_ascii_alphabetic() && bytes[i] != b'_' {
                return None;
            }
            i += 1;
            while i < bytes.len() && is_ident_char(bytes[i]) {
                i += 1;
            }
            Some(i)
        }
    }
}

/// 跳过引号字符串/标识符，支持 SQL 双写转义
fn skip_quoted(sql: &str, mut i: usize, quote: u8) -> Option<usize> {
    let bytes = sql.as_bytes();
    if i >= bytes.len() || bytes[i] != quote {
        return None;
    }
    i += 1;

    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i = (i + 2).min(bytes.len());
            continue;
        }
        if bytes[i] == quote {
            if i + 1 < bytes.len() && bytes[i + 1] == quote {
                i += 2;
                continue;
            }
            return Some(i + 1);
        }
        i += 1;
    }

    None
}

/// 跳过平衡括号表达式，要求当前位置为 '('
fn skip_balanced_parens(sql: &str, mut i: usize) -> Option<usize> {
    let bytes = sql.as_bytes();
    if i >= bytes.len() || bytes[i] != b'(' {
        return None;
    }

    let mut depth = 1usize;
    i += 1;

    while i < bytes.len() {
        if bytes[i] == b'\'' || bytes[i] == b'"' || bytes[i] == b'`' {
            i = skip_quoted(sql, i, bytes[i])?;
            continue;
        }

        if i + 1 < bytes.len() && bytes[i] == b'-' && bytes[i + 1] == b'-' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }

        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() {
                if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                    i += 2;
                    break;
                }
                i += 1;
            }
            continue;
        }

        match bytes[i] {
            b'(' => {
                depth += 1;
                i += 1;
            }
            b')' => {
                depth -= 1;
                i += 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => i += 1,
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_query_statement_with_select() {
        let sql = "WITH cte AS (SELECT 1 AS id) SELECT id FROM cte";
        assert!(is_query_statement(sql, &DatabaseType::PostgreSQL));
        assert!(is_query_statement(sql, &DatabaseType::MySQL));
        assert!(is_query_statement(sql, &DatabaseType::SQLite));
    }

    #[test]
    fn test_is_query_statement_with_update() {
        let sql = "WITH cte AS (SELECT 1 AS id) UPDATE users SET active = 1 WHERE id IN (SELECT id FROM cte)";
        assert!(!is_query_statement(sql, &DatabaseType::PostgreSQL));
        assert!(!is_query_statement(sql, &DatabaseType::MySQL));
        assert!(!is_query_statement(sql, &DatabaseType::SQLite));
    }

    #[test]
    fn test_is_query_statement_with_insert_returning_like_case() {
        let sql = "WITH src AS (SELECT 1 AS id) INSERT INTO logs(id) SELECT id FROM src";
        assert!(!is_query_statement(sql, &DatabaseType::PostgreSQL));
        assert!(!is_query_statement(sql, &DatabaseType::MySQL));
        assert!(!is_query_statement(sql, &DatabaseType::SQLite));
    }

    #[test]
    fn test_is_query_statement_table_and_desc_shortcut() {
        assert!(is_query_statement("TABLE users", &DatabaseType::PostgreSQL));
        assert!(is_query_statement("DESC users", &DatabaseType::MySQL));
    }

    #[test]
    fn test_is_query_statement_dml_returning() {
        let sql = "INSERT INTO users(name) VALUES ('alice') RETURNING id";
        assert!(is_query_statement(sql, &DatabaseType::PostgreSQL));
        assert!(is_query_statement(sql, &DatabaseType::SQLite));
        assert!(is_query_statement(sql, &DatabaseType::MySQL));
    }

    #[test]
    fn test_is_query_statement_dml_without_returning() {
        let sql = "UPDATE users SET note = 'returning home' WHERE id = 1";
        assert!(!is_query_statement(sql, &DatabaseType::PostgreSQL));
        assert!(!is_query_statement(sql, &DatabaseType::SQLite));
        assert!(!is_query_statement(sql, &DatabaseType::MySQL));
    }

    #[test]
    fn test_analyze_sql_for_ui_with_comments_and_cte() {
        let hints = analyze_sql_for_ui("/* c */ WITH c AS (SELECT 1) UPDATE t SET v = 1");
        assert!(hints.is_update_or_delete);
        assert!(!hints.is_insert);
        assert!(!hints.is_drop_table);
    }

    #[test]
    fn test_analyze_sql_for_ui_drop_table_with_prefix() {
        let hints = analyze_sql_for_ui("-- x\nDROP   TABLE users");
        assert!(!hints.is_update_or_delete);
        assert!(!hints.is_insert);
        assert!(hints.is_drop_table);
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
    pub parameters: String,          // 参数列表
    pub return_type: Option<String>, // 返回类型（仅函数）
    pub definition: String,          // SQL 定义
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
