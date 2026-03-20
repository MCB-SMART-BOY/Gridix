//! 数据库模块测试

use gridix::database::{
    ConnectionConfig, DatabaseType, DriverCapabilities, DriverInfo, DriverRegistry, MySqlSslMode,
    PostgresSslMode, SshAuthMethod, SshTunnelConfig,
};

// ============================================================================
// Driver 测试
// ============================================================================

#[test]
fn test_driver_capabilities() {
    let sqlite = DriverCapabilities::for_db_type(DatabaseType::SQLite);
    assert!(!sqlite.user_management);
    assert!(sqlite.transactions);

    let postgres = DriverCapabilities::for_db_type(DatabaseType::PostgreSQL);
    assert!(postgres.user_management);
    assert!(postgres.stored_procedures);

    let mysql = DriverCapabilities::for_db_type(DatabaseType::MySQL);
    assert!(mysql.user_management);
    assert!(mysql.batch_insert);
}

#[test]
fn test_driver_registry() {
    let registry = DriverRegistry::new();
    assert!(registry.registered_types().is_empty());
}

#[test]
fn test_driver_info() {
    let info = DriverInfo::new(
        "SQLite Driver",
        "1.0.0",
        DatabaseType::SQLite,
        "SQLite database driver",
    );
    assert_eq!(info.name, "SQLite Driver");
    assert_eq!(info.db_type, DatabaseType::SQLite);
}

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
