//! 连接池管理

use super::config::ConnectionConfig;
use super::error::DbError;
use super::types::{DatabaseType, MySqlSslMode, PostgresSslMode};
use crate::core::constants;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
                )
                .expect("连接池约束无效"),
            )
            // 空闲连接超时：超过此时间未使用的连接将被关闭
            .with_inactive_connection_ttl(std::time::Duration::from_secs(
                constants::database::pool::MYSQL_IDLE_TIMEOUT_SECS,
            ))
            // 连接最大生存时间：无论是否活跃，超过此时间的连接将被回收
            .with_abs_conn_ttl(Some(std::time::Duration::from_secs(
                constants::database::pool::MYSQL_MAX_LIFETIME_SECS,
            )));

        let mut opts = mysql_async::OptsBuilder::from_opts(
            mysql_async::Opts::from_url(config.connection_string().as_str())
                .map_err(|e| DbError::Connection(format!("MySQL URL 解析失败: {}", e)))?,
        )
        .pool_opts(pool_opts);

        // 配置 SSL 选项
        opts = Self::configure_mysql_ssl(opts, config)?;

        let pool = mysql_async::Pool::new(opts);

        // 测试连接
        let _conn = pool
            .get_conn()
            .await
            .map_err(|e| DbError::Connection(format!("MySQL 连接失败: {}", e)))?;

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
                let mut ssl_opts = SslOpts::default().with_danger_skip_domain_validation(true);

                // 如果指定了 CA 证书路径
                if !config.ssl_ca_cert.is_empty() {
                    let ca_path = Path::new(&config.ssl_ca_cert);
                    if !ca_path.exists() {
                        return Err(DbError::Connection(format!(
                            "CA 证书文件不存在: {}",
                            config.ssl_ca_cert
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
                            "CA 证书文件不存在: {}",
                            config.ssl_ca_cert
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

        // 创建新连接（根据 SSL 模式选择连接方式）
        let client = Self::connect_pg_with_ssl(config).await?;
        let client = Arc::new(client);

        // 存入缓存（限制缓存数量，防止内存溢出）
        {
            let mut clients = self.pg_clients.write().await;

            // 如果缓存已满，移除最早的客户端
            if clients.len() >= constants::database::pool::MAX_POSTGRES_CLIENTS
                && let Some(oldest_key) = clients.keys().next().cloned() {
                    clients.remove(&oldest_key);
                }

            clients.insert(key, client.clone());
        }

        Ok(client)
    }

    /// 根据 SSL 模式连接 PostgreSQL
    async fn connect_pg_with_ssl(
        config: &ConnectionConfig,
    ) -> Result<tokio_postgres::Client, DbError> {
        let conn_string = config.connection_string();

        match config.postgres_ssl_mode {
            PostgresSslMode::Disable => {
                // 不使用 TLS
                let (client, conn) = tokio_postgres::connect(&conn_string, tokio_postgres::NoTls)
                    .await
                    .map_err(|e| DbError::Connection(format!("PostgreSQL 连接失败: {}", e)))?;

                Self::spawn_pg_connection(conn, &config.pool_key());
                Ok(client)
            }
            PostgresSslMode::Prefer => {
                // 优先使用 TLS，失败则回退到非 TLS
                match Self::connect_pg_tls(config, true).await {
                    Ok(client) => Ok(client),
                    Err(_) => {
                        // TLS 失败，尝试非 TLS
                        let (client, conn) =
                            tokio_postgres::connect(&conn_string, tokio_postgres::NoTls)
                                .await
                                .map_err(|e| {
                                    DbError::Connection(format!("PostgreSQL 连接失败: {}", e))
                                })?;

                        Self::spawn_pg_connection(conn, &config.pool_key());
                        Ok(client)
                    }
                }
            }
            PostgresSslMode::Require => {
                // 必须使用 TLS，但不验证证书
                Self::connect_pg_tls(config, true).await
            }
            PostgresSslMode::VerifyCa | PostgresSslMode::VerifyFull => {
                // 验证证书
                Self::connect_pg_tls(config, false).await
            }
        }
    }

    /// 使用 TLS 连接 PostgreSQL
    async fn connect_pg_tls(
        config: &ConnectionConfig,
        accept_invalid_certs: bool,
    ) -> Result<tokio_postgres::Client, DbError> {
        use native_tls::TlsConnector as NativeTlsConnector;
        use postgres_native_tls::MakeTlsConnector;
        use std::path::Path;

        let mut builder = NativeTlsConnector::builder();

        // 配置证书验证
        if accept_invalid_certs {
            builder.danger_accept_invalid_certs(true);
            builder.danger_accept_invalid_hostnames(true);
        }

        // 如果指定了 CA 证书
        if !config.ssl_ca_cert.is_empty() {
            let ca_path = Path::new(&config.ssl_ca_cert);
            if !ca_path.exists() {
                return Err(DbError::Connection(format!(
                    "CA 证书文件不存在: {}",
                    config.ssl_ca_cert
                )));
            }

            let ca_data = std::fs::read(&config.ssl_ca_cert).map_err(|e| {
                DbError::Connection(format!("读取 CA 证书失败: {}", e))
            })?;

            let cert = native_tls::Certificate::from_pem(&ca_data).map_err(|e| {
                DbError::Connection(format!("解析 CA 证书失败: {}", e))
            })?;

            builder.add_root_certificate(cert);
        }

        // 验证主机名（仅 VerifyFull 模式）
        if config.postgres_ssl_mode != PostgresSslMode::VerifyFull {
            builder.danger_accept_invalid_hostnames(true);
        }

        let connector = builder.build().map_err(|e| {
            DbError::Connection(format!("TLS 连接器构建失败: {}", e))
        })?;

        let tls = MakeTlsConnector::new(connector);

        let (client, conn) = tokio_postgres::connect(&config.connection_string(), tls)
            .await
            .map_err(|e| DbError::Connection(format!("PostgreSQL TLS 连接失败: {}", e)))?;

        Self::spawn_pg_connection(conn, &config.pool_key());
        Ok(client)
    }

    /// 在后台处理 PostgreSQL 连接
    fn spawn_pg_connection<S, T>(conn: tokio_postgres::Connection<S, T>, key: &str)
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
        T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let conn_key = key.to_string();
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::warn!(connection = %conn_key, error = %e, "PostgreSQL 连接错误");
            }
        });
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
