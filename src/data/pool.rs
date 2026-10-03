//! 连接池管理

use super::config::ConnectionConfig;
use super::error::DbError;
use crate::core::constants;
use crate::types::{DatabaseType, MySqlSslMode, PostgresSslMode};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, RwLock};

/// 全局连接池管理器
///
/// 使用 LazyLock 实现单例，避免每次查询都创建新连接
pub struct PoolManager {
    /// MySQL 连接池缓存
    mysql_pools: RwLock<HashMap<String, (mysql_async::Pool, Instant)>>,
    /// PostgreSQL 客户端缓存；连接任务与最后一个借用者共同拥有生命周期。
    pg_clients: RwLock<HashMap<String, PgClientEntry>>,
}

/// An in-use client keeps its connection task alive even after cache eviction.
/// Dropping the last lease closes the backend without interrupting borrowed operations.
pub struct PgPooledClient {
    client: Mutex<tokio_postgres::Client>,
    connection_task: tokio::task::JoinHandle<()>,
}

impl std::ops::Deref for PgPooledClient {
    type Target = Mutex<tokio_postgres::Client>;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl Drop for PgPooledClient {
    fn drop(&mut self) {
        self.connection_task.abort();
    }
}

type PgClientEntry = (Arc<PgPooledClient>, Instant);
/// Dedicated SQL-tab backend. Dropping it closes the socket and rolls back open transactions.
pub(crate) struct PgDedicatedConnection {
    pub client: tokio_postgres::Client,
    connection_task: tokio::task::JoinHandle<()>,
}

impl Drop for PgDedicatedConnection {
    fn drop(&mut self) {
        self.connection_task.abort();
    }
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

        // Health checks can wait for a connection when this pool is saturated. Never hold
        // the cache lock across that await: unrelated databases still need their own pools.
        if let Some(pool) = self.get_healthy_mysql_pool(&key).await {
            return Ok(pool);
        }

        // 创建新连接池，使用常量配置连接池参数
        let constraints = mysql_async::PoolConstraints::new(
            constants::database::pool::MYSQL_POOL_MIN_CONNECTIONS,
            constants::database::pool::MYSQL_POOL_MAX_CONNECTIONS,
        )
        .ok_or_else(|| {
            DbError::Connection("MySQL 连接池配置无效：最小连接数不能大于最大连接数".to_string())
        })?;

        let pool_opts = mysql_async::PoolOpts::default()
            .with_constraints(constraints)
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
        let evicted_pool = {
            let mut pools = self.mysql_pools.write().await;

            // 如果缓存已满，移除最久未使用的连接池
            let mut evicted: Option<mysql_async::Pool> = None;
            if pools.len() >= constants::database::pool::MAX_MYSQL_POOLS
                && let Some(oldest_key) = pools
                    .iter()
                    .min_by_key(|(_, (_, last_used))| *last_used)
                    .map(|(key, _)| key.clone())
            {
                evicted = pools.remove(&oldest_key).map(|(pool, _)| pool);
            }

            pools.insert(key, (pool.clone(), Instant::now()));
            evicted
        };
        if let Some(pool) = evicted_pool {
            pool.disconnect().await.ok();
        }

        Ok(pool)
    }

    async fn get_healthy_mysql_pool(&self, key: &str) -> Option<mysql_async::Pool> {
        loop {
            let pool = self
                .mysql_pools
                .read()
                .await
                .get(key)
                .map(|(pool, _)| pool.clone())?;
            let is_healthy = pool.get_conn().await.is_ok();
            let mut pools = self.mysql_pools.write().await;
            if !pools
                .get(key)
                .is_some_and(|(current, _)| Arc::ptr_eq(&current.metrics(), &pool.metrics()))
            {
                continue;
            }
            if is_healthy {
                if let Some((_, last_used)) = pools.get_mut(key) {
                    *last_used = Instant::now();
                }
                return Some(pool);
            }
            let stale = pools.remove(key).map(|(pool, _)| pool);
            drop(pools);
            if let Some(stale) = stale
                && let Err(error) = stale.disconnect().await
            {
                tracing::warn!(%error, "disconnect unhealthy MySQL pool failed");
            }
            return None;
        }
    }

    /// 配置 MySQL SSL 选项
    pub(crate) fn configure_mysql_ssl(
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
            MySqlSslMode::Required => {
                // 必须使用 SSL，验证 CA 证书但不验证主机名
                let mut ssl_opts = SslOpts::default().with_danger_skip_domain_validation(true);
                if !config.ssl_ca_cert.is_empty() {
                    let ca_path = Path::new(&config.ssl_ca_cert);
                    if !ca_path.exists() {
                        return Err(DbError::Connection(format!(
                            "CA 证书文件不存在: {}",
                            config.ssl_ca_cert
                        )));
                    }
                    ssl_opts = ssl_opts.with_root_certs(vec![ca_path.to_path_buf().into()]);
                }
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
                if let Some(server_name) = config.tls_server_name.as_deref() {
                    ssl_opts =
                        ssl_opts.with_danger_tls_hostname_override(Some(server_name.to_owned()));
                }

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
    ) -> Result<Arc<PgPooledClient>, DbError> {
        let key = config.pool_key();

        if let Some(client) = self.get_healthy_pg_client(&key).await {
            return Ok(client);
        }

        // Connection establishment happens outside the cache lock. Concurrent cold misses
        // may connect twice, but only one backend becomes the shared cache entry.
        let (connection, connection_task) = Self::connect_pg_with_ssl(config).await?;
        let client = Arc::new(PgPooledClient {
            client: Mutex::new(connection),
            connection_task,
        });
        let mut clients = self.pg_clients.write().await;
        if let Some((existing, last_used)) = clients.get_mut(&key) {
            *last_used = Instant::now();
            return Ok(existing.clone());
        }

        // Removing a borrowed entry only detaches it from the cache; its lease
        // keeps the backend alive until the last borrower finishes.
        if clients.len() >= constants::database::pool::MAX_POSTGRES_CLIENTS
            && let Some(oldest_key) = clients
                .iter()
                .min_by_key(|(_, (_, last_used))| *last_used)
                .map(|(key, _)| key.clone())
        {
            clients.remove(&oldest_key);
        }
        // The cache stays bounded even when the LRU has active borrowers.
        clients.insert(key, (client.clone(), Instant::now()));
        Ok(client)
    }

    async fn get_healthy_pg_client(&self, key: &str) -> Option<Arc<PgPooledClient>> {
        loop {
            let client = self
                .pg_clients
                .read()
                .await
                .get(key)
                .map(|(client, _)| client.clone())?;
            let is_healthy = !client.lock().await.is_closed();
            let mut clients = self.pg_clients.write().await;
            if !clients
                .get(key)
                .is_some_and(|(current, _)| Arc::ptr_eq(current, &client))
            {
                continue;
            }
            if is_healthy {
                if let Some((_, last_used)) = clients.get_mut(key) {
                    *last_used = Instant::now();
                }
                return Some(client);
            }
            clients.remove(key);
            return None;
        }
    }

    /// Open a PostgreSQL backend outside the configuration-keyed metadata/grid pool.
    pub(crate) async fn connect_pg_dedicated(
        config: &ConnectionConfig,
    ) -> Result<PgDedicatedConnection, DbError> {
        let (client, connection_task) = Self::connect_pg_with_ssl(config).await?;
        Ok(PgDedicatedConnection {
            client,
            connection_task,
        })
    }

    /// 根据 SSL 模式连接 PostgreSQL。返回客户端及其后台连接任务句柄。
    async fn connect_pg_with_ssl(
        config: &ConnectionConfig,
    ) -> Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>), DbError> {
        match config.postgres_ssl_mode {
            PostgresSslMode::Disable => {
                Self::connect_pg_plain(config, tokio_postgres::config::SslMode::Disable).await
            }
            PostgresSslMode::Require | PostgresSslMode::VerifyCa | PostgresSslMode::VerifyFull => {
                Self::connect_pg_tls(config, tokio_postgres::config::SslMode::Require).await
            }
        }
    }

    async fn connect_pg_plain(
        config: &ConnectionConfig,
        ssl_mode: tokio_postgres::config::SslMode,
    ) -> Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>), DbError> {
        let pg_config = build_pg_connection_config(config, ssl_mode)?;
        let (client, conn) = pg_config
            .connect(tokio_postgres::NoTls)
            .await
            .map_err(|e| DbError::Connection(format!("PostgreSQL 连接失败: {}", e)))?;
        let handle = Self::spawn_pg_connection(conn, &config.pool_key());
        Ok((client, handle))
    }
    /// 使用 TLS 连接 PostgreSQL。返回客户端及其后台连接任务句柄。
    async fn connect_pg_tls(
        config: &ConnectionConfig,
        ssl_mode: tokio_postgres::config::SslMode,
    ) -> Result<(tokio_postgres::Client, tokio::task::JoinHandle<()>), DbError> {
        let pg_config = build_pg_connection_config(config, ssl_mode)?;
        let tls = build_pg_tls_connector(config)?;
        let (client, conn) = pg_config
            .connect(tls)
            .await
            .map_err(|e| DbError::Connection(format!("PostgreSQL TLS 连接失败: {}", e)))?;
        let handle = Self::spawn_pg_connection(conn, &config.pool_key());
        Ok((client, handle))
    }

    /// 在后台处理 PostgreSQL 连接，返回任务句柄以便断开时中止（修复审计 CONN-F7）。
    fn spawn_pg_connection<S, T>(
        conn: tokio_postgres::Connection<S, T>,
        key: &str,
    ) -> tokio::task::JoinHandle<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
        T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
    {
        let conn_key = key.to_string();
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::warn!(connection = %conn_key, error = %e, "PostgreSQL 连接错误");
            }
        })
    }

    /// 清除指定配置的连接池
    pub async fn remove_pool(&self, config: &ConnectionConfig) {
        let key = config.pool_key();

        match config.db_type {
            DatabaseType::MySQL => {
                let pool = self
                    .mysql_pools
                    .write()
                    .await
                    .remove(&key)
                    .map(|(pool, _)| pool);
                if let Some(pool) = pool
                    && let Err(error) = pool.disconnect().await
                {
                    tracing::warn!(%error, "disconnect removed MySQL pool failed");
                }
            }
            DatabaseType::PostgreSQL => {
                // Existing leases finish before their backend task is stopped.
                self.pg_clients.write().await.remove(&key);
            }
            DatabaseType::SQLite => {
                // SQLite 不需要连接池
            }
        }
    }

    /// 清除所有连接池
    pub async fn clear_all(&self) {
        let pools: Vec<_> = self
            .mysql_pools
            .write()
            .await
            .drain()
            .map(|(_, (pool, _))| pool)
            .collect();
        for pool in pools {
            if let Err(error) = pool.disconnect().await {
                tracing::warn!(%error, "disconnect cleared MySQL pool failed");
            }
        }
        // Disconnect idle backends now; active leases close on their final drop.
        self.pg_clients.write().await.clear();
    }
}

fn build_pg_connection_config(
    config: &ConnectionConfig,
    ssl_mode: tokio_postgres::config::SslMode,
) -> Result<tokio_postgres::Config, DbError> {
    let mut pg_config = config
        .connection_string()
        .parse::<tokio_postgres::Config>()
        .map_err(|e| DbError::Connection(format!("PostgreSQL URL 解析失败: {}", e)))?;
    pg_config.ssl_mode(ssl_mode);
    Ok(pg_config)
}

#[derive(Debug)]
struct CaOnlyServerCertVerifier {
    verifier: Arc<rustls::client::WebPkiServerVerifier>,
}

impl rustls::client::danger::ServerCertVerifier for CaOnlyServerCertVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &rustls::pki_types::CertificateDer<'_>,
        intermediates: &[rustls::pki_types::CertificateDer<'_>],
        server_name: &rustls::pki_types::ServerName<'_>,
        ocsp_response: &[u8],
        now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let result = rustls::client::danger::ServerCertVerifier::verify_server_cert(
            self.verifier.as_ref(),
            end_entity,
            intermediates,
            server_name,
            ocsp_response,
            now,
        );
        match result {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::NotValidForName
                | rustls::CertificateError::NotValidForNameContext { .. },
            )) => Ok(rustls::client::danger::ServerCertVerified::assertion()),
            result => result,
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::client::danger::ServerCertVerifier::verify_tls12_signature(
            self.verifier.as_ref(),
            message,
            cert,
            dss,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &rustls::pki_types::CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::client::danger::ServerCertVerifier::verify_tls13_signature(
            self.verifier.as_ref(),
            message,
            cert,
            dss,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::client::danger::ServerCertVerifier::supported_verify_schemes(self.verifier.as_ref())
    }
}

fn load_pg_root_store(config: &ConnectionConfig) -> Result<rustls::RootCertStore, DbError> {
    use std::path::Path;

    if config.ssl_ca_cert.is_empty() {
        return Ok(rustls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        ));
    }

    let ca_path = Path::new(&config.ssl_ca_cert);
    if !ca_path.exists() {
        return Err(DbError::Connection(format!(
            "CA 证书文件不存在: {}",
            config.ssl_ca_cert
        )));
    }
    let ca_data = std::fs::read(&config.ssl_ca_cert)
        .map_err(|e| DbError::Connection(format!("读取 CA 证书失败: {}", e)))?;
    let certs = rustls_pemfile::certs(&mut ca_data.as_slice())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DbError::Connection(format!("解析 CA 证书失败: {}", e)))?;
    let mut root_store = rustls::RootCertStore::empty();
    for cert in certs {
        root_store
            .add(cert)
            .map_err(|e| DbError::Connection(format!("添加 CA 证书失败: {}", e)))?;
    }
    Ok(root_store)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PgTlsVerifierMode {
    CaOnly,
    Full,
}

const fn pg_tls_verifier_mode(mode: PostgresSslMode) -> PgTlsVerifierMode {
    match mode {
        PostgresSslMode::Require | PostgresSslMode::VerifyCa => PgTlsVerifierMode::CaOnly,
        PostgresSslMode::Disable | PostgresSslMode::VerifyFull => PgTlsVerifierMode::Full,
    }
}

pub(crate) fn build_pg_tls_connector(
    config: &ConnectionConfig,
) -> Result<tokio_postgres_rustls::MakeRustlsConnect, DbError> {
    let root_store = load_pg_root_store(config)?;
    let tls_config = match pg_tls_verifier_mode(config.postgres_ssl_mode) {
        PgTlsVerifierMode::CaOnly => {
            let verifier = rustls::client::WebPkiServerVerifier::builder(Arc::new(root_store))
                .build()
                .map_err(|e| {
                    DbError::Connection(format!("构建 PostgreSQL CA 验证器失败: {}", e))
                })?;
            rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(Arc::new(CaOnlyServerCertVerifier { verifier }))
                .with_no_client_auth()
        }
        PgTlsVerifierMode::Full => rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth(),
    };

    Ok(tokio_postgres_rustls::MakeRustlsConnect::new(tls_config))
}

impl Default for PoolManager {
    fn default() -> Self {
        Self::new()
    }
}

// 全局连接池实例
pub static POOL_MANAGER: std::sync::LazyLock<PoolManager> =
    std::sync::LazyLock::new(PoolManager::new);
#[cfg(test)]
mod tests {
    use super::{
        PgTlsVerifierMode, PoolManager, build_pg_connection_config, build_pg_tls_connector,
        pg_tls_verifier_mode,
    };
    use crate::data::ConnectionConfig;
    use crate::types::{DatabaseType, MySqlSslMode, PostgresSslMode};
    use std::net::IpAddr;
    use tokio_postgres::config::Host;
    const TEST_TUNNEL_PORT: u16 = 15_432;
    const TEST_ALTERNATE_TUNNEL_PORT: u16 = 15_433;

    fn configured_mysql_ssl_opts(config: &ConnectionConfig) -> Option<mysql_async::SslOpts> {
        let builder = PoolManager::configure_mysql_ssl(mysql_async::OptsBuilder::default(), config)
            .expect("SSL options should be valid");
        mysql_async::Opts::from(builder).ssl_opts().cloned()
    }

    #[test]
    fn ssh_pool_key_reuses_tunnel_but_separates_tls_server_names() {
        for db_type in [DatabaseType::PostgreSQL, DatabaseType::MySQL] {
            let mut first = ConnectionConfig::new("test", db_type);
            first.host = "127.0.0.1".to_string();
            first.port = TEST_TUNNEL_PORT;
            first.tls_server_name = Some("db.internal".to_string());
            first.ssh_config.enabled = true;
            first.ssh_config.ssh_host = "jump.internal".to_string();
            first.ssh_config.ssh_port = 22;
            first.ssh_config.ssh_username = "ssh-user".to_string();
            first.ssh_config.remote_host = "db.internal".to_string();
            first.ssh_config.remote_port = db_type.default_port();

            let mut same_tunnel = first.clone();
            same_tunnel.port = TEST_ALTERNATE_TUNNEL_PORT;
            assert_eq!(
                first.pool_key(),
                same_tunnel.pool_key(),
                "同一 SSH 隧道的本地端口变化不应改变连接池身份"
            );

            let mut different_tls_name = first.clone();
            different_tls_name.tls_server_name = Some("db.alias.internal".to_string());
            assert_ne!(
                first.pool_key(),
                different_tls_name.pool_key(),
                "同一 SSH 隧道下不同 TLS server name 不应共享连接池"
            );
        }
    }

    #[test]
    fn mysql_default_tls_verifies_certificate_and_hostname() {
        let config = ConnectionConfig::new("test", DatabaseType::MySQL);
        let ssl_opts = configured_mysql_ssl_opts(&config).expect("TLS must be enabled");

        assert!(!ssl_opts.accept_invalid_certs());
        assert!(!ssl_opts.skip_domain_validation());
    }

    #[test]
    fn mysql_verify_identity_uses_original_hostname_through_tunnel() {
        let mut config = ConnectionConfig::new("test", DatabaseType::MySQL);
        config.host = "127.0.0.1".to_string();
        config.tls_server_name = Some("db.example.com".to_string());

        let ssl_opts = configured_mysql_ssl_opts(&config).expect("TLS must be enabled");

        assert_eq!(ssl_opts.tls_hostname_override(), Some("db.example.com"));
        assert!(!ssl_opts.skip_domain_validation());
    }

    #[test]
    fn mysql_verify_ca_keeps_hostname_validation_disabled_through_tunnel() {
        let mut config = ConnectionConfig::new("test", DatabaseType::MySQL);
        config.host = "127.0.0.1".to_string();
        config.tls_server_name = Some("db.example.com".to_string());
        config.mysql_ssl_mode = MySqlSslMode::VerifyCa;

        let ssl_opts = configured_mysql_ssl_opts(&config).expect("TLS must be enabled");

        assert!(ssl_opts.tls_hostname_override().is_none());
        assert!(ssl_opts.skip_domain_validation());
    }

    #[test]
    fn mysql_required_verifies_ca_without_hostname_validation_through_tunnel() {
        let mut config = ConnectionConfig::new("test", DatabaseType::MySQL);
        config.host = "127.0.0.1".to_string();
        config.tls_server_name = Some("db.example.com".to_string());
        config.mysql_ssl_mode = MySqlSslMode::Required;

        let ssl_opts = configured_mysql_ssl_opts(&config).expect("TLS must be enabled");

        assert!(!ssl_opts.accept_invalid_certs());
        assert!(ssl_opts.skip_domain_validation());
        assert!(ssl_opts.tls_hostname_override().is_none());
    }

    #[test]
    fn postgres_require_uses_ca_only_verifier() {
        assert_eq!(
            pg_tls_verifier_mode(PostgresSslMode::Require),
            PgTlsVerifierMode::CaOnly
        );
        assert_eq!(
            pg_tls_verifier_mode(PostgresSslMode::VerifyCa),
            PgTlsVerifierMode::CaOnly
        );
        assert_eq!(
            pg_tls_verifier_mode(PostgresSslMode::VerifyFull),
            PgTlsVerifierMode::Full
        );
    }

    #[test]
    fn postgres_tunneled_endpoint_uses_loopback_tcp_and_original_tls_host() {
        let mut config = ConnectionConfig::new("test", DatabaseType::PostgreSQL);
        config.host = "127.0.0.1".to_string();
        config.port = TEST_TUNNEL_PORT;
        config.tls_server_name = Some("db.example.com".to_string());

        let pg_config =
            build_pg_connection_config(&config, tokio_postgres::config::SslMode::Require)
                .expect("PostgreSQL config should parse");

        assert!(matches!(
            pg_config.get_hosts(),
            [Host::Tcp(host)] if host == "db.example.com"
        ));
        assert_eq!(
            pg_config.get_hostaddrs(),
            &[IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)]
        );
    }

    #[test]
    fn mysql_disabled_mode_removes_tls_options() {
        let mut config = ConnectionConfig::new("test", DatabaseType::MySQL);
        config.mysql_ssl_mode = MySqlSslMode::Disabled;

        assert!(configured_mysql_ssl_opts(&config).is_none());
    }

    #[test]
    fn postgres_tls_rejects_missing_custom_ca() {
        let mut config = ConnectionConfig::new("test", DatabaseType::PostgreSQL);
        config.ssl_ca_cert = "/path/that/does/not/exist.pem".to_string();

        assert!(build_pg_tls_connector(&config).is_err());
    }
}
