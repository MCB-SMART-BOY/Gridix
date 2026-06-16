//! 查询与请求生命周期管理
//!
//! 负责请求 ID、执行状态、取消信号与 pending 任务清理。


use super::DbManagerApp;

impl DbManagerApp {

    pub(in crate::app) fn cancel_query_request_silently(&mut self, request_id: u64) {
        self.cancel_query_request_with_visibility(request_id, false);
    }

    /// 检查是否有任何模态对话框打开
    /// 用于在对话框打开时禁用其他区域的键盘响应
    pub(in crate::app) fn has_modal_dialog_open(&self) -> bool {
        self.active_dialog_id().is_some() || self.grid_state.show_save_confirm
    }

    /// 从当前活动 Tab 同步 SQL 和结果到主视图
    pub(crate) fn sync_from_active_tab(&mut self) {
        if let Some(tab) = self.session.tab_manager.get_active() {
            self.result = tab.result.clone();
            self.session.last_query_time_ms = tab.query_time_ms;
            self.selected_table = tab.selected_table.clone();
            self.search_text = tab.search_text.clone();
            self.search_column = tab.search_column.clone();
            self.active_grid_workspace_enabled = tab.uses_grid_workspace;
        } else {
            self.session.last_query_time_ms = None;
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

    /// 更新活动 Tab 的元数据（modified 标记、标题）
    pub(in crate::app) fn sync_sql_to_active_tab(&mut self) {
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            tab.modified = !tab.sql.trim().is_empty();
            tab.update_title();
        }
    }

    /// 查询完成后清理任务跟踪
    pub(in crate::app) fn finalize_query_task(&mut self, request_id: u64) {
        self.session.pending_query_tasks.remove(&request_id);
        self.session.pending_query_connections.remove(&request_id);
        self.session.pending_query_cancellers.remove(&request_id);
    }

    /// 取消指定查询请求
    fn cancel_query_request_with_visibility(&mut self, request_id: u64, user_visible: bool) {
        let cancel_sent = self
            .session.pending_query_cancellers
            .remove(&request_id)
            .is_some_and(|sender| {
                sender
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take()
                    .is_some_and(|cancel| cancel.send(()).is_ok())
            });
        if cancel_sent && user_visible {
            self.session.user_cancelled_query_requests.insert(request_id);
        } else {
            self.session.user_cancelled_query_requests.remove(&request_id);
        }
        if !cancel_sent && let Some(handle) = self.session.pending_query_tasks.remove(&request_id) {
            handle.abort();
        }
        self.session.pending_query_connections.remove(&request_id);
        self.pending_drop_requests.remove(&request_id);
        self.clear_tab_pending_request(request_id);
        self.session.refresh_executing_flag();
    }

    /// 取消某个连接关联的所有查询请求
    pub(in crate::app) fn cancel_queries_for_connection(&mut self, conn_name: &str) {
        let request_ids: Vec<u64> = self
            .session.pending_query_connections
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
        for tab in &mut self.session.tab_manager.tabs {
            if tab.pending_request_id == Some(request_id) {
                tab.pending_request_id = None;
                tab.executing = false;
            }
        }
    }
}
