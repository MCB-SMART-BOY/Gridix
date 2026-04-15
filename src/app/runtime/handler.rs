//! 消息处理模块
//!
//! 处理从异步任务返回的各种消息，更新应用状态。

use std::collections::HashSet;

use eframe::egui;

use super::{DbManagerApp, GridSaveContext, Message};
use crate::app::GridWorkspaceId;
use crate::ui;

struct QueryDonePayload {
    sql: String,
    conn_name: String,
    tab_id: String,
    request_id: u64,
    result: Result<crate::database::QueryResult, String>,
    elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErDiagramReadyKind {
    Explicit(usize),
    Inferred(usize),
    Empty,
}

fn is_cancelled_query_error(err: &str) -> bool {
    let trimmed = err.trim();
    if trimmed.starts_with("查询已取消") {
        return true;
    }

    let lower = trimmed.to_ascii_lowercase();
    lower.contains("query canceled")
        || lower.contains("query cancelled")
        || lower.contains("canceling statement due to user request")
        || lower.contains("query execution was interrupted")
}

fn clamp_grid_cursor_for_result(
    cursor: (usize, usize),
    result: &crate::database::QueryResult,
) -> (usize, usize) {
    if result.rows.is_empty() || result.columns.is_empty() {
        return (0, 0);
    }

    (
        cursor.0.min(result.rows.len().saturating_sub(1)),
        cursor.1.min(result.columns.len().saturating_sub(1)),
    )
}

fn grid_save_context_matches_current(
    active_workspace_id: Option<&GridWorkspaceId>,
    active_tab_id: Option<&str>,
    context: &GridSaveContext,
) -> bool {
    active_workspace_id == Some(&context.workspace_id)
        && active_tab_id == Some(context.tab_id.as_str())
}

fn should_drop_query_error_as_stale(
    is_stale_for_existing_tab: bool,
    is_cancelled: bool,
    was_user_cancelled: bool,
) -> bool {
    is_stale_for_existing_tab && !(is_cancelled && was_user_cancelled)
}

fn should_record_active_query_time(
    target_tab_index: Option<usize>,
    active_index: usize,
    is_stale_for_existing_tab: bool,
) -> bool {
    !is_stale_for_existing_tab && target_tab_index == Some(active_index)
}

fn collect_er_foreign_key_columns(
    fks: &[crate::database::ForeignKeyInfo],
) -> HashSet<(String, String)> {
    fks.iter()
        .map(|fk| (fk.from_table.clone(), fk.from_column.clone()))
        .collect()
}

fn collect_er_relationships_from_foreign_keys(
    fks: Vec<crate::database::ForeignKeyInfo>,
) -> Vec<ui::Relationship> {
    fks.into_iter()
        .map(|fk| ui::Relationship {
            from_table: fk.from_table,
            from_column: fk.from_column,
            to_table: fk.to_table,
            to_column: fk.to_column,
            relation_type: ui::RelationType::OneToMany,
        })
        .collect()
}

fn resolve_er_diagram_ready_state(
    explicit_relationships: Vec<ui::Relationship>,
    inferred_relationships: Vec<ui::Relationship>,
) -> (Vec<ui::Relationship>, ErDiagramReadyKind) {
    if explicit_relationships.is_empty() {
        if inferred_relationships.is_empty() {
            (Vec::new(), ErDiagramReadyKind::Empty)
        } else {
            let rel_count = inferred_relationships.len();
            (
                inferred_relationships,
                ErDiagramReadyKind::Inferred(rel_count),
            )
        }
    } else {
        let rel_count = explicit_relationships.len();
        (
            explicit_relationships,
            ErDiagramReadyKind::Explicit(rel_count),
        )
    }
}

fn er_diagram_ready_message(table_count: usize, ready_kind: ErDiagramReadyKind) -> String {
    match ready_kind {
        ErDiagramReadyKind::Explicit(rel_count) => {
            format!("ER图: {} 张表, {} 个关系", table_count, rel_count)
        }
        ErDiagramReadyKind::Inferred(rel_count) => {
            format!("ER图: {} 张表, 推断出 {} 个关系", table_count, rel_count)
        }
        ErDiagramReadyKind::Empty => {
            format!("ER图: {} 张表（未发现外键关系）", table_count)
        }
    }
}

impl DbManagerApp {
    fn apply_successful_grid_save_state(&mut self, context: &GridSaveContext) {
        let active_tab_id = self.tab_manager.get_active().map(|tab| tab.id.as_str());
        if grid_save_context_matches_current(
            self.active_grid_workspace_id().as_ref(),
            active_tab_id,
            context,
        ) {
            self.grid_state.clear_save_state();
        }
        self.grid_workspaces.clear_save_state(&context.workspace_id);
    }

    fn finalize_er_diagram_load_if_ready(&mut self) {
        if self.er_diagram_state.loading || self.er_diagram_state.tables.is_empty() {
            return;
        }

        ui::grid_layout(
            &mut self.er_diagram_state.tables,
            4,
            egui::Vec2::new(60.0, 50.0),
        );

        let inferred_relationships = if self.er_diagram_state.relationships.is_empty() {
            self.infer_relationships_from_columns()
        } else {
            Vec::new()
        };
        let explicit_relationships = std::mem::take(&mut self.er_diagram_state.relationships);
        let (relationships, ready_kind) =
            resolve_er_diagram_ready_state(explicit_relationships, inferred_relationships);
        self.er_diagram_state.relationships = relationships;

        self.notifications.info(er_diagram_ready_message(
            self.er_diagram_state.tables.len(),
            ready_kind,
        ));
    }

    /// 处理异步消息
    ///
    /// 轮询消息通道，处理数据库连接、查询结果、ER图数据等异步任务的返回结果。
    pub fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::ConnectedWithTables(name, request_id, result) => {
                    self.handle_connected_with_tables(ctx, name, request_id, result);
                }
                Message::ConnectedWithDatabases(name, request_id, result) => {
                    self.handle_connected_with_databases(ctx, name, request_id, result);
                }
                Message::DatabaseSelected(conn_name, db_name, request_id, result) => {
                    self.handle_database_selected(ctx, conn_name, db_name, request_id, result);
                }
                Message::DatabaseDropped(conn_name, db_name, result) => {
                    self.handle_database_dropped(ctx, conn_name, db_name, result);
                }
                Message::TableDropped(conn_name, table_name, result) => {
                    self.handle_table_dropped(ctx, conn_name, table_name, result);
                }
                Message::QueryDone(sql, conn_name, tab_id, request_id, result, elapsed_ms) => {
                    self.handle_query_done(
                        ctx,
                        QueryDonePayload {
                            sql,
                            conn_name,
                            tab_id,
                            request_id,
                            result,
                            elapsed_ms,
                        },
                    );
                }
                Message::ImportDone(result, elapsed_ms) => {
                    self.handle_import_done(ctx, result, elapsed_ms);
                }
                Message::GridSaveDone(request_id, result, elapsed_ms) => {
                    self.handle_grid_save_done(ctx, request_id, result, elapsed_ms);
                }
                Message::PrimaryKeyFetched(table_name, pk_column) => {
                    self.handle_primary_key_fetched(ctx, table_name, pk_column);
                }
                Message::TriggersFetched(conn_name, db_name, request_id, result) => {
                    self.handle_triggers_fetched(ctx, conn_name, db_name, request_id, result);
                }
                Message::RoutinesFetched(conn_name, db_name, request_id, result) => {
                    self.handle_routines_fetched(ctx, conn_name, db_name, request_id, result);
                }
                Message::ForeignKeysFetched(result) => {
                    self.handle_foreign_keys_fetched(ctx, result);
                }
                Message::ERTableColumnsFetched(table_name, result) => {
                    self.handle_er_table_columns_fetched(ctx, table_name, result);
                }
            }
        }
    }

    /// 处理 SQLite 连接完成消息
    fn handle_connected_with_tables(
        &mut self,
        ctx: &egui::Context,
        name: String,
        request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        let is_latest = self
            .pending_connect_requests
            .get(&name)
            .is_some_and(|id| *id == request_id);
        if !is_latest {
            tracing::debug!(
                connection = %name,
                request_id,
                "忽略过期连接回包（SQLite）"
            );
            return;
        }
        self.pending_connect_requests.remove(&name);
        self.refresh_connecting_flag();

        let is_active = self.manager.active.as_deref() == Some(name.as_str());
        match result {
            Ok(tables) => {
                if let Some(conn) = self.manager.connections.get_mut(&name) {
                    conn.set_connected(tables.clone());
                }

                if is_active {
                    self.notifications.success(format!(
                        "已连接到 {} ({} 张表)",
                        name,
                        tables.len()
                    ));
                    self.load_history_for_connection(&name);
                    self.autocomplete.set_tables(tables);
                    self.sidebar_panel_state
                        .selection
                        .reset_for_connection_change();
                    self.load_triggers();
                    self.load_routines();
                }
            }
            Err(e) => {
                if is_active {
                    self.handle_connection_error(&name, e);
                } else if let Some(conn) = self.manager.connections.get_mut(&name) {
                    conn.set_error(e);
                }
            }
        }
        ctx.request_repaint();
    }

    /// 处理 MySQL/PostgreSQL 连接完成消息
    fn handle_connected_with_databases(
        &mut self,
        ctx: &egui::Context,
        name: String,
        request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        let is_latest = self
            .pending_connect_requests
            .get(&name)
            .is_some_and(|id| *id == request_id);
        if !is_latest {
            tracing::debug!(
                connection = %name,
                request_id,
                "忽略过期连接回包（多数据库）"
            );
            return;
        }
        self.pending_connect_requests.remove(&name);
        self.refresh_connecting_flag();

        let is_active = self.manager.active.as_deref() == Some(name.as_str());
        match result {
            Ok(databases) => {
                if let Some(conn) = self.manager.connections.get_mut(&name) {
                    conn.set_connected_with_databases(databases.clone());
                }

                if is_active {
                    self.notifications.success(format!(
                        "已连接到 {} ({} 个数据库)",
                        name,
                        databases.len()
                    ));
                    self.load_history_for_connection(&name);
                    self.autocomplete.clear();
                    self.sidebar_panel_state
                        .selection
                        .reset_for_connection_change();
                }
            }
            Err(e) => {
                if is_active {
                    self.handle_connection_error(&name, e);
                } else if let Some(conn) = self.manager.connections.get_mut(&name) {
                    conn.set_error(e);
                }
            }
        }
        ctx.request_repaint();
    }

    /// 处理数据库选择完成消息
    fn handle_database_selected(
        &mut self,
        ctx: &egui::Context,
        conn_name: String,
        db_name: String,
        request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        let is_latest = self.pending_database_requests.get(&conn_name).is_some_and(
            |(pending_db, pending_id)| pending_db == &db_name && *pending_id == request_id,
        );
        if !is_latest {
            tracing::debug!(
                connection = %conn_name,
                database = %db_name,
                request_id,
                "忽略过期数据库切换回包"
            );
            return;
        }
        self.pending_database_requests.remove(&conn_name);
        self.refresh_connecting_flag();

        let is_active = self.manager.active.as_deref() == Some(conn_name.as_str());
        match result {
            Ok(tables) => {
                if let Some(conn) = self.manager.connections.get_mut(&conn_name) {
                    conn.set_database(db_name.clone(), tables.clone());
                }

                if is_active {
                    self.notifications.success(format!(
                        "已选择数据库 {} ({} 张表)",
                        db_name,
                        tables.len()
                    ));
                    self.autocomplete.set_tables(tables);
                    self.sidebar_panel_state
                        .selection
                        .reset_for_database_change();
                    self.load_triggers();
                    self.load_routines();
                    self.switch_grid_workspace(None);
                    self.result = None;
                }
            }
            Err(e) => {
                if is_active {
                    self.notifications.error(format!("选择数据库失败: {}", e));
                }
            }
        }
        ctx.request_repaint();
    }

    /// 处理数据库删除完成消息
    fn handle_database_dropped(
        &mut self,
        ctx: &egui::Context,
        conn_name: String,
        db_name: String,
        result: Result<(), String>,
    ) {
        let is_active = self.manager.active.as_deref() == Some(conn_name.as_str());

        match result {
            Ok(()) => {
                let mut dropped_selected_database = false;
                if let Some(conn) = self.manager.connections.get_mut(&conn_name) {
                    conn.databases.retain(|database| database != &db_name);
                    if conn.selected_database.as_deref() == Some(db_name.as_str()) {
                        conn.selected_database = None;
                        conn.config.database.clear();
                        conn.tables.clear();
                        dropped_selected_database = true;
                    }
                }

                self.remove_grid_workspaces_for_database(&db_name);
                if is_active {
                    self.sidebar_panel_state
                        .selection
                        .reset_for_database_change();
                    if dropped_selected_database {
                        self.switch_grid_workspace(None);
                        self.result = None;
                        self.selected_table = None;
                        self.search_text.clear();
                        self.search_column = None;
                        self.autocomplete.clear();
                        self.sidebar_panel_state.clear_triggers();
                        self.sidebar_panel_state.clear_routines();
                        self.sidebar_panel_state.loading_triggers = false;
                        self.sidebar_panel_state.loading_routines = false;
                        self.sidebar_section = ui::SidebarSection::Databases;
                        self.set_focus_area(ui::FocusArea::Sidebar);
                    }
                }

                self.notifications
                    .success(format!("数据库 '{}' 已删除", db_name));
            }
            Err(error) => {
                self.notifications
                    .error(format!("删除数据库 '{}' 失败: {}", db_name, error));
            }
        }

        ctx.request_repaint();
    }

    /// 处理表删除完成消息
    fn handle_table_dropped(
        &mut self,
        ctx: &egui::Context,
        conn_name: String,
        table_name: String,
        result: Result<(), String>,
    ) {
        let is_active = self.manager.active.as_deref() == Some(conn_name.as_str());

        match result {
            Ok(()) => {
                if let Some(conn) = self.manager.connections.get_mut(&conn_name) {
                    conn.tables.retain(|table| table != &table_name);
                    if is_active {
                        self.autocomplete.set_tables(conn.tables.clone());
                    }
                }

                self.remove_grid_workspace_for_table(&table_name);
                if is_active && self.selected_table.as_deref() == Some(table_name.as_str()) {
                    self.switch_grid_workspace(None);
                    self.result = None;
                    self.selected_table = None;
                    self.sidebar_section = ui::SidebarSection::Tables;
                    self.set_focus_area(ui::FocusArea::Sidebar);
                }

                self.notifications
                    .success(format!("表 '{}' 已删除", table_name));
            }
            Err(error) => {
                self.notifications
                    .error(format!("删除表 '{}' 失败: {}", table_name, error));
            }
        }

        ctx.request_repaint();
    }

    /// 处理查询完成消息
    fn handle_query_done(&mut self, ctx: &egui::Context, payload: QueryDonePayload) {
        use crate::core::constants;
        let QueryDonePayload {
            sql,
            conn_name,
            tab_id,
            request_id,
            result,
            elapsed_ms,
        } = payload;

        self.finalize_query_task(request_id);
        let was_user_cancelled = self.user_cancelled_query_requests.remove(&request_id);

        let target_tab_index = self.tab_manager.tabs.iter().position(|t| t.id == tab_id);
        let is_stale_for_existing_tab = target_tab_index
            .and_then(|idx| self.tab_manager.tabs.get(idx))
            .is_some_and(|tab| tab.pending_request_id != Some(request_id));
        let should_update_active_query_time = should_record_active_query_time(
            target_tab_index,
            self.tab_manager.active_index,
            is_stale_for_existing_tab,
        );

        let sql_hints = crate::database::analyze_sql_for_ui(&sql);
        let is_update_or_delete = sql_hints.is_update_or_delete;
        let is_insert = sql_hints.is_insert;
        let is_drop_table = sql_hints.is_drop_table;

        let db_type = self
            .manager
            .connections
            .get(&conn_name)
            .map(|c| c.config.db_type.display_name().to_string())
            .unwrap_or_default();

        match result {
            Ok(mut res) => {
                if sql_hints.is_create_database {
                    self.mark_onboarding_database_initialized();
                }
                if sql_hints.is_create_user_or_role {
                    self.mark_onboarding_user_created();
                }
                self.mark_onboarding_first_query_executed();
                // 查询层已执行结果集限流；这里保留兼容兜底（避免旧路径漏限流）
                let mut original_rows = res.original_row_count.unwrap_or(res.rows.len());
                let mut was_truncated = res.truncated;
                if !was_truncated && res.rows.len() > constants::database::MAX_RESULT_SET_ROWS {
                    original_rows = res.rows.len();
                    was_truncated = true;
                    res.rows.truncate(constants::database::MAX_RESULT_SET_ROWS);
                    res.truncated = true;
                    res.original_row_count = Some(original_rows);
                }

                self.query_history.add(
                    sql,
                    db_type,
                    true,
                    if res.affected_rows > 0 {
                        Some(res.affected_rows)
                    } else {
                        None
                    },
                );

                if is_stale_for_existing_tab {
                    if is_drop_table {
                        self.pending_drop_requests.remove(&request_id);
                    }
                    self.pending_grid_refresh_restores.remove(&request_id);
                    tracing::debug!(
                        tab_id = %tab_id,
                        request_id,
                        "忽略过期查询回包（请求已被新查询覆盖或标签已关闭）"
                    );
                    self.refresh_executing_flag();
                    ctx.request_repaint();
                    return;
                }

                let msg = if res.columns.is_empty() {
                    format!("执行成功，影响 {} 行 ({}ms)", res.affected_rows, elapsed_ms)
                } else if was_truncated {
                    format!(
                        "查询完成，返回 {} 行（已截断，原始 {} 行，建议使用 LIMIT）({}ms)",
                        res.rows.len(),
                        original_rows,
                        elapsed_ms
                    )
                } else {
                    format!("查询完成，返回 {} 行 ({}ms)", res.rows.len(), elapsed_ms)
                };

                if let Some(idx) = target_tab_index {
                    let is_active_tab = idx == self.tab_manager.active_index;
                    if let Some(tab) = self.tab_manager.tabs.get_mut(idx) {
                        tab.result = Some(res.clone());
                        tab.executing = false;
                        tab.last_error = None;
                        tab.pending_request_id = None;
                        tab.query_time_ms = Some(elapsed_ms);
                        tab.last_message = Some(msg.clone());
                    }

                    if is_active_tab {
                        if should_update_active_query_time {
                            self.last_query_time_ms = Some(elapsed_ms);
                        }
                        self.notifications.success(&msg);
                        self.selected_row = None;
                        self.selected_cell = None;
                        self.search_text.clear();

                        // 根据 SQL 类型设置光标位置
                        if is_update_or_delete {
                            self.grid_state.scroll_to_row = Some(self.grid_state.cursor.0);
                        } else if is_insert {
                            let last_row = res.rows.len().saturating_sub(1);
                            self.grid_state.cursor = (last_row, 0);
                            self.grid_state.scroll_to_row = Some(last_row);
                        }

                        if self.focus_area == ui::FocusArea::DataGrid {
                            self.grid_state.focused = true;
                        }

                        // 更新自动补全
                        if let Some(table) = &self.selected_table
                            && !res.columns.is_empty()
                        {
                            self.autocomplete
                                .set_columns(table.clone(), res.columns.clone());
                        }

                        self.result = Some(res.clone());

                        if let Some(restore) =
                            self.pending_grid_refresh_restores.remove(&request_id)
                        {
                            self.switch_grid_workspace(Some(restore.table_name.clone()));
                            self.search_text = restore.search_text;
                            self.search_column = restore.search_column;
                            self.grid_state.cursor =
                                clamp_grid_cursor_for_result(restore.cursor, &res);
                            self.grid_state.scroll_to_row = Some(self.grid_state.cursor.0);
                            self.grid_state.scroll_to_col = Some(self.grid_state.cursor.1);
                            self.grid_state.focused = true;
                            self.set_focus_area(ui::FocusArea::DataGrid);
                        }
                    } else {
                        self.pending_grid_refresh_restores.remove(&request_id);
                    }
                } else {
                    self.pending_grid_refresh_restores.remove(&request_id);
                    tracing::debug!(tab_id = %tab_id, "查询回包对应的标签页已不存在");
                }

                if is_drop_table
                    && let Some((drop_conn_name, dropped_table)) =
                        self.pending_drop_requests.remove(&request_id)
                {
                    let is_current_active =
                        self.manager.active.as_deref() == Some(drop_conn_name.as_str());

                    if let Some(conn) = self.manager.connections.get_mut(&drop_conn_name) {
                        conn.tables.retain(|t| t != &dropped_table);
                        if is_current_active {
                            self.autocomplete.set_tables(conn.tables.clone());
                        }
                    }

                    if is_current_active && self.selected_table.as_deref() == Some(&dropped_table) {
                        self.switch_grid_workspace(None);
                        self.remove_grid_workspace_for_table(&dropped_table);
                        self.result = None;
                    }
                }
            }
            Err(e) => {
                let is_cancelled = is_cancelled_query_error(&e);
                if !is_cancelled {
                    self.query_history.add(sql, db_type, false, None);
                }
                if should_drop_query_error_as_stale(
                    is_stale_for_existing_tab,
                    is_cancelled,
                    was_user_cancelled,
                ) {
                    if is_drop_table {
                        self.pending_drop_requests.remove(&request_id);
                    }
                    self.pending_grid_refresh_restores.remove(&request_id);
                    tracing::debug!(
                        tab_id = %tab_id,
                        request_id,
                        error = %e,
                        "忽略过期查询错误回包（请求已被新查询覆盖或标签已关闭）"
                    );
                    self.refresh_executing_flag();
                    ctx.request_repaint();
                    return;
                }

                if target_tab_index.is_none() {
                    tracing::debug!(tab_id = %tab_id, "查询错误回包对应的标签页已不存在");
                    if is_drop_table {
                        self.pending_drop_requests.remove(&request_id);
                    }
                    self.pending_grid_refresh_restores.remove(&request_id);
                    self.refresh_executing_flag();
                    ctx.request_repaint();
                    return;
                }

                let err_msg = if is_cancelled {
                    if e.starts_with("查询已取消") {
                        e.clone()
                    } else {
                        format!("查询已取消 ({})", e)
                    }
                } else {
                    format!("错误: {}", e)
                };
                if is_cancelled {
                    self.notifications.warning(&err_msg);
                } else {
                    self.notifications.error(&err_msg);
                }
                if let Some(idx) = target_tab_index {
                    let is_active_tab = idx == self.tab_manager.active_index;
                    if let Some(tab) = self.tab_manager.tabs.get_mut(idx) {
                        tab.executing = false;
                        if !is_cancelled {
                            tab.result = None;
                        }
                        tab.query_time_ms = Some(elapsed_ms);
                        tab.last_error = if is_cancelled {
                            None
                        } else {
                            Some(err_msg.clone())
                        };
                        tab.pending_request_id = None;
                        tab.last_message = Some(err_msg.clone());
                    }
                    if is_active_tab {
                        if should_update_active_query_time {
                            self.last_query_time_ms = Some(elapsed_ms);
                        }
                        if !is_cancelled {
                            self.result = None;
                        }
                    }
                }
                self.pending_grid_refresh_restores.remove(&request_id);

                if is_drop_table {
                    self.pending_drop_requests.remove(&request_id);
                }
            }
        }
        self.refresh_executing_flag();
        ctx.request_repaint();
    }

    fn handle_grid_save_done(
        &mut self,
        ctx: &egui::Context,
        request_id: u64,
        result: Result<crate::database::ImportExecutionReport, String>,
        elapsed_ms: u64,
    ) {
        let Some(context) = self.pending_grid_save_requests.remove(&request_id) else {
            tracing::debug!(request_id, "忽略未知的表格保存回包");
            return;
        };
        self.refresh_executing_flag();

        match result {
            Ok(report) => {
                if report.failed == 0 {
                    self.apply_successful_grid_save_state(&context);
                    self.notifications.success(format!(
                        "表格保存完成：成功 {} / {} 条 ({}ms)",
                        report.succeeded, report.total, elapsed_ms
                    ));
                    self.refresh_table_after_grid_save(ctx, context);
                    return;
                }

                let detail = report.first_error.as_deref().unwrap_or("部分 SQL 执行失败");
                self.notifications.error(format!(
                    "表格保存失败：成功 {}，失败 {}，总计 {} 条 ({}ms)。首个错误: {}",
                    report.succeeded, report.failed, report.total, elapsed_ms, detail
                ));
            }
            Err(error) => {
                self.notifications.error(format!("表格保存失败: {}", error));
            }
        }

        ctx.request_repaint();
    }

    /// 处理导入完成消息
    fn handle_import_done(
        &mut self,
        ctx: &egui::Context,
        result: Result<crate::database::ImportExecutionReport, String>,
        elapsed_ms: u64,
    ) {
        self.import_executing = false;
        self.refresh_executing_flag();

        match result {
            Ok(report) => {
                if report.failed == 0 {
                    self.notifications.success(format!(
                        "导入完成：成功 {} / {} 条 ({}ms)",
                        report.succeeded, report.total, elapsed_ms
                    ));
                } else {
                    let detail = report.first_error.as_deref().unwrap_or("部分语句执行失败");
                    self.notifications.warning(format!(
                        "导入部分完成：成功 {}，失败 {}，总计 {} 条 ({}ms)。首个错误: {}",
                        report.succeeded, report.failed, report.total, elapsed_ms, detail
                    ));
                }
            }
            Err(e) => {
                self.notifications.error(format!("导入失败: {}", e));
            }
        }

        ctx.request_repaint();
    }

    /// 处理主键获取完成消息
    fn handle_primary_key_fetched(
        &mut self,
        ctx: &egui::Context,
        table_name: String,
        pk_column: Option<String>,
    ) {
        if self.selected_table.as_deref() == Some(&table_name) {
            if let Some(pk_name) = pk_column {
                if let Some(result) = &self.result
                    && let Some(idx) = result.columns.iter().position(|c| c == &pk_name)
                {
                    self.grid_state.primary_key_column = Some(idx);
                }
            } else {
                self.grid_state.primary_key_column = None;
            }
        }
        ctx.request_repaint();
    }

    /// 检查异步元数据回包是否仍对应当前连接上下文
    fn metadata_context_matches_current(&self, conn_name: &str, db_name: &Option<String>) -> bool {
        if self.manager.active.as_deref() != Some(conn_name) {
            return false;
        }

        let Some(conn) = self.manager.connections.get(conn_name) else {
            return false;
        };

        match conn.config.db_type {
            crate::database::DatabaseType::SQLite => true,
            _ => conn.selected_database == *db_name,
        }
    }

    /// 处理触发器获取完成消息
    fn handle_triggers_fetched(
        &mut self,
        ctx: &egui::Context,
        conn_name: String,
        db_name: Option<String>,
        request_id: u64,
        result: Result<Vec<crate::database::TriggerInfo>, String>,
    ) {
        let is_latest = self.pending_triggers_request.as_ref().is_some_and(
            |(pending_conn, pending_db, pending_id)| {
                pending_conn == &conn_name && pending_db == &db_name && *pending_id == request_id
            },
        );
        if !is_latest {
            tracing::debug!(
                connection = %conn_name,
                database = ?db_name,
                request_id,
                "忽略过期触发器回包（请求ID不匹配）"
            );
            return;
        }
        self.pending_triggers_request = None;

        if !self.metadata_context_matches_current(&conn_name, &db_name) {
            tracing::debug!(
                connection = %conn_name,
                database = ?db_name,
                "忽略过期触发器回包"
            );
            self.sidebar_panel_state.loading_triggers = false;
            return;
        }

        self.sidebar_panel_state.loading_triggers = false;
        match result {
            Ok(triggers) => {
                self.sidebar_panel_state.set_triggers(triggers);
            }
            Err(e) => {
                self.notifications.error(format!("加载触发器失败: {}", e));
            }
        }
        ctx.request_repaint();
    }

    /// 处理存储过程/函数获取完成消息
    fn handle_routines_fetched(
        &mut self,
        ctx: &egui::Context,
        conn_name: String,
        db_name: Option<String>,
        request_id: u64,
        result: Result<Vec<crate::database::RoutineInfo>, String>,
    ) {
        let is_latest = self.pending_routines_request.as_ref().is_some_and(
            |(pending_conn, pending_db, pending_id)| {
                pending_conn == &conn_name && pending_db == &db_name && *pending_id == request_id
            },
        );
        if !is_latest {
            tracing::debug!(
                connection = %conn_name,
                database = ?db_name,
                request_id,
                "忽略过期存储过程回包（请求ID不匹配）"
            );
            return;
        }
        self.pending_routines_request = None;

        if !self.metadata_context_matches_current(&conn_name, &db_name) {
            tracing::debug!(
                connection = %conn_name,
                database = ?db_name,
                "忽略过期存储过程回包"
            );
            self.sidebar_panel_state.loading_routines = false;
            return;
        }

        self.sidebar_panel_state.loading_routines = false;
        match result {
            Ok(routines) => {
                self.sidebar_panel_state.set_routines(routines);
            }
            Err(e) => {
                // 对于 SQLite 不显示错误，因为它不支持存储过程
                if !e.contains("不支持") {
                    self.notifications.error(format!("加载存储过程失败: {}", e));
                }
            }
        }
        ctx.request_repaint();
    }

    /// 处理外键获取完成消息
    fn handle_foreign_keys_fetched(
        &mut self,
        ctx: &egui::Context,
        result: Result<Vec<crate::database::ForeignKeyInfo>, String>,
    ) {
        match result {
            Ok(fks) => {
                let foreign_key_columns = collect_er_foreign_key_columns(&fks);
                self.er_diagram_state
                    .set_foreign_key_columns(foreign_key_columns);

                let relationships = collect_er_relationships_from_foreign_keys(fks);
                let rel_count = relationships.len();
                self.er_diagram_state.relationships = relationships;
                tracing::debug!(relationship_count = rel_count, "ER 图外键关系已返回");
                self.finalize_er_diagram_load_if_ready();
            }
            Err(e) => {
                self.er_diagram_state.mark_foreign_keys_resolved();
                self.notifications.error(format!("加载外键关系失败: {}", e));
                self.finalize_er_diagram_load_if_ready();
            }
        }
        ctx.request_repaint();
    }

    /// 处理 ER 表列信息获取完成消息
    fn handle_er_table_columns_fetched(
        &mut self,
        ctx: &egui::Context,
        table_name: String,
        result: Result<Vec<crate::database::ColumnInfo>, String>,
    ) {
        match result {
            Ok(columns) => {
                let er_columns: Vec<ui::ERColumn> = columns
                    .into_iter()
                    .map(|c| ui::ERColumn {
                        is_foreign_key: self
                            .er_diagram_state
                            .is_foreign_key_column(&table_name, &c.name),
                        name: c.name,
                        data_type: c.data_type,
                        is_primary_key: c.is_primary_key,
                        nullable: c.is_nullable,
                        default_value: c.default_value,
                    })
                    .collect();

                if let Some(er_table) = self
                    .er_diagram_state
                    .tables
                    .iter_mut()
                    .find(|t| t.name == table_name)
                {
                    er_table.columns = er_columns;
                    // 立即计算表格尺寸，确保布局和关系线渲染正确
                    ui::calculate_table_size(er_table);
                }
            }
            Err(e) => {
                self.notifications
                    .warning(format!("获取表 {} 结构失败: {}", table_name, e));
            }
        }
        self.er_diagram_state
            .mark_table_request_resolved(&table_name);
        self.finalize_er_diagram_load_if_ready();
        ctx.request_repaint();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErDiagramReadyKind, clamp_grid_cursor_for_result,
        collect_er_relationships_from_foreign_keys, er_diagram_ready_message,
        grid_save_context_matches_current, is_cancelled_query_error,
        resolve_er_diagram_ready_state, should_drop_query_error_as_stale,
        should_record_active_query_time,
    };
    use crate::app::{GridSaveContext, GridWorkspaceId};
    use crate::database::ForeignKeyInfo;
    use crate::database::QueryResult;
    use crate::ui::{RelationType, Relationship};

    #[test]
    fn test_is_cancelled_query_error_chinese() {
        assert!(is_cancelled_query_error("查询已取消"));
        assert!(is_cancelled_query_error("查询已取消（权限不足）"));
    }

    #[test]
    fn test_is_cancelled_query_error_english_patterns() {
        assert!(is_cancelled_query_error(
            "canceling statement due to user request"
        ));
        assert!(is_cancelled_query_error("Query execution was interrupted"));
        assert!(is_cancelled_query_error("query canceled by user"));
    }

    #[test]
    fn test_is_cancelled_query_error_negative_case() {
        assert!(!is_cancelled_query_error("syntax error near from"));
    }

    #[test]
    fn clamp_grid_cursor_for_result_respects_result_bounds() {
        let result = QueryResult::with_rows(
            vec!["id".to_string(), "name".to_string()],
            vec![
                vec!["1".to_string(), "alice".to_string()],
                vec!["2".to_string(), "bob".to_string()],
            ],
        );

        assert_eq!(clamp_grid_cursor_for_result((8, 5), &result), (1, 1));
        assert_eq!(clamp_grid_cursor_for_result((0, 1), &result), (0, 1));
    }

    #[test]
    fn clamp_grid_cursor_for_result_falls_back_to_origin_for_empty_result() {
        assert_eq!(
            clamp_grid_cursor_for_result((4, 2), &QueryResult::default()),
            (0, 0)
        );
    }

    #[test]
    fn grid_save_context_matches_current_requires_matching_workspace_and_tab() {
        let workspace = GridWorkspaceId {
            tab_id: "tab-a".to_string(),
            connection_name: "local".to_string(),
            database_name: Some("main".to_string()),
            table_name: "users".to_string(),
        };
        let context = GridSaveContext {
            workspace_id: workspace.clone(),
            table_name: "users".to_string(),
            tab_id: "tab-a".to_string(),
            cursor: (2, 1),
            search_text: String::new(),
            search_column: None,
        };

        assert!(grid_save_context_matches_current(
            Some(&workspace),
            Some("tab-a"),
            &context
        ));
        assert!(!grid_save_context_matches_current(
            Some(&workspace),
            Some("tab-b"),
            &context
        ));

        let other_workspace = GridWorkspaceId {
            tab_id: "tab-b".to_string(),
            connection_name: "local".to_string(),
            database_name: Some("main".to_string()),
            table_name: "orders".to_string(),
        };
        assert!(!grid_save_context_matches_current(
            Some(&other_workspace),
            Some("tab-a"),
            &context
        ));
    }

    #[test]
    fn cancelled_query_error_from_user_cancel_is_not_dropped_as_stale() {
        assert!(!should_drop_query_error_as_stale(true, true, true));
        assert!(should_drop_query_error_as_stale(true, true, false));
        assert!(should_drop_query_error_as_stale(true, false, true));
        assert!(!should_drop_query_error_as_stale(false, true, true));
    }

    #[test]
    fn active_query_time_updates_only_for_non_stale_active_tab() {
        assert!(should_record_active_query_time(Some(2), 2, false));
        assert!(!should_record_active_query_time(Some(1), 2, false));
        assert!(!should_record_active_query_time(Some(2), 2, true));
        assert!(!should_record_active_query_time(None, 2, false));
    }

    #[test]
    fn er_diagram_ready_message_reports_explicit_relationships() {
        assert_eq!(
            er_diagram_ready_message(8, ErDiagramReadyKind::Explicit(3)),
            "ER图: 8 张表, 3 个关系"
        );
    }

    #[test]
    fn er_diagram_ready_message_reports_inferred_relationships() {
        assert_eq!(
            er_diagram_ready_message(8, ErDiagramReadyKind::Inferred(2)),
            "ER图: 8 张表, 推断出 2 个关系"
        );
    }

    #[test]
    fn er_diagram_ready_message_reports_empty_relationships() {
        assert_eq!(
            er_diagram_ready_message(8, ErDiagramReadyKind::Empty),
            "ER图: 8 张表（未发现外键关系）"
        );
    }

    fn relationship(from_table: &str, to_table: &str) -> Relationship {
        Relationship {
            from_table: from_table.to_string(),
            from_column: "source_id".to_string(),
            to_table: to_table.to_string(),
            to_column: "id".to_string(),
            relation_type: RelationType::OneToMany,
        }
    }

    #[test]
    fn resolve_er_diagram_ready_state_prefers_explicit_relationships() {
        let explicit = vec![relationship("orders", "customers")];
        let inferred = vec![relationship("payments", "orders")];

        let (relationships, ready_kind) = resolve_er_diagram_ready_state(explicit, inferred);

        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].from_table, "orders");
        assert_eq!(ready_kind, ErDiagramReadyKind::Explicit(1));
    }

    #[test]
    fn resolve_er_diagram_ready_state_uses_inferred_fallback_when_explicit_is_empty() {
        let inferred = vec![
            relationship("orders", "customers"),
            relationship("payments", "orders"),
        ];

        let (relationships, ready_kind) = resolve_er_diagram_ready_state(Vec::new(), inferred);

        assert_eq!(relationships.len(), 2);
        assert_eq!(relationships[0].from_table, "orders");
        assert_eq!(relationships[1].from_table, "payments");
        assert_eq!(ready_kind, ErDiagramReadyKind::Inferred(2));
    }

    #[test]
    fn resolve_er_diagram_ready_state_reports_empty_when_no_relationships_exist() {
        let (relationships, ready_kind) = resolve_er_diagram_ready_state(Vec::new(), Vec::new());

        assert!(relationships.is_empty());
        assert_eq!(ready_kind, ErDiagramReadyKind::Empty);
    }

    #[test]
    fn collect_er_relationships_from_foreign_keys_keeps_empty_results_empty() {
        let relationships = collect_er_relationships_from_foreign_keys(Vec::new());

        assert!(relationships.is_empty());
    }

    #[test]
    fn collect_er_relationships_from_foreign_keys_maps_explicit_edges_only() {
        let relationships = collect_er_relationships_from_foreign_keys(vec![ForeignKeyInfo {
            from_table: "orders".to_string(),
            from_column: "customer_id".to_string(),
            to_table: "customers".to_string(),
            to_column: "id".to_string(),
        }]);

        assert_eq!(relationships.len(), 1);
        assert_eq!(relationships[0].from_table, "orders");
        assert_eq!(relationships[0].from_column, "customer_id");
        assert_eq!(relationships[0].to_table, "customers");
        assert_eq!(relationships[0].to_column, "id");
        assert_eq!(relationships[0].relation_type, RelationType::OneToMany);
    }
}
