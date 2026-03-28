//! 查询与请求生命周期管理
//!
//! 负责请求 ID、执行状态、取消信号与 pending 任务清理。

use parking_lot::Mutex;
use std::sync::Arc;

use super::DbManagerApp;

impl DbManagerApp {
    /// 检查是否有任何模态对话框打开
    /// 用于在对话框打开时禁用其他区域的键盘响应
    pub(super) fn has_modal_dialog_open(&self) -> bool {
        self.show_connection_dialog
            || self.show_export_dialog
            || self.show_import_dialog
            || self.show_delete_confirm
            || self.show_help
            || self.show_about
            || self.show_welcome_setup_dialog
            || self.show_history_panel
            || self.ddl_dialog_state.show
            || self.create_db_dialog_state.show
            || self.create_user_dialog_state.show
            || self.keybindings_dialog_state.show
    }

    /// 从当前活动 Tab 同步 SQL 和结果到主视图
    pub(super) fn sync_from_active_tab(&mut self) {
        if let Some(tab) = self.tab_manager.get_active() {
            self.sql = tab.sql.clone();
            self.result = tab.result.clone();
        }
    }

    /// 将当前编辑中的 SQL 草稿同步回活动 Tab
    pub(super) fn sync_sql_to_active_tab(&mut self) {
        if let Some(tab) = self.tab_manager.get_active_mut()
            && tab.sql != self.sql
        {
            tab.sql = self.sql.clone();
            tab.modified = !self.sql.trim().is_empty();
            tab.update_title();
        }
    }

    /// 根据导入状态和 Tab 执行状态刷新全局 executing 标记
    pub(super) fn refresh_executing_flag(&mut self) {
        let query_executing = self.tab_manager.tabs.iter().any(|t| t.executing);
        self.executing = self.import_executing || query_executing;
    }

    /// 生成新的连接请求 ID
    pub(super) fn next_connect_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_connect_request_id)
    }

    /// 生成新的元数据请求 ID
    pub(super) fn next_metadata_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_metadata_request_id)
    }

    /// 根据当前活动连接的 pending 请求刷新 connecting 标记
    pub(super) fn refresh_connecting_flag(&mut self) {
        let has_pending = self.manager.active.as_ref().is_some_and(|active_name| {
            self.pending_connect_requests.contains_key(active_name)
                || self.pending_database_requests.contains_key(active_name)
        });
        self.connecting = has_pending;
    }

    /// 记录进行中的查询任务
    pub(super) fn track_query_task(
        &mut self,
        request_id: u64,
        conn_name: String,
        handle: tokio::task::JoinHandle<()>,
        cancel_sender: Arc<Mutex<Option<tokio::sync::oneshot::Sender<()>>>>,
    ) {
        if let Some(prev_handle) = self.pending_query_tasks.insert(request_id, handle) {
            prev_handle.abort();
        }
        self.pending_query_connections.insert(request_id, conn_name);
        self.pending_query_cancellers
            .insert(request_id, cancel_sender);
    }

    /// 查询完成后清理任务跟踪
    pub(super) fn finalize_query_task(&mut self, request_id: u64) {
        self.pending_query_tasks.remove(&request_id);
        self.pending_query_connections.remove(&request_id);
        self.pending_query_cancellers.remove(&request_id);
    }

    /// 取消指定查询请求
    pub(super) fn cancel_query_request(&mut self, request_id: u64) {
        let cancel_sent = self
            .pending_query_cancellers
            .remove(&request_id)
            .is_some_and(|sender| {
                sender
                    .lock()
                    .take()
                    .is_some_and(|cancel| cancel.send(()).is_ok())
            });
        if !cancel_sent && let Some(handle) = self.pending_query_tasks.remove(&request_id) {
            handle.abort();
        }
        self.pending_query_connections.remove(&request_id);
        self.pending_drop_requests.remove(&request_id);
        self.clear_tab_pending_request(request_id);
        self.refresh_executing_flag();
    }

    /// 取消某个连接关联的所有查询请求
    pub(super) fn cancel_queries_for_connection(&mut self, conn_name: &str) {
        let request_ids: Vec<u64> = self
            .pending_query_connections
            .iter()
            .filter_map(|(request_id, request_conn)| {
                if request_conn == conn_name {
                    Some(*request_id)
                } else {
                    None
                }
            })
            .collect();

        for request_id in request_ids {
            self.cancel_query_request(request_id);
        }
    }

    fn clear_tab_pending_request(&mut self, request_id: u64) {
        for tab in &mut self.tab_manager.tabs {
            if tab.pending_request_id == Some(request_id) {
                tab.pending_request_id = None;
                tab.executing = false;
            }
        }
    }

    fn next_nonzero_request_id(counter: &mut u64) -> u64 {
        *counter = counter.wrapping_add(1);
        if *counter == 0 {
            *counter = 1;
        }
        *counter
    }
}
