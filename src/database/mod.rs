//! 数据库模块 - 连接管理、查询执行
//!
//! 支持 SQLite、PostgreSQL、MySQL 三种数据库，使用连接池优化性能。

use crate::core::constants;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;

// ============================================================================
// 错误类型
// ============================================================================

/// 数据库操作错误
#[allow(dead_code)] // 公开 API，供外部使用
#[derive(Error, Debug)]
pub enum DbError {
    #[error("连接错误: {0}")]
    Connection(String),
    #[error("连接错误 [{db_type}]: {message}")]
    ConnectionTyped {
        db_type: String,
        message: String,
    },
    #[error("查询错误: {0}")]
    Query(String),
    #[error("查询错误 [{db_type}]: {message}\nSQL: {sql}")]
    QueryWithContext {
        db_type: String,
        message: String,
        sql: String,
    },
    #[error("连接池错误: {0}")]
    Pool(String),
}

#[allow(dead_code)] // 公开 API，供外部使用
impl DbError {
    /// 创建带数据库类型的连接错误
    pub fn connection_typed(db_type: &DatabaseType, message: impl Into<String>) -> Self {
        Self::ConnectionTyped {
            db_type: db_type.display_name().to_string(),
            message: message.into(),
        }
    }

    /// 创建带上下文的查询错误
    pub fn query_with_context(db_type: &DatabaseType, message: impl Into<String>, sql: &str) -> Self {
        // 截断 SQL 以避免日志过长
        let sql_preview = if sql.len() > 200 {
            format!("{}...", &sql[..200])
        } else {
            sql.to_string()
        };
        Self::QueryWithContext {
            db_type: db_type.display_name().to_string(),
            message: message.into(),
            sql: sql_preview,
        }
    }
}

// ============================================================================
// URL 编码辅助函数
// ============================================================================

/// 对字符串进行 URL 编码，用于 MySQL 连接字符串
/// 
/// 处理特殊字符如 #、@、:、/ 等，确保连接字符串正确解析
fn url_encode(s: &str) -> String {
    let mut result = String::with_capacity(s.len() * 3);
    for c in s.chars() {
        match c {
            // 安全字符，不需要编码
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => {
                result.push(c);
            }
            // 特殊字符需要编码
            _ => {
                for byte in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
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
    #[allow(dead_code)] // 公开 API，供外部使用
    pub const fn requires_network(&self) -> bool {
        !matches!(self, Self::SQLite)
    }
}

// ============================================================================
// 连接配置
// ============================================================================

/// MySQL SSL 模式
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub enum MySqlSslMode {
    /// 禁用 SSL（默认）
    #[default]
    Disabled,
    /// 优先使用 SSL，但允许不安全连接
    Preferred,
    /// 必须使用 SSL
    Required,
    /// 验证 CA 证书
    VerifyCa,
    /// 验证 CA 证书和主机名
    VerifyIdentity,
}

impl MySqlSslMode {
    /// 获取显示名称
    pub const fn display_name(&self) -> &'static str {
        match self {
            Self::Disabled => "禁用",
            Self::Preferred => "优先",
            Self::Required => "必需",
            Self::VerifyCa => "验证 CA",
            Self::VerifyIdentity => "完全验证",
        }
    }

    /// 获取描述
    pub const fn description(&self) -> &'static str {
        match self {
            Self::Disabled => "不使用 SSL 加密",
            Self::Preferred => "优先 SSL，允许不安全连接",
            Self::Required => "必须使用 SSL 加密",
            Self::VerifyCa => "验证服务器 CA 证书",
            Self::VerifyIdentity => "验证证书和主机名",
        }
    }

    /// 获取所有选项
    pub const fn all() -> &'static [MySqlSslMode] {
        &[
            Self::Disabled,
            Self::Preferred,
            Self::Required,
            Self::VerifyCa,
            Self::VerifyIdentity,
        ]
    }
}

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
    /// SSH 隧道配置
    #[serde(default)]
    pub ssh_config: ssh_tunnel::SshTunnelConfig,
    /// MySQL SSL 模式
    #[serde(default)]
    pub mysql_ssl_mode: MySqlSslMode,
    /// CA 证书路径（可选，用于 VerifyCa/VerifyIdentity 模式）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ssl_ca_cert: String,
}

/// 获取机器特定的加密密钥
/// 使用 hostname 作为密钥派生的基础，确保配置文件在不同机器上不可直接读取
/// 同时保证用户迁移目录后仍能解密
fn get_machine_key() -> [u8; 32] {
    use ring::digest::{digest, SHA256};
    
    // 使用机器标识信息来派生密钥（更稳定，不受用户目录变化影响）
    let mut key_material = String::new();
    
    // 使用 hostname（跨平台，更稳定）
    if let Ok(hostname) = hostname::get() {
        key_material.push_str(&hostname.to_string_lossy());
    }
    
    // 备用：使用用户名（如果 hostname 获取失败）
    if key_material.is_empty() {
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            key_material.push_str(&user);
        }
    }
    
    // 添加固定盐值（带版本号，便于未来升级）
    key_material.push_str("rust-db-manager-v2");
    
    let hash = digest(&SHA256, key_material.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_ref());
    key
}

/// 获取旧版机器密钥（使用用户目录路径，用于向后兼容）
fn get_legacy_machine_key() -> [u8; 32] {
    use ring::digest::{digest, SHA256};
    
    let mut key_material = String::new();
    
    // 旧版使用配置目录路径
    if let Some(config_dir) = dirs::config_dir() {
        key_material.push_str(&config_dir.to_string_lossy());
    }
    
    // 备用：使用用户名
    if key_material.is_empty() {
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            key_material.push_str(&user);
        }
    }
    
    // 旧版盐值
    key_material.push_str("rust-db-manager-v2");
    
    let hash = digest(&SHA256, key_material.as_bytes());
    let mut key = [0u8; 32];
    key.copy_from_slice(hash.as_ref());
    key
}

/// 使用 AES-GCM 加密密码
fn encrypt_password(password: &str) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    use ring::rand::{SecureRandom, SystemRandom};
    
    if password.is_empty() {
        return Ok(String::new());
    }
    
    let key_bytes = get_machine_key();
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| "Failed to create encryption key")?;
    let key = LessSafeKey::new(unbound_key);
    
    // 生成随机 nonce
    let rng = SystemRandom::new();
    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes)
        .map_err(|_| "Failed to generate nonce")?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    
    // 加密
    let mut in_out = password.as_bytes().to_vec();
    key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "Encryption failed")?;
    
    // 将 nonce 和密文组合后 base64 编码
    let mut result = nonce_bytes.to_vec();
    result.extend(in_out);
    
    // 添加版本前缀以区分加密格式
    Ok(format!("v1:{}", STANDARD.encode(&result)))
}

/// 使用指定密钥尝试解密
fn try_decrypt_with_key(combined: &[u8], key_bytes: [u8; 32]) -> Result<String, String> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    
    if combined.len() < 12 + 16 {
        return Err("Invalid encrypted data".to_string());
    }
    
    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let mut nonce_arr = [0u8; 12];
    nonce_arr.copy_from_slice(nonce_bytes);
    let nonce = Nonce::assume_unique_for_key(nonce_arr);
    
    let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
        .map_err(|_| "Failed to create decryption key")?;
    let key = LessSafeKey::new(unbound_key);
    
    let mut in_out = ciphertext.to_vec();
    let plaintext = key.open_in_place(nonce, Aad::empty(), &mut in_out)
        .map_err(|_| "Decryption failed")?;
    
    String::from_utf8(plaintext.to_vec())
        .map_err(|_| "Invalid UTF-8 in decrypted password".to_string())
}

/// 解密密码
fn decrypt_password(encrypted: &str) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    
    if encrypted.is_empty() {
        return Ok(String::new());
    }
    
    // 检查版本前缀
    if let Some(data) = encrypted.strip_prefix("v1:") {
        // 新版加密格式
        let combined = STANDARD.decode(data)
            .map_err(|_| "Invalid base64 encoding")?;
        
        // 首先尝试使用新密钥（hostname）解密
        if let Ok(password) = try_decrypt_with_key(&combined, get_machine_key()) {
            return Ok(password);
        }
        
        // 如果失败，尝试使用旧密钥（用户目录路径）解密
        if let Ok(password) = try_decrypt_with_key(&combined, get_legacy_machine_key()) {
            // 使用旧密钥解密成功，密码将在下次保存时用新密钥重新加密
            return Ok(password);
        }
        
        Err("Decryption failed - password may have been encrypted on different machine".to_string())
    } else {
        // 尝试旧版 base64 格式（向后兼容）
        match STANDARD.decode(encrypted) {
            Ok(bytes) => String::from_utf8(bytes)
                .map_err(|_| "Invalid UTF-8 in password".to_string()),
            Err(_) => {
                // 可能是非常老的明文密码
                Ok(encrypted.to_string())
            }
        }
    }
}

/// 将密码加密后存储
fn encode_password<S>(password: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::Error;
    
    if password.is_empty() {
        serializer.serialize_str("")
    } else {
        let encrypted = encrypt_password(password)
            .map_err(S::Error::custom)?;
        serializer.serialize_str(&encrypted)
    }
}

/// 解密密码
fn decode_password<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;

    let s: String = String::deserialize(deserializer)?;
    if s.is_empty() {
        return Ok(String::new());
    }

    decrypt_password(&s)
        .map_err(D::Error::custom)
}

#[allow(dead_code)] // 公开 API，供外部使用
impl ConnectionConfig {
    /// 创建新的连接配置
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
                // URL 编码用户名和密码，处理特殊字符（如 #、@、: 等）
                let encoded_user = url_encode(&self.username);
                let encoded_pass = url_encode(&self.password);
                if let Some(db) = database.filter(|s| !s.is_empty()) {
                    format!(
                        "mysql://{}:{}@{}:{}/{}",
                        encoded_user, encoded_pass, self.host, self.port, db
                    )
                } else {
                    format!(
                        "mysql://{}:{}@{}:{}",
                        encoded_user, encoded_pass, self.host, self.port
                    )
                }
            }
        }
    }

    /// 生成唯一的连接标识符（用于连接池缓存，按用户+主机+数据库区分）
    pub fn pool_key(&self) -> String {
        match self.db_type {
            DatabaseType::SQLite => format!("sqlite:{}", self.database),
            DatabaseType::PostgreSQL => {
                // 包含数据库名，确保不同数据库使用不同连接
                format!("pg:{}:{}:{}:{}", self.host, self.port, self.username, self.database)
            }
            DatabaseType::MySQL => {
                // 包含数据库名，确保不同数据库使用不同连接
                format!("mysql:{}:{}:{}:{}", self.host, self.port, self.username, self.database)
            }
        }
    }

    /// 生成安全的连接字符串描述（密码遮蔽，用于日志）
    pub fn connection_string_masked(&self) -> String {
        match self.db_type {
            DatabaseType::SQLite => format!("sqlite://{}", self.database),
            DatabaseType::PostgreSQL => {
                format!(
                    "postgres://{}:****@{}:{}/{}",
                    self.username, self.host, self.port, self.database
                )
            }
            DatabaseType::MySQL => {
                format!(
                    "mysql://{}:****@{}:{}/{}",
                    self.username, self.host, self.port, self.database
                )
            }
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
    pub affected_rows: u64,
    /// 是否被截断（原始结果超过限制）
    pub truncated: bool,
    /// 原始总行数（如果被截断）
    pub original_row_count: Option<usize>,
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
        self.selected_database = Some(database.clone());
        self.config.database = database;
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

impl ConnectionManager {
    /// 添加新连接配置
    pub fn add(&mut self, config: ConnectionConfig) {
        let name = config.name.clone();
        self.connections.insert(name, Connection::new(config));
    }

    /// 移除连接
    #[allow(dead_code)] // 公开 API，供外部使用
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
    #[allow(dead_code)] // 公开 API，供外部使用
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
    #[allow(dead_code)] // 公开 API，供外部使用
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
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// 是否为空
    #[allow(dead_code)] // 公开 API，供外部使用
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

        // 先尝试读取缓存并验证连接池健康
        {
            let pools = self.mysql_pools.read().await;
            if let Some(pool) = pools.get(&key) {
                // 尝试获取连接以验证连接池是否健康
                match pool.get_conn().await {
                    Ok(_) => return Ok(pool.clone()),
                    Err(_) => {
                        // 连接池不健康，稍后会重新创建
                    }
                }
            }
        }

        // 移除失效的连接池
        {
            let mut pools = self.mysql_pools.write().await;
            pools.remove(&key);
        }

        // 创建新连接池，使用常量配置连接池参数
        let pool_opts = mysql_async::PoolOpts::default()
            .with_constraints(
                mysql_async::PoolConstraints::new(
                    constants::database::pool::MYSQL_POOL_MIN_CONNECTIONS,
                    constants::database::pool::MYSQL_POOL_MAX_CONNECTIONS,
                ).expect("连接池约束无效")
            );
        
        let mut opts = mysql_async::OptsBuilder::from_opts(
            mysql_async::Opts::from_url(config.connection_string().as_str())
                .map_err(|e| DbError::Connection(format!("MySQL URL 解析失败: {}", e)))?
        ).pool_opts(pool_opts);
        
        // 配置 SSL 选项
        opts = Self::configure_mysql_ssl(opts, config)?;
        
        let pool = mysql_async::Pool::new(opts);

        // 测试连接
        let _conn = pool.get_conn().await.map_err(|e| {
            DbError::Connection(format!("MySQL 连接失败: {}", e))
        })?;

        // 存入缓存（限制缓存数量，防止内存溢出）
        {
            let mut pools = self.mysql_pools.write().await;
            
            // 如果缓存已满，移除最早的连接池
            if pools.len() >= constants::database::pool::MAX_MYSQL_POOLS {
                // 移除第一个键（HashMap 无序，但这里只是简单清理）
                if let Some(oldest_key) = pools.keys().next().cloned() {
                    pools.remove(&oldest_key);
                }
            }
            
            pools.insert(key, pool.clone());
        }

        Ok(pool)
    }
    
    /// 配置 MySQL SSL 选项
    fn configure_mysql_ssl(
        opts: mysql_async::OptsBuilder,
        config: &ConnectionConfig,
    ) -> Result<mysql_async::OptsBuilder, DbError> {
        use mysql_async::SslOpts;
        use std::path::Path;
        
        match config.mysql_ssl_mode {
            MySqlSslMode::Disabled => {
                // 不使用 SSL
                Ok(opts.ssl_opts(None::<SslOpts>))
            }
            MySqlSslMode::Preferred => {
                // 优先 SSL，但接受无效证书（允许回退到不安全连接）
                let ssl_opts = SslOpts::default()
                    .with_danger_accept_invalid_certs(true)
                    .with_danger_skip_domain_validation(true);
                Ok(opts.ssl_opts(Some(ssl_opts)))
            }
            MySqlSslMode::Required => {
                // 必须使用 SSL，但不验证证书
                let ssl_opts = SslOpts::default()
                    .with_danger_accept_invalid_certs(true)
                    .with_danger_skip_domain_validation(true);
                Ok(opts.ssl_opts(Some(ssl_opts)))
            }
            MySqlSslMode::VerifyCa => {
                // 验证 CA 证书，但不验证主机名
                let mut ssl_opts = SslOpts::default()
                    .with_danger_skip_domain_validation(true);
                
                // 如果指定了 CA 证书路径
                if !config.ssl_ca_cert.is_empty() {
                    let ca_path = Path::new(&config.ssl_ca_cert);
                    if !ca_path.exists() {
                        return Err(DbError::Connection(format!(
                            "CA 证书文件不存在: {}", config.ssl_ca_cert
                        )));
                    }
                    // 使用 PathBuf 拥有路径所有权
                    ssl_opts = ssl_opts.with_root_certs(vec![ca_path.to_path_buf().into()]);
                }
                
                Ok(opts.ssl_opts(Some(ssl_opts)))
            }
            MySqlSslMode::VerifyIdentity => {
                // 完全验证：验证 CA 证书和主机名
                let mut ssl_opts = SslOpts::default();
                
                // 如果指定了 CA 证书路径
                if !config.ssl_ca_cert.is_empty() {
                    let ca_path = Path::new(&config.ssl_ca_cert);
                    if !ca_path.exists() {
                        return Err(DbError::Connection(format!(
                            "CA 证书文件不存在: {}", config.ssl_ca_cert
                        )));
                    }
                    // 使用 PathBuf 拥有路径所有权
                    ssl_opts = ssl_opts.with_root_certs(vec![ca_path.to_path_buf().into()]);
                }
                
                Ok(opts.ssl_opts(Some(ssl_opts)))
            }
        }
    }

    /// 获取或创建 PostgreSQL 客户端
    pub async fn get_pg_client(
        &self,
        config: &ConnectionConfig,
    ) -> Result<Arc<tokio_postgres::Client>, DbError> {
        let key = config.pool_key();

        // 先尝试读取缓存并验证连接健康
        {
            let clients = self.pg_clients.read().await;
            if let Some(client) = clients.get(&key) {
                // 检查连接是否仍然有效
                if !client.is_closed() {
                    return Ok(client.clone());
                }
            }
        }

        // 移除失效的连接
        {
            let mut clients = self.pg_clients.write().await;
            clients.remove(&key);
        }

        // 创建新连接
        let (client, conn) =
            tokio_postgres::connect(&config.connection_string(), tokio_postgres::NoTls)
                .await
                .map_err(|e| DbError::Connection(format!("PostgreSQL 连接失败: {}", e)))?;

        // 在后台处理连接（tokio_postgres 要求）
        // 连接任务会在客户端关闭或出错时自动终止
        let conn_key = key.clone();
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                eprintln!("[warn] PostgreSQL 连接 '{}' 错误: {}", conn_key, e);
            }
        });

        let client = Arc::new(client);

        // 存入缓存（限制缓存数量，防止内存溢出）
        {
            let mut clients = self.pg_clients.write().await;
            
            // 如果缓存已满，移除最早的客户端
            if clients.len() >= constants::database::pool::MAX_POSTGRES_CLIENTS {
                if let Some(oldest_key) = clients.keys().next().cloned() {
                    clients.remove(&oldest_key);
                }
            }
            
            clients.insert(key, client.clone());
        }

        Ok(client)
    }

    /// 清除指定配置的连接池
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

mod driver;
mod query;
pub mod ssh_tunnel;

#[allow(unused_imports)] // get_primary_key_column 预留供将来使用
pub use query::{
    connect_database, execute_query, get_tables_for_database, get_primary_key_column, ConnectResult,
    // 触发器查询
    get_triggers, TriggerInfo,
    // 外键和列信息查询（用于 ER 图）
    get_foreign_keys, get_table_columns, ForeignKeyInfo, ColumnInfo,
};
#[allow(unused_imports)] // SshTunnelConfig 公开 API
pub use ssh_tunnel::{SshAuthMethod, SshTunnelConfig};
#[allow(unused_imports)] // 驱动抽象 API，供未来扩展使用
pub use driver::{
    ColumnMeta, ConnectResultType, DatabaseDriver, DriverCapabilities, DriverInfo,
    DriverRegistry, TableMeta,
};
