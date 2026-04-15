//! 查询与请求生命周期管理
//!
//! 负责请求 ID、执行状态、取消信号与 pending 任务清理。

use parking_lot::Mutex;
use std::sync::Arc;

use super::DbManagerApp;

impl DbManagerApp {
    #[allow(dead_code)] // 预留给显式“取消当前查询”动作路径
    pub(in crate::app) fn cancel_query_request(&mut self, request_id: u64) {
        self.cancel_query_request_with_visibility(request_id, true);
    }

    pub(in crate::app) fn cancel_query_request_silently(&mut self, request_id: u64) {
        self.cancel_query_request_with_visibility(request_id, false);
    }

    /// 检查是否有任何模态对话框打开
    /// 用于在对话框打开时禁用其他区域的键盘响应
    pub(in crate::app) fn has_modal_dialog_open(&self) -> bool {
        self.active_dialog_id().is_some() || self.grid_state.show_save_confirm
    }

    /// 从当前活动 Tab 同步 SQL 和结果到主视图
    pub(in crate::app) fn sync_from_active_tab(&mut self) {
        if let Some(tab) = self.tab_manager.get_active() {
            self.sql = tab.sql.clone();
            self.result = tab.result.clone();
            self.last_query_time_ms = tab.query_time_ms;
            self.selected_table = tab.selected_table.clone();
            self.search_text = tab.search_text.clone();
            self.search_column = tab.search_column.clone();
            self.active_grid_workspace_enabled = tab.uses_grid_workspace;
        } else {
            self.last_query_time_ms = None;
            self.selected_table = None;
            self.search_text.clear();
            self.search_column = None;
            self.active_grid_workspace_enabled = false;
        }
        self.selected_row = None;
        self.selected_cell = None;
        self.restore_grid_surface_from_active_tab();
    }

    /// 在切换/打开其它 Tab 前持久化当前活动 Tab 的状态
    pub(in crate::app) fn persist_active_tab_state_for_navigation(&mut self) {
        self.persist_active_grid_workspace();
        self.sync_sql_to_active_tab();
        self.sync_active_surface_binding_to_tab();
    }

    /// 将当前编辑中的 SQL 草稿同步回活动 Tab
    pub(in crate::app) fn sync_sql_to_active_tab(&mut self) {
        if let Some(tab) = self.tab_manager.get_active_mut()
            && tab.sql != self.sql
        {
            tab.sql = self.sql.clone();
            tab.modified = !self.sql.trim().is_empty();
            tab.update_title();
        }
    }

    /// 根据导入状态和 Tab 执行状态刷新全局 executing 标记
    pub(in crate::app) fn refresh_executing_flag(&mut self) {
        let query_executing = self.tab_manager.tabs.iter().any(|t| t.executing);
        let grid_save_executing = !self.pending_grid_save_requests.is_empty();
        self.executing = self.import_executing || query_executing || grid_save_executing;
    }

    /// 生成新的连接请求 ID
    pub(in crate::app) fn next_connect_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_connect_request_id)
    }

    /// 生成新的元数据请求 ID
    pub(in crate::app) fn next_metadata_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_metadata_request_id)
    }

    /// 生成新的表格保存请求 ID
    pub(in crate::app) fn next_grid_save_request_id(&mut self) -> u64 {
        Self::next_nonzero_request_id(&mut self.next_grid_save_request_id)
    }

    /// 根据当前活动连接的 pending 请求刷新 connecting 标记
    pub(in crate::app) fn refresh_connecting_flag(&mut self) {
        let has_pending = self.manager.active.as_ref().is_some_and(|active_name| {
            self.pending_connect_requests.contains_key(active_name)
                || self.pending_database_requests.contains_key(active_name)
        });
        self.connecting = has_pending;
    }

    /// 记录进行中的查询任务
    pub(in crate::app) fn track_query_task(
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
    pub(in crate::app) fn finalize_query_task(&mut self, request_id: u64) {
        self.pending_query_tasks.remove(&request_id);
        self.pending_query_connections.remove(&request_id);
        self.pending_query_cancellers.remove(&request_id);
    }

    /// 取消指定查询请求
    fn cancel_query_request_with_visibility(&mut self, request_id: u64, user_visible: bool) {
        let cancel_sent = self
            .pending_query_cancellers
            .remove(&request_id)
            .is_some_and(|sender| {
                sender
                    .lock()
                    .take()
                    .is_some_and(|cancel| cancel.send(()).is_ok())
            });
        if cancel_sent && user_visible {
            self.user_cancelled_query_requests.insert(request_id);
        } else {
            self.user_cancelled_query_requests.remove(&request_id);
        }
        if !cancel_sent && let Some(handle) = self.pending_query_tasks.remove(&request_id) {
            handle.abort();
        }
        self.pending_query_connections.remove(&request_id);
        self.pending_drop_requests.remove(&request_id);
        self.clear_tab_pending_request(request_id);
        self.refresh_executing_flag();
    }

    /// 取消某个连接关联的所有查询请求
    pub(in crate::app) fn cancel_queries_for_connection(&mut self, conn_name: &str) {
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
            self.cancel_query_request_silently(request_id);
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
