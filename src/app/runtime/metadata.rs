//! 侧边栏元数据加载
//!
//! 包含触发器、存储过程等按需异步加载逻辑。

use crate::core::constants;
use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};

use super::{DbManagerApp, Message};

impl DbManagerApp {
    /// 加载当前数据库的触发器
    pub(in crate::app) fn load_triggers(&mut self) {
        if let Some(active_name) = self.session.manager.active.clone()
            && let Some(conn) = self.session.manager.connections.get(&active_name)
        {
            let config = conn.config.clone();
            let database = conn.selected_database.clone();
            let connection_id = conn.id;
            let tx = self.session.tx.clone();

            // TaskRegistry 注册；完成事件统一走 RuntimeEvent。
            let meta_key = crate::session::task_registry::OperationKey::Metadata {
                connection: connection_id,
                scope: crate::session::task_registry::MetadataScope::Triggers,
            };
            let (task_id, cancel_token) = self.session.task_registry.register(
                meta_key.clone(),
                crate::session::task_registry::TaskKind::Metadata,
            );

            self.state.sidebar_panel_state.loading_triggers = true;
            self.state.sidebar_panel_state.clear_triggers();
            self.session.pending_triggers_request = Some((active_name.clone(), database.clone()));

            let token_for_attach = cancel_token.clone();
            let meta_key_for_attach = meta_key.clone();
            let handle = self.session.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};

                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = tokio::select! {
                    result = timeout(
                        Duration::from_secs(timeout_secs),
                        crate::data::get_triggers(&config),
                    ) => result
                        .map_err(|_| format!("加载触发器超时 ({}秒)", timeout_secs))
                        .and_then(|r| r.map_err(|e| e.to_string())),
                    _ = cancel_token.cancelled() => Err("加载触发器已取消".to_string()),
                };

                let _ = tx.send(Message::RuntimeEvent(RuntimeEvent {
                    task_id,
                    key: meta_key,
                    outcome: RuntimeOutcome::TriggersFetched {
                        connection: connection_id,
                        database: database.clone(),
                        result,
                    },
                }));
            });
            self.session.task_registry.attach(
                task_id,
                meta_key_for_attach,
                crate::session::task_registry::TaskKind::Metadata,
                handle,
                token_for_attach,
            );
        }
    }
    pub(in crate::app) fn load_routines(&mut self) {
        if let Some(active_name) = self.session.manager.active.clone()
            && let Some(conn) = self.session.manager.connections.get(&active_name)
        {
            let config = conn.config.clone();
            let database = conn.selected_database.clone();
            let connection_id = conn.id;
            let tx = self.session.tx.clone();

            // TaskRegistry 注册；完成事件统一走 RuntimeEvent。
            let meta_key = crate::session::task_registry::OperationKey::Metadata {
                connection: connection_id,
                scope: crate::session::task_registry::MetadataScope::Routines,
            };
            let (task_id, cancel_token) = self.session.task_registry.register(
                meta_key.clone(),
                crate::session::task_registry::TaskKind::Metadata,
            );

            self.state.sidebar_panel_state.loading_routines = true;
            self.state.sidebar_panel_state.clear_routines();
            self.session.pending_routines_request = Some((active_name.clone(), database.clone()));

            let token_for_attach = cancel_token.clone();
            let meta_key_for_attach = meta_key.clone();
            let handle = self.session.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};

                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = tokio::select! {
                    result = timeout(
                        Duration::from_secs(timeout_secs),
                        crate::data::get_routines(&config),
                    ) => result
                        .map_err(|_| format!("加载存储过程超时 ({}秒)", timeout_secs))
                        .and_then(|r| r.map_err(|e| e.to_string())),
                    _ = cancel_token.cancelled() => Err("加载存储过程已取消".to_string()),
                };
                let _ = tx.send(Message::RuntimeEvent(RuntimeEvent {
                    task_id,
                    key: meta_key,
                    outcome: RuntimeOutcome::RoutinesFetched {
                        connection: connection_id,
                        database: database.clone(),
                        result,
                    },
                }));
            });
            self.session.task_registry.attach(
                task_id,
                meta_key_for_attach,
                crate::session::task_registry::TaskKind::Metadata,
                handle,
                token_for_attach,
            );
        }
    }
}
