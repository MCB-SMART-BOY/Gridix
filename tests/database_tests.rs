//! 数据库模块测试

use gridix::data::{
    ConnectionConfig, DatabaseType, MySqlSslMode, PostgresSslMode, SshAuthMethod, SshTunnelConfig,
};

// ============================================================================
// SSH Tunnel 测试
// ============================================================================

#[test]
fn test_ssh_config_validation_disabled() {
    let config = SshTunnelConfig {
        enabled: false,
        ..Default::default()
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ssh_config_validation_missing_host() {
    let config = SshTunnelConfig {
        enabled: true,
        ssh_host: String::new(),
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_ssh_config_validation_password() {
    let config = SshTunnelConfig {
        enabled: true,
        ssh_host: "example.com".to_string(),
        ssh_port: 22,
        ssh_username: "user".to_string(),
        auth_method: SshAuthMethod::Password,
        ssh_password: "pass".to_string(),
        password_ref: None,
        remote_host: "localhost".to_string(),
        remote_port: 3306,
        ..Default::default()
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_ssh_auth_method_display() {
    assert_eq!(SshAuthMethod::Password.display_name(), "密码");
    assert_eq!(SshAuthMethod::PrivateKey.display_name(), "私钥");
}

#[test]
fn test_postgres_connection_string_escapes_special_chars() {
    let config = ConnectionConfig {
        db_type: DatabaseType::PostgreSQL,
        host: "db host".to_string(),
        port: 5432,
        username: "user'name".to_string(),
        password: "pa'ss\\word".to_string(),
        database: "my db".to_string(),
        ..Default::default()
    };

    let conn_str = config.connection_string();
    assert!(conn_str.contains("host='db host'"));
    assert!(conn_str.contains("user='user\\'name'"));
    assert!(conn_str.contains("password='pa\\'ss\\\\word'"));
    assert!(conn_str.contains("dbname='my db'"));
}

#[test]
fn test_pool_key_changes_with_password_and_ssl_postgres() {
    let base = ConnectionConfig {
        db_type: DatabaseType::PostgreSQL,
        host: "localhost".to_string(),
        port: 5432,
        username: "user".to_string(),
        password: "secret1".to_string(),
        database: "app".to_string(),
        postgres_ssl_mode: PostgresSslMode::Disable,
        ..Default::default()
    };

    let mut changed_password = base.clone();
    changed_password.password = "secret2".to_string();
    assert_ne!(base.pool_key(), changed_password.pool_key());

    let mut changed_ssl = base.clone();
    changed_ssl.postgres_ssl_mode = PostgresSslMode::Require;
    assert_ne!(base.pool_key(), changed_ssl.pool_key());
}

#[test]
fn test_pool_key_changes_with_password_and_ssl_mysql() {
    let base = ConnectionConfig {
        db_type: DatabaseType::MySQL,
        host: "localhost".to_string(),
        port: 3306,
        username: "user".to_string(),
        password: "secret1".to_string(),
        database: "app".to_string(),
        mysql_ssl_mode: MySqlSslMode::Disabled,
        ..Default::default()
    };

    let mut changed_password = base.clone();
    changed_password.password = "secret2".to_string();
    assert_ne!(base.pool_key(), changed_password.pool_key());

    let mut changed_ssl = base.clone();
    changed_ssl.mysql_ssl_mode = MySqlSslMode::Required;
    assert_ne!(base.pool_key(), changed_ssl.pool_key());
}

#[test]
fn test_ssh_pool_key_is_stable_after_runtime_host_rewrite() {
    let base = ConnectionConfig {
        db_type: DatabaseType::PostgreSQL,
        host: "db.internal".to_string(),
        port: 5432,
        username: "app_user".to_string(),
        password: "db-secret".to_string(),
        database: "app".to_string(),
        ssh_config: SshTunnelConfig {
            enabled: true,
            ssh_host: "jump.internal".to_string(),
            ssh_port: 22,
            ssh_username: "ssh-user".to_string(),
            ssh_password: "ssh-secret".to_string(),
            password_ref: None,
            auth_method: SshAuthMethod::Password,
            remote_host: "db.internal".to_string(),
            remote_port: 5432,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut effective = base.clone();
    effective.host = "127.0.0.1".to_string();
    effective.port = 15432;

    assert_eq!(base.pool_key(), effective.pool_key());

    // auth_fingerprint 不再包含密码 — 同一身份（host+port+user+auth_method）产生相同 pool_key
    let mut changed_ssh_password = base.clone();
    changed_ssh_password.ssh_config.ssh_password = "other-ssh-secret".to_string();
    assert_eq!(
        base.pool_key(),
        changed_ssh_password.pool_key(),
        "相同 SSH 身份的 pool_key 应一致（密码变更不影响 tunnel 身份）"
    );
}

// ============================================================================
// 数据库类型测试
// ============================================================================

#[test]
fn test_database_type_display_names() {
    assert_eq!(DatabaseType::SQLite.display_name(), "SQLite");
    assert_eq!(DatabaseType::PostgreSQL.display_name(), "PostgreSQL");
    assert_eq!(DatabaseType::MySQL.display_name(), "MySQL");
}

#[test]
fn test_database_type_default_is_sqlite() {
    assert_eq!(DatabaseType::default(), DatabaseType::SQLite);
}

// ============================================================================
// ConnectionConfig 验证测试
// ============================================================================

#[test]
fn test_connection_config_sqlite_defaults_to_local_file() {
    let config = ConnectionConfig::default();
    assert_eq!(config.db_type, DatabaseType::SQLite);
    assert!(config.database.is_empty()); // SQLite defaults to in-memory
}

#[test]
fn test_connection_config_ssl_mode_defaults() {
    let config = ConnectionConfig::default();
    assert_eq!(config.postgres_ssl_mode, PostgresSslMode::Prefer);
    assert_eq!(config.mysql_ssl_mode, MySqlSslMode::Preferred);
}

#[test]
fn test_pool_key_is_deterministic() {
    let config = ConnectionConfig {
        db_type: DatabaseType::PostgreSQL,
        host: "db.example.com".to_string(),
        port: 5432,
        username: "user".to_string(),
        password: "secret".to_string(),
        database: "app".to_string(),
        ..Default::default()
    };
    let key1 = config.pool_key();
    let key2 = config.pool_key();
    assert_eq!(key1, key2);
}

#[test]
fn test_pool_key_includes_ssl_mode() {
    let mut config = ConnectionConfig {
        db_type: DatabaseType::PostgreSQL,
        host: "localhost".to_string(),
        port: 5432,
        username: "user".to_string(),
        password: "secret".to_string(),
        database: "app".to_string(),
        postgres_ssl_mode: PostgresSslMode::Disable,
        ..Default::default()
    };

    let key_no_ssl = config.pool_key();
    config.postgres_ssl_mode = PostgresSslMode::Require;
    let key_require_ssl = config.pool_key();
    assert_ne!(key_no_ssl, key_require_ssl);
}

// ============================================================================
// Debug 输出不泄漏密码
// ============================================================================

#[test]
fn test_connection_config_debug_redacts_password() {
    let config = ConnectionConfig {
        name: "test".to_string(),
        db_type: DatabaseType::PostgreSQL,
        host: "localhost".to_string(),
        port: 5432,
        username: "admin".to_string(),
        password: "super-secret-123".to_string(),
        database: "mydb".to_string(),
        ..Default::default()
    };
    let debug_output = format!("{:?}", config);
    assert!(
        !debug_output.contains("super-secret-123"),
        "Debug must not leak password: {debug_output}"
    );
    assert!(
        debug_output.contains("<REDACTED>"),
        "Debug should show <REDACTED>: {debug_output}"
    );
    // 确认非敏感字段仍然可见
    assert!(debug_output.contains("admin"), "Username should be visible");
    assert!(debug_output.contains("mydb"), "Database should be visible");
}

#[test]
fn test_ssh_tunnel_config_debug_redacts_passwords() {
    use gridix::data::SshTunnelConfig;
    let config = SshTunnelConfig {
        enabled: true,
        ssh_host: "jump.example.com".to_string(),
        ssh_port: 22,
        ssh_username: "deployer".to_string(),
        ssh_password: "ssh-secret-456".to_string(),
        password_ref: None,
        credential_revision: 0,
        private_key_path: "/home/deployer/.ssh/id_rsa".to_string(),
        private_key_passphrase: "key-pass-789".to_string(),
        auth_method: gridix::data::SshAuthMethod::Password,
        remote_host: "db.internal".to_string(),
        remote_port: 5432,
        local_port: 0,
    };
    let debug_output = format!("{:?}", config);
    assert!(
        !debug_output.contains("ssh-secret-456"),
        "Debug must not leak SSH password: {debug_output}"
    );
    assert!(
        !debug_output.contains("key-pass-789"),
        "Debug must not leak key passphrase: {debug_output}"
    );
    assert!(
        debug_output.contains("<REDACTED>"),
        "Debug should show <REDACTED>: {debug_output}"
    );
    // 确认非敏感字段仍然可见
    assert!(
        debug_output.contains("deployer"),
        "SSH username should be visible"
    );
    assert!(
        debug_output.contains("jump.example.com"),
        "SSH host should be visible"
    );
}
