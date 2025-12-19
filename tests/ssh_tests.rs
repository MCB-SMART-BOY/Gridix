//! SSH 隧道测试
//!
//! 测试 SSH 配置验证、认证方式等

use gridix::database::{SshAuthMethod, SshTunnelConfig};

#[test]
fn test_config_validation_disabled() {
    let config = SshTunnelConfig {
        enabled: false,
        ..Default::default()
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_missing_host() {
    let config = SshTunnelConfig {
        enabled: true,
        ssh_host: String::new(),
        ..Default::default()
    };
    assert!(config.validate().is_err());
}

#[test]
fn test_config_validation_password() {
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
fn test_auth_method_display() {
    assert_eq!(SshAuthMethod::Password.display_name(), "密码");
    assert_eq!(SshAuthMethod::PrivateKey.display_name(), "私钥");
}
