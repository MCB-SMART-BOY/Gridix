//! 数据库操作模块
//!
//! 处理数据库连接、断开、查询执行等操作。

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use crate::app::dialogs::host::DialogId;
use crate::core::constants;
use crate::data::{
    ConnectResult, ConnectionConfig, DatabaseType, connect_database, drop_database,
    execute_import_batch, execute_query, execute_query_cancellable, get_primary_key_column,
    get_tables_for_database, ssh_tunnel::SSH_TUNNEL_MANAGER,
};
use crate::ui;

use super::DbManagerApp;
use super::message::Message;

fn prepare_tab_for_query_execution(tab: &mut crate::ui::QueryTab, sql: &str, request_id: u64) {
    tab.sql = sql.to_string();
    tab.result = None;
    tab.modified = false;
    tab.executing = true;
    tab.last_message = None;
    tab.last_error = None;
    tab.query_time_ms = None;
    tab.pending_request_id = Some(request_id);
    tab.update_title();
}

/// 连接错误是否应该打开「安装/初始化引导」。
///
/// 只有「需要安装或初始化」类错误（SQLite 文件缺失、目标数据库不存在/未初始化）才打开引导；
/// 临时性错误（超时、连接被拒、认证失败）说明环境已配置好，弹引导只会干扰已上手的用户。
/// 修复审计 CONN-F5。
fn connection_error_warrants_onboarding(db_type: DatabaseType, raw_error: &str) -> bool {
    let lower = raw_error.trim().to_ascii_lowercase();

    // 明确属于「临时性/凭据」类的错误：不打开引导。
    let transient = lower.contains("timeout")
        || lower.contains("超时")
        || lower.contains("refused")
        || lower.contains("can't connect")
        || lower.contains("could not connect")
        || lower.contains("access denied")
        || lower.contains("authentication failed")
        || lower.contains("password authentication failed");
    if transient {
        return false;
    }

    match db_type {
        DatabaseType::SQLite => {
            // 文件不存在/不可访问 → 需要选择或创建数据库文件，引导有用。
            lower.contains("unable to open database file")
                || lower.contains("no such file")
                || lower.contains("permission denied")
        }
        DatabaseType::PostgreSQL | DatabaseType::MySQL => {
            // 目标数据库不存在/未初始化 → 引导有用。
            lower.contains("unknown database")
                || lower.contains("does not exist")
                || (lower.contains("database") && lower.contains("不存在"))
        }
    }
}

impl DbManagerApp {
    /// 打开连接编辑对话框（预填当前配置）
    pub(in crate::app) fn open_connection_editor(&mut self, name: &str) {
        if let Some(conn) = self.session.manager.connections.get(name) {
            self.state.new_config = conn.config.clone();
            self.state.editing_connection_name = Some(name.to_string());
            self.open_dialog(DialogId::Connection);
        } else {
            self.session
                .notifications
                .warning(format!("连接 '{}' 不存在", name));
        }
    }

    /// 保存连接对话框结果（新建或编辑）
    pub(in crate::app) fn save_connection_from_dialog(&mut self) {
        let config = std::mem::take(&mut self.state.new_config);
        let saved_db_type = config.db_type;
        let name = config.name.clone();

        let editing_name = self.state.editing_connection_name.clone();
        let has_name_conflict = match editing_name.as_deref() {
            Some(old_name) => {
                old_name != name && self.session.manager.connections.contains_key(&name)
            }
            None => self.session.manager.connections.contains_key(&name),
        };
        if has_name_conflict {
            self.session
                .notifications
                .error(format!("连接名 '{}' 已存在，请使用其他名称", name));
            self.state.new_config = config;
            self.open_dialog(DialogId::Connection);
            return;
        }

        if let Some(old_name) = self.state.editing_connection_name.take() {
            if self.session.manager.connections.contains_key(&old_name) {
                self.disconnect(old_name.clone());
                self.session.manager.connections.remove(&old_name);
            }

            if old_name != name {
                if let Some(history) = self.app_config.command_history.remove(&old_name) {
                    self.app_config
                        .command_history
                        .insert(name.clone(), history);
                }
                if self.session.current_history_connection.as_deref() == Some(&old_name) {
                    self.session.current_history_connection = Some(name.clone());
                }
            }

            self.session
                .notifications
                .success(format!("连接 '{}' 已更新", name));
        }

        self.session.manager.add(config);
        self.mark_onboarding_connection_created();
        if !self.welcome_onboarding_status().is_complete() {
            self.open_welcome_setup_dialog(saved_db_type);
        }
        self.save_config_debounced();
        self.connect(name);
    }

    /// 连接到数据库
    pub(in crate::app) fn connect(&mut self, name: String) {
        if let Some(conn) = self.session.manager.connections.get(&name) {
            let config = conn.config.clone();
            let tx = self.session.tx.clone();
            let request_id = self.session.next_connect_request_id();

            self.session.manager.active = Some(name.clone());
            self.session
                .pending_connect_requests
                .insert(name.clone(), request_id);
            self.session.pending_database_requests.remove(&name);
            self.session.pending_triggers_request = None;
            self.session.pending_routines_request = None;
            self.state.sidebar_panel_state.loading_triggers = false;
            self.state.sidebar_panel_state.loading_routines = false;
            self.state.sidebar_panel_state.clear_triggers();
            self.state.sidebar_panel_state.clear_routines();
            self.session.refresh_connecting_flag();

            tracing::info!(connection = %name, db_type = ?config.db_type, "开始连接数据库");

            self.session.runtime.spawn(async move {
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
                            crate::data::DatabaseType::SQLite => {
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

    /// 静默重新拉取当前活动连接的表列表（schema 变更后失效重载用）。
    ///
    /// 与 `connect()`/`select_database()` 不同：不发"已连接/已选库"提示，
    /// 只在回包里静默刷新表列表与 autocomplete，避免每次 DDL 都弹连接提示。
    /// 复用 `next_connect_request_id` 与 pending_database_requests 的 stale-guard。
    pub(in crate::app) fn reload_active_tables(&mut self) {
        let Some(active_name) = self.session.manager.active.clone() else {
            return;
        };
        let Some(conn) = self.session.manager.connections.get(&active_name) else {
            return;
        };
        let config = conn.config.clone();
        // SQLite 用文件路径作为库名；多库用 selected_database。
        let database = conn
            .selected_database
            .clone()
            .unwrap_or_else(|| config.database.clone());
        let tx = self.session.tx.clone();
        let request_id = self.session.next_connect_request_id();

        self.session.runtime.spawn(async move {
            use tokio::time::{Duration, timeout};
            let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
            let result = timeout(
                Duration::from_secs(timeout_secs),
                get_tables_for_database(&config, &database),
            )
            .await;
            let tables_result = match result {
                Ok(Ok(tables)) => Ok(tables),
                Ok(Err(e)) => Err(e.to_string()),
                Err(_) => Err(format!("刷新表列表超时 ({}秒)", timeout_secs)),
            };
            if tx
                .send(Message::ActiveTablesReloaded(
                    active_name,
                    request_id,
                    tables_result,
                ))
                .is_err()
            {
                tracing::warn!("无法发送表列表刷新结果：接收端已关闭");
            }
        });
    }

    /// 选择数据库（MySQL/PostgreSQL）
    pub(in crate::app) fn select_database(&mut self, database: String) {
        let Some(active_name) = self.session.manager.active.clone() else {
            return;
        };
        let Some(conn) = self.session.manager.connections.get(&active_name) else {
            return;
        };
        let config = conn.config.clone();
        let tx = self.session.tx.clone();
        let request_id = self.session.next_connect_request_id();

        self.session
            .pending_database_requests
            .insert(active_name.clone(), (database.clone(), request_id));
        self.session.pending_triggers_request = None;
        self.session.pending_routines_request = None;
        self.state.sidebar_panel_state.loading_triggers = false;
        self.state.sidebar_panel_state.loading_routines = false;
        self.state.sidebar_panel_state.clear_triggers();
        self.state.sidebar_panel_state.clear_routines();
        self.session.refresh_connecting_flag();

        self.session.runtime.spawn(async move {
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
    pub(in crate::app) fn disconnect(&mut self, name: String) {
        // 清理 SSH 隧道和连接池
        if let Some(conn) = self.session.manager.connections.get(&name) {
            let config = conn.config.clone();
            let handle = self.session.runtime.handle().clone();

            // 停止关联的 SSH 隧道
            if config.ssh_config.enabled {
                let tunnel_name = config.ssh_config.tunnel_name();
                handle.spawn(async move {
                    SSH_TUNNEL_MANAGER.stop(&tunnel_name).await;
                });
            }

            // 清理连接池
            handle.spawn(async move {
                crate::data::POOL_MANAGER.remove_pool(&config).await;
            });
        }

        self.session.manager.disconnect(&name);
        self.cancel_queries_for_connection(&name);
        self.session.pending_connect_requests.remove(&name);
        self.session.pending_database_requests.remove(&name);
        // Only clear metadata requests belonging to the disconnecting connection
        if self
            .session
            .pending_triggers_request
            .as_ref()
            .is_some_and(|(cn, _, _)| cn == &name)
        {
            self.session.pending_triggers_request = None;
        }
        if self
            .session
            .pending_routines_request
            .as_ref()
            .is_some_and(|(cn, _, _)| cn == &name)
        {
            self.session.pending_routines_request = None;
        }
        self.state
            .pending_drop_requests
            .retain(|_, (conn_name, _)| conn_name != &name);
        self.remove_grid_workspaces_for_connection(&name);
        if self.session.manager.active.as_deref() == Some(&name) {
            self.session.manager.active = None;
            self.switch_grid_workspace(None);
            self.clear_result();
            self.session.autocomplete.clear();
            self.state.sidebar_panel_state.clear_triggers();
            self.state.sidebar_panel_state.clear_routines();
            self.state.sidebar_panel_state.loading_triggers = false;
            self.state.sidebar_panel_state.loading_routines = false;
            // 清除 ER 图并使在途 ER 回包失效（审计 CONN-F1 / B6-ER）。
            self.state.er_diagram_state.clear();
            self.state.er_diagram_state.loading = false;
        }
        self.session.refresh_connecting_flag();
    }

    /// 删除连接配置
    pub(in crate::app) fn delete_connection(&mut self, name: &str) {
        let was_active = self.session.manager.active.as_deref() == Some(name);
        if let Some(password_ref) = self
            .session
            .manager
            .connections
            .get(name)
            .and_then(|connection| connection.config.password_ref.clone())
            && let Err(e) = crate::data::delete_password_secret(&password_ref)
        {
            tracing::warn!(%e, password_ref = %password_ref, "删除密钥环中的密码失败");
        }
        if self.session.manager.connections.contains_key(name) {
            self.disconnect(name.to_string());
            self.session.manager.connections.remove(name);
        }
        // 删除该连接的历史记录
        self.app_config.command_history.remove(name);
        // 如果删除的是当前连接，清空当前状态
        if was_active {
            self.session.manager.active = None;
            self.switch_grid_workspace(None);
            self.clear_result();
            self.session.command_history.clear();
            self.session.current_history_connection = None;
        }
        self.save_config_debounced();
    }

    /// 删除数据库（执行 DROP DATABASE）。
    pub(in crate::app) fn delete_database(&mut self, connection_name: &str, database: &str) {
        let Some(conn) = self.session.manager.connections.get(connection_name) else {
            self.session.notifications.warning("目标连接已失效");
            return;
        };
        if !conn.connected {
            self.session.notifications.warning("请先连接数据库");
            return;
        };
        if matches!(conn.config.db_type, crate::data::DatabaseType::SQLite) {
            self.session
                .notifications
                .warning("SQLite 不支持独立删除数据库；请删除连接或数据库文件");
            return;
        }

        let config = conn.config.clone();
        let tx = self.session.tx.clone();
        let connection_name = connection_name.to_string();
        let database_name = database.to_string();
        let remove_active_pool = conn.selected_database.as_deref() == Some(database);

        self.session.runtime.spawn(async move {
            let result = drop_database(&config, &database_name)
                .await
                .map_err(|e| e.to_string());
            if result.is_ok() && remove_active_pool {
                crate::data::POOL_MANAGER.remove_pool(&config).await;
            }
            if tx
                .send(Message::DatabaseDropped(
                    connection_name,
                    database_name,
                    result,
                ))
                .is_err()
            {
                tracing::warn!("无法发送数据库删除结果：接收端已关闭");
            }
        });
    }

    /// 删除表（执行 DROP TABLE）
    pub(in crate::app) fn delete_table(&mut self, connection_name: &str, table: &str) {
        let Some(conn) = self.session.manager.connections.get(connection_name) else {
            self.session.notifications.warning("目标连接已失效");
            return;
        };
        if !conn.connected {
            self.session.notifications.warning("请先连接数据库");
            return;
        }
        let target_connection = conn.config.name.clone();

        let use_backticks = matches!(conn.config.db_type, crate::data::DatabaseType::MySQL);
        let quoted_table = match ui::quote_identifier(table, use_backticks) {
            Ok(name) => name,
            Err(e) => {
                self.session.notifications.error(format!("表名无效: {}", e));
                return;
            }
        };
        let config = conn.config.clone();
        let tx = self.session.tx.clone();
        let table_name = table.to_string();
        let sql = format!("DROP TABLE {};", quoted_table);

        self.session.runtime.spawn(async move {
            let result = execute_query(&config, &sql)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
            if tx
                .send(Message::TableDropped(target_connection, table_name, result))
                .is_err()
            {
                tracing::warn!("无法发送表删除结果：接收端已关闭");
            }
        });
    }

    /// 执行 SQL 查询
    pub(in crate::app) fn execute(&mut self, sql: String) -> Option<u64> {
        if sql.trim().is_empty() {
            tracing::debug!("SQL 为空，跳过执行");
            return None;
        }

        // 提前检查连接状态
        let Some(active_name) = self.session.manager.active.clone() else {
            tracing::warn!("尝试执行查询但未连接数据库");
            self.session.notifications.warning("请先连接数据库");
            return None;
        };
        let Some(conn) = self.session.manager.connections.get(&active_name) else {
            tracing::warn!(connection = %active_name, "连接配置不存在");
            self.session.notifications.warning("请先连接数据库");
            return None;
        };

        let config = conn.config.clone();
        let tx = self.session.tx.clone();

        tracing::info!(connection = %active_name, sql_length = sql.len(), "开始执行查询");

        // 添加到命令历史
        if self.session.command_history.first() != Some(&sql) {
            self.session.command_history.insert(0, sql.clone());
            // 限制每个连接最多保存历史记录
            if self.session.command_history.len()
                > constants::history::MAX_COMMAND_HISTORY_PER_CONNECTION
            {
                self.session.command_history.pop();
            }
            // 保存历史记录到配置文件
            self.save_current_history();
            if let Err(e) = self.app_config.save() {
                tracing::warn!(%e, "保存配置失败");
            }
        }
        self.session.history_index = None;

        self.session.executing = true;
        self.clear_result();
        self.session.last_query_time_ms = None;
        let request_id = self.session.next_query_request_id();
        let mut target_tab_id = String::new();

        // 同步 SQL 到当前 Tab 并设置执行状态
        let mut previous_request_id = None;
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            previous_request_id = tab.pending_request_id.take();
            prepare_tab_for_query_execution(tab, &sql, request_id);
            target_tab_id = tab.id.clone();
        }
        if let Some(prev_request_id) = previous_request_id {
            self.cancel_query_request_silently(prev_request_id);
        }
        self.session.refresh_executing_flag();

        let tx_tab_id = target_tab_id;
        let task_conn_name = active_name.clone();
        let tx_conn_name = active_name;
        let (cancel_tx, cancel_rx) = tokio::sync::oneshot::channel();
        let cancel_sender = Arc::new(Mutex::new(Some(cancel_tx)));
        let timeout_cancel_sender = Arc::clone(&cancel_sender);
        let query_task = self.session.runtime.spawn(async move {
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
                    if let Some(sender) = timeout_cancel_sender.lock().unwrap_or_else(|e| e.into_inner()).take() {
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
        self.session
            .track_query_task(request_id, task_conn_name, query_task, cancel_sender);

        Some(request_id)
    }

    /// 异步获取表的主键列
    pub(in crate::app) fn fetch_primary_key(&self, table_name: &str) {
        let Some(conn) = self.session.manager.get_active() else {
            return;
        };

        let config = conn.config.clone();
        let table = table_name.to_string();
        let tx = self.session.tx.clone();

        self.session.runtime.spawn(async move {
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
    pub(in crate::app) fn handle_connection_error(&mut self, name: &str, error: String) {
        let conn_config = self
            .session
            .manager
            .connections
            .get(name)
            .map(|c| c.config.clone());
        let friendly = conn_config
            .as_ref()
            .map(|cfg| Self::friendly_connection_error(cfg, &error))
            .unwrap_or_else(|| format!("连接失败：{}", error));

        self.session.notifications.error(friendly);
        if let Some(config) = conn_config {
            // 只在「需要安装/初始化」类错误时打开引导，避免临时超时/认证失败也弹向导（修复审计 CONN-F5）。
            if connection_error_warrants_onboarding(config.db_type, &error) {
                self.open_welcome_setup_dialog(config.db_type);
                self.session.notifications.info(format!(
                    "已打开 {} 安装与初始化引导",
                    config.db_type.display_name()
                ));
            }
        }
        self.session.autocomplete.clear();
        if let Some(conn) = self.session.manager.connections.get_mut(name) {
            conn.set_error(error);
        }
        // 清除僵尸 active：连接失败后 manager.active 不能继续指向这个已断开的连接，
        // 否则 execute()/select_database() 等会对一个 connected=false 的连接静默操作（修复审计 B5）。
        if self.session.manager.active.as_deref() == Some(name) {
            self.session.manager.active = None;
            self.session.refresh_connecting_flag();
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

    /// 将网格编辑保存为一个事务批次。
    ///
    /// 所有 UPDATE/DELETE/INSERT 语句通过 `execute_import_batch` 以单事务提交：
    /// 要么全部成功，要么整体回滚。成功回包后由 `handle_grid_save_done` 清除编辑状态
    /// 并刷新该表；失败时保留编辑（事务已回滚，DB 未变）。
    ///
    /// 修复审计 BLOCKER B1（保存后清编辑）、B2（原子性）、B3（按 db_type 引用标识符）。
    pub(in crate::app) fn execute_grid_save(&mut self, table: String, statements: Vec<String>) {
        if statements.is_empty() {
            return;
        }

        let Some(active_name) = self.session.manager.active.clone() else {
            self.session.notifications.warning("请先连接数据库");
            return;
        };
        let Some(conn) = self.session.manager.connections.get(&active_name) else {
            self.session.notifications.warning("请先连接数据库");
            return;
        };
        let config = conn.config.clone();

        let request_id = self.session.next_query_request_id();
        self.session.pending_grid_save_request = Some(request_id);

        let tx = self.session.tx.clone();
        self.session.grid_save_executing = true;
        self.session.refresh_executing_flag();
        self.session.last_query_time_ms = None;

        let statement_count = statements.len();

        self.session.runtime.spawn(async move {
            let start = Instant::now();
            // use_transaction=true（原子性），stop_on_error=true（首错即回滚）
            let result = execute_import_batch(&config, statements, true, true)
                .await
                .map_err(|e| e.to_string());
            let elapsed_ms = start.elapsed().as_millis() as u64;

            if tx
                .send(Message::GridSaveDone {
                    result,
                    table,
                    request_id,
                    elapsed_ms,
                })
                .is_err()
            {
                tracing::warn!("无法发送网格保存结果：接收端已关闭");
            }
        });

        tracing::info!(connection = %active_name, statement_count, "开始网格保存批次");
    }
}

#[cfg(test)]
mod tests {
    use super::connection_error_warrants_onboarding;
    use super::prepare_tab_for_query_execution;

    #[test]
    fn onboarding_opens_only_for_setup_errors_not_transient() {
        // 审计 CONN-F5：临时/凭据错误不弹引导；安装/初始化类才弹。
        // 临时性 → 不打开
        assert!(!connection_error_warrants_onboarding(
            DatabaseType::PostgreSQL,
            "connection timeout after 10s"
        ));
        assert!(!connection_error_warrants_onboarding(
            DatabaseType::MySQL,
            "connection refused"
        ));
        assert!(!connection_error_warrants_onboarding(
            DatabaseType::PostgreSQL,
            "password authentication failed for user"
        ));
        // 需要初始化 → 打开
        assert!(connection_error_warrants_onboarding(
            DatabaseType::MySQL,
            "Unknown database 'shop'"
        ));
        assert!(connection_error_warrants_onboarding(
            DatabaseType::PostgreSQL,
            "database \"shop\" does not exist"
        ));
        assert!(connection_error_warrants_onboarding(
            DatabaseType::SQLite,
            "unable to open database file"
        ));
        // 即便 SQLite 文件类，但若是超时也不弹（临时优先）。
        assert!(!connection_error_warrants_onboarding(
            DatabaseType::SQLite,
            "timeout opening file"
        ));
    }
    use crate::app::DbManagerApp;
    use crate::data::{ConnectionConfig, DatabaseType, QueryResult};
    use crate::ui::QueryTab;

    #[test]
    fn connection_error_clears_zombie_active() {
        // 审计 B5：连接失败后 manager.active 不能继续指向失败的连接。
        let mut app = DbManagerApp::new_for_test();
        let config = ConnectionConfig {
            name: "broken".to_string(),
            db_type: DatabaseType::SQLite,
            ..Default::default()
        };
        app.session.manager.add(config);
        app.session.manager.active = Some("broken".to_string());

        app.handle_connection_error("broken", "connection refused".to_string());

        assert_eq!(
            app.session.manager.active, None,
            "active must be cleared after the active connection fails"
        );
    }

    #[test]
    fn connection_error_for_inactive_connection_keeps_active() {
        // 仅清除失败的那个 active；其它连接保持 active 不受影响。
        let mut app = DbManagerApp::new_for_test();
        app.session.manager.add(ConnectionConfig {
            name: "good".to_string(),
            db_type: DatabaseType::SQLite,
            ..Default::default()
        });
        app.session.manager.add(ConnectionConfig {
            name: "broken".to_string(),
            db_type: DatabaseType::SQLite,
            ..Default::default()
        });
        app.session.manager.active = Some("good".to_string());

        app.handle_connection_error("broken", "connection refused".to_string());

        assert_eq!(
            app.session.manager.active,
            Some("good".to_string()),
            "an unrelated connection's failure must not clear the active connection"
        );
    }

    #[test]
    fn cancel_active_query_is_noop_when_nothing_running() {
        // 审计 B4：没有在执行的查询时取消是安全的无操作。
        let mut app = DbManagerApp::new_for_test();
        assert!(
            !app.cancel_active_query(),
            "cancel must report false when no query is in flight"
        );
    }

    #[test]
    fn prepare_tab_for_query_execution_clears_stale_result_and_sets_request_state() {
        let mut tab = QueryTab::from_sql("select 1");
        tab.result = Some(QueryResult::with_rows(
            vec!["id".to_string()],
            vec![vec!["1".to_string()]],
        ));
        tab.last_message = Some("查询完成，返回 1 行 (3ms)".to_string());
        tab.last_error = Some("错误: previous".to_string());
        tab.pending_request_id = Some(7);
        tab.executing = false;

        prepare_tab_for_query_execution(&mut tab, "select 2", 8);

        assert_eq!(tab.sql, "select 2");
        assert!(tab.result.is_none());
        assert!(tab.last_message.is_none());
        assert!(tab.last_error.is_none());
        assert_eq!(tab.query_time_ms, None);
        assert!(tab.executing);
        assert_eq!(tab.pending_request_id, Some(8));
        assert!(!tab.modified);
    }
}
