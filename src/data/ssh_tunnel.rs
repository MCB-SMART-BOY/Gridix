//! SSH 隧道模块
//!
//! 提供 SSH 隧道功能，允许通过 SSH 跳板机连接远程数据库。

use russh::client::{Config, Handle, Handler};
use russh::keys::{HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use thiserror::Error;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

const SSH_DISCONNECT_TIMEOUT: tokio::time::Duration = tokio::time::Duration::from_secs(5);

// ============================================================================
// 错误类型
// ============================================================================

#[derive(Error, Debug)]
pub enum SshError {
    #[error("SSH 连接失败: {0}")]
    Connection(String),
    #[error("SSH 认证失败: {0}")]
    Authentication(String),
    #[error("SSH 主机密钥验证失败: {0}")]
    HostKeyVerification(String),
    #[error("隧道创建失败: {0}")]
    Tunnel(String),
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("密钥加载失败: {0}")]
    Key(String),
}

// ============================================================================
// SSH 隧道配置
// ============================================================================

/// SSH 认证方式
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, Hash)]
pub enum SshAuthMethod {
    /// 密码认证
    #[default]
    Password,
    /// 私钥认证
    PrivateKey,
}

impl SshAuthMethod {
    /// 获取认证方式的显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Password => "密码",
            Self::PrivateKey => "私钥",
        }
    }

    pub(crate) fn cache_key(&self) -> &'static str {
        match self {
            Self::Password => "password",
            Self::PrivateKey => "private_key",
        }
    }
}

/// SSH 隧道配置
#[derive(Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
pub struct SshTunnelConfig {
    /// 是否启用 SSH 隧道
    pub enabled: bool,
    /// SSH 服务器地址
    pub ssh_host: String,
    /// SSH 服务器端口
    pub ssh_port: u16,
    /// SSH 用户名
    pub ssh_username: String,
    /// SSH 密码（密码认证时使用）
    /// 注意：此字段在序列化时会被跳过，密码通过 OS keyring 存储。
    #[serde(default, skip_serializing)]
    pub ssh_password: String,
    /// 持久化密码的 keyring key（None 表示仅内存密码）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password_ref: Option<String>,
    /// 密码最近一次实际编辑的版本，用于轮换非秘密隧道身份。
    #[serde(default)]
    pub credential_revision: u64,
    /// 私钥路径（私钥认证时使用）
    pub private_key_path: String,
    /// 私钥密码（如果私钥有密码保护）
    /// 注意：此字段在序列化时会被跳过，密码通过 OS keyring 存储。
    #[serde(default, skip_serializing)]
    pub private_key_passphrase: String,
    /// 认证方式
    pub auth_method: SshAuthMethod,
    /// 远程数据库主机（从 SSH 服务器视角）
    pub remote_host: String,
    /// 远程数据库端口
    pub remote_port: u16,
    /// 本地绑定端口（0 表示自动分配）
    pub local_port: u16,
}

impl std::fmt::Debug for SshTunnelConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshTunnelConfig")
            .field("enabled", &self.enabled)
            .field("ssh_host", &self.ssh_host)
            .field("ssh_port", &self.ssh_port)
            .field("ssh_username", &self.ssh_username)
            .field("ssh_password", &"<REDACTED>")
            .field("private_key_path", &self.private_key_path)
            .field("private_key_passphrase", &"<REDACTED>")
            .field("auth_method", &self.auth_method)
            .field("credential_revision", &self.credential_revision)
            .field("remote_host", &self.remote_host)
            .field("remote_port", &self.remote_port)
            .field("local_port", &self.local_port)
            .finish()
    }
}

#[allow(dead_code)] // 公开 API，供外部使用
impl SshTunnelConfig {
    /// 创建新的 SSH 隧道配置
    pub fn new() -> Self {
        Self {
            ssh_port: 22,
            ..Default::default()
        }
    }

    /// 获取 SSH 服务器地址
    pub fn ssh_addr(&self) -> String {
        format!("{}:{}", self.ssh_host, self.ssh_port)
    }

    fn auth_fingerprint(&self) -> String {
        let credential_material = match self.auth_method {
            SshAuthMethod::Password => format!(
                "password:{}:{}",
                self.password_ref.as_deref().unwrap_or("memory"),
                self.credential_revision
            ),
            SshAuthMethod::PrivateKey => self.private_key_path.clone(),
        };
        let material = format!(
            "ssh:{}@{}:{}:{}",
            self.ssh_username, self.ssh_host, self.ssh_port, credential_material
        );
        crate::core::hash::sha256_hex(&material)
    }

    /// 记录 SSH 密码的用户编辑，以强制创建使用新凭据的隧道。
    pub fn mark_password_edited(&mut self) {
        self.credential_revision = self.credential_revision.wrapping_add(1);
    }

    /// 获取隧道唯一名称（用于复用与回收）
    pub fn tunnel_name(&self) -> String {
        format!(
            "{}:{}:{}:{}:{}:{}->{}:{}",
            self.ssh_host,
            self.ssh_port,
            self.ssh_username,
            self.auth_method.cache_key(),
            self.auth_fingerprint(),
            self.local_port,
            self.remote_host,
            self.remote_port
        )
    }

    /// 验证配置是否有效
    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        if self.ssh_host.is_empty() {
            return Err("SSH 主机地址不能为空".to_string());
        }

        if self.ssh_username.is_empty() {
            return Err("SSH 用户名不能为空".to_string());
        }

        if self.remote_host.is_empty() {
            return Err("远程数据库主机不能为空".to_string());
        }

        if self.remote_port == 0 {
            return Err("远程数据库端口无效".to_string());
        }

        match self.auth_method {
            SshAuthMethod::Password => {
                if self.ssh_password.is_empty() {
                    return Err("SSH 密码不能为空".to_string());
                }
            }
            SshAuthMethod::PrivateKey => {
                if self.private_key_path.is_empty() {
                    return Err("私钥路径不能为空".to_string());
                }
                if !std::path::Path::new(&self.private_key_path).exists() {
                    return Err("私钥文件不存在".to_string());
                }
            }
        }

        Ok(())
    }
}

// ============================================================================
// SSH 客户端处理器
// ============================================================================

struct SshClientHandler {
    host: String,
    port: u16,
}

impl SshClientHandler {
    fn new(host: String, port: u16) -> Self {
        Self { host, port }
    }
}

impl Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_identity: &PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        let PublicKeyOrCertificate::PublicKey { key, .. } = server_identity else {
            tracing::error!(
                host = %self.host,
                port = self.port,
                "SSH 主机证书不受信任：未配置证书 CA 验证，拒绝连接"
            );
            return Ok(false);
        };

        let fingerprint = key.fingerprint(HashAlg::Sha256);

        match russh::keys::check_known_hosts(&self.host, self.port, key) {
            Ok(true) => Ok(true),
            Ok(false) => {
                tracing::error!(
                    host = %self.host,
                    port = self.port,
                    fingerprint = %fingerprint,
                    "SSH 主机密钥不在 known_hosts 中。\
                     请先通过 ssh <user>@{} 手动连接以信任主机密钥",
                    self.host,
                );
                Ok(false)
            }
            Err(russh::keys::Error::KeyChanged { line }) => {
                tracing::error!(
                    host = %self.host,
                    port = self.port,
                    fingerprint = %fingerprint,
                    known_hosts_line = line,
                    "SSH 主机密钥不匹配：known_hosts 中已有不同密钥。\
                     可能是服务器重装或中间人攻击。\
                     如果确认服务器已重装，请运行: ssh-keygen -R [{}]:{}",
                    self.host, self.port,
                );
                Ok(false)
            }
            Err(error) => {
                tracing::error!(
                    host = %self.host,
                    port = self.port,
                    fingerprint = %fingerprint,
                    error = %error,
                    "读取 known_hosts 失败，拒绝 SSH 连接"
                );
                Ok(false)
            }
        }
    }
}

// ============================================================================
// SSH 隧道
// ============================================================================

/// SSH 隧道状态
pub struct SshTunnel {
    /// 本地监听地址
    local_addr: SocketAddr,
    /// 停止监听器和所有转发连接的取消信号
    cancellation: CancellationToken,
    /// 拥有监听任务，停止时等待所有子任务退出
    task_handle: Mutex<Option<JoinHandle<()>>>,
}

impl SshTunnel {
    /// 创建并启动 SSH 隧道
    pub async fn start(config: &SshTunnelConfig) -> Result<Self, SshError> {
        // 验证配置
        config.validate().map_err(SshError::Connection)?;

        // 创建本地监听器
        let local_addr = format!("127.0.0.1:{}", config.local_port);
        let listener = TcpListener::bind(&local_addr).await?;
        let actual_local_addr = listener.local_addr()?;

        // 建立 SSH 连接
        let ssh_handle = Self::connect_ssh(config).await?;
        let ssh_handle = Arc::new(Mutex::new(ssh_handle));

        let cancellation = CancellationToken::new();
        let remote_host = config.remote_host.clone();
        let remote_port = config.remote_port;

        let task_cancellation = cancellation.clone();
        let task_handle = tokio::spawn(async move {
            Self::run_tunnel(
                listener,
                ssh_handle,
                remote_host,
                remote_port,
                task_cancellation,
            )
            .await;
        });

        Ok(Self {
            local_addr: actual_local_addr,
            cancellation,
            task_handle: Mutex::new(Some(task_handle)),
        })
    }

    /// 建立 SSH 连接
    async fn connect_ssh(config: &SshTunnelConfig) -> Result<Handle<SshClientHandler>, SshError> {
        let ssh_config = Config::default();
        let ssh_config = Arc::new(ssh_config);

        let handler = SshClientHandler::new(config.ssh_host.clone(), config.ssh_port);

        // 连接 SSH 服务器
        let mut session = russh::client::connect(ssh_config, config.ssh_addr(), handler)
            .await
            .map_err(|e| SshError::Connection(format!("连接失败: {}", e)))?;

        // 认证
        let auth_result = match config.auth_method {
            SshAuthMethod::Password => session
                .authenticate_password(&config.ssh_username, &config.ssh_password)
                .await
                .map_err(|e| SshError::Authentication(format!("密码认证失败: {}", e)))?,
            SshAuthMethod::PrivateKey => {
                let key_data = std::fs::read_to_string(&config.private_key_path)
                    .map_err(|e| SshError::Key(format!("读取私钥文件失败: {}", e)))?;

                let key_pair = if config.private_key_passphrase.is_empty() {
                    russh::keys::decode_secret_key(&key_data, None)
                } else {
                    russh::keys::decode_secret_key(&key_data, Some(&config.private_key_passphrase))
                }
                .map_err(|e| SshError::Key(format!("解析私钥失败: {}", e)))?;

                let key_with_alg = PrivateKeyWithHashAlg::new(Arc::new(key_pair), None);

                session
                    .authenticate_publickey(&config.ssh_username, key_with_alg)
                    .await
                    .map_err(|e| SshError::Authentication(format!("私钥认证失败: {}", e)))?
            }
        };

        if !auth_result.success() {
            return Err(SshError::Authentication("认证失败".to_string()));
        }

        Ok(session)
    }

    /// 运行隧道转发，所有连接均隶属监听任务，关闭前必须收束。
    async fn run_tunnel(
        listener: TcpListener,
        ssh_handle: Arc<Mutex<Handle<SshClientHandler>>>,
        remote_host: String,
        remote_port: u16,
        cancellation: CancellationToken,
    ) {
        let mut forwards = JoinSet::new();
        loop {
            let has_forwards = !forwards.is_empty();
            tokio::select! {
                biased;
                _ = cancellation.cancelled() => break,
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((local_stream, _)) => {
                            let ssh_handle = ssh_handle.clone();
                            let remote_host = remote_host.clone();
                            forwards.spawn(async move {
                                if let Err(e) = Self::forward_connection(
                                    local_stream,
                                    ssh_handle,
                                    &remote_host,
                                    remote_port,
                                ).await {
                                    tracing::warn!(error = %e, "SSH 隧道转发错误");
                                }
                            });
                        }
                        Err(e) => tracing::warn!(error = %e, "SSH 隧道接受连接错误"),
                    }
                }
                Some(result) = forwards.join_next(), if has_forwards => {
                    if let Err(error) = result {
                        tracing::warn!(error = %error, "SSH 隧道转发任务失败");
                    }
                }
            }
        }
        drop(listener);
        forwards.abort_all();
        while let Some(result) = forwards.join_next().await {
            if let Err(error) = result
                && !error.is_cancelled()
            {
                tracing::warn!(error = %error, "SSH 隧道转发任务失败");
            }
        }
        Self::disconnect_ssh(ssh_handle).await;
    }

    async fn disconnect_ssh(ssh_handle: Arc<Mutex<Handle<SshClientHandler>>>) {
        let handle = ssh_handle.lock().await;
        match tokio::time::timeout(
            SSH_DISCONNECT_TIMEOUT,
            handle.disconnect(russh::Disconnect::ByApplication, "Tunnel stopped", "en"),
        )
        .await
        {
            Ok(Ok(())) => {}
            Ok(Err(error)) => tracing::warn!(error = %error, "关闭 SSH 隧道会话失败"),
            Err(error) => tracing::warn!(error = %error, "关闭 SSH 隧道会话超时"),
        }
    }

    /// 转发单个连接
    async fn forward_connection(
        mut local_stream: TcpStream,
        ssh_handle: Arc<Mutex<Handle<SshClientHandler>>>,
        remote_host: &str,
        remote_port: u16,
    ) -> Result<(), SshError> {
        let channel = {
            let handle = ssh_handle.lock().await;
            handle
                .channel_open_direct_tcpip(remote_host, remote_port as u32, "127.0.0.1", 0)
                .await
                .map_err(|error| SshError::Tunnel(format!("创建通道失败: {error}")))?
        };
        let mut channel_stream = channel.into_stream();

        copy_bidirectional(&mut local_stream, &mut channel_stream)
            .await
            .map_err(|error| SshError::Tunnel(format!("双向转发失败: {error}")))?;
        Ok(())
    }

    /// 获取本地监听地址
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// 获取本地端口
    pub fn local_port(&self) -> u16 {
        self.local_addr.port()
    }

    /// 停止隧道，关闭监听器和活动的转发流后返回。
    pub async fn stop(&self) {
        self.cancellation.cancel();
        let mut task_handle = self.task_handle.lock().await;
        if let Some(handle) = task_handle.take()
            && let Err(error) = handle.await
            && !error.is_cancelled()
        {
            tracing::warn!(error = %error, "SSH 隧道监听任务失败");
        }
    }

    /// 检查隧道是否正在运行
    pub async fn is_running(&self) -> bool {
        !self.cancellation.is_cancelled()
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(handle) = self.task_handle.get_mut().take() {
            handle.abort();
        }
    }
}

// ============================================================================
// SSH 隧道管理器
// ============================================================================

/// SSH 隧道管理器
pub struct SshTunnelManager {
    tunnels: RwLock<std::collections::HashMap<String, Arc<SshTunnel>>>,
}

impl SshTunnelManager {
    /// 创建新的隧道管理器
    pub fn new() -> Self {
        Self {
            tunnels: RwLock::new(std::collections::HashMap::new()),
        }
    }

    /// 创建或获取隧道
    pub async fn get_or_create(
        &self,
        name: &str,
        config: &SshTunnelConfig,
    ) -> Result<Arc<SshTunnel>, SshError> {
        // 检查是否已有运行中的隧道
        {
            let tunnels = self.tunnels.read().await;
            if let Some(tunnel) = tunnels.get(name)
                && tunnel.is_running().await
            {
                return Ok(tunnel.clone());
            }
        }

        let tunnel = Arc::new(SshTunnel::start(config).await?);
        Ok(self.register_started(name, tunnel).await)
    }

    /// 已经启动的竞争者必须在返回胜者之前关闭。
    async fn register_started(&self, name: &str, tunnel: Arc<SshTunnel>) -> Arc<SshTunnel> {
        let existing = {
            let mut tunnels = self.tunnels.write().await;
            match tunnels.get(name) {
                Some(existing) if !existing.cancellation.is_cancelled() => Some(existing.clone()),
                _ => {
                    tunnels.insert(name.to_string(), tunnel.clone());
                    None
                }
            }
        };
        if let Some(existing) = existing {
            tunnel.stop().await;
            existing
        } else {
            tunnel
        }
    }

    /// 停止指定隧道
    pub async fn stop(&self, name: &str) {
        let tunnel = {
            let mut tunnels = self.tunnels.write().await;
            tunnels.remove(name)
        };

        if let Some(tunnel) = tunnel {
            tunnel.stop().await;
        }
    }

    /// 停止所有隧道
    #[allow(dead_code)] // 公开 API，供外部使用
    pub async fn stop_all(&self) {
        let tunnels: Vec<_> = {
            let mut tunnels = self.tunnels.write().await;
            tunnels.drain().map(|(_, t)| t).collect()
        };

        for tunnel in tunnels {
            tunnel.stop().await;
        }
    }
}

impl Default for SshTunnelManager {
    fn default() -> Self {
        Self::new()
    }
}

// 全局隧道管理器
pub static SSH_TUNNEL_MANAGER: std::sync::LazyLock<SshTunnelManager> =
    std::sync::LazyLock::new(SshTunnelManager::new);

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn make_local_tunnel() -> Arc<SshTunnel> {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let local_addr = listener.local_addr().unwrap();
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        let task_handle = tokio::spawn(async move {
            tokio::select! {
                _ = task_cancellation.cancelled() => {}
                _ = listener.accept() => {}
            }
        });
        Arc::new(SshTunnel {
            local_addr,
            cancellation,
            task_handle: Mutex::new(Some(task_handle)),
        })
    }

    #[tokio::test]
    async fn tunnel_registration_race_loser_closes_listener_before_return() {
        let manager = SshTunnelManager::new();
        let winner = make_local_tunnel().await;
        let loser = make_local_tunnel().await;
        manager.register_started("same", winner.clone()).await;
        let selected = manager.register_started("same", loser.clone()).await;

        assert!(Arc::ptr_eq(&selected, &winner));
        assert!(!loser.is_running().await);
        assert!(TcpStream::connect(loser.local_addr()).await.is_err());
        manager.stop_all().await;
        assert!(TcpStream::connect(winner.local_addr()).await.is_err());
    }
}
