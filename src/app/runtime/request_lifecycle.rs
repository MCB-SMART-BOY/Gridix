//! 查询与请求生命周期管理
//!
//! 负责请求 ID、执行状态、取消信号与 pending 任务清理。

use super::DbManagerApp;

impl DbManagerApp {
    pub(in crate::app) fn cancel_query_request_silently(&mut self, request_id: u64) {
        self.cancel_query_request_with_visibility(request_id, false);
    }

    /// 取消当前活动标签页正在执行的查询（用户主动取消，会显示反馈）。
    ///
    /// 修复审计 B4：取消机制原本只有 silent 入口，没有任何用户可达的取消路径。
    /// 返回是否确实存在一个在执行的查询被取消。
    pub(in crate::app) fn cancel_active_query(&mut self) -> bool {
        let Some(tab) = self.session.tab_manager.get_active() else {
            return false;
        };
        let Some(_request_id) = tab.pending_request_id else {
            return false;
        };
        // 从 Tab ID 构造与 execute() 一致的 DocumentId
        let document = uuid::Uuid::parse_str(&tab.id)
            .map(crate::domain::ids::DocumentId::from)
            .unwrap_or_else(|_| {
                crate::domain::ids::DocumentId::from(uuid::Uuid::new_v5(
                    &uuid::Uuid::NAMESPACE_OID,
                    tab.id.as_bytes(),
                ))
            });
        let key = crate::session::task_registry::OperationKey::Query { document };
        self.session.task_registry.cancel_by_key(&key);
        // 保留 request_id 清理用于旧路径
        self.session
            .user_cancelled_query_requests
            .insert(_request_id);
        self.clear_tab_pending_request(_request_id);
        self.session.notifications.warning("已取消查询");
        self.session.refresh_executing_flag();
        self.session.needs_repaint = true;
        true
    }

    /// 检查是否有任何模态对话框打开
    /// 用于在对话框打开时禁用其他区域的键盘响应
    pub(in crate::app) fn has_modal_dialog_open(&self) -> bool {
        self.active_dialog_id().is_some()
            || self.state.grid_state.show_save_confirm
            // WelcomeSetup 可能在 active_dialog_owner 尚未协调的帧里就已可见；
            // 直接把它的可见标志视为模态，避免工作区快捷键穿透覆盖层（修复审计 B8）。
            || self.state.show_welcome_setup_dialog
    }

    /// 从当前活动 Tab 同步 SQL 和结果到主视图
    pub(crate) fn sync_from_active_tab(&mut self) {
        let mut active_result_set = None;
        let mut query_bottom_panel_tab = None;
        if let Some(tab) = self.session.tab_manager.get_active() {
            active_result_set = tab.result_set.clone();
            self.session.last_query_time_ms = tab.query_time_ms;
            self.state.selected_table = tab.selected_table.clone();
            self.state.search_text = tab.search_text.clone();
            self.state.search_column = tab.search_column.clone();
            self.active_grid_workspace_enabled = tab.uses_grid_workspace;
            query_bottom_panel_tab = if tab.last_error.is_some() {
                Some(crate::core::BottomPanelTab::Messages)
            } else if tab.result_set.is_some() {
                Some(crate::core::BottomPanelTab::Results)
            } else {
                None
            };
        } else {
            self.session.last_query_time_ms = None;
            self.state.selected_table = None;
            self.clear_search();
            self.state.grid_state.result_set = None;
            self.state.search_column = None;
            self.active_grid_workspace_enabled = false;
        }
        self.sync_table_metadata();
        self.state.selected_row = None;
        self.state.selected_cell = None;
        self.restore_grid_surface_from_active_tab();
        self.state.grid_state.result_set = active_result_set;
        if let Some(tab) = query_bottom_panel_tab {
            self.reveal_bottom_panel_for_query(tab);
        }
    }

    /// 在切换/打开其它 Tab 前持久化当前活动 Tab 的状态
    pub(in crate::app) fn persist_active_tab_state_for_navigation(&mut self) {
        self.persist_active_grid_workspace();
        self.sync_sql_to_active_tab();
    }

    /// 更新活动 Tab 的元数据（modified 标记、标题）
    pub(in crate::app) fn sync_sql_to_active_tab(&mut self) {
        if let Some(tab) = self.session.tab_manager.get_active_mut() {
            tab.modified = !tab.sql.trim().is_empty();
            tab.update_title();
        }
    }

    /// 取消指定查询请求
    /// 取消指定查询请求。
    /// 优先通过 TaskRegistry （T1 路径）；同时保留对旧 pending_* 映射的兼容清理。
    fn cancel_query_request_with_visibility(&mut self, request_id: u64, user_visible: bool) {
        // 查找 tab 并构造 OperationKey 用于 TaskRegistry 取消
        let target_tab_id = self
            .session
            .tab_manager
            .tabs
            .iter()
            .find(|t| t.pending_request_id == Some(request_id))
            .map(|t| t.id.clone());

        if let Some(ref tab_id) = target_tab_id {
            let document = uuid::Uuid::parse_str(tab_id)
                .map(crate::domain::ids::DocumentId::from)
                .unwrap_or_else(|_| {
                    crate::domain::ids::DocumentId::from(uuid::Uuid::new_v5(
                        &uuid::Uuid::NAMESPACE_OID,
                        tab_id.as_bytes(),
                    ))
                });
            let key = crate::session::task_registry::OperationKey::Query { document };
            self.session.task_registry.cancel_by_key(&key);
        }

        if user_visible {
            self.session
                .user_cancelled_query_requests
                .insert(request_id);
        } else {
            self.session
                .user_cancelled_query_requests
                .remove(&request_id);
        }
        self.state.pending_drop_requests.remove(&request_id);
        self.clear_tab_pending_request(request_id);
        self.session.refresh_executing_flag();
    }

    /// 取消某个连接关联的所有查询请求
    pub(in crate::app) fn cancel_queries_for_connection(&mut self, _conn_name: &str) {
        // 通过 TaskRegistry 取消所有活跃查询
        let query_keys: Vec<crate::session::task_registry::OperationKey> = self
            .session
            .task_registry
            .active_keys()
            .filter(|(k, _)| matches!(k, crate::session::task_registry::OperationKey::Query { .. }))
            .map(|(k, _)| k.clone())
            .collect();
        for key in query_keys {
            self.session.task_registry.cancel_by_key(&key);
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
