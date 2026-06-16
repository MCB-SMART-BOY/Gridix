//! 连接状态和连接管理器

use super::config::ConnectionConfig;
use std::collections::HashMap;

// ============================================================================
// 连接状态
// ============================================================================

/// 单个数据库连接状态
#[derive(Default)]
pub struct Connection {
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
    pub connections: HashMap<String, Connection>,
    pub active: Option<String>,
}

impl ConnectionManager {
    /// 添加新连接配置
    pub fn add(&mut self, config: ConnectionConfig) {
        let name = config.name.clone();
        self.connections.insert(name, Connection::new(config));
    }

    /// 移除连接
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn remove(&mut self, name: &str) -> Option<Connection> {
        if self.active.as_deref() == Some(name) {
            self.active = None;
        }
        self.connections.remove(name)
    }

    /// 获取当前活动连接
    pub fn get_active(&self) -> Option<&Connection> {
        self.active
            .as_ref()
            .and_then(|name| self.connections.get(name))
    }

    /// 获取当前活动连接（可变）
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn get_active_mut(&mut self) -> Option<&mut Connection> {
        let name = self.active.clone()?;
        self.connections.get_mut(&name)
    }

    /// 断开指定连接
    pub fn disconnect(&mut self, name: &str) {
        if let Some(conn) = self.connections.get_mut(name) {
            conn.reset();
        }
    }

    /// 处理连接结果
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn handle_connect_result(&mut self, name: &str, result: Result<Vec<String>, String>) {
        if let Some(conn) = self.connections.get_mut(name) {
            match result {
                Ok(tables) => conn.set_connected(tables),
                Err(e) => {
                    conn.set_error(e);
                    if self.active.as_deref() == Some(name) {
                        self.active = None;
                    }
                }
            }
        }
    }

    /// 连接数量
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn len(&self) -> usize {
        self.connections.len()
    }

    /// 是否为空
    #[allow(dead_code)] // 公开 API，供外部使用
    pub fn is_empty(&self) -> bool {
        self.connections.is_empty()
    }
}
