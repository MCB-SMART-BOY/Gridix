//! 数据库操作模块
//!
//! 处理数据库连接、断开、查询执行等操作。

use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Instant;

use crate::core::constants;
use crate::database::{
    ConnectResult, ConnectionConfig, DatabaseType, connect_database, execute_query_cancellable,
    get_primary_key_column, get_tables_for_database, ssh_tunnel::SSH_TUNNEL_MANAGER,
};
use crate::ui;

use super::DbManagerApp;
use super::message::Message;

impl DbManagerApp {
    /// 打开连接编辑对话框（预填当前配置）
    pub(super) fn open_connection_editor(&mut self, name: &str) {
        if let Some(conn) = self.manager.connections.get(name) {
            self.new_config = conn.config.clone();
            self.editing_connection_name = Some(name.to_string());
            self.show_connection_dialog = true;
        } else {
            self.notifications
                .warning(format!("连接 '{}' 不存在", name));
        }
    }

    /// 保存连接对话框结果（新建或编辑）
    pub(super) fn save_connection_from_dialog(&mut self) {
        let config = std::mem::take(&mut self.new_config);
        let saved_db_type = config.db_type;
        let name = config.name.clone();

        let editing_name = self.editing_connection_name.clone();
        let has_name_conflict = match editing_name.as_deref() {
            Some(old_name) => old_name != name && self.manager.connections.contains_key(&name),
            None => self.manager.connections.contains_key(&name),
        };
        if has_name_conflict {
            self.notifications
                .error(format!("连接名 '{}' 已存在，请使用其他名称", name));
            self.new_config = config;
            self.show_connection_dialog = true;
            return;
        }

        if let Some(old_name) = self.editing_connection_name.take() {
            if self.manager.connections.contains_key(&old_name) {
                self.disconnect(old_name.clone());
                self.manager.connections.remove(&old_name);
            }

            if old_name != name {
                if let Some(history) = self.app_config.command_history.remove(&old_name) {
                    self.app_config
                        .command_history
                        .insert(name.clone(), history);
                }
                if self.current_history_connection.as_deref() == Some(&old_name) {
                    self.current_history_connection = Some(name.clone());
                }
            }

            self.notifications
                .success(format!("连接 '{}' 已更新", name));
        }

        self.manager.add(config);
        self.mark_onboarding_connection_created();
        self.welcome_setup_target = saved_db_type;
        if !self.welcome_onboarding_status().is_complete() {
            self.show_welcome_setup_dialog = true;
        }
        self.save_config();
        self.connect(name);
    }

    /// 连接到数据库
    pub(super) fn connect(&mut self, name: String) {
        if let Some(conn) = self.manager.connections.get(&name) {
            let config = conn.config.clone();
            let tx = self.tx.clone();
            let request_id = self.next_connect_request_id();

            self.manager.active = Some(name.clone());
            self.pending_connect_requests
                .insert(name.clone(), request_id);
            self.pending_database_requests.remove(&name);
            self.pending_triggers_request = None;
            self.pending_routines_request = None;
            self.sidebar_panel_state.loading_triggers = false;
            self.sidebar_panel_state.loading_routines = false;
            self.sidebar_panel_state.clear_triggers();
            self.sidebar_panel_state.clear_routines();
            self.refresh_connecting_flag();

            tracing::info!(connection = %name, db_type = ?config.db_type, "开始连接数据库");

            self.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};
                // 连接超时
                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result =
                    timeout(Duration::from_secs(timeout_secs), connect_database(&config)).await;
                let message = match result {
                    Ok(Ok(ConnectResult::Tables(tables))) => {
                        tracing::info!(connection = %name, tables_count = tables.len(), "数据库连接成功");
                        Message::ConnectedWithTables(name, request_id, Ok(tables))
                    }
                    Ok(Ok(ConnectResult::Databases(databases))) => {
                        tracing::info!(connection = %name, databases_count = databases.len(), "数据库连接成功，获取到数据库列表");
                        Message::ConnectedWithDatabases(name, request_id, Ok(databases))
                    }
                    Ok(Err(e)) => {
                        tracing::error!(connection = %name, error = %e, "数据库连接失败");
                        Message::ConnectedWithTables(name, request_id, Err(e.to_string()))
                    }
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
                        Message::ConnectedWithTables(name, request_id, Err(err_msg))
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
        let request_id = self.next_connect_request_id();

        self.pending_database_requests
            .insert(active_name.clone(), (database.clone(), request_id));
        self.pending_triggers_request = None;
        self.pending_routines_request = None;
        self.sidebar_panel_state.loading_triggers = false;
        self.sidebar_panel_state.loading_routines = false;
        self.sidebar_panel_state.clear_triggers();
        self.sidebar_panel_state.clear_routines();
        self.refresh_connecting_flag();

        self.runtime.spawn(async move {
            use tokio::time::{Duration, timeout};
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
                    request_id,
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
                let tunnel_name = config.ssh_config.tunnel_name();
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
        self.cancel_queries_for_connection(&name);
        self.pending_connect_requests.remove(&name);
        self.pending_database_requests.remove(&name);
        self.pending_triggers_request = None;
        self.pending_routines_request = None;
        self.pending_drop_requests
            .retain(|_, (conn_name, _)| conn_name != &name);
        if self.manager.active.as_deref() == Some(&name) {
            self.manager.active = None;
            self.selected_table = None;
            self.result = None;
            self.autocomplete.clear();
            self.sidebar_panel_state.clear_triggers();
            self.sidebar_panel_state.clear_routines();
            self.sidebar_panel_state.loading_triggers = false;
            self.sidebar_panel_state.loading_routines = false;
        }
        self.refresh_connecting_flag();
    }

    /// 删除连接配置
    pub(super) fn delete_connection(&mut self, name: &str) {
        let was_active = self.manager.active.as_deref() == Some(name);
        if self.manager.connections.contains_key(name) {
            self.disconnect(name.to_string());
            self.manager.connections.remove(name);
        }
        // 删除该连接的历史记录
        self.app_config.command_history.remove(name);
        // 如果删除的是当前连接，清空当前状态
        if was_active {
            self.manager.active = None;
            self.selected_table = None;
            self.result = None;
            self.command_history.clear();
            self.current_history_connection = None;
        }
        self.save_config();
    }

    /// 删除表（执行 DROP TABLE）
    pub(super) fn delete_table(&mut self, table: &str) {
        let Some(conn) = self.manager.get_active() else {
            self.notifications.warning("请先连接数据库");
            return;
        };
        let active_name = conn.config.name.clone();

        let use_backticks = matches!(conn.config.db_type, crate::database::DatabaseType::MySQL);
        let quoted_table = match ui::quote_identifier(table, use_backticks) {
            Ok(name) => name,
            Err(e) => {
                self.notifications.error(format!("表名无效: {}", e));
                return;
            }
        };

        if let Some(request_id) = self.execute(format!("DROP TABLE {};", quoted_table)) {
            self.pending_drop_requests
                .insert(request_id, (active_name, table.to_string()));
        }
    }

    /// 执行 SQL 查询
    pub(super) fn execute(&mut self, sql: String) -> Option<u64> {
        if sql.trim().is_empty() {
            tracing::debug!("SQL 为空，跳过执行");
            return None;
        }

        // 提前检查连接状态
        let Some(active_name) = self.manager.active.clone() else {
            tracing::warn!("尝试执行查询但未连接数据库");
            self.notifications.warning("请先连接数据库");
            return None;
        };
        let Some(conn) = self.manager.connections.get(&active_name) else {
            tracing::warn!(connection = %active_name, "连接配置不存在");
            self.notifications.warning("请先连接数据库");
            return None;
        };

        let config = conn.config.clone();
        let tx = self.tx.clone();

        tracing::info!(connection = %active_name, sql_length = sql.len(), "开始执行查询");

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
        self.next_query_request_id = self.next_query_request_id.wrapping_add(1);
        if self.next_query_request_id == 0 {
            self.next_query_request_id = 1;
        }
        let request_id = self.next_query_request_id;
        let mut target_tab_id = String::new();

        // 同步 SQL 到当前 Tab 并设置执行状态
        let mut previous_request_id = None;
        if let Some(tab) = self.tab_manager.get_active_mut() {
            previous_request_id = tab.pending_request_id.take();
            tab.sql = sql.clone();
            tab.executing = true;
            tab.pending_request_id = Some(request_id);
            tab.update_title();
            target_tab_id = tab.id.clone();
        }
        if let Some(prev_request_id) = previous_request_id {
            self.cancel_query_request(prev_request_id);
        }
        self.refresh_executing_flag();

        let tx_tab_id = target_tab_id;
        let task_conn_name = active_name.clone();
        let tx_conn_name = active_name;
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let cancel_sender = Arc::new(Mutex::new(Some(cancel_tx)));
        let timeout_cancel_sender = Arc::clone(&cancel_sender);
        let query_task = self.runtime.spawn(async move {
            use tokio::time::{Duration, sleep, timeout};
            let start = Instant::now();
            let timeout_secs = constants::database::QUERY_TIMEOUT_SECS;
            let sql_for_exec = sql.clone();
            let mut execute_fut =
                std::pin::pin!(execute_query_cancellable(&config, &sql_for_exec, cancel_rx));
            let mut timeout_fut = std::pin::pin!(sleep(Duration::from_secs(timeout_secs)));

            let query_result = tokio::select! {
                result = &mut execute_fut => result.map_err(|e| e.to_string()),
                _ = &mut timeout_fut => {
                    if let Some(sender) = timeout_cancel_sender.lock().take() {
                        let _ = sender.send(());
                    }

                    // 继续短暂轮询执行 future，让数据库侧取消请求有机会发送/生效。
                    let _ = timeout(
                        Duration::from_secs(constants::database::QUERY_CANCEL_GRACE_SECS),
                        &mut execute_fut,
                    )
                    .await;

                    Err(format!(
                        "查询超时 ({}秒)。建议: 1) 添加 LIMIT 限制结果集 2) 优化查询条件 3) 检查索引",
                        timeout_secs
                    ))
                }
            };
            let elapsed_ms = start.elapsed().as_millis() as u64;
            match &query_result {
                Ok(res) => {
                    tracing::info!(rows = res.rows.len(), columns = res.columns.len(), elapsed_ms, "查询执行成功");
                }
                Err(err) => {
                    if err.starts_with("查询超时") {
                        tracing::error!(timeout_secs, elapsed_ms, "查询执行超时");
                    } else {
                        tracing::error!(error = %err, elapsed_ms, "查询执行失败");
                    }
                }
            }
            if tx
                .send(Message::QueryDone(
                    sql,
                    tx_conn_name,
                    tx_tab_id,
                    request_id,
                    query_result,
                    elapsed_ms,
                ))
                .is_err()
            {
                tracing::warn!("无法发送查询结果：接收端已关闭");
            }
        });
        self.track_query_task(request_id, task_conn_name, query_task, cancel_sender);

        Some(request_id)
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
        let conn_config = self.manager.connections.get(name).map(|c| c.config.clone());
        let friendly = conn_config
            .as_ref()
            .map(|cfg| Self::friendly_connection_error(cfg, &error))
            .unwrap_or_else(|| format!("连接失败：{}", error));

        self.notifications.error(friendly);
        if let Some(config) = conn_config {
            self.welcome_setup_target = config.db_type;
            self.show_welcome_setup_dialog = true;
            self.notifications.info(format!(
                "已打开 {} 安装与初始化引导",
                config.db_type.display_name()
            ));
        }
        self.autocomplete.clear();
        if let Some(conn) = self.manager.connections.get_mut(name) {
            conn.set_error(error);
        }
    }

    fn friendly_connection_error(config: &ConnectionConfig, raw_error: &str) -> String {
        let err = raw_error.trim();
        let lower = err.to_ascii_lowercase();

        match config.db_type {
            DatabaseType::SQLite => {
                if lower.contains("unable to open database file")
                    || lower.contains("no such file")
                    || lower.contains("permission denied")
                {
                    return "连接失败：SQLite 文件不可访问。请检查文件路径是否存在、目录权限是否允许读写。".to_string();
                }
                format!("连接失败：{}。请先确认 SQLite 文件路径可用。", err)
            }
            DatabaseType::PostgreSQL | DatabaseType::MySQL => {
                if lower.contains("timeout") || lower.contains("超时") {
                    return format!(
                        "连接超时：无法访问 {}:{}。请先确认数据库服务已启动、防火墙未拦截、主机端口填写正确。",
                        config.host, config.port
                    );
                }
                if lower.contains("refused")
                    || lower.contains("can't connect")
                    || lower.contains("could not connect")
                {
                    return format!(
                        "连接被拒绝：{}:{} 未接受连接。通常是数据库服务未启动，或端口配置错误。",
                        config.host, config.port
                    );
                }
                if lower.contains("access denied")
                    || lower.contains("authentication failed")
                    || lower.contains("password authentication failed")
                {
                    return format!(
                        "认证失败：用户名或密码不正确（当前用户：{}）。请检查账号密码并重试。",
                        if config.username.is_empty() {
                            "<未填写>"
                        } else {
                            &config.username
                        }
                    );
                }
                if lower.contains("unknown database")
                    || lower.contains("does not exist")
                    || lower.contains("database") && lower.contains("不存在")
                {
                    return format!(
                        "目标数据库不存在：{}。请先初始化数据库，或改用已存在的数据库名。",
                        if config.database.is_empty() {
                            "<未填写>"
                        } else {
                            &config.database
                        }
                    );
                }
                format!(
                    "连接失败：{}。请检查主机、端口、账号密码以及数据库服务状态。",
                    err
                )
            }
        }
    }
}
