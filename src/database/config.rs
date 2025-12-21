//! 数据库连接配置

use super::ssh_tunnel::SshTunnelConfig;
use super::types::{DatabaseType, MySqlSslMode, PostgresSslMode};
use serde::{Deserialize, Serialize};

// ============================================================================
// URL 编码辅助函数
// ============================================================================

/// 对字符串进行 URL 编码，用于 MySQL 连接字符串
///
/// 处理特殊字符如 #、@、:、/ 等，确保连接字符串正确解析
pub(crate) fn url_encode(s: &str) -> String {
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
// 密码加密
// ============================================================================

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
    if key_material.is_empty()
        && let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            key_material.push_str(&user);
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
    if key_material.is_empty()
        && let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("USERNAME")) {
            key_material.push_str(&user);
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
    let unbound_key =
        UnboundKey::new(&AES_256_GCM, &key_bytes).map_err(|_| "Failed to create encryption key")?;
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
    let plaintext = key
        .open_in_place(nonce, Aad::empty(), &mut in_out)
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
        let combined = STANDARD
            .decode(data)
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

        Err(
            "Decryption failed - password may have been encrypted on different machine".to_string(),
        )
    } else {
        // 尝试旧版 base64 格式（向后兼容）
        match STANDARD.decode(encrypted) {
            Ok(bytes) => {
                String::from_utf8(bytes).map_err(|_| "Invalid UTF-8 in password".to_string())
            }
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
        let encrypted = encrypt_password(password).map_err(S::Error::custom)?;
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

    decrypt_password(&s).map_err(D::Error::custom)
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
    /// SSH 隧道配置
    #[serde(default)]
    pub ssh_config: SshTunnelConfig,
    /// MySQL SSL 模式
    #[serde(default)]
    pub mysql_ssl_mode: MySqlSslMode,
    /// PostgreSQL SSL 模式
    #[serde(default)]
    pub postgres_ssl_mode: PostgresSslMode,
    /// CA 证书路径（可选，用于 VerifyCa/VerifyIdentity 模式）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ssl_ca_cert: String,
}

#[allow(dead_code)] // 公开 API，供外部使用
impl ConnectionConfig {
    /// 创建新的连接配置
    pub fn new(name: impl Into<String>, db_type: DatabaseType) -> Self {
        let db_type_clone = db_type;
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
                format!(
                    "pg:{}:{}:{}:{}",
                    self.host, self.port, self.username, self.database
                )
            }
            DatabaseType::MySQL => {
                // 包含数据库名，确保不同数据库使用不同连接
                format!(
                    "mysql:{}:{}:{}:{}",
                    self.host, self.port, self.username, self.database
                )
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
