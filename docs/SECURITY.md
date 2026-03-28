# Security Notes | 安全说明

## 1. Credential Storage | 凭据存储

Gridix does not store database password as plain text in config.
Gridix 不以明文形式在配置中存储数据库密码。

Current implementation:
- Password field is serialized with encryption helpers in `src/database/config.rs`.
  密码字段通过 `src/database/config.rs` 的加密函数序列化。
- Encryption algorithm: `AES-256-GCM` (via `ring::aead`).
  加密算法：`AES-256-GCM`（基于 `ring::aead`）。
- Encrypted payload is stored with `v1:` prefix and Base64 encoding.
  密文使用 `v1:` 前缀并进行 Base64 编码。

## 2. Machine-Bound Key Derivation | 机器绑定密钥派生

- Encryption key is derived from hostname/user fallback + fixed salt.
  加密密钥由主机名（失败时退化为用户名）和固定盐派生。
- Legacy key path fallback is kept for backward compatibility.
  保留旧版密钥派生路径作为兼容回退。

Operational note:
- Config copied to another machine may fail to decrypt passwords.
  配置迁移到其他机器后，密码可能无法解密。

## 3. Config File Permissions | 配置文件权限

- Config writes are atomic (`temp file -> rename`).
  配置写入采用原子流程（临时文件 -> 重命名）。
- On Unix-like systems, temporary config file permission is set to `0600`.
  在 Unix 类系统上，临时配置文件权限设置为 `0600`。

## 4. Transport Security | 传输安全

### 4.1 SSL/TLS Modes
- PostgreSQL SSL modes:
  `Disable`, `Prefer`, `Require`, `VerifyCa`, `VerifyFull`
- MySQL SSL modes:
  `Disabled`, `Preferred`, `Required`, `VerifyCa`, `VerifyIdentity`

Both are exposed in connection configuration and mapped in database pool/query logic.
上述模式均在连接配置中可设置，并在数据库连接逻辑中生效。

### 4.2 SSH Tunnel
- SSH tunnel supports:
  - Password auth
  - Private key auth
- SSH server key is checked against `known_hosts`.
  SSH 服务端密钥会按 `known_hosts` 进行校验。

## 5. Recommended Operational Practices | 建议操作实践

1. Use `VerifyCa`/`VerifyFull` (`VerifyIdentity`) in production.
   生产环境优先使用证书校验模式。
2. Prefer SSH tunnel for remote/private network access.
   远程或内网访问优先使用 SSH 隧道。
3. Do not commit real credentials into screenshots/config examples.
   不要在截图或示例配置中提交真实凭据。
4. Rotate DB user passwords periodically.
   定期轮换数据库用户密码。

## 6. Security Reporting | 安全问题反馈

For sensitive issues, avoid public issue details first.
涉及敏感漏洞时，优先避免公开细节。

Project issue tracker:
<https://github.com/MCB-SMART-BOY/Gridix/issues>
