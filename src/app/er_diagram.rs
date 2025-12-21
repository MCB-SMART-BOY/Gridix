//! ER 关系图模块
//!
//! 处理 ER 图数据加载和关系推断。

use crate::ui;
use super::{DbManagerApp, Message};

impl DbManagerApp {
    /// 加载 ER 图数据
    ///
    /// 从当前连接获取所有表信息，异步加载每个表的列结构和外键关系。
    pub fn load_er_diagram_data(&mut self) {
        // 清空旧数据
        self.er_diagram_state.clear();
        self.er_diagram_state.loading = true;

        if let Some(conn) = self.manager.get_active() {
            let tables = conn.tables.clone();
            let db_name = conn.selected_database.clone().unwrap_or_else(|| "未选择".to_string());
            let config = conn.config.clone();

            if tables.is_empty() {
                self.notifications.warning(format!("数据库 {} 没有表，请先选择数据库", db_name));
                self.er_diagram_state.loading = false;
                return;
            }

            // 创建 ER 表结构
            for table_name in &tables {
                let er_table = ui::ERTable::new(table_name.clone());
                self.er_diagram_state.tables.push(er_table);
            }

            // 应用初始布局
            ui::grid_layout(
                &mut self.er_diagram_state.tables,
                4,
                eframe::egui::Vec2::new(60.0, 50.0),
            );

            self.notifications.info(format!(
                "ER图: 加载 {} 张表，正在获取结构... ({})",
                tables.len(),
                db_name
            ));

            // 异步加载每个表的列信息
            for table_name in &tables {
                let tx = self.tx.clone();
                let config_clone = config.clone();
                let table_clone = table_name.clone();
                self.runtime.spawn(async move {
                    let result = crate::database::get_table_columns(&config_clone, &table_clone).await;
                    let _ = tx.send(Message::ERTableColumnsFetched(
                        table_clone,
                        result.map_err(|e| e.to_string()),
                    ));
                });
            }

            // 异步加载外键关系
            let tx = self.tx.clone();
            self.runtime.spawn(async move {
                let result = crate::database::get_foreign_keys(&config).await;
                let _ = tx.send(Message::ForeignKeysFetched(result.map_err(|e| e.to_string())));
            });
        } else {
            self.notifications.warning("请先连接数据库");
            self.er_diagram_state.loading = false;
        }

        self.er_diagram_state.needs_layout = false;
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
            .er_diagram_state
            .tables
            .iter()
            .map(|t| t.name.as_str())
            .collect();

        for table in &self.er_diagram_state.tables {
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
