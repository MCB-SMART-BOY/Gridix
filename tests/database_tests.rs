//! 数据库模块测试

use gridix::database::{
    DatabaseType,
    DriverCapabilities, DriverRegistry, DriverInfo,
    SshTunnelConfig, SshAuthMethod,
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
