//! ER 关系图模块
//!
//! 处理 ER 图数据加载和关系推断。

use super::DbManagerApp;
use crate::data::Connection;
use crate::domain::ids::ConnectionId;
use crate::ui;

#[derive(Debug, Clone, PartialEq, Eq)]
enum ErDiagramLoadPlan {
    NoActiveConnection,
    EmptyTables {
        db_name: String,
    },
    Load {
        tables: Vec<String>,
        db_name: String,
        connection_id: ConnectionId,
    },
}

fn plan_er_diagram_load(active_connection: Option<&Connection>) -> ErDiagramLoadPlan {
    let Some(conn) = active_connection else {
        return ErDiagramLoadPlan::NoActiveConnection;
    };

    let tables = conn.tables.clone();
    let db_name = conn
        .selected_database
        .clone()
        .unwrap_or_else(|| "未选择".to_string());

    if tables.is_empty() {
        return ErDiagramLoadPlan::EmptyTables { db_name };
    }

    ErDiagramLoadPlan::Load {
        tables,
        db_name,
        connection_id: conn.id,
    }
}

impl DbManagerApp {
    /// 加载 ER 图数据
    ///
    /// 从 `SchemaCatalog` 同步读取所有表的列结构和外键关系，
    /// 不再通过异步 N+1 查询。
    pub fn load_er_diagram_data(&mut self) {
        match plan_er_diagram_load(self.session.manager.get_active()) {
            ErDiagramLoadPlan::NoActiveConnection => {
                self.state.er_diagram_state.clear();
                self.session.notifications.warning("请先连接数据库");
                self.state.er_diagram_state.loading = false;
            }
            ErDiagramLoadPlan::EmptyTables { db_name } => {
                self.state.er_diagram_state.clear();
                self.session
                    .notifications
                    .warning(format!("数据库 {} 没有表，请先选择数据库", db_name));
                self.state.er_diagram_state.loading = false;
            }
            ErDiagramLoadPlan::Load {
                tables,
                db_name,
                connection_id,
            } => {
                let layout_snapshot = self.state.er_diagram_state.capture_layout_snapshot();
                let catalog_key = (connection_id, db_name.clone());
                let catalog = self.session.schema_catalogs.get(&catalog_key);

                let Some(catalog) = catalog else {
                    // Catalog 未加载：创建表壳（无列信息），保持与旧 N+1 加载初始态的兼容。
                    let layout_snapshot = self.state.er_diagram_state.capture_layout_snapshot();
                    self.state.er_diagram_state.clear();
                    self.state
                        .er_diagram_state
                        .set_pending_layout_restore(layout_snapshot);

                    let display_mode = self.state.er_diagram_state.card_display_mode();
                    let mut er_tables = Vec::with_capacity(tables.len());
                    for table_name in &tables {
                        let mut er_table = ui::ERTable::new(table_name.clone());
                        ui::calculate_table_size_for_mode(&mut er_table, display_mode);
                        er_tables.push(er_table);
                    }
                    self.state.er_diagram_state.tables = er_tables;
                    self.state.er_diagram_state.mark_foreign_keys_resolved();
                    self.session.notifications.info(format!(
                        "ER图: {} 张表（schema目录未加载，结构信息待重载）",
                        tables.len()
                    ));
                    self.state.er_diagram_state.needs_layout = false;
                    return;
                };

                // 清空旧状态，保留布局快照用于后续恢复
                self.state.er_diagram_state.clear();
                self.state
                    .er_diagram_state
                    .set_pending_layout_restore(layout_snapshot);

                // 从 catalog 构建 ER 表（含列信息）
                let display_mode = self.state.er_diagram_state.card_display_mode();
                let mut er_tables = Vec::with_capacity(tables.len());
                for table_name in &tables {
                    let mut er_table = ui::ERTable::new(table_name.clone());
                    if let Some(table_meta) = catalog.table(table_name) {
                        er_table.columns = table_meta
                            .columns
                            .iter()
                            .map(|c| ui::ERColumn {
                                name: c.name.clone(),
                                data_type: c.type_info.native_name.clone(),
                                is_primary_key: c.is_primary_key,
                                is_foreign_key: false, // 下一步统一设置
                                nullable: c.is_nullable,
                                default_value: c.default_value.clone(),
                            })
                            .collect();
                    }
                    ui::calculate_table_size_for_mode(&mut er_table, display_mode);
                    er_tables.push(er_table);
                }

                // 从 catalog 构建外键关系与 FK 列集合（支持复合外键逐列展开）
                use std::collections::HashSet;
                let mut fk_columns: HashSet<(String, String)> = HashSet::new();
                let mut relationships = Vec::new();
                for table_meta in &catalog.tables {
                    for fk in &table_meta.foreign_keys {
                        for (from_col, ref_col) in fk.from_columns.iter().zip(fk.ref_columns.iter())
                        {
                            fk_columns.insert((table_meta.name.clone(), from_col.clone()));
                            relationships.push(ui::Relationship {
                                from_table: table_meta.name.clone(),
                                from_column: from_col.clone(),
                                to_table: fk.ref_table.clone(),
                                to_column: ref_col.clone(),
                                relation_type: ui::RelationType::OneToMany,
                                origin: ui::RelationshipOrigin::Explicit,
                            });
                        }
                    }
                }

                // 将 FK 标记写入列
                for table in &mut er_tables {
                    for col in &mut table.columns {
                        col.is_foreign_key =
                            fk_columns.contains(&(table.name.clone(), col.name.clone()));
                    }
                }

                self.state.er_diagram_state.tables = er_tables;
                self.state
                    .er_diagram_state
                    .set_foreign_key_columns(fk_columns);
                self.state.er_diagram_state.relationships = relationships;

                self.finalize_er_diagram_load_if_ready();
            }
        }

        self.state.er_diagram_state.needs_layout = false;
    }

    /// 基于列名推断表之间的关系
    ///
    /// 规则：如果列名是 `xxx_id` 或 `xxxid`，尝试匹配名为 `xxx` 或 `xxxs` 的表。
    ///
    /// # 返回
    ///
    /// 推断出的关系列表
    pub fn infer_relationships_from_columns(&self) -> Vec<ui::Relationship> {
        let mut relationships = Vec::new();
        let table_names: Vec<&str> = self
            .state
            .er_diagram_state
            .tables
            .iter()
            .map(|t| t.name.as_str())
            .collect();

        for table in &self.state.er_diagram_state.tables {
            for col in &table.columns {
                // 跳过主键列
                if col.is_primary_key {
                    continue;
                }

                let col_lower = col.name.to_lowercase();

                // 检查是否是可能的外键列
                let potential_ref = if col_lower.ends_with("_id") {
                    Some(col_lower.trim_end_matches("_id").to_string())
                } else if col_lower.ends_with("id") && col_lower.len() > 2 {
                    Some(col_lower.trim_end_matches("id").to_string())
                } else {
                    None
                };

                if let Some(ref_name) = potential_ref {
                    // 尝试匹配表名
                    for &target_table in &table_names {
                        if target_table == table.name {
                            continue; // 跳过自引用
                        }

                        let target_lower = target_table.to_lowercase();

                        // 匹配：user, users, user_info 等
                        if target_lower == ref_name
                            || target_lower == format!("{}s", ref_name)
                            || target_lower == format!("{}_info", ref_name)
                            || target_lower.starts_with(&format!("{}_", ref_name))
                        {
                            relationships.push(ui::Relationship {
                                from_table: table.name.clone(),
                                from_column: col.name.clone(),
                                to_table: target_table.to_string(),
                                to_column: "id".to_string(),
                                relation_type: ui::RelationType::OneToMany,
                                origin: ui::RelationshipOrigin::Inferred,
                            });
                            break;
                        }
                    }
                }
            }
        }

        relationships
    }
}

#[cfg(test)]
mod tests {
    use super::{ErDiagramLoadPlan, plan_er_diagram_load};
    use crate::data::{Connection, ConnectionConfig, DatabaseType};

    fn sqlite_connection(name: &str) -> Connection {
        Connection::new(ConnectionConfig::new(name, DatabaseType::SQLite))
    }

    #[test]
    fn er_diagram_load_plan_requires_active_connection() {
        assert_eq!(
            plan_er_diagram_load(None),
            ErDiagramLoadPlan::NoActiveConnection
        );
    }

    #[test]
    fn er_diagram_load_plan_reports_empty_tables_for_selected_database() {
        let mut connection = sqlite_connection("demo");
        connection.connected = true;
        connection.selected_database = Some("main".to_string());

        assert_eq!(
            plan_er_diagram_load(Some(&connection)),
            ErDiagramLoadPlan::EmptyTables {
                db_name: "main".to_string()
            }
        );
    }

    #[test]
    fn er_diagram_load_plan_preserves_tables_and_falls_back_to_unselected_database_name() {
        let mut connection = sqlite_connection("demo");
        connection.connected = true;
        connection.tables = vec!["users".to_string(), "orders".to_string()];

        match plan_er_diagram_load(Some(&connection)) {
            ErDiagramLoadPlan::Load {
                tables,
                db_name,
                connection_id,
            } => {
                assert_eq!(db_name, "未选择");
                assert_eq!(tables, vec!["users", "orders"]);
                // connection_id 是由 ConnectionId::default() 生成的 UUID
                assert_eq!(connection_id, connection.id);
            }
            other => panic!("unexpected load plan: {other:?}"),
        }
    }
}
