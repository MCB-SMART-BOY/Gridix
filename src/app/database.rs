//! 数据库操作模块
//!
//! 处理数据库连接、断开、查询执行等操作。

use std::time::Instant;

use crate::core::constants;
use crate::database::{
    connect_database, execute_query, get_primary_key_column, get_tables_for_database,
    ConnectResult,
    ssh_tunnel::SSH_TUNNEL_MANAGER,
};

use super::message::Message;
use super::DbManagerApp;

impl DbManagerApp {
    /// 连接到数据库
    pub(super) fn connect(&mut self, name: String) {
        if let Some(conn) = self.manager.connections.get(&name) {
            let config = conn.config.clone();
            let tx = self.tx.clone();

            self.connecting = true;
            self.manager.active = Some(name.clone());

            self.runtime.spawn(async move {
                use tokio::time::{timeout, Duration};
                // 连接超时
                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = timeout(
                    Duration::from_secs(timeout_secs),
                    connect_database(&config),
                )
                .await;
                let message = match result {
                    Ok(Ok(ConnectResult::Tables(tables))) => {
                        Message::ConnectedWithTables(name, Ok(tables))
                    }
                    Ok(Ok(ConnectResult::Databases(databases))) => {
                        Message::ConnectedWithDatabases(name, Ok(databases))
                    }
                    Ok(Err(e)) => Message::ConnectedWithTables(name, Err(e.to_string())),
                    Err(_) => {
                        // 提供更详细的超时错误信息
                        let host_info = match &config.db_type {
                            crate::database::DatabaseType::SQLite => {
                                format!("文件: {}", if config.database.is_empty() { "未指定" } else { &config.database })
                            }
                            _ => format!("{}:{}", config.host, config.port),
                        };
                        let err_msg = format!(
                            "连接超时 ({}秒)。目标: {}。请检查: 1) 网络连接 2) 防火墙设置 3) 数据库服务是否运行",
                            timeout_secs, host_info
                        );
                        Message::ConnectedWithTables(name, Err(err_msg))
                    }
                };
                if tx.send(message).is_err() {
                    tracing::warn!("无法发送连接结果：接收端已关闭");
                }
            });
        }
    }

    /// 选择数据库（MySQL/PostgreSQL）
    pub(super) fn select_database(&mut self, database: String) {
        let Some(active_name) = self.manager.active.clone() else {
            return;
        };
        let Some(conn) = self.manager.connections.get(&active_name) else {
            return;
        };
        let config = conn.config.clone();
        let tx = self.tx.clone();

        self.connecting = true;

        self.runtime.spawn(async move {
            use tokio::time::{timeout, Duration};
            let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
            let db_name = database.clone();
            let result = timeout(
                Duration::from_secs(timeout_secs),
                get_tables_for_database(&config, &database),
            )
            .await;
            let tables_result = match result {
                Ok(Ok(tables)) => Ok(tables),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err(format!(
                    "获取表列表超时 ({}秒)。数据库: {}。可能原因: 表数量过多或网络延迟",
                    timeout_secs, db_name
                )),
            };
            if tx
                .send(Message::DatabaseSelected(
                    active_name,
                    database,
                    tables_result,
                ))
                .is_err()
            {
                tracing::warn!("无法发送数据库选择结果：接收端已关闭");
            }
        });
    }

    /// 断开数据库连接
    pub(super) fn disconnect(&mut self, name: String) {
        // 清理 SSH 隧道和连接池
        if let Some(conn) = self.manager.connections.get(&name) {
            let config = conn.config.clone();
            let handle = self.runtime.handle().clone();

            // 停止关联的 SSH 隧道
            if config.ssh_config.enabled {
                let tunnel_name = format!("{}_{}", name, config.ssh_config.remote_host);
                let handle_clone = handle.clone();
                std::thread::spawn(move || {
                    handle_clone.block_on(async {
                        SSH_TUNNEL_MANAGER.stop(&tunnel_name).await;
                    });
                });
            }

            // 清理连接池
            std::thread::spawn(move || {
                handle.block_on(async {
                    crate::database::POOL_MANAGER.remove_pool(&config).await;
                });
            });
        }

        self.manager.disconnect(&name);
        if self.manager.active.as_deref() == Some(&name) {
            self.manager.active = None;
            self.selected_table = None;
            self.result = None;
        }
    }

    /// 删除连接配置
    pub(super) fn delete_connection(&mut self, name: &str) {
        self.manager.connections.remove(name);
        // 删除该连接的历史记录
        self.app_config.command_history.remove(name);
        // 如果删除的是当前连接，清空当前状态
        if self.manager.active.as_deref() == Some(name) {
            self.manager.active = None;
            self.selected_table = None;
            self.result = None;
            self.command_history.clear();
            self.current_history_connection = None;
        }
        self.save_config();
    }

    /// 执行 SQL 查询
    pub(super) fn execute(&mut self, sql: String) {
        if sql.trim().is_empty() {
            return;
        }

        // 提前检查连接状态
        let Some(active_name) = &self.manager.active else {
            self.notifications.warning("请先连接数据库");
            return;
        };
        let Some(conn) = self.manager.connections.get(active_name) else {
            self.notifications.warning("请先连接数据库");
            return;
        };

        let config = conn.config.clone();
        let tx = self.tx.clone();

        // 添加到命令历史
        if self.command_history.first() != Some(&sql) {
            self.command_history.insert(0, sql.clone());
            // 限制每个连接最多保存历史记录
            if self.command_history.len() > constants::history::MAX_COMMAND_HISTORY_PER_CONNECTION {
                self.command_history.pop();
            }
            // 保存历史记录到配置文件
            self.save_current_history();
            let _ = self.app_config.save();
        }
        self.history_index = None;

        self.executing = true;
        self.result = None;
        self.last_query_time_ms = None;

        // 同步 SQL 到当前 Tab 并设置执行状态
        if let Some(tab) = self.tab_manager.get_active_mut() {
            tab.sql = sql.clone();
            tab.executing = true;
            tab.update_title();
        }

        self.runtime.spawn(async move {
            use tokio::time::{timeout, Duration};
            let start = Instant::now();
            let timeout_secs = constants::database::QUERY_TIMEOUT_SECS;
            // 查询超时
            let result = timeout(
                Duration::from_secs(timeout_secs),
                execute_query(&config, &sql),
            )
            .await;
            let elapsed_ms = start.elapsed().as_millis() as u64;
            let query_result = match result {
                Ok(Ok(res)) => Ok(res),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err(format!(
                    "查询超时 ({}秒)。建议: 1) 添加 LIMIT 限制结果集 2) 优化查询条件 3) 检查索引",
                    timeout_secs
                )),
            };
            if tx
                .send(Message::QueryDone(sql, query_result, elapsed_ms))
                .is_err()
            {
                tracing::warn!("无法发送查询结果：接收端已关闭");
            }
        });
    }

    /// 异步获取表的主键列
    pub(super) fn fetch_primary_key(&self, table_name: &str) {
        let Some(conn) = self.manager.get_active() else {
            return;
        };

        let config = conn.config.clone();
        let table = table_name.to_string();
        let tx = self.tx.clone();

        self.runtime.spawn(async move {
            let pk_result = get_primary_key_column(&config, &table).await;
            let pk_column = pk_result.ok().flatten();
            if tx
                .send(Message::PrimaryKeyFetched(table, pk_column))
                .is_err()
            {
                tracing::warn!("无法发送主键信息：接收端已关闭");
            }
        });
    }

    /// 处理连接错误的通用逻辑
    pub(super) fn handle_connection_error(&mut self, name: &str, error: String) {
        self.notifications.error(format!("连接失败: {}", error));
        self.autocomplete.clear();
        if let Some(conn) = self.manager.connections.get_mut(name) {
            conn.set_error(error);
        }
    }
}
