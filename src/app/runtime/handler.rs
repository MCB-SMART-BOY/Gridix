//! 消息处理模块
//!
//! 处理从异步任务返回的各种消息，更新应用状态。

use eframe::egui;

use super::{DbManagerApp, Message};
use crate::domain::execution::StatementOutcome;
use crate::domain::explain::is_explain_sql;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ErDiagramReadyKind {
    Explicit(usize),
    Inferred(usize),
    Empty,
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

/// 网格保存批次的可确认状态；未知结果不允许重试同一草稿。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GridSaveOutcome {
    CommittedClearEdits,
    RolledBackKeepEdits,
    UnknownKeepEditsLocked,
}

fn classify_grid_save_outcome(
    result: &Result<crate::data::ImportExecutionReport, crate::data::DbError>,
) -> GridSaveOutcome {
    match result {
        Ok(report) if report.failed == 0 => GridSaveOutcome::CommittedClearEdits,
        Err(crate::data::DbError::MutationOutcomeUnknown { .. }) => {
            GridSaveOutcome::UnknownKeepEditsLocked
        }
        _ => GridSaveOutcome::RolledBackKeepEdits,
    }
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
    pub(super) fn finalize_er_diagram_load_if_ready(&mut self) {
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
    pub fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.session.rx.try_recv() {
            match msg {
                Message::RuntimeEvent(event) => self.handle_runtime_event(ctx, event),
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
        connection: crate::domain::ids::ConnectionId,
        name: String,
        result: Result<Vec<String>, String>,
    ) {
        // 连接必须仍是回包所指的那一个：同名重建（编辑连接配置）会分配新的 `ConnectionId`，
        // 旧连接的慢回包否则会写进新连接对象并顶掉新回包。
        if !self.does_runtime_connection_match(&name, connection) {
            tracing::debug!(
                connection = %name,
                "忽略连接到其他连接实例的回包"
            );
            return;
        }
        // 请求是否仍在途由 pending 表表达；过期回包由事件入口的
        // `TaskRegistry::is_current()` 拦截，无需再比较请求号。
        if !self.session.pending_connect_requests.contains(&name) {
            tracing::debug!(
                connection = %name,
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
                    if conn.config.db_type == crate::types::DatabaseType::SQLite {
                        conn.set_connected(tables.clone());
                    } else {
                        conn.set_connected_with_databases(tables.clone());
                    }
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
                    // 连接后若 ER 图打开，为新连接重载 schema（修复审计 CONN-F2）。
                    if self.state.show_er_diagram {
                        self.load_er_diagram_data();
                    }
                }
            }
            Err(e) => {
                if is_active {
                    self.handle_connection_error(&name, e);
                } else if let Some(conn) = self.session.manager.connections.get_mut(&name) {
                    conn.set_error(e);
                    self.cancel_queries_for_connection(connection);
                    crate::data::close_pg_connection_sessions(connection);
                }
            }
        }
        self.session.needs_repaint = true;
    }

    /// 处理数据库选择完成消息
    fn handle_database_selected(
        &mut self,
        _ctx: &egui::Context,
        connection: crate::domain::ids::ConnectionId,
        conn_name: String,
        db_name: String,
        result: Result<Vec<String>, String>,
    ) {
        if !self.does_runtime_connection_match(&conn_name, connection) {
            tracing::debug!(
                connection = %conn_name,
                database = %db_name,
                "忽略连接到其他连接实例的切库回包"
            );
            return;
        }
        let still_pending = self
            .session
            .pending_database_requests
            .get(&conn_name)
            .is_some_and(|pending_db| pending_db == &db_name);
        if !still_pending {
            tracing::debug!(
                connection = %conn_name,
                database = %db_name,
                "忽略过期数据库切换回包"
            );
            return;
        }
        self.session.pending_database_requests.remove(&conn_name);
        self.session.refresh_connecting_flag();

        let is_active = self.session.manager.active.as_deref() == Some(conn_name.as_str());
        match result {
            Ok(tables) => {
                if self
                    .session
                    .manager
                    .connections
                    .get(&conn_name)
                    .is_some_and(|conn| conn.selected_database.as_deref() != Some(db_name.as_str()))
                {
                    self.cancel_queries_for_connection(connection);
                    crate::data::close_pg_connection_sessions(connection);
                }
                if is_active {
                    self.switch_grid_workspace(None);
                }
                if let Some(conn) = self.session.manager.connections.get_mut(&conn_name) {
                    conn.set_database(db_name.clone(), tables.clone());
                }

                if is_active {
                    self.session.notifications.success(format!(
                        "已选择数据库 {} ({} 张表)",
                        db_name,
                        tables.len()
                    ));
                    if let Some(catalog) = self
                        .session
                        .schema_catalogs
                        .get(&(connection, db_name.clone()))
                    {
                        self.session.autocomplete.set_from_catalog(catalog);
                    } else {
                        self.session.autocomplete.clear();
                        self.session.autocomplete.set_tables(tables);
                    }
                    self.state
                        .sidebar_panel_state
                        .selection
                        .reset_for_database_change();
                    self.load_triggers();
                    self.load_routines();
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

    fn does_runtime_connection_match(
        &self,
        conn_name: &str,
        connection: crate::domain::ids::ConnectionId,
    ) -> bool {
        self.session
            .manager
            .connections
            .get(conn_name)
            .is_some_and(|conn| conn.id == connection)
    }

    /// 处理数据库删除完成消息
    fn handle_database_dropped(
        &mut self,
        _ctx: &egui::Context,
        connection: crate::domain::ids::ConnectionId,
        conn_name: String,
        db_name: String,
        result: Result<(), String>,
    ) {
        if !self.does_runtime_connection_match(&conn_name, connection) {
            tracing::debug!(
                connection = ?connection,
                conn_name = %conn_name,
                "忽略连接身份不匹配的数据库删除回包"
            );
            return;
        }
        let is_active = self.session.manager.active.as_deref() == Some(conn_name.as_str());

        match result {
            Ok(()) => {
                let is_selected_database = self
                    .session
                    .manager
                    .connections
                    .get(&conn_name)
                    .is_some_and(|conn| conn.selected_database.as_deref() == Some(&db_name));
                if is_active && is_selected_database {
                    self.switch_grid_workspace(None);
                }
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

                self.remove_grid_workspaces_for_database(&conn_name, &db_name);
                if is_active {
                    self.state
                        .sidebar_panel_state
                        .selection
                        .reset_for_database_change();
                    if dropped_selected_database {
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
        connection: crate::domain::ids::ConnectionId,
        conn_name: String,
        database: Option<String>,
        table_name: String,
        result: Result<(), String>,
    ) {
        if !self.does_runtime_connection_match(&conn_name, connection) {
            tracing::debug!(
                connection = ?connection,
                conn_name = %conn_name,
                "忽略连接身份不匹配的表删除回包"
            );
            return;
        }
        let is_active_database = self.session.manager.active.as_deref() == Some(conn_name.as_str())
            && self
                .session
                .manager
                .get_active()
                .is_some_and(|conn| conn.selected_database.as_ref() == database.as_ref());
        let is_active_target =
            is_active_database && self.state.selected_table.as_deref() == Some(table_name.as_str());

        match result {
            Ok(()) => {
                if is_active_target {
                    self.switch_grid_workspace(None);
                }
                if let Some(conn) = self.session.manager.connections.get_mut(&conn_name)
                    && conn.selected_database.as_ref() == database.as_ref()
                {
                    conn.tables.retain(|table| table != &table_name);
                    if is_active_database {
                        self.session.autocomplete.set_tables(conn.tables.clone());
                    }
                }

                self.remove_grid_workspace_for_table(&conn_name, &database, &table_name);
                if is_active_target {
                    self.clear_result();
                    self.state.selected_table = None;
                    self.state.sidebar_section = ui::SidebarSection::Tables;
                    self.set_focus_area(ui::FocusArea::Sidebar);
                }

                self.session
                    .notifications
                    .success(format!("表 '{}' 已删除", table_name));

                // 侧栏删表后，若 ER 图打开则重载，避免显示已删除的表（修复审计 ER-5）。
                if is_active_database && self.state.show_er_diagram {
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
        connection: crate::domain::ids::ConnectionId,
        conn_name: String,
        database: String,
        result: Result<Vec<String>, String>,
    ) {
        let is_current_database = self
            .session
            .manager
            .connections
            .get(&conn_name)
            .is_some_and(|conn| {
                conn.selected_database
                    .as_deref()
                    .unwrap_or(&conn.config.database)
                    == database.as_str()
            });
        if self.session.manager.active.as_deref() != Some(conn_name.as_str())
            || !self.does_runtime_connection_match(&conn_name, connection)
            || !is_current_database
        {
            tracing::debug!(
                connection = ?connection,
                conn_name = %conn_name,
                database = %database,
                "忽略非当前数据库的表列表刷新"
            );
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

    /// schema 变更失效级联：DDL 成功后重载受影响的派生视图。
    ///
    /// 仅对当前 active 连接生效；异步完成统一通过 TaskRegistry stale guard。
    /// 复用既有重载原语，不新增异步通道。
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
        result: Result<crate::data::ImportExecutionReport, crate::data::DbError>,
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
            Err(error @ crate::data::DbError::ImportOutcomeUnknown { .. }) => {
                self.import_outcome_unknown = true;
                self.state.import_state.requires_unknown_confirmation = true;
                self.state.import_state.has_confirmed_unknown_outcome = false;
                self.session.notifications.error(format!(
                    "导入事务结果未知，不能直接重试；先核对数据库状态，再明确确认重新导入: {error}"
                ));
            }
            Err(e) => {
                self.session.notifications.error(format!("导入失败: {e}"));
            }
        }

        self.session.needs_repaint = true;
    }

    /// 处理网格保存批次完成消息
    ///
    /// 成功（整批提交）→ 清除编辑状态并刷新该表；确认回滚→保留编辑供重试；
    /// 提交/回滚回执未知→隔离草稿，等待用户核对数据库状态。
    fn handle_grid_save_done(
        &mut self,
        ctx: &egui::Context,
        task_id: crate::domain::ids::TaskId,
        result: Result<crate::data::ImportExecutionReport, crate::data::DbError>,
        table: String,
        elapsed_ms: u64,
    ) {
        let submitted = self
            .pending_grid_saves
            .remove(&task_id)
            .filter(|submitted| {
                self.does_runtime_connection_match(
                    &submitted.workspace_id.connection_name,
                    submitted.connection_id,
                )
            });
        self.session.grid_save_executing = !self.pending_grid_saves.is_empty();
        self.session.refresh_executing_flag();

        match (classify_grid_save_outcome(&result), result) {
            (GridSaveOutcome::CommittedClearEdits, Ok(report)) => {
                self.session.notifications.success(format!(
                    "已保存 {} 处修改到「{}」({}ms)",
                    report.succeeded, table, elapsed_ms
                ));
                if let Some(submitted) = submitted {
                    let should_refresh = self.retire_submitted_grid_edits(&submitted);
                    self.finish_grid_save_guard(&submitted.workspace_id, true, false);
                    if should_refresh {
                        self.dispatch_app_action(
                            ctx,
                            crate::app::action::action_system::AppAction::RefreshSelectedTable,
                        );
                    }
                }
            }
            (GridSaveOutcome::UnknownKeepEditsLocked, Err(error)) => {
                self.session.notifications.error(format!(
                    "保存结果未知，不能直接重试「{table}」；请先核对数据库状态，再放弃草稿并刷新表格: {error}"
                ));
                if let Some(submitted) = submitted {
                    self.finish_grid_save_guard(&submitted.workspace_id, false, true);
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
                if let Some(submitted) = submitted {
                    self.finish_grid_save_guard(&submitted.workspace_id, false, false);
                }
            }
            (_, Err(e)) => {
                self.session
                    .notifications
                    .error(format!("保存失败，已回滚: {e}"));
                if let Some(submitted) = submitted {
                    self.finish_grid_save_guard(&submitted.workspace_id, false, false);
                }
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
        result: Result<Vec<crate::data::TriggerInfo>, String>,
    ) {
        // 回包有效性由事件分支判定（连接 id + database 匹配）与
        // `metadata_context_matches_current` 判定；这里只把 pending 视为"在途"并消费它。
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
        result: Result<Vec<crate::data::RoutineInfo>, String>,
    ) {
        // 回包有效性由事件分支判定（连接 id + database 匹配）与
        // `metadata_context_matches_current` 判定；这里只把 pending 视为"在途"并消费它。
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

    fn notify_stale_transaction_completion(
        &mut self,
        outcome: &crate::session::runtime_event::RuntimeOutcome,
    ) {
        use crate::session::runtime_event::{RuntimeOutcome, TransactionCompletion};
        let RuntimeOutcome::ExecutionFinished {
            transaction_completion: Some(completion),
            result,
            ..
        } = outcome
        else {
            return;
        };
        match completion {
            TransactionCompletion::Committed => {
                self.session
                    .notifications
                    .warning("先前 PostgreSQL COMMIT 已完成；取消或后续查询未撤销提交");
            }
            TransactionCompletion::RolledBack => {
                self.session
                    .notifications
                    .warning("先前 PostgreSQL ROLLBACK 已完成");
            }
            TransactionCompletion::Unknown => {
                let detail = result
                    .as_ref()
                    .err()
                    .map_or("无更多错误信息", String::as_str);
                self.session.notifications.error(format!(
                    "先前 PostgreSQL 事务结果不确定；重试前请核对数据库: {detail}"
                ));
            }
        }
    }

    /// A task may still be current for connection A's key after the Tab is reused by B.
    /// Both the submission ID and live connection must match before touching UI state.
    fn is_current_query_event(&self, event: &crate::session::runtime_event::RuntimeEvent) -> bool {
        use crate::session::runtime_event::RuntimeOutcome;
        use crate::session::task_registry::OperationKey;
        let RuntimeOutcome::ExecutionFinished {
            document,
            request_id,
            connection_name,
            tab_id,
            ..
        } = &event.outcome
        else {
            return true;
        };
        let OperationKey::Query {
            connection,
            document: key_document,
        } = &event.key
        else {
            return false;
        };
        *key_document == *document
            && self.session.manager.active.as_deref() == Some(connection_name.as_str())
            && self.does_runtime_connection_match(connection_name, *connection)
            && self.session.tab_manager.tabs.iter().any(|tab| {
                tab.id == *tab_id
                    && super::request_lifecycle::query_document_id(&tab.id) == *document
                    && tab.pending_request_id == Some(*request_id)
            })
    }

    /// A rejected completion still ends its own pending request, never a replacement request.
    fn finish_rejected_query_request(
        &mut self,
        outcome: &crate::session::runtime_event::RuntimeOutcome,
    ) {
        let crate::session::runtime_event::RuntimeOutcome::ExecutionFinished {
            tab_id,
            request_id,
            ..
        } = outcome
        else {
            return;
        };
        if let Some(tab) = self
            .session
            .tab_manager
            .tabs
            .iter_mut()
            .find(|tab| tab.id == *tab_id && tab.pending_request_id == Some(*request_id))
        {
            tab.pending_request_id = None;
            tab.executing = false;
            self.session.refresh_executing_flag();
        }
    }

    /// 处理统一运行时事件（T1 cutover）。
    fn handle_runtime_event(
        &mut self,
        ctx: &egui::Context,
        event: crate::session::runtime_event::RuntimeEvent,
    ) {
        if !self
            .session
            .task_registry
            .is_current(&event.key, event.task_id)
            || !self.is_current_query_event(&event)
        {
            if let crate::session::runtime_event::RuntimeOutcome::GridSaved {
                result,
                table,
                elapsed_ms,
                ..
            } = event.outcome
            {
                // An older save can still have committed even after a newer save was submitted.
                self.handle_grid_save_done(ctx, event.task_id, result, table, elapsed_ms);
            } else {
                self.notify_stale_transaction_completion(&event.outcome);
                self.finish_rejected_query_request(&event.outcome);
            }
            self.session.task_registry.complete(event.task_id);
            self.session.task_registry.cleanup();
            self.session.needs_repaint = true;
            return;
        }

        use crate::session::runtime_event::RuntimeOutcome;
        match event.outcome {
            RuntimeOutcome::Connected {
                connection,
                conn_name,
                result,
            } => {
                self.handle_connected_with_tables(ctx, connection, conn_name, result);
            }
            RuntimeOutcome::DatabaseSelected {
                connection,
                conn_name,
                database,
                result,
            } => {
                self.handle_database_selected(ctx, connection, conn_name, database, result);
            }
            RuntimeOutcome::ActiveTablesReloaded {
                connection,
                conn_name,
                database,
                result,
            } => {
                self.handle_active_tables_reloaded(ctx, connection, conn_name, database, result);
            }
            RuntimeOutcome::DatabaseDropped {
                connection,
                conn_name,
                database,
                result,
            } => {
                self.handle_database_dropped(ctx, connection, conn_name, database, result);
            }
            RuntimeOutcome::TableDropped {
                connection,
                conn_name,
                database,
                table,
                result,
            } => {
                self.handle_table_dropped(ctx, connection, conn_name, database, table, result);
            }
            RuntimeOutcome::GridSaved {
                result,
                table,
                elapsed_ms,
                ..
            } => {
                self.handle_grid_save_done(ctx, event.task_id, result, table, elapsed_ms);
            }
            RuntimeOutcome::ExecutionFinished {
                request_id,
                sql,
                connection_name: conn_name,
                tab_id,
                result,
                elapsed_ms,
                ..
            } => {
                self.handle_query_execution_finished(
                    sql, conn_name, tab_id, request_id, result, elapsed_ms,
                );
            }
            RuntimeOutcome::ImportDone { result, elapsed_ms } => {
                self.handle_import_done(ctx, result, elapsed_ms);
            }
            RuntimeOutcome::CatalogLoaded {
                connection_id,
                database,
                catalog,
                ..
            } => match catalog {
                Ok(schema) => {
                    let is_active_catalog = self.session.manager.get_active().is_some_and(|conn| {
                        conn.id == connection_id
                            && conn
                                .selected_database
                                .as_deref()
                                .unwrap_or(&conn.config.database)
                                == database
                    });
                    if is_active_catalog {
                        self.session.autocomplete.set_from_catalog(&schema);
                    }
                    self.session
                        .schema_catalogs
                        .insert((connection_id, database), schema);
                    if is_active_catalog {
                        self.sync_table_metadata();
                        if self.state.show_er_diagram {
                            self.load_er_diagram_data();
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(connection_id = ?connection_id, database = %database, error = %e, "Schema 目录加载失败");
                }
            },
            RuntimeOutcome::TriggersFetched {
                connection,
                database,
                result,
            } => {
                if let Some((conn_name, pending_db)) = self.session.pending_triggers_request.clone()
                {
                    if self
                        .session
                        .manager
                        .connections
                        .get(&conn_name)
                        .is_some_and(|conn| conn.id == connection)
                        && pending_db == database
                    {
                        self.handle_triggers_fetched(ctx, conn_name, database, result);
                    } else {
                        tracing::debug!(?connection, "忽略不匹配连接上下文的触发器回包");
                    }
                } else {
                    tracing::debug!(?connection, "忽略没有待处理请求的触发器回包");
                }
            }
            RuntimeOutcome::RoutinesFetched {
                connection,
                database,
                result,
            } => {
                if let Some((conn_name, pending_db)) = self.session.pending_routines_request.clone()
                {
                    if self
                        .session
                        .manager
                        .connections
                        .get(&conn_name)
                        .is_some_and(|conn| conn.id == connection)
                        && pending_db == database
                    {
                        self.handle_routines_fetched(ctx, conn_name, database, result);
                    } else {
                        tracing::debug!(?connection, "忽略不匹配连接上下文的存储过程回包");
                    }
                } else {
                    tracing::debug!(?connection, "忽略没有待处理请求的存储过程回包");
                }
            }
            _ => {
                tracing::debug!(task_id = ?event.task_id, "收到未迁移的 RuntimeEvent");
            }
        }
        self.session.task_registry.complete(event.task_id);
        self.session.task_registry.cleanup();
        self.session.needs_repaint = true;
    }

    fn is_query_result_blocked_by_grid_draft(&self, tab_index: usize, conn_name: &str) -> bool {
        let Some(tab) = self.session.tab_manager.tabs.get(tab_index) else {
            return false;
        };
        if !tab.uses_grid_workspace {
            return false;
        }
        let (Some(table_name), Some(conn)) = (
            tab.selected_table.as_ref(),
            self.session.manager.connections.get(conn_name),
        ) else {
            return false;
        };
        let workspace_id = crate::app::GridWorkspaceId {
            tab_id: tab.id.clone(),
            connection_name: conn_name.to_owned(),
            connection_id: conn.id,
            database_name: conn.selected_database.clone(),
            table_name: table_name.clone(),
        };
        let state = if self.active_grid_workspace_id().as_ref() == Some(&workspace_id) {
            Some(&self.state.grid_state)
        } else {
            self.grid_workspaces.states.get(&workspace_id)
        };
        state.is_some_and(|state| {
            state.has_changes() || state.save_in_flight || state.has_unknown_save_outcome
        })
    }

    /// 处理查询执行完成（T1 cutover — 替代 handle_query_done）。
    fn handle_query_execution_finished(
        &mut self,
        sql: String,
        conn_name: String,
        tab_id: String,
        request_id: u64,
        result: Result<crate::domain::execution::ExecutionOutcome, String>,
        elapsed_ms: u64,
    ) {
        let target_tab_index = self
            .session
            .tab_manager
            .tabs
            .iter()
            .position(|t| t.id == tab_id);
        let Some(tab_index) = target_tab_index else {
            self.session.refresh_executing_flag();
            self.session.needs_repaint = true;
            return;
        };
        let is_active_tab = tab_index == self.session.tab_manager.active_index;
        let is_explain = is_explain_sql(&sql);
        let completed_request_id = Some(request_id);
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
        if result.is_ok()
            && !is_explain
            && self.is_query_result_blocked_by_grid_draft(tab_index, &conn_name)
        {
            if let Some(tab) = self.session.tab_manager.tabs.get_mut(tab_index) {
                tab.pending_request_id = None;
                tab.executing = false;
            }
            self.session
                .query_history
                .add(sql.clone(), db_type, true, None);
            self.session
                .notifications
                .warning("查询已完成，但表格有未保存或待核对的修改；保留原结果与草稿");
            self.invalidate_after_schema_change(&sql_hints, &conn_name);
            return;
        }

        match result {
            Ok(outcome) => {
                if outcome.statements.len() > 1 {
                    tracing::warn!(
                        stmt_count = outcome.statements.len(),
                        "多条语句仅显示第一条"
                    );
                    self.session
                        .notifications
                        .warning("检测到多条语句，仅显示第一条结果");
                }
                let (result_set, affected_rows) = match outcome.statements.first() {
                    Some(StatementOutcome::ResultSet(rs)) => (Some(rs.clone()), None),
                    Some(StatementOutcome::AffectedRows { rows }) => (None, Some(*rows)),
                    _ => (None, None),
                };

                if sql_hints.is_create_database {
                    self.mark_onboarding_database_initialized();
                }
                if sql_hints.is_create_user_or_role {
                    self.mark_onboarding_user_created();
                }
                self.mark_onboarding_first_query_executed();

                let typed_arc = result_set.map(std::sync::Arc::new);
                let row_count = typed_arc.as_ref().map(|a| a.row_count).unwrap_or(0);
                let columns_empty = typed_arc.as_ref().is_none_or(|a| a.columns.is_empty());

                self.session
                    .query_history
                    .add(sql.clone(), db_type, true, affected_rows);

                if let Some(tab) = self.session.tab_manager.tabs.get_mut(tab_index) {
                    if !is_explain {
                        tab.result_set = typed_arc.clone();
                    }
                    tab.executing = false;
                    tab.last_error = None;
                    tab.pending_request_id = None;
                    tab.query_time_ms = Some(elapsed_ms);
                }
                if is_explain {
                    self.state.explain_state.record_success(
                        tab_id.clone(),
                        sql.clone(),
                        typed_arc.clone(),
                        elapsed_ms,
                    );
                } else {
                    self.state.explain_state.hide_for_tab(&tab_id);
                }
                if is_active_tab {
                    self.session.last_query_time_ms = Some(elapsed_ms);
                    let msg = if is_explain {
                        format!("Explain 完成，返回 {} 行 ({}ms)", row_count, elapsed_ms)
                    } else if columns_empty {
                        format!(
                            "执行成功，影响 {} 行 ({}ms)",
                            affected_rows.unwrap_or(0),
                            elapsed_ms
                        )
                    } else {
                        format!("查询完成，返回 {} 行 ({}ms)", row_count, elapsed_ms)
                    };
                    self.session.notifications.success(&msg);
                    if is_explain {
                        self.reveal_bottom_panel_for_query(crate::core::BottomPanelTab::Explain);
                    } else {
                        self.state.selected_row = None;
                        self.state.selected_cell = None;
                        self.clear_search();
                        if is_update_or_delete {
                            self.state.grid_state.scroll_to_row =
                                Some(self.state.grid_state.cursor.0);
                        } else if is_insert {
                            let last_row = row_count.saturating_sub(1);
                            self.state.grid_state.cursor = (last_row, 0);
                            self.state.grid_state.scroll_to_row = Some(last_row);
                        }
                        if self.state.focus_area == crate::ui::FocusArea::DataGrid {
                            self.state.grid_state.focused = true;
                        }
                        if let Some(table) = &self.state.selected_table.clone()
                            && !columns_empty
                            && let Some(arc) = &typed_arc
                        {
                            self.session
                                .autocomplete
                                .set_columns(table.clone(), arc.column_names());
                        }
                        self.state.grid_state.result_set = typed_arc;
                        if !columns_empty
                            && !self.state.grid_state.save_in_flight
                            && !self.state.grid_state.has_unknown_save_outcome
                            && self.state.grid_state.refresh_after_save_request_id
                                == completed_request_id
                            && completed_request_id.is_some()
                        {
                            self.state.grid_state.needs_refresh_after_save = false;
                            self.state.grid_state.refresh_after_save_request_id = None;
                        }
                        self.reveal_bottom_panel_for_query(crate::core::BottomPanelTab::Results);
                        if !self.state.grid_state.save_in_flight
                            && !self.state.grid_state.has_unknown_save_outcome
                        {
                            self.state.grid_state.rows_to_delete.clear();
                        }
                        self.persist_active_grid_workspace();
                    }
                } else if !is_explain && !columns_empty {
                    self.grid_workspaces
                        .unlock_refreshed_tab(&tab_id, &conn_name, request_id);
                }
                if is_drop_table
                    && let Some(conn) = self.session.manager.connections.get_mut(&conn_name)
                    && self.session.manager.active.as_deref() == Some(&conn_name)
                {
                    self.session.autocomplete.set_tables(conn.tables.clone());
                }
                self.invalidate_after_schema_change(&sql_hints, &conn_name);
            }
            Err(e) => {
                tracing::error!(error = %e, elapsed_ms, "查询执行失败");
                self.session.notifications.error(format!("查询失败: {}", e));
                if let Some(tab) = self.session.tab_manager.tabs.get_mut(tab_index) {
                    tab.pending_request_id = None;
                    tab.executing = false;
                    if !is_explain {
                        tab.last_error = Some(e.clone());
                    }
                }
                if is_explain {
                    self.state.explain_state.record_error(
                        tab_id.clone(),
                        sql.clone(),
                        e.clone(),
                        elapsed_ms,
                    );
                    if is_active_tab {
                        self.reveal_bottom_panel_for_query(crate::core::BottomPanelTab::Explain);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ErDiagramReadyKind, GridSaveOutcome, apply_default_er_diagram_layout,
        apply_ready_state_er_diagram_layout, classify_grid_save_outcome, er_diagram_ready_message,
        resolve_er_diagram_ready_state, schema_invalidation_for,
        select_ready_state_er_layout_strategy,
    };
    use crate::app::DbManagerApp;
    use crate::app::GridSubmittedEdits;
    use crate::data::{
        Connection, ConnectionConfig, DatabaseType, ImportExecutionReport, analyze_sql_for_ui,
    };
    use crate::domain::execution::ExecutionOutcome;
    use crate::domain::ids::TaskId;
    use crate::domain::result::ResultSet;
    use crate::ui::{ERLayoutStrategy, ERTable, RelationType, Relationship, RelationshipOrigin};
    use eframe::egui;
    use std::num::NonZeroU64;

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
    fn inactive_catalog_completion_does_not_change_active_grid_metadata() {
        use crate::domain::ids::SchemaRevision;
        use crate::domain::metadata::{KeyMetadata, SchemaCatalog, TableMetadata};
        use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
        use crate::session::task_registry::{OperationKey, TaskKind};

        let mut app = DbManagerApp::new_for_test();
        for name in ["old", "current"] {
            let mut config = ConnectionConfig::new(name, DatabaseType::SQLite);
            config.database = format!("/tmp/{name}.sqlite");
            app.session.manager.add(config);
        }
        app.session.manager.active = Some("current".into());
        app.state.selected_table = Some("users".into());
        let current_id = app.session.manager.get_active().expect("current").id;
        let old_id = app.session.manager.connections.get("old").expect("old").id;
        let table = |name: &str| TableMetadata {
            name: name.into(),
            schema: None,
            columns: vec![],
            primary_key: None,
            unique_keys: vec![],
            foreign_keys: vec![],
        };
        let current_table = table("users");
        app.session.schema_catalogs.insert(
            (current_id, "/tmp/current.sqlite".into()),
            SchemaCatalog {
                revision: SchemaRevision(0),
                tables: vec![current_table.clone()],
            },
        );
        app.state.grid_state.table_metadata = Some(std::sync::Arc::new(current_table));
        let key = OperationKey::Catalog {
            connection: old_id,
            database: "/tmp/old.sqlite".into(),
        };
        let (task_id, _) = app
            .session
            .task_registry
            .register(key.clone(), TaskKind::Catalog);
        app.handle_runtime_event(
            &egui::Context::default(),
            RuntimeEvent {
                task_id,
                key,
                outcome: RuntimeOutcome::CatalogLoaded {
                    connection_id: old_id,
                    database: "/tmp/old.sqlite".into(),
                    catalog: Ok(SchemaCatalog {
                        revision: SchemaRevision(0),
                        tables: vec![TableMetadata {
                            primary_key: Some(KeyMetadata {
                                name: Some("old_primary_key".into()),
                                columns: vec!["old_id".into()],
                            }),
                            ..table("users")
                        }],
                    }),
                    revision: SchemaRevision(0),
                },
            },
        );

        assert_eq!(
            app.state
                .grid_state
                .table_metadata
                .as_ref()
                .and_then(|metadata| metadata.primary_key.as_ref()),
            None,
        );
        assert_eq!(
            app.session
                .schema_catalogs
                .get(&(old_id, "/tmp/old.sqlite".into()))
                .expect("old cache")
                .tables[0]
                .primary_key
                .as_ref()
                .expect("cached old primary key")
                .columns[0],
            "old_id",
        );
    }

    #[test]
    fn active_catalog_completion_refreshes_open_sqlite_er_shell() {
        use crate::domain::ids::SchemaRevision;
        use crate::domain::metadata::{ColumnMetadata, SchemaCatalog, TableMetadata};
        use crate::domain::value::{DbTypeFamily, DbTypeInfo};
        use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
        use crate::session::task_registry::{OperationKey, TaskKind};

        let mut app = DbManagerApp::new_for_test();
        let mut config = ConnectionConfig::new("demo", DatabaseType::SQLite);
        config.database = "/tmp/er-catalog.sqlite".into();
        app.session.manager.add(config);
        app.session.manager.active = Some("demo".into());
        let conn = app
            .session
            .manager
            .connections
            .get_mut("demo")
            .expect("connection");
        conn.set_connected(vec!["users".into()]);
        let id = conn.id;
        app.load_er_diagram_data();
        assert!(app.state.er_diagram_state.tables[0].columns.is_empty());

        let key = OperationKey::Catalog {
            connection: id,
            database: "/tmp/er-catalog.sqlite".into(),
        };
        let (task_id, _) = app
            .session
            .task_registry
            .register(key.clone(), TaskKind::Catalog);
        app.handle_runtime_event(
            &egui::Context::default(),
            RuntimeEvent {
                task_id,
                key,
                outcome: RuntimeOutcome::CatalogLoaded {
                    connection_id: id,
                    database: "/tmp/er-catalog.sqlite".into(),
                    catalog: Ok(SchemaCatalog {
                        revision: SchemaRevision(0),
                        tables: vec![TableMetadata {
                            name: "users".into(),
                            schema: None,
                            columns: vec![ColumnMetadata {
                                name: "id".into(),
                                position: 1,
                                type_info: DbTypeInfo {
                                    family: DbTypeFamily::Integer,
                                    native_name: "INTEGER".into(),
                                    nullable: Some(false),
                                },
                                is_nullable: false,
                                is_primary_key: true,
                                default_value: None,
                            }],
                            primary_key: None,
                            unique_keys: vec![],
                            foreign_keys: vec![],
                        }],
                    }),
                    revision: SchemaRevision(0),
                },
            },
        );
        assert_eq!(app.state.er_diagram_state.tables[0].columns[0].name, "id");
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
        let err: Result<ImportExecutionReport, crate::data::DbError> =
            Err(crate::data::DbError::Query("constraint violation".into()));
        assert_eq!(
            classify_grid_save_outcome(&err),
            GridSaveOutcome::RolledBackKeepEdits
        );
    }

    #[test]
    fn database_switch_keeps_unsaved_grid_rows_in_original_database() {
        let first = tempfile::NamedTempFile::new().expect("first temporary database");
        let second = tempfile::NamedTempFile::new().expect("second temporary database");
        let first_name = first.path().to_string_lossy().into_owned();
        let second_name = second.path().to_string_lossy().into_owned();
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("local", DatabaseType::SQLite));
        app.session.manager.active = Some("local".into());
        let connection_id = app.session.manager.get_active().expect("connection").id;
        app.session
            .manager
            .connections
            .get_mut("local")
            .expect("connection")
            .set_database(first_name.clone(), vec![]);
        app.switch_grid_workspace(Some("users".into()));
        app.state
            .grid_state
            .new_rows
            .push(vec!["first draft".into()]);
        app.state.grid_state.save_in_flight = true;

        app.session
            .pending_database_requests
            .insert("local".into(), second_name.clone());
        app.handle_database_selected(
            &egui::Context::default(),
            connection_id,
            "local".into(),
            second_name.clone(),
            Ok(vec![]),
        );
        app.switch_grid_workspace(Some("users".into()));
        assert!(app.state.grid_state.new_rows.is_empty());
        assert!(!app.state.grid_state.save_in_flight);
        app.state
            .grid_state
            .new_rows
            .push(vec!["second draft".into()]);

        app.session
            .pending_database_requests
            .insert("local".into(), first_name.clone());
        app.handle_database_selected(
            &egui::Context::default(),
            connection_id,
            "local".into(),
            first_name,
            Ok(vec![]),
        );
        app.switch_grid_workspace(Some("users".into()));
        assert_eq!(app.state.grid_state.new_rows, vec![vec!["first draft"]]);
        assert!(app.state.grid_state.save_in_flight);
    }

    #[test]
    fn grid_save_inactive_owner_does_not_restore_committed_drafts() {
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("local", DatabaseType::SQLite));
        app.session.manager.active = Some("local".into());
        app.session.tab_manager.tabs[0].selected_table = Some("users".into());
        app.session.tab_manager.tabs[0].uses_grid_workspace = true;
        app.switch_grid_workspace(Some("users".into()));
        app.state.grid_state.new_rows.push(vec!["inserted".into()]);
        app.state
            .grid_state
            .modified_cells
            .insert((0, 0), "changed-pk".into());
        let owner = app.active_grid_workspace_id().expect("table workspace");
        let connection_id = app
            .session
            .manager
            .get_active()
            .expect("local connection")
            .id;
        let submitted =
            GridSubmittedEdits::capture(owner.clone(), connection_id, &app.state.grid_state);
        let task_id = TaskId(NonZeroU64::new(1).expect("nonzero task id"));
        app.pending_grid_saves.insert(task_id, submitted);
        app.state.grid_state.save_in_flight = true;
        app.open_new_query_tab();
        app.switch_grid_workspace(Some("orders".into()));
        app.state
            .grid_state
            .new_rows
            .push(vec!["other draft".into()]);

        app.handle_grid_save_done(
            &egui::Context::default(),
            task_id,
            Ok(ImportExecutionReport {
                total: 2,
                succeeded: 2,
                failed: 0,
                first_error: None,
            }),
            "users".into(),
            1,
        );

        assert_eq!(
            app.state.grid_state.new_rows,
            vec![vec!["other draft".to_string()]]
        );
        app.activate_query_tab(0);
        assert!(
            !app.state.grid_state.has_changes(),
            "the committed insert and PK edit must not replay"
        );
        assert!(
            app.state.grid_state.needs_refresh_after_save,
            "the owner's old result cannot be edited"
        );
        assert!(!app.state.grid_state.save_in_flight);
    }

    fn deliver_committed_grid_save(app: &mut DbManagerApp, task_id: TaskId) {
        let mut report = ImportExecutionReport::new(1);
        report.succeeded = 1;
        app.handle_grid_save_done(
            &egui::Context::default(),
            task_id,
            Ok(report),
            "users".into(),
            1,
        );
    }

    #[test]
    fn grid_save_replaced_same_name_connection_cannot_retire_new_draft() {
        let mut app = DbManagerApp::new_for_test();
        let config = ConnectionConfig::new("local", DatabaseType::SQLite);
        app.session.manager.add(config.clone());
        app.session.manager.active = Some("local".into());
        app.switch_grid_workspace(Some("users".into()));
        app.state
            .grid_state
            .new_rows
            .push(vec!["same draft".into()]);
        let workspace_id = app.active_grid_workspace_id().expect("users workspace");
        let old_id = app.session.manager.get_active().expect("old connection").id;
        let old_task = TaskId(NonZeroU64::new(10).expect("nonzero task id"));
        app.pending_grid_saves.insert(
            old_task,
            GridSubmittedEdits::capture(workspace_id.clone(), old_id, &app.state.grid_state),
        );
        app.state.grid_state.save_in_flight = true;

        app.disconnect("local".into());
        app.session.manager.connections.remove("local");
        app.session.manager.add(config);
        app.session.manager.active = Some("local".into());
        app.switch_grid_workspace(Some("users".into()));
        let new_id = app.session.manager.get_active().expect("new connection").id;
        assert_ne!(old_id, new_id);
        let new_workspace_id = app.active_grid_workspace_id().expect("new users workspace");
        assert_ne!(new_workspace_id, workspace_id);
        assert_eq!(new_workspace_id.connection_id, new_id);
        assert!(app.state.grid_state.new_rows.is_empty());
        app.state
            .grid_state
            .new_rows
            .push(vec!["same draft".into()]);
        let new_task = TaskId(NonZeroU64::new(11).expect("nonzero task id"));
        app.pending_grid_saves.insert(
            new_task,
            GridSubmittedEdits::capture(new_workspace_id, new_id, &app.state.grid_state),
        );
        app.state.grid_state.save_in_flight = true;
        deliver_committed_grid_save(&mut app, old_task);
        assert_eq!(app.state.grid_state.new_rows, vec![vec!["same draft"]]);
        assert!(app.state.grid_state.save_in_flight);
        assert!(!app.state.grid_state.needs_refresh_after_save);
        assert!(app.pending_grid_saves.contains_key(&new_task));

        app.switch_grid_workspace(Some("orders".into()));
        deliver_committed_grid_save(&mut app, new_task);
        app.switch_grid_workspace(Some("users".into()));
        assert!(!app.state.grid_state.has_changes());
        assert!(!app.state.grid_state.save_in_flight);
        assert!(app.state.grid_state.needs_refresh_after_save);
    }

    #[test]
    fn grid_save_rollback_unlocks_original_draft_for_retry() {
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("local", DatabaseType::SQLite));
        app.session.manager.active = Some("local".into());
        app.switch_grid_workspace(Some("users".into()));
        app.state.grid_state.new_rows.push(vec!["unsaved".into()]);
        let submitted = GridSubmittedEdits::capture(
            app.active_grid_workspace_id().expect("users workspace"),
            app.session
                .manager
                .get_active()
                .expect("local connection")
                .id,
            &app.state.grid_state,
        );
        let task_id = TaskId(NonZeroU64::new(3).expect("nonzero task id"));
        app.pending_grid_saves.insert(task_id, submitted);
        app.state.grid_state.save_in_flight = true;

        app.handle_grid_save_done(
            &egui::Context::default(),
            task_id,
            Err(crate::data::DbError::Query("constraint violation".into())),
            "users".into(),
            1,
        );

        assert_eq!(
            app.state.grid_state.new_rows,
            vec![vec!["unsaved".to_string()]]
        );
        assert!(!app.state.grid_state.is_save_locked());
    }

    #[test]
    fn grid_save_unknown_commit_keeps_draft_locked_until_discard_and_refresh() {
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("local", DatabaseType::SQLite));
        app.session.manager.active = Some("local".into());
        app.switch_grid_workspace(Some("users".into()));
        app.state.grid_state.new_rows.push(vec!["uncertain".into()]);
        let workspace_id = app.active_grid_workspace_id().expect("users workspace");
        let connection_id = app
            .session
            .manager
            .get_active()
            .expect("local connection")
            .id;
        let submitted =
            GridSubmittedEdits::capture(workspace_id, connection_id, &app.state.grid_state);
        let task_id = TaskId(NonZeroU64::new(4).expect("nonzero task id"));
        app.pending_grid_saves.insert(task_id, submitted);
        app.state.grid_state.save_in_flight = true;

        app.handle_grid_save_done(
            &egui::Context::default(),
            task_id,
            Err(crate::data::DbError::MutationOutcomeUnknown {
                backend: "PostgreSQL",
                operation: "COMMIT",
                context: "test batch".into(),
                source: Box::new(std::io::Error::other("acknowledgement lost")),
            }),
            "users".into(),
            1,
        );
        assert_eq!(app.state.grid_state.new_rows, vec![vec!["uncertain"]]);
        assert!(app.state.grid_state.has_unknown_save_outcome);
        assert!(app.state.grid_state.is_save_locked());
        app.state.grid_state.clear_edits();
        assert!(!app.state.grid_state.has_unknown_save_outcome);
        assert!(app.state.grid_state.is_save_locked());
    }

    fn deliver_query_completion(
        app: &mut DbManagerApp,
        connection_name: &str,
        connection: crate::domain::ids::ConnectionId,
        task_id: TaskId,
        request_id: u64,
        sql: &str,
        result: Result<ExecutionOutcome, String>,
    ) {
        use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
        use crate::session::task_registry::OperationKey;
        let tab_id = app
            .session
            .tab_manager
            .get_active()
            .expect("active tab")
            .id
            .clone();
        let document = super::super::request_lifecycle::query_document_id(&tab_id);
        app.handle_runtime_event(
            &egui::Context::default(),
            RuntimeEvent {
                task_id,
                key: OperationKey::Query {
                    connection,
                    document,
                },
                outcome: RuntimeOutcome::ExecutionFinished {
                    document,
                    request_id,
                    sql: sql.into(),
                    connection_name: connection_name.into(),
                    tab_id,
                    result,
                    transaction_completion: None,
                    elapsed_ms: 1,
                },
            },
        );
    }

    #[test]
    fn grid_save_pk_change_waits_for_post_commit_table_refresh() {
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("local", DatabaseType::SQLite));
        app.session.manager.active = Some("local".into());
        app.switch_grid_workspace(Some("users".into()));
        let owner = app.active_grid_workspace_id().expect("users workspace");
        app.state
            .grid_state
            .modified_cells
            .insert((0, 0), "new-pk".into());
        let connection_id = app
            .session
            .manager
            .get_active()
            .expect("local connection")
            .id;
        let submitted =
            GridSubmittedEdits::capture(owner.clone(), connection_id, &app.state.grid_state);
        let task_id = TaskId(NonZeroU64::new(2).expect("nonzero task id"));
        app.pending_grid_saves.insert(task_id, submitted);
        app.state.grid_state.save_in_flight = true;
        // Avoid launching a database request: the owning workspace is inactive at completion.
        app.switch_grid_workspace(Some("orders".into()));
        app.handle_grid_save_done(
            &egui::Context::default(),
            task_id,
            Ok(ImportExecutionReport {
                total: 1,
                succeeded: 1,
                failed: 0,
                first_error: None,
            }),
            "users".into(),
            1,
        );
        app.switch_grid_workspace(Some("users".into()));
        assert!(app.state.grid_state.is_save_locked());

        let connection = app
            .session
            .manager
            .get_active()
            .expect("local connection")
            .id;
        let document = super::super::request_lifecycle::query_document_id(
            &app.session.tab_manager.get_active().expect("active tab").id,
        );
        let key = crate::session::task_registry::OperationKey::Query {
            connection,
            document,
        };
        let (old_task, _) = app
            .session
            .task_registry
            .register(key.clone(), crate::session::task_registry::TaskKind::Query);
        let refresh_sql = format!(
            "SELECT * FROM \"users\" LIMIT {};",
            crate::core::constants::database::DEFAULT_QUERY_LIMIT
        );
        app.state.grid_state.refresh_after_save_request_id = Some(42);
        let tab = app
            .session
            .tab_manager
            .get_active_mut()
            .expect("active tab");
        tab.pending_request_id = Some(42);
        tab.executing = true;
        deliver_query_completion(
            &mut app,
            "local",
            connection,
            old_task,
            41,
            &refresh_sql,
            Ok(ExecutionOutcome::single_result(ResultSet::empty())),
        );
        assert!(
            app.state.grid_state.is_save_locked(),
            "an earlier response cannot unlock the stale PK"
        );
        let tab = app.session.tab_manager.get_active().expect("active tab");
        assert_eq!(tab.pending_request_id, Some(42));
        assert!(tab.executing);
        let (current_task, _) = app
            .session
            .task_registry
            .register(key, crate::session::task_registry::TaskKind::Query);
        deliver_query_completion(
            &mut app,
            "local",
            connection,
            current_task,
            42,
            &refresh_sql,
            Ok(ExecutionOutcome::single_result(ResultSet {
                columns: std::sync::Arc::new([crate::domain::result::ResultColumn {
                    name: "id".into(),
                    type_info: crate::domain::value::DbTypeInfo {
                        family: crate::domain::value::DbTypeFamily::Text,
                        native_name: "TEXT".into(),
                        nullable: None,
                    },
                }]),
                cells: vec![crate::domain::value::DbValue::Text("new-pk".into())],
                row_count: 1,
                completeness: crate::domain::result::ResultCompleteness::Complete,
            })),
        );
        assert!(!app.state.grid_state.is_save_locked());
        assert_eq!(
            app.state
                .grid_state
                .result_set
                .as_ref()
                .expect("fresh result")
                .cell(0, 0)
                .display(),
            "new-pk"
        );
    }

    #[test]
    fn grid_refresh_inactive_tab_unlocks_after_matching_success() {
        use crate::session::task_registry::{OperationKey, TaskKind};
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("local", DatabaseType::SQLite));
        app.session.manager.active = Some("local".into());
        app.switch_grid_workspace(Some("users".into()));
        app.state.grid_state.needs_refresh_after_save = true;
        app.state.grid_state.refresh_after_save_request_id = Some(19);
        let tab_id = app
            .session
            .tab_manager
            .get_active()
            .expect("owner tab")
            .id
            .clone();
        let connection = app.session.manager.connections["local"].id;
        let document = super::super::request_lifecycle::query_document_id(&tab_id);
        let (task_id, _) = app.session.task_registry.register(
            OperationKey::Query {
                connection,
                document,
            },
            TaskKind::Query,
        );
        app.session
            .tab_manager
            .get_active_mut()
            .expect("owner tab")
            .pending_request_id = Some(19);
        app.open_new_query_tab();
        let result = ResultSet {
            columns: std::sync::Arc::new([crate::domain::result::ResultColumn {
                name: "id".into(),
                type_info: crate::domain::value::DbTypeInfo {
                    family: crate::domain::value::DbTypeFamily::Integer,
                    native_name: "INTEGER".into(),
                    nullable: None,
                },
            }]),
            cells: vec![crate::domain::value::DbValue::Int(1)],
            row_count: 1,
            completeness: crate::domain::result::ResultCompleteness::Complete,
        };
        deliver_query_completion(
            &mut app,
            "local",
            connection,
            task_id,
            19,
            "SELECT * FROM \"users\" LIMIT 1000;",
            Ok(ExecutionOutcome::single_result(result)),
        );
        app.activate_query_tab(0);
        assert!(!app.state.grid_state.is_save_locked());
    }

    #[test]
    fn explain_success_uses_explain_surface_without_replacing_results() {
        let mut app = DbManagerApp::new_for_test();
        let previous_result = std::sync::Arc::new(ResultSet::empty());
        app.state.grid_state.result_set = Some(previous_result.clone());
        app.session
            .tab_manager
            .get_active_mut()
            .expect("test app should have an active query tab")
            .result_set = Some(previous_result);
        let tab_id = app
            .session
            .tab_manager
            .get_active()
            .expect("test app should have an active query tab")
            .id
            .clone();

        app.handle_query_execution_finished(
            "EXPLAIN SELECT 1".to_string(),
            "missing".to_string(),
            tab_id.clone(),
            1,
            Ok(ExecutionOutcome::single_result(ResultSet::empty())),
            12,
        );

        assert!(app.state.explain_state.should_show_for_tab(&tab_id));
        assert!(app.state.explain_state.result.is_some());
        assert!(app.state.grid_state.result_set.is_some());
        assert_eq!(
            app.state.workbench.bottom_panel.active_tab,
            crate::core::BottomPanelTab::Explain
        );
        assert!(
            app.session
                .tab_manager
                .get_active()
                .and_then(|tab| tab.result_set.as_ref())
                .is_some()
        );
    }

    #[test]
    fn explain_error_is_retained_and_reveals_explain_surface() {
        let mut app = DbManagerApp::new_for_test();
        let tab_id = app
            .session
            .tab_manager
            .get_active()
            .expect("test app should have an active query tab")
            .id
            .clone();
        if let Some(tab) = app.session.tab_manager.get_active_mut() {
            tab.executing = true;
            tab.pending_request_id = Some(7);
        }

        app.handle_query_execution_finished(
            "EXPLAIN SELECT missing_column".to_string(),
            "missing".to_string(),
            tab_id.clone(),
            7,
            Err("no such column: missing_column".to_string()),
            3,
        );

        assert_eq!(
            app.state.explain_state.error.as_deref(),
            Some("no such column: missing_column")
        );
        assert!(app.state.explain_state.should_show_for_tab(&tab_id));
        assert_eq!(
            app.state.workbench.bottom_panel.active_tab,
            crate::core::BottomPanelTab::Explain
        );
        let tab = app
            .session
            .tab_manager
            .get_active()
            .expect("test app should have an active query tab");
        assert!(!tab.executing);
        assert_eq!(tab.pending_request_id, None);
        assert_eq!(tab.last_error, None);
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
    fn active_tables_runtime_event_stale_guard_preserves_latest_tables() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        let connection_name = "demo".to_string();
        let mut connection = Connection::new(ConnectionConfig::new(
            &connection_name,
            DatabaseType::SQLite,
        ));
        connection.connected = true;
        connection.tables = vec!["initial".to_string()];
        let connection_id = connection.id;
        app.session
            .manager
            .connections
            .insert(connection_name.clone(), connection);
        app.session.manager.active = Some(connection_name.clone());

        let key = crate::session::task_registry::OperationKey::ActiveTables {
            connection: connection_id,
        };
        let (stale_task_id, _) = app.session.task_registry.register(
            key.clone(),
            crate::session::task_registry::TaskKind::ActiveTables,
        );
        let (current_task_id, _) = app.session.task_registry.register(
            key.clone(),
            crate::session::task_registry::TaskKind::ActiveTables,
        );

        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id: stale_task_id,
                key: key.clone(),
                outcome: crate::session::runtime_event::RuntimeOutcome::ActiveTablesReloaded {
                    connection: connection_id,
                    conn_name: connection_name.clone(),
                    database: String::new(),
                    result: Ok(vec!["stale".to_string()]),
                },
            },
        );
        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id: current_task_id,
                key,
                outcome: crate::session::runtime_event::RuntimeOutcome::ActiveTablesReloaded {
                    connection: connection_id,
                    conn_name: connection_name.clone(),
                    database: String::new(),
                    result: Ok(vec!["latest".to_string()]),
                },
            },
        );

        assert_eq!(
            app.session.manager.connections[&connection_name].tables,
            vec!["latest".to_string()]
        );
        let completions = app.session.autocomplete.get_completions("lat", 3);
        assert!(
            completions
                .iter()
                .any(|completion| completion.label == "latest")
        );
        assert!(
            !completions
                .iter()
                .any(|completion| completion.label == "stale")
        );
    }

    #[test]
    fn import_runtime_event_stale_guard_finishes_only_current_import() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        app.session.import_executing = true;
        app.session.refresh_executing_flag();

        let key = crate::session::task_registry::OperationKey::Import;
        let (stale_task_id, _) = app
            .session
            .task_registry
            .register(key.clone(), crate::session::task_registry::TaskKind::Import);
        let (current_task_id, _) = app
            .session
            .task_registry
            .register(key.clone(), crate::session::task_registry::TaskKind::Import);

        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id: stale_task_id,
                key: key.clone(),
                outcome: crate::session::runtime_event::RuntimeOutcome::ImportDone {
                    result: Err(crate::data::DbError::Query("stale import".into())),
                    elapsed_ms: 1,
                },
            },
        );
        assert!(app.session.import_executing);

        let mut report = ImportExecutionReport::new(1);
        report.succeeded = 1;
        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id: current_task_id,
                key,
                outcome: crate::session::runtime_event::RuntimeOutcome::ImportDone {
                    result: Ok(report),
                    elapsed_ms: 2,
                },
            },
        );

        assert!(!app.session.import_executing);
        assert!(
            app.session
                .notifications
                .latest_message()
                .is_some_and(|message| message.contains("导入完成"))
        );
    }

    #[test]
    fn stale_query_event_does_not_overwrite_current_tab_state() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        let tab = app
            .session
            .tab_manager
            .get_active_mut()
            .expect("test app has an active query tab");
        tab.executing = true;
        tab.last_message = Some("current query is running".to_string());
        let tab_id = tab.id.clone();
        let document = crate::domain::ids::DocumentId::from(
            uuid::Uuid::parse_str(&tab_id).expect("query tab IDs are UUIDs"),
        );
        let key = crate::session::task_registry::OperationKey::Query {
            connection: crate::domain::ids::ConnectionId::from(uuid::Uuid::new_v4()),
            document,
        };
        let (stale_task_id, stale_token) = app
            .session
            .task_registry
            .register(key.clone(), crate::session::task_registry::TaskKind::Query);
        let stale_handle = app.session.runtime.spawn({
            let stale_token = stale_token.clone();
            async move {
                stale_token.cancelled().await;
            }
        });
        app.session.task_registry.attach(
            stale_task_id,
            key.clone(),
            crate::session::task_registry::TaskKind::Query,
            stale_handle,
            stale_token,
        );
        app.session.task_registry.cancel_by_key(&key);
        let (current_task_id, _) = app
            .session
            .task_registry
            .register(key.clone(), crate::session::task_registry::TaskKind::Query);
        let current_handle = app
            .session
            .runtime
            .spawn(async { std::future::pending::<()>().await });
        app.session.task_registry.attach(
            current_task_id,
            key.clone(),
            crate::session::task_registry::TaskKind::Query,
            current_handle,
            tokio_util::sync::CancellationToken::new(),
        );

        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id: stale_task_id,
                key: key.clone(),
                outcome: crate::session::runtime_event::RuntimeOutcome::ExecutionFinished {
                    document,
                    request_id: 1,
                    sql: "SELECT stale".to_string(),
                    connection_name: "test".to_string(),
                    tab_id,
                    result: Err("stale query failed".to_string()),
                    transaction_completion: None,
                    elapsed_ms: 1,
                },
            },
        );

        let tab = app
            .session
            .tab_manager
            .get_active()
            .expect("active query tab remains");
        assert!(tab.executing);
        assert_eq!(
            tab.last_message.as_deref(),
            Some("current query is running")
        );
        assert!(tab.last_error.is_none());
        assert!(app.session.task_registry.is_current(&key, current_task_id));
    }

    #[test]
    fn query_completion_previous_connection_cannot_overwrite_new_submission() {
        use crate::session::task_registry::{OperationKey, TaskKind};
        for is_same_name_reconnect in [false, true] {
            let mut app = DbManagerApp::new_for_test();
            app.session
                .manager
                .add(ConnectionConfig::new("old", DatabaseType::SQLite));
            let old_connection = app.session.manager.connections["old"].id;
            let document = super::super::request_lifecycle::query_document_id(
                &app.session.tab_manager.get_active().expect("active tab").id,
            );
            let old_key = OperationKey::Query {
                connection: old_connection,
                document,
            };
            let (old_task, _) = app
                .session
                .task_registry
                .register(old_key.clone(), TaskKind::Query);
            let new_name = if is_same_name_reconnect { "old" } else { "new" };
            app.session
                .manager
                .add(ConnectionConfig::new(new_name, DatabaseType::SQLite));
            app.session.manager.active = Some(new_name.into());
            let new_connection = app.session.manager.connections[new_name].id;
            assert_ne!(old_connection, new_connection);
            let new_key = OperationKey::Query {
                connection: new_connection,
                document,
            };
            let (new_task, _) = app
                .session
                .task_registry
                .register(new_key.clone(), TaskKind::Query);
            let tab = app
                .session
                .tab_manager
                .get_active_mut()
                .expect("active tab");
            tab.pending_request_id = Some(2);
            tab.executing = true;
            deliver_query_completion(
                &mut app,
                "old",
                old_connection,
                old_task,
                1,
                "SELECT old",
                Err("old query error".into()),
            );
            let tab = app.session.tab_manager.get_active().expect("active tab");
            assert!(tab.executing);
            assert_eq!(tab.pending_request_id, Some(2));
            assert!(tab.last_error.is_none());
            assert!(app.session.task_registry.is_current(&new_key, new_task));
            deliver_query_completion(
                &mut app,
                new_name,
                new_connection,
                new_task,
                2,
                "SELECT new",
                Ok(ExecutionOutcome::affected_rows(1)),
            );
            let tab = app.session.tab_manager.get_active().expect("active tab");
            assert!(!tab.executing);
            assert_eq!(tab.pending_request_id, None);
            assert!(tab.last_error.is_none());
        }
    }
    #[test]
    fn query_completion_after_connection_switch_clears_only_its_pending_request() {
        use crate::session::task_registry::{OperationKey, TaskKind};
        let mut app = DbManagerApp::new_for_test();
        app.session
            .manager
            .add(ConnectionConfig::new("old", DatabaseType::SQLite));
        let old_connection = app.session.manager.connections["old"].id;
        let tab_id = app
            .session
            .tab_manager
            .get_active()
            .expect("tab")
            .id
            .clone();
        let document = super::super::request_lifecycle::query_document_id(&tab_id);
        let key = OperationKey::Query {
            connection: old_connection,
            document,
        };
        let (task_id, _) = app.session.task_registry.register(key, TaskKind::Query);
        let tab = app.session.tab_manager.get_active_mut().expect("tab");
        tab.pending_request_id = Some(17);
        tab.executing = true;
        app.session
            .manager
            .add(ConnectionConfig::new("new", DatabaseType::SQLite));
        app.session.manager.active = Some("new".into());

        deliver_query_completion(
            &mut app,
            "old",
            old_connection,
            task_id,
            17,
            "SELECT old",
            Ok(ExecutionOutcome::affected_rows(1)),
        );
        let tab = app.session.tab_manager.get_active().expect("tab");
        assert_eq!(tab.pending_request_id, None);
        assert!(!tab.executing);
        assert!(tab.last_error.is_none());
    }

    fn deliver_stale_commit(
        completion: crate::session::runtime_event::TransactionCompletion,
        result: Result<ExecutionOutcome, String>,
    ) -> DbManagerApp {
        use crate::session::runtime_event::{RuntimeEvent, RuntimeOutcome};
        use crate::session::task_registry::{OperationKey, TaskKind};
        let mut app = DbManagerApp::new_for_test();
        let tab_id = app.session.tab_manager.get_active().unwrap().id.clone();
        let document =
            crate::domain::ids::DocumentId::from(uuid::Uuid::parse_str(&tab_id).unwrap());
        let key = OperationKey::Query {
            connection: crate::domain::ids::ConnectionId::from(uuid::Uuid::new_v4()),
            document,
        };
        let (task_id, _) = app
            .session
            .task_registry
            .register(key.clone(), TaskKind::Query);
        app.session.task_registry.remove_key(&key);
        app.handle_runtime_event(
            &egui::Context::default(),
            RuntimeEvent {
                task_id,
                key,
                outcome: RuntimeOutcome::ExecutionFinished {
                    document,
                    request_id: 1,
                    sql: "COMMIT".into(),
                    connection_name: "test".into(),
                    tab_id,
                    result,
                    transaction_completion: Some(completion),
                    elapsed_ms: 1,
                },
            },
        );
        app
    }

    #[test]
    fn stale_commit_unknown_outcome_warns_without_overwriting_tab() {
        let app = deliver_stale_commit(
            crate::session::runtime_event::TransactionCompletion::Unknown,
            Err("结果不确定".into()),
        );
        assert!(
            app.session
                .notifications
                .latest_message()
                .is_some_and(
                    |message| message.contains("结果不确定") && message.contains("核对数据库")
                )
        );
        assert!(
            app.session
                .tab_manager
                .get_active()
                .unwrap()
                .last_error
                .is_none()
        );
    }

    #[test]
    fn stale_commit_success_reports_actual_commit() {
        let app = deliver_stale_commit(
            crate::session::runtime_event::TransactionCompletion::Committed,
            Ok(ExecutionOutcome::affected_rows(0)),
        );
        assert!(
            app.session
                .notifications
                .latest_message()
                .is_some_and(|message| message.contains("COMMIT 已完成"))
        );
    }

    #[test]
    fn triggers_event_without_pending_request_does_not_replace_list() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        let connection_id = crate::domain::ids::ConnectionId::from(uuid::Uuid::new_v4());
        let key = crate::session::task_registry::OperationKey::Metadata {
            connection: connection_id,
            scope: crate::session::task_registry::MetadataScope::Triggers,
        };
        let (task_id, token) = app.session.task_registry.register(
            key.clone(),
            crate::session::task_registry::TaskKind::Metadata,
        );
        let handle = app
            .session
            .runtime
            .spawn(async { std::future::pending::<()>().await });
        app.session.task_registry.attach(
            task_id,
            key.clone(),
            crate::session::task_registry::TaskKind::Metadata,
            handle,
            token,
        );

        let sentinel = vec![crate::data::TriggerInfo {
            name: "keep_me".to_string(),
            table_name: "t".to_string(),
            event: "INSERT".to_string(),
            timing: "AFTER".to_string(),
            definition: String::new(),
        }];
        app.state.sidebar_panel_state.set_triggers(sentinel);
        assert!(app.session.pending_triggers_request.is_none());

        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id,
                key,
                outcome: crate::session::runtime_event::RuntimeOutcome::TriggersFetched {
                    connection: connection_id,
                    database: None,
                    result: Ok(Vec::new()),
                },
            },
        );

        assert_eq!(app.state.sidebar_panel_state.triggers.len(), 1);
        assert_eq!(app.state.sidebar_panel_state.triggers[0].name, "keep_me");
        assert!(app.state.sidebar_panel_state.error_triggers.is_none());
    }

    #[test]
    fn triggers_event_for_inactive_connection_does_not_replace_list() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        let mut active_config =
            crate::data::ConnectionConfig::new("active_conn", crate::types::DatabaseType::SQLite);
        active_config.database = "active.db".to_string();
        app.session.manager.add(active_config);
        let mut other_config =
            crate::data::ConnectionConfig::new("other_conn", crate::types::DatabaseType::SQLite);
        other_config.database = "other.db".to_string();
        app.session.manager.add(other_config);
        app.session.manager.active = Some("active_conn".to_string());

        let other_id = app
            .session
            .manager
            .connection_id("other_conn")
            .expect("other connection is registered");
        let active_database = app
            .session
            .manager
            .connections
            .get("active_conn")
            .and_then(|conn| conn.selected_database.clone());
        app.session.pending_triggers_request =
            Some(("active_conn".to_string(), active_database.clone()));

        let key = crate::session::task_registry::OperationKey::Metadata {
            connection: other_id,
            scope: crate::session::task_registry::MetadataScope::Triggers,
        };
        let (task_id, token) = app.session.task_registry.register(
            key.clone(),
            crate::session::task_registry::TaskKind::Metadata,
        );
        let handle = app
            .session
            .runtime
            .spawn(async { std::future::pending::<()>().await });
        app.session.task_registry.attach(
            task_id,
            key.clone(),
            crate::session::task_registry::TaskKind::Metadata,
            handle,
            token,
        );

        let sentinel = vec![crate::data::TriggerInfo {
            name: "active_trigger".to_string(),
            table_name: "t".to_string(),
            event: "INSERT".to_string(),
            timing: "AFTER".to_string(),
            definition: String::new(),
        }];
        app.state.sidebar_panel_state.set_triggers(sentinel);
        app.state.sidebar_panel_state.loading_triggers = true;

        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id,
                key,
                outcome: crate::session::runtime_event::RuntimeOutcome::TriggersFetched {
                    connection: other_id,
                    database: active_database.clone(),
                    result: Ok(Vec::new()),
                },
            },
        );

        assert_eq!(app.state.sidebar_panel_state.triggers.len(), 1);
        assert_eq!(
            app.state.sidebar_panel_state.triggers[0].name,
            "active_trigger"
        );
        assert_eq!(
            app.session.pending_triggers_request,
            Some(("active_conn".to_string(), active_database)),
            "a foreign reply must not consume the active request"
        );
        assert!(app.state.sidebar_panel_state.loading_triggers);
    }

    #[test]
    fn connected_event_from_superseded_connection_instance_is_ignored() {
        let mut app = crate::app::DbManagerApp::new_for_test();
        let ctx = egui::Context::default();
        let config =
            crate::data::ConnectionConfig::new("rebuild", crate::types::DatabaseType::SQLite);
        app.session.manager.add(config.clone());
        let stale_id = app
            .session
            .manager
            .connection_id("rebuild")
            .expect("registered");
        app.session.manager.active = Some("rebuild".to_string());
        app.session
            .pending_connect_requests
            .insert("rebuild".to_string());

        // 编辑同名连接配置会替换连接实例并分配新的 `ConnectionId`，旧任务仍持旧 id。
        app.session.manager.connections.remove("rebuild");
        app.session.manager.add(config);
        let fresh_id = app
            .session
            .manager
            .connection_id("rebuild")
            .expect("re-registered");
        assert_ne!(stale_id, fresh_id);

        let key = crate::session::task_registry::OperationKey::Connect(stale_id);
        let (task_id, token) = app.session.task_registry.register(
            key.clone(),
            crate::session::task_registry::TaskKind::Connect,
        );
        let handle = app
            .session
            .runtime
            .spawn(async { std::future::pending::<()>().await });
        app.session.task_registry.attach(
            task_id,
            key.clone(),
            crate::session::task_registry::TaskKind::Connect,
            handle,
            token,
        );

        app.handle_runtime_event(
            &ctx,
            crate::session::runtime_event::RuntimeEvent {
                task_id,
                key,
                outcome: crate::session::runtime_event::RuntimeOutcome::Connected {
                    connection: stale_id,
                    conn_name: "rebuild".to_string(),
                    result: Ok(vec!["stale_table".to_string()]),
                },
            },
        );

        let conn = app
            .session
            .manager
            .connections
            .get("rebuild")
            .expect("connection remains registered");
        assert!(!conn.connected, "旧连接实例的回包不得把新实例标记为已连接");
        assert!(conn.tables.is_empty());
        assert!(
            !app.session
                .notifications
                .latest_message()
                .is_some_and(|message| message.contains("已连接到"))
        );
    }
}

#[cfg(test)]
#[path = "grid_identity_tests.rs"]
mod grid_identity_tests;
