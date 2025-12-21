//! 消息处理模块
//!
//! 处理从异步任务返回的各种消息，更新应用状态。

use eframe::egui;

use crate::ui;
use super::{DbManagerApp, Message};

impl DbManagerApp {
    /// 处理异步消息
    ///
    /// 轮询消息通道，处理数据库连接、查询结果、ER图数据等异步任务的返回结果。
    pub fn handle_messages(&mut self, ctx: &egui::Context) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Message::ConnectedWithTables(name, result) => {
                    self.handle_connected_with_tables(ctx, name, result);
                }
                Message::ConnectedWithDatabases(name, result) => {
                    self.handle_connected_with_databases(ctx, name, result);
                }
                Message::DatabaseSelected(conn_name, db_name, result) => {
                    self.handle_database_selected(ctx, conn_name, db_name, result);
                }
                Message::QueryDone(sql, result, elapsed_ms) => {
                    self.handle_query_done(ctx, sql, result, elapsed_ms);
                }
                Message::PrimaryKeyFetched(table_name, pk_column) => {
                    self.handle_primary_key_fetched(ctx, table_name, pk_column);
                }
                Message::TriggersFetched(result) => {
                    self.handle_triggers_fetched(ctx, result);
                }
                Message::RoutinesFetched(result) => {
                    self.handle_routines_fetched(ctx, result);
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
        result: Result<Vec<String>, String>,
    ) {
        self.connecting = false;
        match result {
            Ok(tables) => {
                self.notifications.success(
                    format!("已连接到 {} ({} 张表)", name, tables.len())
                );
                self.load_history_for_connection(&name);
                self.autocomplete.set_tables(tables.clone());
                if let Some(conn) = self.manager.connections.get_mut(&name) {
                    conn.set_connected(tables);
                }
                self.sidebar_panel_state.selection.reset_for_connection_change();
                self.load_triggers();
                self.load_routines();
            }
            Err(e) => self.handle_connection_error(&name, e),
        }
        ctx.request_repaint();
    }

    /// 处理 MySQL/PostgreSQL 连接完成消息
    fn handle_connected_with_databases(
        &mut self,
        ctx: &egui::Context,
        name: String,
        result: Result<Vec<String>, String>,
    ) {
        self.connecting = false;
        match result {
            Ok(databases) => {
                self.notifications.success(
                    format!("已连接到 {} ({} 个数据库)", name, databases.len())
                );
                self.load_history_for_connection(&name);
                self.autocomplete.clear();
                if let Some(conn) = self.manager.connections.get_mut(&name) {
                    conn.set_connected_with_databases(databases);
                }
                self.sidebar_panel_state.selection.reset_for_connection_change();
            }
            Err(e) => self.handle_connection_error(&name, e),
        }
        ctx.request_repaint();
    }

    /// 处理数据库选择完成消息
    fn handle_database_selected(
        &mut self,
        ctx: &egui::Context,
        conn_name: String,
        db_name: String,
        result: Result<Vec<String>, String>,
    ) {
        self.connecting = false;
        match result {
            Ok(tables) => {
                self.notifications.success(
                    format!("已选择数据库 {} ({} 张表)", db_name, tables.len())
                );
                self.autocomplete.set_tables(tables.clone());
                if let Some(conn) = self.manager.connections.get_mut(&conn_name) {
                    conn.set_database(db_name, tables);
                }
                self.sidebar_panel_state.selection.reset_for_database_change();
                self.load_triggers();
                self.load_routines();
            }
            Err(e) => {
                self.notifications.error(format!("选择数据库失败: {}", e));
            }
        }
        self.selected_table = None;
        self.result = None;
        ctx.request_repaint();
    }

    /// 处理查询完成消息
    fn handle_query_done(
        &mut self,
        ctx: &egui::Context,
        sql: String,
        result: Result<crate::database::QueryResult, String>,
        elapsed_ms: u64,
    ) {
        use crate::core::constants;

        self.executing = false;
        self.last_query_time_ms = Some(elapsed_ms);

        let sql_lower = sql.trim().to_lowercase();
        let is_update_or_delete = sql_lower.starts_with("update") || sql_lower.starts_with("delete");
        let is_insert = sql_lower.starts_with("insert");

        let db_type = self
            .manager
            .get_active()
            .map(|c| c.config.db_type.display_name().to_string())
            .unwrap_or_default();

        match result {
            Ok(mut res) => {
                // 限制结果集大小
                let original_rows = res.rows.len();
                let was_truncated = original_rows > constants::database::MAX_RESULT_SET_ROWS;
                if was_truncated {
                    res.rows.truncate(constants::database::MAX_RESULT_SET_ROWS);
                    res.truncated = true;
                    res.original_row_count = Some(original_rows);
                }

                self.query_history.add(
                    sql,
                    db_type,
                    true,
                    if res.affected_rows > 0 { Some(res.affected_rows) } else { None },
                );

                let msg = if res.columns.is_empty() {
                    format!("执行成功，影响 {} 行 ({}ms)", res.affected_rows, elapsed_ms)
                } else if was_truncated {
                    format!(
                        "查询完成，返回 {} 行（已截断，原始 {} 行，建议使用 LIMIT）({}ms)",
                        res.rows.len(), original_rows, elapsed_ms
                    )
                } else {
                    format!("查询完成，返回 {} 行 ({}ms)", res.rows.len(), elapsed_ms)
                };
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

                // 同步到当前 Tab
                if let Some(tab) = self.tab_manager.get_active_mut() {
                    tab.result = Some(res.clone());
                    tab.executing = false;
                    tab.query_time_ms = Some(elapsed_ms);
                    tab.last_message = Some(msg);
                }

                // 更新自动补全
                if let Some(table) = &self.selected_table
                    && !res.columns.is_empty() {
                        self.autocomplete.set_columns(table.clone(), res.columns.clone());
                    }

                self.result = Some(res);
            }
            Err(e) => {
                self.query_history.add(sql, db_type, false, None);
                let err_msg = format!("错误: {}", e);
                self.notifications.error(&err_msg);
                self.result = Some(crate::database::QueryResult::default());

                if let Some(tab) = self.tab_manager.get_active_mut() {
                    tab.executing = false;
                    tab.last_message = Some(err_msg);
                }
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
                    && let Some(idx) = result.columns.iter().position(|c| c == &pk_name) {
                        self.grid_state.primary_key_column = Some(idx);
                    }
            } else {
                self.grid_state.primary_key_column = None;
            }
        }
        ctx.request_repaint();
    }

    /// 处理触发器获取完成消息
    fn handle_triggers_fetched(
        &mut self,
        ctx: &egui::Context,
        result: Result<Vec<crate::database::TriggerInfo>, String>,
    ) {
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
        result: Result<Vec<crate::database::RoutineInfo>, String>,
    ) {
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
                // 更新表中列的外键标记
                for fk in &fks {
                    if let Some(table) = self.er_diagram_state.tables.iter_mut().find(|t| t.name == fk.from_table)
                        && let Some(col) = table.columns.iter_mut().find(|c| c.name == fk.from_column) {
                            col.is_foreign_key = true;
                        }
                }

                // 转换为 ER 图关系
                let mut relationships: Vec<ui::Relationship> = fks
                    .into_iter()
                    .map(|fk| ui::Relationship {
                        from_table: fk.from_table,
                        from_column: fk.from_column,
                        to_table: fk.to_table,
                        to_column: fk.to_column,
                        relation_type: ui::RelationType::OneToMany,
                    })
                    .collect();

                // 如果没有外键，尝试推断
                if relationships.is_empty() {
                    relationships = self.infer_relationships_from_columns();
                }

                let rel_count = relationships.len();
                self.er_diagram_state.relationships = relationships;
                self.er_diagram_state.loading = false;

                if rel_count > 0 {
                    self.notifications.info(format!(
                        "ER图: {} 张表, {} 个关系",
                        self.er_diagram_state.tables.len(),
                        rel_count
                    ));
                } else {
                    self.notifications.info(format!(
                        "ER图: {} 张表（未发现外键关系）",
                        self.er_diagram_state.tables.len()
                    ));
                }
            }
            Err(e) => {
                self.er_diagram_state.loading = false;
                self.notifications.error(format!("加载外键关系失败: {}", e));
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
                if let Some(er_table) = self.er_diagram_state.tables.iter_mut().find(|t| t.name == table_name) {
                    er_table.columns = columns
                        .into_iter()
                        .map(|c| ui::ERColumn {
                            name: c.name,
                            data_type: c.data_type,
                            is_primary_key: c.is_primary_key,
                            is_foreign_key: false,
                            nullable: c.is_nullable,
                            default_value: c.default_value,
                        })
                        .collect();
                    // 立即计算表格尺寸，确保布局和关系线渲染正确
                    ui::calculate_table_size(er_table);
                }

                // 检查是否所有表都加载完成
                let all_loaded = self.er_diagram_state.tables.iter().all(|t| !t.columns.is_empty());
                if all_loaded && !self.er_diagram_state.tables.is_empty() {
                    ui::grid_layout(
                        &mut self.er_diagram_state.tables,
                        4,
                        egui::Vec2::new(60.0, 50.0),
                    );

                    if self.er_diagram_state.relationships.is_empty() {
                        let inferred = self.infer_relationships_from_columns();
                        if !inferred.is_empty() {
                            self.er_diagram_state.relationships = inferred;
                            self.notifications.info(format!(
                                "ER图: 推断出 {} 个关系",
                                self.er_diagram_state.relationships.len()
                            ));
                        }
                    }
                }
            }
            Err(e) => {
                self.notifications.warning(format!("获取表 {} 结构失败: {}", table_name, e));
            }
        }
        ctx.request_repaint();
    }
}
