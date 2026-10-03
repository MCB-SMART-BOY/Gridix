//! 查询与请求生命周期管理
//!
//! 负责请求 ID、执行状态、取消信号与 pending 任务清理。

use super::DbManagerApp;
pub(in crate::app) fn query_document_id(tab_id: &str) -> crate::domain::ids::DocumentId {
    uuid::Uuid::parse_str(tab_id)
        .map(crate::domain::ids::DocumentId::from)
        .unwrap_or_else(|_| {
            crate::domain::ids::DocumentId::from(uuid::Uuid::new_v5(
                &uuid::Uuid::NAMESPACE_OID,
                tab_id.as_bytes(),
            ))
        })
}

impl DbManagerApp {
    pub(in crate::app) fn close_pg_sessions_for_tab(&self, tab_id: &str) {
        crate::data::close_pg_document_sessions(query_document_id(tab_id));
    }

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
        let document = query_document_id(&tab.id);
        self.session
            .task_registry
            .cancel_queries_for_document(document);
        // 保留 request_id 清理用于旧路径
        self.session
            .user_cancelled_query_requests
            .insert(_request_id);
        self.clear_tab_pending_request(_request_id);
        self.session
            .notifications
            .warning("已请求取消查询；执行结果仍需确认");
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
        let is_current_result = self.session.manager.get_active().is_some_and(|connection| {
            self.session.tab_manager.get_active().is_some_and(|tab| {
                (tab.result_origin.is_none() && tab.result_set.is_none())
                    || tab.is_result_from(connection.id, connection.selected_database.as_deref())
            })
        });
        let mut active_result_set = None;
        let mut query_bottom_panel_tab = None;
        if let Some(tab) = self.session.tab_manager.get_active() {
            if is_current_result {
                active_result_set = tab.result_set.clone();
            }
            self.session.last_query_time_ms =
                is_current_result.then_some(tab.query_time_ms).flatten();
            self.state.selected_table = is_current_result
                .then(|| tab.selected_table.clone())
                .flatten();
            self.state.search_text = if is_current_result {
                tab.search_text.clone()
            } else {
                String::new()
            };
            self.state.search_column = is_current_result
                .then(|| tab.search_column.clone())
                .flatten();
            self.active_grid_workspace_enabled = is_current_result && tab.uses_grid_workspace;
            query_bottom_panel_tab = if !is_current_result {
                None
            } else if self.state.explain_state.should_show_for_tab(&tab.id) {
                Some(crate::core::BottomPanelTab::Explain)
            } else if tab.last_error.is_some() {
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
        self.state.selected_row = None;
        self.state.selected_cell = None;
        self.restore_grid_surface_from_active_tab();
        self.sync_table_metadata();
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

    /// Navigate through the same persistence boundary for dock, shortcuts and actions.
    pub(crate) fn activate_query_tab(&mut self, index: usize) {
        if index >= self.session.tab_manager.tabs.len()
            || index == self.session.tab_manager.active_index
        {
            return;
        }
        self.persist_active_tab_state_for_navigation();
        self.session.tab_manager.set_active(index);
        self.sync_from_active_tab();
        self.activate_active_sql_dock_tab();
    }

    pub(in crate::app) fn activate_active_sql_dock_tab(&mut self) {
        crate::ui::dock_tabs::sync_sql_documents(&mut self.dock_state, &self.session.tab_manager);
        if let Some(tab) = self.session.tab_manager.get_active() {
            crate::ui::dock_tabs::activate_sql_document(&mut self.dock_state, &tab.id);
        }
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
            let document = query_document_id(tab_id);
            self.session
                .task_registry
                .cancel_queries_for_document(document);
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
        self.clear_tab_pending_request(request_id);
        self.session.refresh_executing_flag();
    }

    /// 取消某个连接关联的所有查询请求（仅该连接本身，不影响其它连接）。
    pub(in crate::app) fn cancel_queries_for_connection(
        &mut self,
        connection: crate::domain::ids::ConnectionId,
    ) {
        self.session
            .task_registry
            .cancel_queries_for_connection(connection);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{ConnectionConfig, DatabaseType};
    use crate::domain::ids::SchemaRevision;
    use crate::domain::metadata::{SchemaCatalog, TableMetadata};

    #[test]
    fn returning_to_tab_projects_catalog_loaded_while_another_tab_was_active() {
        let mut app = DbManagerApp::new_for_test();
        let mut config = ConnectionConfig::new("demo", DatabaseType::SQLite);
        config.database = "main".into();
        app.session.manager.add(config);
        app.session.manager.active = Some("demo".into());
        let connection = app
            .session
            .manager
            .connections
            .get_mut("demo")
            .expect("connection");
        connection.selected_database = Some("main".into());
        let connection_id = connection.id;

        app.session.tab_manager.tabs[0].selected_table = Some("users".into());
        app.session.tab_manager.tabs[0].uses_grid_workspace = true;
        app.switch_grid_workspace(Some("users".into()));
        app.state
            .grid_state
            .modified_cells
            .insert((0, 0), "draft".into());
        app.open_new_query_tab();
        assert!(app.state.grid_state.table_metadata.is_none());

        app.session.schema_catalogs.insert(
            (connection_id, "main".into()),
            SchemaCatalog {
                revision: SchemaRevision(0),
                tables: vec![TableMetadata {
                    name: "users".into(),
                    schema: None,
                    columns: vec![],
                    primary_key: None,
                    unique_keys: vec![],
                    foreign_keys: vec![],
                }],
            },
        );
        app.activate_query_tab(0);

        assert_eq!(
            app.state
                .grid_state
                .table_metadata
                .as_ref()
                .map(|table| table.name.as_str()),
            Some("users"),
        );
        assert_eq!(app.state.grid_state.modified_cells[&(0, 0)], "draft");
    }
}
