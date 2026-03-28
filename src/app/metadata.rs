//! 侧边栏元数据加载
//!
//! 包含触发器、存储过程等按需异步加载逻辑。

use crate::core::constants;

use super::{DbManagerApp, Message};

impl DbManagerApp {
    /// 加载当前数据库的触发器
    pub(super) fn load_triggers(&mut self) {
        if let Some(active_name) = self.manager.active.clone()
            && let Some(conn) = self.manager.connections.get(&active_name)
        {
            let config = conn.config.clone();
            let database = conn.selected_database.clone();
            let request_id = self.next_metadata_request_id();
            let tx = self.tx.clone();

            self.sidebar_panel_state.loading_triggers = true;
            self.sidebar_panel_state.clear_triggers();
            self.pending_triggers_request =
                Some((active_name.clone(), database.clone(), request_id));

            self.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};

                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = timeout(
                    Duration::from_secs(timeout_secs),
                    crate::database::get_triggers(&config),
                )
                .await
                .map_err(|_| format!("加载触发器超时 ({}秒)", timeout_secs))
                .and_then(|r| r.map_err(|e| e.to_string()));
                let _ = tx.send(Message::TriggersFetched(
                    active_name,
                    database,
                    request_id,
                    result,
                ));
            });
        }
    }

    pub(super) fn load_routines(&mut self) {
        if let Some(active_name) = self.manager.active.clone()
            && let Some(conn) = self.manager.connections.get(&active_name)
        {
            let config = conn.config.clone();
            let database = conn.selected_database.clone();
            let request_id = self.next_metadata_request_id();
            let tx = self.tx.clone();

            self.sidebar_panel_state.loading_routines = true;
            self.sidebar_panel_state.clear_routines();
            self.pending_routines_request =
                Some((active_name.clone(), database.clone(), request_id));

            self.runtime.spawn(async move {
                use tokio::time::{Duration, timeout};

                let timeout_secs = constants::database::CONNECTION_TIMEOUT_SECS;
                let result = timeout(
                    Duration::from_secs(timeout_secs),
                    crate::database::get_routines(&config),
                )
                .await
                .map_err(|_| format!("加载存储过程超时 ({}秒)", timeout_secs))
                .and_then(|r| r.map_err(|e| e.to_string()));
                let _ = tx.send(Message::RoutinesFetched(
                    active_name,
                    database,
                    request_id,
                    result,
                ));
            });
        }
    }
}
