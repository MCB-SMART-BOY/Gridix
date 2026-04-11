# Security Notes | 安全说明

## 1. Credential Storage | 凭据存储

Gridix does not store database password as plain text in config.
Gridix 不以明文形式在配置中存储数据库密码。

Current implementation:
- Connection config stores only a `password_ref` pointer in `config.toml`.
  连接配置仅在 `config.toml` 中保存 `password_ref` 引用。
- Actual secrets are written to the OS keyring via the `keyring` crate.
  实际密码通过 `keyring` crate 写入操作系统密钥链。
- If keyring lookup fails or the secret is missing, the connection is kept but password must be re-entered.
  如果密钥链读取失败或秘密缺失，连接配置仍会保留，但需要重新输入密码。

## 2. Legacy Password Migration | 旧版密码迁移

- Legacy encrypted `password` fields are still readable for backward compatibility.
  为兼容旧版本，历史加密 `password` 字段仍可读取。
- On successful load, Gridix migrates the secret into the OS keyring and rewrites config without embedding the password.
  旧密码读取成功后，Gridix 会迁移到系统密钥链，并在后续保存时移除配置中的嵌入密码。
- Legacy decryption still supports the previous machine-bound AES-GCM scheme and config-dir fallback.
  旧版解密仍兼容此前基于机器信息派生的 AES-GCM 方案及旧配置目录回退逻辑。

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
