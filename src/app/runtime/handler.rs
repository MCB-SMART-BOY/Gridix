//! 消息处理模块
//!
//! 处理从异步任务返回的各种消息，更新应用状态。

use std::collections::HashSet;

use eframe::egui;

use super::{DbManagerApp, Message};
use crate::ui;

struct QueryDonePayload {
    sql: String,
    conn_name: String,
    tab_id: String,
    request_id: u64,
    result: Result<crate::data::QueryResult, String>,
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

#[allow(dead_code)] // utility — tested but not called in production (future grid cursor restore)
fn clamp_grid_cursor_for_result(
    cursor: (usize, usize),
    result: &crate::data::QueryResult,
) -> (usize, usize) {
    if result.rows.is_empty() || result.columns.is_empty() {
        return (0, 0);
    }

    (
        cursor.0.min(result.rows.len().saturating_sub(1)),
        cursor.1.min(result.columns.len().saturating_sub(1)),
    )
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

/// schema 变更失效级联需要执行哪些重载（纯决策，便于单测）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SchemaInvalidation {
    reload_tables: bool,
    reload_triggers: bool,
    reload_routines: bool,
}

fn schema_invalidation_for(hints: &crate::data::SqlUiHints) -> SchemaInvalidation {
    SchemaInvalidation {
        reload_tables: hints.is_table_schema_change,
        reload_triggers: hints.is_trigger_change,
        reload_routines: hints.is_routine_change,
    }
}

/// 网格保存批次回包的处置结果（纯决策，便于单测 B1 不变量）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridSaveOutcome {
    /// 整批成功：清除编辑状态并刷新该表。
    CommittedClearEdits,
    /// 整批回滚（部分失败或执行错误）：保留编辑，显示错误。
    RolledBackKeepEdits,
}

/// 根据批量执行报告判定网格保存的处置方式。
///
/// 仅当整批成功（无失败语句）时才清除编辑；否则事务已回滚，DB 未变，保留编辑供重试。
fn classify_grid_save_outcome(
    result: &Result<crate::data::ImportExecutionReport, String>,
) -> GridSaveOutcome {
    match result {
        Ok(report) if report.failed == 0 => GridSaveOutcome::CommittedClearEdits,
        _ => GridSaveOutcome::RolledBackKeepEdits,
    }
}

fn collect_er_foreign_key_columns(
    fks: &[crate::data::ForeignKeyInfo],
) -> HashSet<(String, String)> {
    fks.iter()
        .map(|fk| (fk.from_table.clone(), fk.from_column.clone()))
        .collect()
}

fn collect_er_relationships_from_foreign_keys(
    fks: Vec<crate::data::ForeignKeyInfo>,
) -> Vec<ui::Relationship> {
    fks.into_iter()
        .map(|fk| ui::Relationship {
            from_table: fk.from_table,
            from_column: fk.from_column,
            to_table: fk.to_table,
            to_column: fk.to_column,
            relation_type: ui::RelationType::OneToMany,
            origin: ui::RelationshipOrigin::Explicit,
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

#[cfg(test)]
fn apply_default_er_diagram_layout(tables: &mut [ui::ERTable], relationships: &[ui::Relationship]) {
    let graph = ui::build_er_graph(tables, relationships);
    let strategy = ui::select_er_layout_strategy(&graph);
    ui::apply_er_layout_strategy(tables, relationships, strategy);
}

fn select_ready_state_er_layout_strategy(
    graph: &ui::ERGraph,
    has_pending_layout_restore: bool,
) -> ui::ERLayoutStrategy {
    if has_pending_layout_restore {
        ui::ERLayoutStrategy::StableIncremental
    } else {
        ui::select_er_layout_strategy(graph)
    }
}

fn apply_stable_incremental_er_diagram_layout(state: &mut ui::ERDiagramState, graph: &ui::ERGraph) {
    if state.restore_layout_snapshot_if_exact_match() {
        return;
    }

    let base_strategy = ui::select_er_layout_strategy(graph);
    ui::apply_er_layout_strategy(&mut state.tables, &state.relationships, base_strategy);

    let restored_names = state.restore_layout_snapshot_for_matching_tables();
    ui::stabilize_incremental_layout_positions(
        &mut state.tables,
        &state.relationships,
        &restored_names,
    );
}

fn apply_ready_state_er_diagram_layout(state: &mut ui::ERDiagramState) {
    let graph = ui::build_er_graph(&state.tables, &state.relationships);
    let strategy =
        select_ready_state_er_layout_strategy(&graph, state.has_pending_layout_restore());

    match strategy {
        ui::ERLayoutStrategy::StableIncremental => {
            apply_stable_incremental_er_diagram_layout(state, &graph);
        }
        strategy => ui::apply_er_layout_strategy(&mut state.tables, &state.relationships, strategy),
    }
}

impl DbManagerApp {
    fn finalize_er_diagram_load_if_ready(&mut self) {
        if self.state.er_diagram_state.loading || self.state.er_diagram_state.tables.is_empty() {
            return;
        }

        let inferred_relationships = if self.state.er_diagram_state.relationships.is_empty() {
            self.infer_relationships_from_columns()
        } else {
            Vec::new()
        };
        let explicit_relationships = std::mem::take(&mut self.state.er_diagram_state.relationships);
        let (relationships, ready_kind) =
            resolve_er_diagram_ready_state(explicit_relationships, inferred_relationships);
        self.state.er_diagram_state.relationships = relationships;
        apply_ready_state_er_diagram_layout(&mut self.state.er_diagram_state);

        self.session.notifications.info(er_diagram_ready_message(
            self.state.er_diagram_state.tables.len(),
            ready_kind,
        ));
    }

    /// 处理异步消息
    ///
    /// 轮询消息通道，处理数据库连接、查询结果、ER图数据等异步任务的返回结果。
    pub fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.session.rx.try_recv() {
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
                Message::ActiveTablesReloaded(conn_name, request_id, result) => {
                    self.handle_active_tables_reloaded(ctx, conn_name, request_id, result);
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
                Message::GridSaveDone {
                    result,
                    table,
                    request_id,
                    elapsed_ms,
                } => {
                    self.handle_grid_save_done(ctx, result, table, request_id, elapsed_ms);
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
                Message::ForeignKeysFetched(generation, result) => {
                    self.handle_foreign_keys_fetched(ctx, generation, result);
                }
                Message::ERTableColumnsFetched(generation, table_name, result) => {
                    self.handle_er_table_columns_fetched(ctx, generation, table_name, result);
                }
            }
        }
        if self.session.needs_repaint {
            ctx.request_repaint();
            self.session.needs_repaint = false;
        }
    }

    /// 处理 SQLite 连接完成消息
    fn handle_connected_with_tables(
        &mut self,
        _ctx: &egui::Context,
        name: String,
        request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        let is_latest = self
            .session
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
        self.session.pending_connect_requests.remove(&name);
        self.session.refresh_connecting_flag();

        let is_active = self.session.manager.active.as_deref() == Some(name.as_str());
        match result {
            Ok(tables) => {
                if let Some(conn) = self.session.manager.connections.get_mut(&name) {
                    conn.set_connected(tables.clone());
                }

                if is_active {
                    self.session.notifications.success(format!(
                        "已连接到 {} ({} 张表)",
                        name,
                        tables.len()
                    ));
                    self.load_history_for_connection(&name);
                    self.session.autocomplete.set_tables(tables);
                    self.state
                        .sidebar_panel_state
                        .selection
                        .reset_for_connection_change();
                    self.load_triggers();
                    self.load_routines();
                }
            }
            Err(e) => {
                if is_active {
                    self.handle_connection_error(&name, e);
                } else if let Some(conn) = self.session.manager.connections.get_mut(&name) {
                    conn.set_error(e);
                }
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理 MySQL/PostgreSQL 连接完成消息
    fn handle_connected_with_databases(
        &mut self,
        _ctx: &egui::Context,
        name: String,
        request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        let is_latest = self
            .session
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
        self.session.pending_connect_requests.remove(&name);
        self.session.refresh_connecting_flag();

        let is_active = self.session.manager.active.as_deref() == Some(name.as_str());
        match result {
            Ok(databases) => {
                if let Some(conn) = self.session.manager.connections.get_mut(&name) {
                    conn.set_connected_with_databases(databases.clone());
                }

                if is_active {
                    self.session.notifications.success(format!(
                        "已连接到 {} ({} 个数据库)",
                        name,
                        databases.len()
                    ));
                    self.load_history_for_connection(&name);
                    self.session.autocomplete.clear();
                    self.state
                        .sidebar_panel_state
                        .selection
                        .reset_for_connection_change();
                }
            }
            Err(e) => {
                if is_active {
                    self.handle_connection_error(&name, e);
                } else if let Some(conn) = self.session.manager.connections.get_mut(&name) {
                    conn.set_error(e);
                }
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理数据库选择完成消息
    fn handle_database_selected(
        &mut self,
        _ctx: &egui::Context,
        conn_name: String,
        db_name: String,
        request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        let is_latest = self
            .session
            .pending_database_requests
            .get(&conn_name)
            .is_some_and(|(pending_db, pending_id)| {
                pending_db == &db_name && *pending_id == request_id
            });
        if !is_latest {
            tracing::debug!(
                connection = %conn_name,
                database = %db_name,
                request_id,
                "忽略过期数据库切换回包"
            );
            return;
        }
        self.session.pending_database_requests.remove(&conn_name);
        self.session.refresh_connecting_flag();

        let is_active = self.session.manager.active.as_deref() == Some(conn_name.as_str());
        match result {
            Ok(tables) => {
                if let Some(conn) = self.session.manager.connections.get_mut(&conn_name) {
                    conn.set_database(db_name.clone(), tables.clone());
                }

                if is_active {
                    self.session.notifications.success(format!(
                        "已选择数据库 {} ({} 张表)",
                        db_name,
                        tables.len()
                    ));
                    self.session.autocomplete.set_tables(tables);
                    self.state
                        .sidebar_panel_state
                        .selection
                        .reset_for_database_change();
                    self.load_triggers();
                    self.load_routines();
                    self.switch_grid_workspace(None);
                    self.clear_result();
                    // 切库后若 ER 图打开，重载为新库的 schema（修复审计 ER-6）。
                    if self.state.show_er_diagram {
                        self.load_er_diagram_data();
                    }
                }
            }
            Err(e) => {
                if is_active {
                    self.session
                        .notifications
                        .error(format!("选择数据库失败: {}", e));
                    // 切库失败：上一个库的补全/触发器/存储过程已不再适用，清除以免显示陈旧元数据（修复审计 B6）。
                    self.session.autocomplete.clear();
                    self.state.sidebar_panel_state.clear_triggers();
                    self.state.sidebar_panel_state.clear_routines();
                    self.state.sidebar_panel_state.loading_triggers = false;
                    self.state.sidebar_panel_state.loading_routines = false;
                }
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理数据库删除完成消息
    fn handle_database_dropped(
        &mut self,
        _ctx: &egui::Context,
        conn_name: String,
        db_name: String,
        result: Result<(), String>,
    ) {
        let is_active = self.session.manager.active.as_deref() == Some(conn_name.as_str());

        match result {
            Ok(()) => {
                let mut dropped_selected_database = false;
                if let Some(conn) = self.session.manager.connections.get_mut(&conn_name) {
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
                    self.state
                        .sidebar_panel_state
                        .selection
                        .reset_for_database_change();
                    if dropped_selected_database {
                        self.switch_grid_workspace(None);
                        self.clear_result();
                        self.state.selected_table = None;
                        self.clear_search();
                        self.session.autocomplete.clear();
                        self.state.sidebar_panel_state.clear_triggers();
                        self.state.sidebar_panel_state.clear_routines();
                        self.state.sidebar_panel_state.loading_triggers = false;
                        self.state.sidebar_panel_state.loading_routines = false;
                        self.state.sidebar_section = ui::SidebarSection::Databases;
                        self.set_focus_area(ui::FocusArea::Sidebar);
                    }
                }

                self.session
                    .notifications
                    .success(format!("数据库 '{}' 已删除", db_name));
            }
            Err(error) => {
                self.session
                    .notifications
                    .error(format!("删除数据库 '{}' 失败: {}", db_name, error));
            }
        }

        self.session.needs_repaint = true;
    }

    /// 处理表删除完成消息
    fn handle_table_dropped(
        &mut self,
        _ctx: &egui::Context,
        conn_name: String,
        table_name: String,
        result: Result<(), String>,
    ) {
        let is_active = self.session.manager.active.as_deref() == Some(conn_name.as_str());

        match result {
            Ok(()) => {
                if let Some(conn) = self.session.manager.connections.get_mut(&conn_name) {
                    conn.tables.retain(|table| table != &table_name);
                    if is_active {
                        self.session.autocomplete.set_tables(conn.tables.clone());
                    }
                }

                self.remove_grid_workspace_for_table(&table_name);
                if is_active && self.state.selected_table.as_deref() == Some(table_name.as_str()) {
                    self.switch_grid_workspace(None);
                    self.clear_result();
                    self.state.selected_table = None;
                    self.state.sidebar_section = ui::SidebarSection::Tables;
                    self.set_focus_area(ui::FocusArea::Sidebar);
                }

                self.session
                    .notifications
                    .success(format!("表 '{}' 已删除", table_name));

                // 侧栏删表后，若 ER 图打开则重载，避免显示已删除的表（修复审计 ER-5）。
                if is_active && self.state.show_er_diagram {
                    self.load_er_diagram_data();
                }
            }
            Err(error) => {
                self.session
                    .notifications
                    .error(format!("删除表 '{}' 失败: {}", table_name, error));
            }
        }

        self.session.needs_repaint = true;
    }

    /// 处理静默表列表重载完成消息（schema 变更后失效重载）。
    ///
    /// 只在该连接仍是 active 时应用，静默刷新表列表与 autocomplete，不发连接提示。
    fn handle_active_tables_reloaded(
        &mut self,
        _ctx: &egui::Context,
        conn_name: String,
        _request_id: u64,
        result: Result<Vec<String>, String>,
    ) {
        if self.session.manager.active.as_deref() != Some(conn_name.as_str()) {
            tracing::debug!(connection = %conn_name, "忽略过期表列表刷新（连接已切换）");
            return;
        }
        match result {
            Ok(tables) => {
                if let Some(conn) = self.session.manager.connections.get_mut(&conn_name) {
                    conn.tables = tables.clone();
                }
                self.session.autocomplete.set_tables(tables);
            }
            Err(e) => {
                tracing::warn!(connection = %conn_name, error = %e, "刷新表列表失败");
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理查询完成消息
    fn handle_query_done(&mut self, _ctx: &egui::Context, payload: QueryDonePayload) {
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
        let was_user_cancelled = self
            .session
            .user_cancelled_query_requests
            .remove(&request_id);

        let target_tab_index = self
            .session
            .tab_manager
            .tabs
            .iter()
            .position(|t| t.id == tab_id);
        let is_stale_for_existing_tab = target_tab_index
            .and_then(|idx| self.session.tab_manager.tabs.get(idx))
            .is_some_and(|tab| tab.pending_request_id != Some(request_id));
        let should_update_active_query_time = should_record_active_query_time(
            target_tab_index,
            self.session.tab_manager.active_index,
            is_stale_for_existing_tab,
        );

        let sql_hints = crate::data::analyze_sql_for_ui(&sql);
        let is_update_or_delete = sql_hints.is_update_or_delete;
        let is_insert = sql_hints.is_insert;
        let is_drop_table = sql_hints.is_drop_table;

        let db_type = self
            .session
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

                self.session.query_history.add(
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
                        self.state.pending_drop_requests.remove(&request_id);
                    }
                    tracing::debug!(
                        tab_id = %tab_id,
                        request_id,
                        "忽略过期查询回包（请求已被新查询覆盖或标签已关闭）"
                    );
                    self.session.refresh_executing_flag();
                    self.session.needs_repaint = true;
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
                    let is_active_tab = idx == self.session.tab_manager.active_index;
                    if let Some(tab) = self.session.tab_manager.tabs.get_mut(idx) {
                        tab.result = Some(res.clone());
                        tab.executing = false;
                        tab.last_error = None;
                        tab.pending_request_id = None;
                        tab.query_time_ms = Some(elapsed_ms);
                        tab.last_message = Some(msg.clone());
                    }

                    if is_active_tab {
                        if should_update_active_query_time {
                            self.session.last_query_time_ms = Some(elapsed_ms);
                        }
                        self.session.notifications.success(&msg);
                        self.state.selected_row = None;
                        self.state.selected_cell = None;
                        self.clear_search();

                        // 根据 SQL 类型设置光标位置
                        if is_update_or_delete {
                            self.state.grid_state.scroll_to_row =
                                Some(self.state.grid_state.cursor.0);
                        } else if is_insert {
                            let last_row = res.rows.len().saturating_sub(1);
                            self.state.grid_state.cursor = (last_row, 0);
                            self.state.grid_state.scroll_to_row = Some(last_row);
                        }

                        if self.state.focus_area == ui::FocusArea::DataGrid {
                            self.state.grid_state.focused = true;
                        }

                        // 更新自动补全
                        if let Some(table) = &self.state.selected_table
                            && !res.columns.is_empty()
                        {
                            self.session
                                .autocomplete
                                .set_columns(table.clone(), res.columns.clone());
                        }

                        self.state.result = Some(res.clone());
                        self.reveal_bottom_panel_for_query(crate::core::BottomPanelTab::Results);
                        // 清除过期行删除标记，防止新数据行数变化后误删
                        self.state.grid_state.rows_to_delete.clear();
                    }
                } else {
                    tracing::debug!(tab_id = %tab_id, "查询回包对应的标签页已不存在");
                }

                if is_drop_table
                    && let Some((drop_conn_name, dropped_table)) =
                        self.state.pending_drop_requests.remove(&request_id)
                {
                    let is_current_active =
                        self.session.manager.active.as_deref() == Some(drop_conn_name.as_str());

                    if let Some(conn) = self.session.manager.connections.get_mut(&drop_conn_name) {
                        conn.tables.retain(|t| t != &dropped_table);
                        if is_current_active {
                            self.session.autocomplete.set_tables(conn.tables.clone());
                        }
                    }

                    if is_current_active
                        && self.state.selected_table.as_deref() == Some(&dropped_table)
                    {
                        self.switch_grid_workspace(None);
                        self.remove_grid_workspace_for_table(&dropped_table);
                        self.clear_result();
                    }
                }

                // 统一的 schema 失效级联：DDL 成功后重载受影响的派生视图
                // （表列表/autocomplete/ER 图、触发器、存储过程）。修复审计 ER-4/ER-6/SM-8。
                self.invalidate_after_schema_change(&sql_hints, &conn_name);
            }
            Err(e) => {
                let is_cancelled = is_cancelled_query_error(&e);
                if !is_cancelled {
                    self.session.query_history.add(sql, db_type, false, None);
                }
                if should_drop_query_error_as_stale(
                    is_stale_for_existing_tab,
                    is_cancelled,
                    was_user_cancelled,
                ) {
                    if is_drop_table {
                        self.state.pending_drop_requests.remove(&request_id);
                    }
                    tracing::debug!(
                        tab_id = %tab_id,
                        request_id,
                        error = %e,
                        "忽略过期查询错误回包（请求已被新查询覆盖或标签已关闭）"
                    );
                    self.session.refresh_executing_flag();
                    self.session.needs_repaint = true;
                    return;
                }

                if target_tab_index.is_none() {
                    tracing::debug!(tab_id = %tab_id, "查询错误回包对应的标签页已不存在");
                    if is_drop_table {
                        self.state.pending_drop_requests.remove(&request_id);
                    }
                    self.session.refresh_executing_flag();
                    self.session.needs_repaint = true;
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
                    self.session.notifications.warning(&err_msg);
                } else {
                    self.session.notifications.error(&err_msg);
                }
                if let Some(idx) = target_tab_index {
                    let is_active_tab = idx == self.session.tab_manager.active_index;
                    if let Some(tab) = self.session.tab_manager.tabs.get_mut(idx) {
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
                            self.session.last_query_time_ms = Some(elapsed_ms);
                        }
                        if !is_cancelled {
                            self.clear_result();
                            self.reveal_bottom_panel_for_query(
                                crate::core::BottomPanelTab::Messages,
                            );
                        }
                    }
                }

                if is_drop_table {
                    self.state.pending_drop_requests.remove(&request_id);
                }
            }
        }
        self.session.refresh_executing_flag();
        self.session.needs_repaint = true;
    }

    /// schema 变更失效级联：DDL 成功后重载受影响的派生视图。
    ///
    /// 仅对当前 active 连接生效（回包能走到这里已保证非 stale）。
    /// 复用既有重载原语，不新增异步通道；各原语自带 request_id/generation stale-guard。
    ///
    /// 修复审计 ER-4（编辑器 CREATE/DROP/ALTER TABLE 后 ER 不刷新）、
    /// ER-6（表结构变更后 ER 陈旧）、SM-8（CREATE/DROP TRIGGER/ROUTINE 后侧栏陈旧）。
    fn invalidate_after_schema_change(&mut self, hints: &crate::data::SqlUiHints, conn_name: &str) {
        if self.session.manager.active.as_deref() != Some(conn_name) {
            return;
        }

        let invalidation = schema_invalidation_for(hints);
        if invalidation.reload_tables {
            // 重新拉取表列表（同时刷新 autocomplete）。
            self.reload_active_tables();
            // ER 图仅在打开时重载，避免无谓异步加载。
            if self.state.show_er_diagram {
                self.load_er_diagram_data();
            }
        }
        if invalidation.reload_triggers {
            self.load_triggers();
        }
        if invalidation.reload_routines {
            self.load_routines();
        }
    }

    /// 处理导入完成消息
    fn handle_import_done(
        &mut self,
        _ctx: &egui::Context,
        result: Result<crate::data::ImportExecutionReport, String>,
        elapsed_ms: u64,
    ) {
        self.session.import_executing = false;
        self.session.refresh_executing_flag();

        match result {
            Ok(report) => {
                if report.failed == 0 {
                    self.session.notifications.success(format!(
                        "导入完成：成功 {} / {} 条 ({}ms)",
                        report.succeeded, report.total, elapsed_ms
                    ));
                } else {
                    let detail = report.first_error.as_deref().unwrap_or("部分语句执行失败");
                    self.session.notifications.warning(format!(
                        "导入部分完成：成功 {}，失败 {}，总计 {} 条 ({}ms)。首个错误: {}",
                        report.succeeded, report.failed, report.total, elapsed_ms, detail
                    ));
                }
            }
            Err(e) => {
                self.session.notifications.error(format!("导入失败: {}", e));
            }
        }

        self.session.needs_repaint = true;
    }

    /// 处理网格保存批次完成消息
    ///
    /// 成功（整批提交）→ 清除编辑状态并刷新该表（修复 B1）。
    /// 失败（整批回滚）→ 保留编辑、显示错误，便于用户修正后重试。
    fn handle_grid_save_done(
        &mut self,
        ctx: &egui::Context,
        result: Result<crate::data::ImportExecutionReport, String>,
        table: String,
        request_id: u64,
        elapsed_ms: u64,
    ) {
        self.session.grid_save_executing = false;
        self.session.refresh_executing_flag();

        // 过期回包保护：仅处理最新一次网格保存的结果。
        if self.session.pending_grid_save_request != Some(request_id) {
            tracing::debug!(request_id, "忽略过期网格保存回包");
            self.session.needs_repaint = true;
            return;
        }
        self.session.pending_grid_save_request = None;

        match (classify_grid_save_outcome(&result), result) {
            (GridSaveOutcome::CommittedClearEdits, Ok(report)) => {
                self.session.notifications.success(format!(
                    "已保存 {} 处修改到「{}」({}ms)",
                    report.succeeded, table, elapsed_ms
                ));
                // 整批成功：清除编辑状态并刷新该表以反映数据库真实数据。
                self.state.grid_state.clear_edits();
                if self.state.selected_table.as_deref() == Some(table.as_str()) {
                    self.dispatch_app_action(
                        ctx,
                        crate::app::action::action_system::AppAction::RefreshSelectedTable,
                    );
                }
            }
            (_, Ok(report)) => {
                // 事务已回滚，DB 未变；保留编辑供用户修正后重试。
                let detail = report.first_error.as_deref().unwrap_or("部分语句执行失败");
                self.session.notifications.error(format!(
                    "保存失败，已回滚（{} 条未提交）。错误: {}",
                    report.total.saturating_sub(report.succeeded),
                    detail
                ));
            }
            (_, Err(e)) => {
                self.session
                    .notifications
                    .error(format!("保存失败，已回滚: {}", e));
            }
        }

        self.session.needs_repaint = true;
    }

    /// 处理主键获取完成消息
    fn handle_primary_key_fetched(
        &mut self,
        _ctx: &egui::Context,
        table_name: String,
        pk_column: Option<String>,
    ) {
        if self.state.selected_table.as_deref() == Some(&table_name) {
            if let Some(pk_name) = pk_column {
                if let Some(result) = &self.state.result
                    && let Some(idx) = result.columns.iter().position(|c| c == &pk_name)
                {
                    self.state.grid_state.primary_key_column = Some(idx);
                }
            } else {
                self.state.grid_state.primary_key_column = None;
            }
        }
        self.session.needs_repaint = true;
    }

    /// 检查异步元数据回包是否仍对应当前连接上下文
    fn metadata_context_matches_current(&self, conn_name: &str, db_name: &Option<String>) -> bool {
        if self.session.manager.active.as_deref() != Some(conn_name) {
            return false;
        }

        let Some(conn) = self.session.manager.connections.get(conn_name) else {
            return false;
        };

        match conn.config.db_type {
            crate::data::DatabaseType::SQLite => true,
            _ => conn.selected_database == *db_name,
        }
    }

    /// 处理触发器获取完成消息
    fn handle_triggers_fetched(
        &mut self,
        _ctx: &egui::Context,
        conn_name: String,
        db_name: Option<String>,
        request_id: u64,
        result: Result<Vec<crate::data::TriggerInfo>, String>,
    ) {
        let is_latest = self.session.pending_triggers_request.as_ref().is_some_and(
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
        self.session.pending_triggers_request = None;

        if !self.metadata_context_matches_current(&conn_name, &db_name) {
            tracing::debug!(
                connection = %conn_name,
                database = ?db_name,
                "忽略过期触发器回包"
            );
            self.state.sidebar_panel_state.loading_triggers = false;
            return;
        }

        self.state.sidebar_panel_state.loading_triggers = false;
        match result {
            Ok(triggers) => {
                self.state.sidebar_panel_state.set_triggers(triggers);
            }
            Err(e) => {
                self.session
                    .notifications
                    .error(format!("加载触发器失败: {}", e));
                // 在面板内记录错误，区分"加载失败"与"确实没有触发器"（审计 SM-6）。
                self.state.sidebar_panel_state.set_triggers_error(e);
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理存储过程/函数获取完成消息
    fn handle_routines_fetched(
        &mut self,
        _ctx: &egui::Context,
        conn_name: String,
        db_name: Option<String>,
        request_id: u64,
        result: Result<Vec<crate::data::RoutineInfo>, String>,
    ) {
        let is_latest = self.session.pending_routines_request.as_ref().is_some_and(
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
        self.session.pending_routines_request = None;

        if !self.metadata_context_matches_current(&conn_name, &db_name) {
            tracing::debug!(
                connection = %conn_name,
                database = ?db_name,
                "忽略过期存储过程回包"
            );
            self.state.sidebar_panel_state.loading_routines = false;
            return;
        }

        self.state.sidebar_panel_state.loading_routines = false;
        match result {
            Ok(routines) => {
                self.state.sidebar_panel_state.set_routines(routines);
            }
            Err(e) => {
                // SQLite 不支持存储过程：当作"确实没有"，不算错误。
                if e.contains("不支持") {
                    self.state.sidebar_panel_state.set_routines(Vec::new());
                } else {
                    self.session
                        .notifications
                        .error(format!("加载存储过程失败: {}", e));
                    // 在面板内记录错误，区分加载失败与空列表（审计 SM-7）。
                    self.state.sidebar_panel_state.set_routines_error(e);
                }
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理外键获取完成消息
    fn handle_foreign_keys_fetched(
        &mut self,
        _ctx: &egui::Context,
        generation: u64,
        result: Result<Vec<crate::data::ForeignKeyInfo>, String>,
    ) {
        // 丢弃过期连接/上一轮加载的外键回包（审计 B6-ER）。
        if generation != self.state.er_diagram_state.current_load_generation() {
            tracing::debug!(generation, "忽略过期 ER 外键回包");
            return;
        }
        match result {
            Ok(fks) => {
                let foreign_key_columns = collect_er_foreign_key_columns(&fks);
                self.state
                    .er_diagram_state
                    .set_foreign_key_columns(foreign_key_columns);

                let relationships = collect_er_relationships_from_foreign_keys(fks);
                let rel_count = relationships.len();
                self.state.er_diagram_state.relationships = relationships;
                tracing::debug!(relationship_count = rel_count, "ER 图外键关系已返回");
                self.finalize_er_diagram_load_if_ready();
            }
            Err(e) => {
                self.state.er_diagram_state.mark_foreign_keys_resolved();
                self.session
                    .notifications
                    .error(format!("加载外键关系失败: {}", e));
                // 在画布上显示错误卡，而不是静默退化为空（审计 ER-3）。
                self.state
                    .er_diagram_state
                    .set_error(format!("加载外键关系失败: {}", e));
                self.finalize_er_diagram_load_if_ready();
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理 ER 表列信息获取完成消息
    fn handle_er_table_columns_fetched(
        &mut self,
        _ctx: &egui::Context,
        generation: u64,
        table_name: String,
        result: Result<Vec<crate::data::ColumnInfo>, String>,
    ) {
        // 丢弃过期连接/上一轮加载的列回包（审计 B6-ER）。
        if generation != self.state.er_diagram_state.current_load_generation() {
            tracing::debug!(generation, table = %table_name, "忽略过期 ER 列回包");
            return;
        }
        match result {
            Ok(columns) => {
                let er_columns: Vec<ui::ERColumn> = columns
                    .into_iter()
                    .map(|c| ui::ERColumn {
                        is_foreign_key: self
                            .state
                            .er_diagram_state
                            .is_foreign_key_column(&table_name, &c.name),
                        name: c.name,
                        data_type: c.data_type,
                        is_primary_key: c.is_primary_key,
                        nullable: c.is_nullable,
                        default_value: c.default_value,
                    })
                    .collect();

                let display_mode = self.state.er_diagram_state.card_display_mode();
                if let Some(er_table) = self
                    .state
                    .er_diagram_state
                    .tables
                    .iter_mut()
                    .find(|t| t.name == table_name)
                {
                    er_table.columns = er_columns;
                    // 立即计算表格尺寸，确保布局和关系线渲染正确
                    ui::calculate_table_size_for_mode(er_table, display_mode);
                }
            }
            Err(e) => {
                self.session
                    .notifications
                    .warning(format!("获取表 {} 结构失败: {}", table_name, e));
            }
        }
        self.state
            .er_diagram_state
            .mark_table_request_resolved(&table_name);
        self.finalize_er_diagram_load_if_ready();
        self.session.needs_repaint = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErDiagramReadyKind, GridSaveOutcome, apply_default_er_diagram_layout,
        apply_ready_state_er_diagram_layout, clamp_grid_cursor_for_result,
        classify_grid_save_outcome, collect_er_relationships_from_foreign_keys,
        er_diagram_ready_message, is_cancelled_query_error, resolve_er_diagram_ready_state,
        schema_invalidation_for, select_ready_state_er_layout_strategy,
        should_drop_query_error_as_stale, should_record_active_query_time,
    };
    use crate::data::ForeignKeyInfo;
    use crate::data::ImportExecutionReport;
    use crate::data::QueryResult;
    use crate::data::analyze_sql_for_ui;
    use crate::ui::{ERLayoutStrategy, ERTable, RelationType, Relationship, RelationshipOrigin};

    #[test]
    fn schema_invalidation_maps_ddl_to_reloads() {
        // 审计级联：CREATE TABLE → 重载表；CREATE TRIGGER → 重载触发器；
        // CREATE FUNCTION → 重载存储过程；普通 DML/SELECT → 无重载。
        let table = schema_invalidation_for(&analyze_sql_for_ui("CREATE TABLE t(id INT);"));
        assert!(table.reload_tables);
        assert!(!table.reload_triggers);
        assert!(!table.reload_routines);

        let alter = schema_invalidation_for(&analyze_sql_for_ui("ALTER TABLE t ADD c INT;"));
        assert!(alter.reload_tables);

        let trig = schema_invalidation_for(&analyze_sql_for_ui(
            "CREATE TRIGGER g AFTER INSERT ON t BEGIN END;",
        ));
        assert!(trig.reload_triggers);
        assert!(!trig.reload_tables);

        let routine = schema_invalidation_for(&analyze_sql_for_ui(
            "CREATE OR REPLACE FUNCTION f() RETURNS INT AS $$ $$;",
        ));
        assert!(routine.reload_routines);
        assert!(!routine.reload_tables);

        let dml = schema_invalidation_for(&analyze_sql_for_ui("UPDATE t SET v = 1;"));
        assert!(!dml.reload_tables && !dml.reload_triggers && !dml.reload_routines);

        let select = schema_invalidation_for(&analyze_sql_for_ui("SELECT * FROM t;"));
        assert!(!select.reload_tables && !select.reload_triggers && !select.reload_routines);
    }

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
    fn grid_save_clears_edits_only_when_whole_batch_commits() {
        // B1: 整批成功 → 清编辑
        let ok = Ok(ImportExecutionReport {
            total: 3,
            succeeded: 3,
            failed: 0,
            first_error: None,
        });
        assert_eq!(
            classify_grid_save_outcome(&ok),
            GridSaveOutcome::CommittedClearEdits
        );
    }

    #[test]
    fn grid_save_keeps_edits_on_partial_or_failed_batch() {
        // B1/B2: 有失败语句 → 保留编辑（事务已回滚）
        let partial = Ok(ImportExecutionReport {
            total: 3,
            succeeded: 1,
            failed: 1,
            first_error: Some("NOT NULL constraint failed".to_string()),
        });
        assert_eq!(
            classify_grid_save_outcome(&partial),
            GridSaveOutcome::RolledBackKeepEdits
        );

        // 执行层直接报错（事务回滚）→ 保留编辑
        let err: Result<ImportExecutionReport, String> =
            Err("事务已回滚，第 2 条语句执行失败".to_string());
        assert_eq!(
            classify_grid_save_outcome(&err),
            GridSaveOutcome::RolledBackKeepEdits
        );
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
            origin: RelationshipOrigin::Explicit,
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
    fn analyze_er_graph_uses_grid_strategy_when_relationships_are_empty() {
        let summary = crate::ui::analyze_er_graph(&[ERTable::new("customers".into())], &[]);

        assert_eq!(summary.strategy, ERLayoutStrategy::Grid);
    }

    #[test]
    fn analyze_er_graph_uses_component_strategy_for_disconnected_relationships() {
        let tables = vec![
            ERTable::new("customers".into()),
            ERTable::new("orders".into()),
            ERTable::new("products".into()),
            ERTable::new("suppliers".into()),
        ];
        let summary = crate::ui::analyze_er_graph(
            &tables,
            &[
                relationship("orders", "customers"),
                relationship("products", "suppliers"),
            ],
        );

        assert_eq!(summary.strategy, ERLayoutStrategy::Component);
    }

    #[test]
    fn select_ready_state_er_layout_strategy_prefers_stable_incremental_with_snapshot() {
        let graph = crate::ui::build_er_graph(&[ERTable::new("customers".into())], &[]);

        let strategy = select_ready_state_er_layout_strategy(&graph, true);

        assert_eq!(strategy, ERLayoutStrategy::StableIncremental);
    }

    #[test]
    fn select_ready_state_er_layout_strategy_delegates_to_graph_selector_without_snapshot() {
        let tables = vec![
            ERTable::new("customers".into()),
            ERTable::new("orders".into()),
        ];
        let relationships = vec![relationship("orders", "customers")];
        let graph = crate::ui::build_er_graph(&tables, &relationships);

        let strategy = select_ready_state_er_layout_strategy(&graph, false);

        assert_eq!(strategy, ERLayoutStrategy::Relation);
    }

    #[test]
    fn apply_default_er_diagram_layout_keeps_grid_positions_without_relationships() {
        let mut tables = vec![
            ERTable::new("customers".into()),
            ERTable::new("orders".into()),
        ];

        apply_default_er_diagram_layout(&mut tables, &[]);

        assert_eq!(tables[0].position, egui::pos2(60.0, 50.0));
        assert_eq!(tables[1].position, egui::pos2(300.0, 50.0));
    }

    #[test]
    fn apply_default_er_diagram_layout_refines_grid_when_relationships_exist() {
        let mut tables = vec![
            ERTable::new("customers".into()),
            ERTable::new("orders".into()),
        ];
        let relationships = vec![relationship("orders", "customers")];

        apply_default_er_diagram_layout(&mut tables, &relationships);

        assert_ne!(tables[0].position, egui::pos2(60.0, 50.0));
        assert_ne!(tables[1].position, egui::pos2(300.0, 50.0));
    }

    #[test]
    fn finalize_er_diagram_load_restores_snapshot_when_table_names_match_exactly() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        app.state.er_diagram_state.tables = vec![
            ERTable::new("customers".into()),
            ERTable::new("orders".into()),
        ];
        app.state.er_diagram_state.set_pending_layout_restore(Some(
            std::collections::HashMap::from([
                ("customers".to_string(), egui::pos2(320.0, 140.0)),
                ("orders".to_string(), egui::pos2(80.0, 420.0)),
            ]),
        ));
        app.state.er_diagram_state.loading = false;
        app.state.er_diagram_state.relationships = vec![relationship("orders", "customers")];

        app.finalize_er_diagram_load_if_ready();

        assert_eq!(
            app.state.er_diagram_state.tables[0].position,
            egui::pos2(320.0, 140.0)
        );
        assert_eq!(
            app.state.er_diagram_state.tables[1].position,
            egui::pos2(80.0, 420.0)
        );
        assert!(!app.state.er_diagram_state.has_pending_layout_restore());
    }

    #[test]
    fn apply_ready_state_er_diagram_layout_uses_stable_incremental_path_for_exact_snapshot_match() {
        let mut customers = ERTable::new("customers".into());
        customers.size = egui::vec2(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = egui::vec2(180.0, 200.0);

        let mut state = crate::ui::ERDiagramState::new();
        state.tables = vec![customers, orders];
        state.relationships = vec![relationship("orders", "customers")];
        state.set_pending_layout_restore(Some(std::collections::HashMap::from([
            ("customers".to_string(), egui::pos2(320.0, 140.0)),
            ("orders".to_string(), egui::pos2(80.0, 420.0)),
        ])));

        apply_ready_state_er_diagram_layout(&mut state);

        assert_eq!(state.tables[0].position, egui::pos2(320.0, 140.0));
        assert_eq!(state.tables[1].position, egui::pos2(80.0, 420.0));
        assert!(!state.has_pending_layout_restore());
    }

    #[test]
    fn finalize_er_diagram_load_restores_matching_snapshot_after_strategy_layout() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        app.state.er_diagram_state.tables = vec![
            ERTable::new("customers".into()),
            ERTable::new("orders".into()),
            ERTable::new("invoices".into()),
        ];
        app.state.er_diagram_state.set_pending_layout_restore(Some(
            std::collections::HashMap::from([
                ("customers".to_string(), egui::pos2(320.0, 140.0)),
                ("orders".to_string(), egui::pos2(80.0, 420.0)),
                ("legacy".to_string(), egui::pos2(920.0, 40.0)),
            ]),
        ));
        app.state.er_diagram_state.loading = false;
        app.state.er_diagram_state.relationships = vec![relationship("orders", "customers")];

        app.finalize_er_diagram_load_if_ready();

        assert_eq!(
            app.state.er_diagram_state.tables[0].position,
            egui::pos2(320.0, 140.0)
        );
        assert_eq!(
            app.state.er_diagram_state.tables[1].position,
            egui::pos2(80.0, 420.0)
        );
        assert_ne!(
            app.state.er_diagram_state.tables[2].position,
            egui::pos2(320.0, 140.0)
        );
        assert_ne!(
            app.state.er_diagram_state.tables[2].position,
            egui::pos2(80.0, 420.0)
        );
        assert!(!app.state.er_diagram_state.has_pending_layout_restore());
    }

    #[test]
    fn finalize_er_diagram_load_partial_restore_moves_new_table_off_restored_tables() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let mut customers = ERTable::new("customers".into());
        customers.size = egui::vec2(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = egui::vec2(180.0, 200.0);
        let mut invoices = ERTable::new("invoices".into());
        invoices.size = egui::vec2(180.0, 200.0);
        app.state.er_diagram_state.tables = vec![customers, orders, invoices];
        app.state.er_diagram_state.set_pending_layout_restore(Some(
            std::collections::HashMap::from([
                ("customers".to_string(), egui::pos2(540.0, 50.0)),
                ("orders".to_string(), egui::pos2(300.0, 50.0)),
                ("legacy".to_string(), egui::pos2(80.0, 420.0)),
            ]),
        ));
        app.state.er_diagram_state.loading = false;

        app.finalize_er_diagram_load_if_ready();

        let customers = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let orders = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "orders")
            .unwrap();
        let invoices = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();

        assert_eq!(customers.position, egui::pos2(540.0, 50.0));
        assert_eq!(orders.position, egui::pos2(300.0, 50.0));
        assert!(!invoices.rect().intersects(customers.rect()));
        assert!(!invoices.rect().intersects(orders.rect()));
        assert!(!app.state.er_diagram_state.has_pending_layout_restore());
    }

    #[test]
    fn finalize_er_diagram_load_partial_restore_reanchors_related_new_table_near_restored_neighbor()
    {
        let mut strategy_customers = ERTable::new("customers".into());
        strategy_customers.size = egui::vec2(180.0, 200.0);
        let mut strategy_orders = ERTable::new("orders".into());
        strategy_orders.size = egui::vec2(180.0, 200.0);
        let mut strategy_invoices = ERTable::new("invoices".into());
        strategy_invoices.size = egui::vec2(180.0, 200.0);
        let relationships = vec![relationship("invoices", "orders")];
        let mut strategy_tables = vec![strategy_customers, strategy_orders, strategy_invoices];
        apply_default_er_diagram_layout(&mut strategy_tables, &relationships);
        let strategy_invoice = strategy_tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap()
            .position
            + egui::vec2(90.0, 100.0);

        let mut app = crate::app::DbManagerApp::new_for_test();
        let mut customers = ERTable::new("customers".into());
        customers.size = egui::vec2(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = egui::vec2(180.0, 200.0);
        let mut invoices = ERTable::new("invoices".into());
        invoices.size = egui::vec2(180.0, 200.0);
        app.state.er_diagram_state.tables = vec![customers, orders, invoices];
        let restored_orders = egui::pos2(660.0, 50.0);
        app.state.er_diagram_state.set_pending_layout_restore(Some(
            std::collections::HashMap::from([
                ("customers".to_string(), egui::pos2(900.0, 50.0)),
                ("orders".to_string(), restored_orders),
            ]),
        ));
        app.state.er_diagram_state.loading = false;
        app.state.er_diagram_state.relationships = relationships;

        let strategy_distance =
            strategy_invoice.distance(restored_orders + egui::vec2(90.0, 100.0));

        app.finalize_er_diagram_load_if_ready();

        let orders = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "orders")
            .unwrap();
        let invoices = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();
        let restored_distance = invoices.center().distance(orders.center());

        assert!(restored_distance < strategy_distance);
        assert!(!invoices.rect().intersects(orders.rect()));
        assert!(!app.state.er_diagram_state.has_pending_layout_restore());
    }

    #[test]
    fn finalize_er_diagram_load_partial_restore_places_referencing_new_table_below_restored_parent()
    {
        let mut orders = ERTable::new("orders".into());
        orders.size = egui::vec2(180.0, 200.0);
        let mut invoices = ERTable::new("invoices".into());
        invoices.size = egui::vec2(180.0, 200.0);
        let relationships = vec![relationship("invoices", "orders")];

        let mut app = crate::app::DbManagerApp::new_for_test();
        app.state.er_diagram_state.tables = vec![orders, invoices];
        app.state.er_diagram_state.set_pending_layout_restore(Some(
            std::collections::HashMap::from([("orders".to_string(), egui::pos2(660.0, 50.0))]),
        ));
        app.state.er_diagram_state.loading = false;
        app.state.er_diagram_state.relationships = relationships;

        app.finalize_er_diagram_load_if_ready();

        let orders = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "orders")
            .unwrap();
        let invoices = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "invoices")
            .unwrap();

        assert!(invoices.rect().top() >= orders.rect().bottom() + 39.0);
        assert!(!invoices.rect().intersects(orders.rect()));
        assert!(!app.state.er_diagram_state.has_pending_layout_restore());
    }

    #[test]
    fn finalize_er_diagram_load_partial_restore_keeps_bridge_table_between_restored_parent_and_child()
     {
        let mut customers = ERTable::new("customers".into());
        customers.size = egui::vec2(180.0, 200.0);
        let mut order_items = ERTable::new("order_items".into());
        order_items.size = egui::vec2(180.0, 200.0);
        let mut orders = ERTable::new("orders".into());
        orders.size = egui::vec2(180.0, 200.0);
        let relationships = vec![
            relationship("orders", "customers"),
            relationship("order_items", "orders"),
        ];

        let mut app = crate::app::DbManagerApp::new_for_test();
        app.state.er_diagram_state.tables = vec![customers, order_items, orders];
        app.state.er_diagram_state.set_pending_layout_restore(Some(
            std::collections::HashMap::from([
                ("customers".to_string(), egui::pos2(660.0, 50.0)),
                ("order_items".to_string(), egui::pos2(940.0, 250.0)),
            ]),
        ));
        app.state.er_diagram_state.loading = false;
        app.state.er_diagram_state.relationships = relationships;

        app.finalize_er_diagram_load_if_ready();

        let customers = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "customers")
            .unwrap();
        let order_items = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "order_items")
            .unwrap();
        let orders = app
            .state
            .er_diagram_state
            .tables
            .iter()
            .find(|table| table.name == "orders")
            .unwrap();

        assert!(orders.center().y > customers.center().y);
        assert!(orders.center().y < order_items.center().y);
        assert!(!orders.rect().intersects(customers.rect()));
        assert!(!orders.rect().intersects(order_items.rect()));
        assert!(!app.state.er_diagram_state.has_pending_layout_restore());
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
