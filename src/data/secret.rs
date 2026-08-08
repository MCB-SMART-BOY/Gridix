//! 秘密值存储与生命周期管理
//!
//! 提供 `SecretStore` trait 统一 DB 密码和 SSH 密码的 keyring 持久化，
//! 以及 `SecretString` 用于安全清除内存中的秘密值。

use crate::data::error::DbError;
use std::fmt;

/// 可安全清除的秘密值
///
/// Drop 时自动 zeroize 内部缓冲区。Debug 输出隐藏实际值。
pub(crate) struct SecretString(String);

impl SecretString {
    /// 从 String 创建秘密值
    pub(crate) fn new(s: String) -> Self {
        Self(s)
    }

    /// 暴露内部引用（仅在需要将密码传给外部 API 时使用）
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.0.zeroize();
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretString(***)")
    }
}

/// 秘密存储后端
///
/// 不同的存储后端（OS keyring、加密文件、内存）实现此 trait。
pub(crate) trait SecretStore: Send + Sync {
    /// 加载秘密值。不存在时返回 None。
    fn load(&self, key: &str) -> Result<Option<SecretString>, DbError>;
    /// 存储秘密值
    fn store(&self, key: &str, secret: &SecretString) -> Result<(), DbError>;
    /// 删除秘密值
    fn delete(&self, key: &str) -> Result<(), DbError>;
}

/// OS keyring 实现
///
/// 使用 `keyring` crate 访问系统凭证存储（Linux: Secret Service, macOS: Keychain, Windows: Credential Manager）。
pub(crate) struct KeyringStore;

impl SecretStore for KeyringStore {
    fn load(&self, key: &str) -> Result<Option<SecretString>, DbError> {
        let entry =
            keyring::Entry::new("gridix", key).map_err(|e| DbError::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(pw) => Ok(Some(SecretString::new(pw))),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(DbError::Keyring(e.to_string())),
        }
    }

    fn store(&self, key: &str, secret: &SecretString) -> Result<(), DbError> {
        let entry =
            keyring::Entry::new("gridix", key).map_err(|e| DbError::Keyring(e.to_string()))?;
        entry
            .set_password(secret.expose())
            .map_err(|e| DbError::Keyring(e.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), DbError> {
        let entry =
            keyring::Entry::new("gridix", key).map_err(|e| DbError::Keyring(e.to_string()))?;
        entry
            .delete_credential()
            .map_err(|e| DbError::Keyring(e.to_string()))
    }
}
