//! 连接状态和连接管理器

use super::config::ConnectionConfig;
use crate::domain::ids::ConnectionId;
use std::collections::HashMap;

// ============================================================================
// 连接状态
// ============================================================================

/// 单个数据库连接状态
#[derive(Default)]
pub struct Connection {
    /// 连接的唯一标识符
    pub id: ConnectionId,
    pub config: ConnectionConfig,
    pub connected: bool,
    /// 可用的数据库列表（MySQL/PostgreSQL）
    pub databases: Vec<String>,
    /// 当前选中的数据库
    pub selected_database: Option<String>,
    /// 当前数据库的表列表
    pub tables: Vec<String>,
    pub error: Option<String>,
}

impl Connection {
    /// 创建新连接
    pub fn new(config: ConnectionConfig) -> Self {
        Self {
            config,
            ..Default::default()
        }
    }

    /// 重置连接状态
    pub fn reset(&mut self) {
        self.connected = false;
        self.databases.clear();
        self.selected_database = None;
        self.tables.clear();
        self.error = None;
    }

    /// 设置连接成功（带数据库列表）
    pub fn set_connected_with_databases(&mut self, databases: Vec<String>) {
        self.connected = true;
        self.databases = databases;
        self.tables.clear();
        self.error = None;
    }

    /// 设置连接成功（SQLite 模式，直接设置表）
    pub fn set_connected(&mut self, tables: Vec<String>) {
        self.connected = true;
        self.databases.clear();
        self.selected_database = None;
        self.tables = tables;
        self.error = None;
    }

    /// 设置选中的数据库及其表列表
    pub fn set_database(&mut self, database: String, tables: Vec<String>) {
        self.selected_database = Some(database.clone());
        self.config.database = database;
        self.tables = tables;
    }

    /// 设置连接失败
    pub fn set_error(&mut self, error: String) {
        self.connected = false;
        self.databases.clear();
        self.selected_database = None;
        self.tables.clear();
        self.error = Some(error);
    }
}

// ============================================================================
// 连接管理器
// ============================================================================

/// 管理多个数据库连接
#[derive(Default)]
pub struct ConnectionManager {
    /// 连接存储（以名称为键，保持向后兼容）
    pub connections: HashMap<String, Connection>,
    /// 名称 → ID 映射（名称不是身份，ID 才是）
    connection_ids: HashMap<String, ConnectionId>,
    pub active: Option<String>,
}

impl ConnectionManager {
    /// 添加新连接配置
    pub fn add(&mut self, mut config: ConnectionConfig) {
        let name = config.name.clone();
        let id = ConnectionId::default();
        config.runtime_connection_id = Some(id);
        self.connection_ids.insert(name.clone(), id);
        let mut conn = Connection::new(config);
        conn.id = id;
        self.connections.insert(name, conn);
    }

    /// 获取当前活动连接
    pub fn get_active(&self) -> Option<&Connection> {
        self.active
            .as_ref()
            .and_then(|name| self.connections.get(name))
    }

    /// 按名称获取连接（推荐：先获取 ID，后续使用 get()）
    pub fn get_by_name(&self, name: &str) -> Option<&Connection> {
        self.connections.get(name)
    }

    /// 按 ID 获取连接
    pub fn get(&self, id: ConnectionId) -> Option<&Connection> {
        self.connections.values().find(|conn| conn.id == id)
    }

    /// 按名称获取 ConnectionId
    pub fn connection_id(&self, name: &str) -> Option<ConnectionId> {
        self.connection_ids.get(name).copied()
    }

    /// 断开指定连接
    pub fn disconnect(&mut self, name: &str) {
        if let Some(conn) = self.connections.get_mut(name) {
            conn.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DatabaseType;

    #[test]
    fn ssh_connections_with_shared_endpoint_keep_independent_pool_ownership() {
        let mut config = ConnectionConfig::new("first", DatabaseType::PostgreSQL);
        config.ssh_config.enabled = true;
        config.ssh_config.ssh_host = "jump.example.com".into();
        config.ssh_config.ssh_username = "alice".into();
        config.ssh_config.remote_host = "db.internal".into();
        config.ssh_config.remote_port = 5432;
        let mut manager = ConnectionManager::default();
        manager.add(config.clone());
        config.name = "second".into();
        manager.add(config.clone());

        let first = &manager.get_by_name("first").unwrap().config;
        let second = &manager.get_by_name("second").unwrap().config;
        assert_ne!(first.ssh_tunnel_name(), second.ssh_tunnel_name());
        assert_ne!(first.pool_key(), second.pool_key());
    }

    #[test]
    fn ssh_connection_replaced_under_same_name_cannot_stop_new_tunnel() {
        let mut config = ConnectionConfig::new("shared", DatabaseType::PostgreSQL);
        config.ssh_config.enabled = true;
        let mut manager = ConnectionManager::default();
        manager.add(config.clone());
        let previous = manager.get_by_name("shared").unwrap();
        let previous_id = previous.id;
        let previous_tunnel = previous.config.ssh_tunnel_name();
        let previous_pool = previous.config.pool_key();

        manager.add(config);
        let replacement = manager.get_by_name("shared").unwrap();
        assert_ne!(previous_id, replacement.id);
        assert_ne!(previous_tunnel, replacement.config.ssh_tunnel_name());
        assert_ne!(previous_pool, replacement.config.pool_key());
    }
}
