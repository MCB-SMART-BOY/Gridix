//! 数据库模块 - 连接管理、查询执行
//!
//! 支持 SQLite、PostgreSQL、MySQL 三种数据库，使用连接池优化性能。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// 错误类型
// ============================================================================

/// 数据库操作错误
#[derive(Error, Debug)]
pub enum DbError {
    #[error("连接错误: {0}")]
    Connection(String),
    #[error("查询错误: {0}")]
    Query(String),
    #[error("连接池错误: {0}")]
    #[allow(dead_code)]
    Pool(String),
}

// ============================================================================
// 数据库类型
// ============================================================================

/// 支持的数据库类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub enum DatabaseType {
    #[default]
    SQLite,
    PostgreSQL,
    MySQL,
}

impl DatabaseType {
    /// 获取显示名称
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::SQLite => "SQLite",
            Self::PostgreSQL => "PostgreSQL",
            Self::MySQL => "MySQL",
        }
    }

    /// 获取所有数据库类型
    pub const fn all() -> &'static [DatabaseType] {
        &[Self::SQLite, Self::PostgreSQL, Self::MySQL]
    }

    /// 获取默认端口
    pub const fn default_port(&self) -> u16 {
        match self {
            Self::SQLite => 0,
            Self::PostgreSQL => 5432,
            Self::MySQL => 3306,
        }
    }

    /// 是否需要网络连接
    #[allow(dead_code)]
    pub const fn requires_network(&self) -> bool {
        !matches!(self, Self::SQLite)
    }
}

// ============================================================================
// 连接配置
// ============================================================================

/// 数据库连接配置
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct ConnectionConfig {
    pub name: String,
    pub db_type: DatabaseType,
    pub host: String,
    pub port: u16,
    pub username: String,
    /// 密码使用 base64 编码存储，避免明文
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        serialize_with = "encode_password",
        deserialize_with = "decode_password"
    )]
    pub password: String,
    /// 数据库名（SQLite 为文件路径，MySQL/PostgreSQL 为可选的默认数据库）
    #[serde(default)]
    pub database: String,
}

/// 将密码编码为 base64 存储
fn encode_password<S>(password: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use base64::{engine::general_purpose::STANDARD, Engine};
    if password.is_empty() {
        serializer.serialize_str("")
    } else {
        let encoded = STANDARD.encode(password.as_bytes());
        serializer.serialize_str(&encoded)
    }
}

/// 从 base64 解码密码
fn decode_password<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde::de::Error;

    let s: String = String::deserialize(deserializer)?;
    if s.is_empty() {
        return Ok(String::new());
    }

    // 尝试 base64 解码，如果失败则假设是旧版明文密码
    match STANDARD.decode(&s) {
        Ok(bytes) => String::from_utf8(bytes)
            .map_err(|e| D::Error::custom(format!("Invalid UTF-8 in password: {}", e))),
        Err(_) => {
            // 可能是旧版本的明文密码，直接返回
            Ok(s)
        }
    }
}

impl ConnectionConfig {
    /// 创建新的连接配置
    #[allow(dead_code)]
    pub fn new(name: impl Into<String>, db_type: DatabaseType) -> Self {
        let db_type_clone = db_type.clone();
        Self {
            name: name.into(),
            db_type,
            port: db_type_clone.default_port(),
            host: if db_type_clone.requires_network() {
                "localhost".into()
            } else {
                String::new()
            },
            ..Default::default()
        }
    }

    /// 生成连接字符串（带数据库名）
    pub fn connection_string(&self) -> String {
        self.connection_string_with_db(Some(&self.database))
    }

    /// 生成连接字符串（可指定数据库名）
    pub fn connection_string_with_db(&self, database: Option<&str>) -> String {
        match self.db_type {
            DatabaseType::SQLite => self.database.clone(),
            DatabaseType::PostgreSQL => {
                let db = database.filter(|s| !s.is_empty()).unwrap_or("postgres");
                format!(
                    "host={} port={} user={} password={} dbname={}",
                    self.host, self.port, self.username, self.password, db
                )
            }
            DatabaseType::MySQL => {
                if let Some(db) = database.filter(|s| !s.is_empty()) {
                    format!(
                        "mysql://{}:{}@{}:{}/{}",
                        self.username, self.password, self.host, self.port, db
                    )
                } else {
                    format!(
                        "mysql://{}:{}@{}:{}",
                        self.username, self.password, self.host, self.port
                    )
                }
            }
        }
    }

    /// 生成唯一的连接标识符（用于连接池缓存，按用户+主机区分）
    pub fn pool_key(&self) -> String {
        match self.db_type {
            DatabaseType::SQLite => format!("sqlite:{}", self.database),
            DatabaseType::PostgreSQL => {
                format!("pg:{}:{}:{}", self.host, self.port, self.username)
            }
            DatabaseType::MySQL => {
                format!("mysql:{}:{}:{}", self.host, self.port, self.username)
            }
        }
    }



    /// 验证配置是否有效
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        if self.name.is_empty() {
            return false;
        }
        match self.db_type {
            DatabaseType::SQLite => !self.database.is_empty(),
            _ => !self.host.is_empty() && !self.database.is_empty() && self.port > 0,
        }
    }
}

// ============================================================================
// 查询结果
// ============================================================================

/// 查询结果
#[derive(Debug, Clone, Default)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
    #[allow(dead_code)]
    pub message: String,
    pub affected_rows: u64,
}

#[allow(dead_code)]
impl QueryResult {
    /// 创建查询结果（有数据返回）
    pub fn with_data(columns: Vec<String>, rows: Vec<Vec<String>>) -> Self {
        Self {
            columns,
            rows,
            message: String::new(),
            affected_rows: 0,
        }
    }

    /// 创建执行结果（无数据返回）
    pub fn with_affected(affected_rows: u64) -> Self {
        Self {
            columns: vec![],
            rows: vec![],
            message: String::new(),
            affected_rows,
        }
    }

    /// 空结果
    pub fn empty() -> Self {
        Self::default()
    }

    /// 是否有数据
    pub fn has_data(&self) -> bool {
        !self.columns.is_empty()
    }

    /// 行数
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

// ============================================================================
// 连接状态
// ============================================================================

/// 单个数据库连接状态
#[derive(Default)]
pub struct Connection {
    pub config: ConnectionConfig,
    pub connected: bool,
    /// 可用的数据库列表（MySQL/PostgreSQL）
    pub databases: Vec<String>,
    /// 当前选中的数据库
    pub selected_database: Option<String>,
    /// 当前数据库的表列表
    pub tables: Vec<String>,
    pub error: Option<String>,
}

impl Connection {
    /// 创建新连接
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// 重置连接状态
    pub fn reset(&mut self) {
        self.connected = false;
        self.databases.clear();
        self.selected_database = None;
        self.tables.clear();
        self.error = None;
    }

    /// 设置连接成功（带数据库列表）
    pub fn set_connected_with_databases(&mut self, databases: Vec<String>) {
        self.connected = true;
        self.databases = databases;
        self.tables.clear();
        self.error = None;
    }

    /// 设置连接成功（SQLite 模式，直接设置表）
    pub fn set_connected(&mut self, tables: Vec<String>) {
        self.connected = true;
        self.databases.clear();
        self.selected_database = None;
        self.tables = tables;
        self.error = None;
    }

    /// 设置选中的数据库及其表列表
    pub fn set_database(&mut self, database: String, tables: Vec<String>) {
        self.selected_database = Some(database);
        self.tables = tables;
    }

    /// 设置连接失败
    pub fn set_error(&mut self, error: String) {
        self.connected = false;
        self.databases.clear();
        self.selected_database = None;
        self.tables.clear();
        self.error = Some(error);
    }
}

// ============================================================================
// 连接管理器
// ============================================================================

/// 管理多个数据库连接
#[derive(Default)]
pub struct ConnectionManager {
    pub connections: HashMap<String, Connection>,
    pub active: Option<String>,
}

#[allow(dead_code)]
impl ConnectionManager {
    /// 添加新连接配置
    pub fn add(&mut self, config: ConnectionConfig) {
        let name = config.name.clone();
        self.connections.insert(name, Connection::new(config));
    }

    /// 移除连接
    pub fn remove(&mut self, name: &str) -> Option<Connection> {
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        self.connections.remove(name)
    }

    /// 获取当前活动连接
    pub fn get_active(&self) -> Option<&Connection> {
        self.active
            .as_ref()
            .and_then(|name| self.connections.get(name))
    }

    /// 获取当前活动连接（可变）
    pub fn get_active_mut(&mut self) -> Option<&mut Connection> {
        let name = self.active.clone()?;
        self.connections.get_mut(&name)
    }

    /// 断开指定连接
    pub fn disconnect(&mut self, name: &str) {
        if let Some(conn) = self.connections.get_mut(name) {
            conn.reset();
        }
    }

    /// 处理连接结果
    pub fn handle_connect_result(&mut self, name: &str, result: Result<Vec<String>, String>) {
        if let Some(conn) = self.connections.get_mut(name) {
            match result {
                Ok(tables) => conn.set_connected(tables),
                Err(e) => {
                    conn.set_error(e);
                    if self.active.as_deref() == Some(name) {
                        self.active = None;
                    }
                }
            }
        }
    }

    /// 连接数量
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}

// ============================================================================
// 连接池管理
// ============================================================================

/// 全局连接池管理器
///
/// 使用 lazy_static 模式实现单例，避免每次查询都创建新连接
pub struct PoolManager {
    /// MySQL 连接池缓存
    mysql_pools: RwLock<HashMap<String, mysql_async::Pool>>,
    /// PostgreSQL 客户端缓存（tokio-postgres 使用长连接）
    pg_clients: RwLock<HashMap<String, Arc<tokio_postgres::Client>>>,
}

impl PoolManager {
    /// 创建新的连接池管理器
    pub fn new() -> Self {
        Self {
            mysql_pools: RwLock::new(HashMap::new()),
            pg_clients: RwLock::new(HashMap::new()),
        }
    }

    /// 获取或创建 MySQL 连接池
    pub async fn get_mysql_pool(
        &self,
        config: &ConnectionConfig,
    ) -> Result<mysql_async::Pool, DbError> {
        let key = config.pool_key();

        // 先尝试读取缓存
        {
            let pools = self.mysql_pools.read().await;
            if let Some(pool) = pools.get(&key) {
                return Ok(pool.clone());
            }
        }

        // 创建新连接池
        let pool = mysql_async::Pool::new(config.connection_string().as_str());

        // 测试连接
        let _conn = pool.get_conn().await.map_err(|e| {
            DbError::Connection(format!("MySQL 连接失败: {}", e))
        })?;

        // 存入缓存
        {
            let mut pools = self.mysql_pools.write().await;
            pools.insert(key, pool.clone());
        }

        Ok(pool)
    }

    /// 获取或创建 PostgreSQL 客户端
    pub async fn get_pg_client(
        &self,
        config: &ConnectionConfig,
    ) -> Result<Arc<tokio_postgres::Client>, DbError> {
        let key = config.pool_key();

        // 先尝试读取缓存
        {
            let clients = self.pg_clients.read().await;
            if let Some(client) = clients.get(&key) {
                // 检查连接是否仍然有效
                if !client.is_closed() {
                    return Ok(client.clone());
                }
            }
        }

        // 创建新连接
        let (client, conn) =
            tokio_postgres::connect(&config.connection_string(), tokio_postgres::NoTls)
                .await
                .map_err(|e| DbError::Connection(format!("PostgreSQL 连接失败: {}", e)))?;

        // 在后台处理连接
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("PostgreSQL 连接错误: {}", e);
            }
        });

        let client = Arc::new(client);

        // 存入缓存
        {
            let mut clients = self.pg_clients.write().await;
            clients.insert(key, client.clone());
        }

        Ok(client)
    }

    /// 清除指定配置的连接池
    #[allow(dead_code)]
    pub async fn remove_pool(&self, config: &ConnectionConfig) {
        let key = config.pool_key();

        match config.db_type {
            DatabaseType::MySQL => {
                let mut pools = self.mysql_pools.write().await;
                if let Some(pool) = pools.remove(&key) {
                    // 断开连接池
                    pool.disconnect().await.ok();
                }
            }
            DatabaseType::PostgreSQL => {
                let mut clients = self.pg_clients.write().await;
                clients.remove(&key);
            }
            DatabaseType::SQLite => {
                // SQLite 不需要连接池
            }
        }
    }

    /// 清除所有连接池
    #[allow(dead_code)]
    pub async fn clear_all(&self) {
        {
            let mut pools = self.mysql_pools.write().await;
            for (_, pool) in pools.drain() {
                pool.disconnect().await.ok();
            }
        }
        {
            let mut clients = self.pg_clients.write().await;
            clients.clear();
        }
    }
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

// 全局连接池实例
lazy_static::lazy_static! {
    pub static ref POOL_MANAGER: PoolManager = PoolManager::new();
}

// ============================================================================
// 查询模块
// ============================================================================

mod query;

pub use query::{connect_database, execute_query, get_tables_for_database, ConnectResult};
