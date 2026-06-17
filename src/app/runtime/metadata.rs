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
            let request_id = self.session.next_metadata_request_id();
            let tx = self.session.tx.clone();

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
                if tx
                    .send(Message::TriggersFetched(
                        active_name,
                        database,
                        request_id,
                        result,
                    ))
                    .is_err()
                {
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
            let request_id = self.session.next_metadata_request_id();
            let tx = self.session.tx.clone();

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
                if tx
                    .send(Message::RoutinesFetched(
                        active_name,
                        database,
                        request_id,
                        result,
                    ))
                    .is_err()
                {
                    tracing::warn!("无法发送存储过程数据：接收端已关闭");
                }
            });
        }
    }
}
