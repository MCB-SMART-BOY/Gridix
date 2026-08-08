//! 侧边栏元数据加载
//!
//! 包含触发器、存储过程等按需异步加载逻辑。

use crate::core::constants;

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
            let request_id = self.session.next_metadata_request_id();
            let tx = self.session.tx.clone();

            // TaskRegistry 注册（双通道迁移）
            let meta_key = crate::session::task_registry::OperationKey::Metadata {
                connection: connection_id,
                scope: crate::session::task_registry::MetadataScope::Triggers,
            };
            let (task_id, _cancel_token) = self.session.task_registry.register(
                meta_key.clone(),
                crate::session::task_registry::TaskKind::Metadata,
            );

            self.state.sidebar_panel_state.loading_triggers = true;
            self.state.sidebar_panel_state.clear_triggers();
            self.session.pending_triggers_request =
                Some((active_name.clone(), database.clone(), request_id));

            self.session.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};

                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = timeout(
                    Duration::from_secs(timeout_secs),
                    crate::data::get_triggers(&config),
                )
                .await
                .map_err(|_| format!("加载触发器超时 ({}秒)", timeout_secs))
                .and_then(|r| r.map_err(|e| e.to_string()));

                // RuntimeEvent path — send first to avoid move conflicts
                {
                    use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
                    let _ = tx.send(Message::RuntimeEvent(RuntimeEvent {
                        task_id,
                        key: meta_key,
                        outcome: RuntimeOutcome::TriggersFetched {
                            connection: connection_id,
                            database: database.clone(),
                            result: result.clone(),
                        },
                    }));
                }

                // Legacy path
                let legacy_msg =
                    Message::TriggersFetched(active_name, database, request_id, result);
                if tx.send(legacy_msg).is_err() {
                    tracing::warn!("无法发送触发器数据：接收端已关闭");
                }
            });
        }
    }
    pub(in crate::app) fn load_routines(&mut self) {
        if let Some(active_name) = self.session.manager.active.clone()
            && let Some(conn) = self.session.manager.connections.get(&active_name)
        {
            let config = conn.config.clone();
            let database = conn.selected_database.clone();
            let connection_id = conn.id;
            let request_id = self.session.next_metadata_request_id();
            let tx = self.session.tx.clone();

            // TaskRegistry 注册（双通道迁移）
            let meta_key = crate::session::task_registry::OperationKey::Metadata {
                connection: connection_id,
                scope: crate::session::task_registry::MetadataScope::Routines,
            };
            let (task_id, _cancel_token) = self.session.task_registry.register(
                meta_key.clone(),
                crate::session::task_registry::TaskKind::Metadata,
            );

            self.state.sidebar_panel_state.loading_routines = true;
            self.state.sidebar_panel_state.clear_routines();
            self.session.pending_routines_request =
                Some((active_name.clone(), database.clone(), request_id));

            self.session.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};

                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = timeout(
                    Duration::from_secs(timeout_secs),
                    crate::data::get_routines(&config),
                )
                .await
                .map_err(|_| format!("加载存储过程超时 ({}秒)", timeout_secs))
                .and_then(|r| r.map_err(|e| e.to_string()));
                {
                    use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
                    let _ = tx.send(Message::RuntimeEvent(RuntimeEvent {
                        task_id,
                        key: meta_key,
                        outcome: RuntimeOutcome::RoutinesFetched {
                            connection: connection_id,
                            database: database.clone(),
                            result: result.clone(),
                        },
                    }));
                }

                // Legacy path
                let legacy_msg =
                    Message::RoutinesFetched(active_name, database, request_id, result);
                if tx.send(legacy_msg).is_err() {
                    tracing::warn!("无法发送存储过程数据：接收端已关闭");
                }
            });
        }
    }
}
