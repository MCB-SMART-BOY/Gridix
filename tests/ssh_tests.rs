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

#[test]
fn test_tunnel_name_changes_with_ssh_identity() {
    let base = SshTunnelConfig {
        enabled: true,
        ssh_host: "jump.example.com".to_string(),
        ssh_port: 22,
        ssh_username: "alice".to_string(),
        ssh_password: "secret-a".to_string(),
        auth_method: SshAuthMethod::Password,
        remote_host: "db.internal".to_string(),
        remote_port: 5432,
        ..Default::default()
    };

    let mut changed_user = base.clone();
    changed_user.ssh_username = "bob".to_string();
    assert_ne!(base.tunnel_name(), changed_user.tunnel_name());

    let mut changed_password = base.clone();
    changed_password.ssh_password = "secret-b".to_string();
    assert_ne!(base.tunnel_name(), changed_password.tunnel_name());

    let mut changed_auth = base.clone();
    changed_auth.auth_method = SshAuthMethod::PrivateKey;
    changed_auth.private_key_path = "/tmp/id_ed25519".to_string();
    changed_auth.ssh_password.clear();
    assert_ne!(base.tunnel_name(), changed_auth.tunnel_name());
}
