//! 表格操作和 SQL 生成

use super::state::DataGridState;

/// 焦点转移方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTransfer {
    /// 转移到侧边栏
    Sidebar,
    /// 转移到 SQL 编辑器
    SqlEditor,
    /// 转移到查询Tab栏
    QueryTabs,
}

/// 表格操作返回值
#[derive(Default)]
pub struct DataGridActions {
    /// 类型化 MutationBatch；所有网格保存统一经由参数绑定执行。
    pub mutation_batch: Option<crate::domain::mutation::MutationBatch>,
    /// 状态消息
    pub message: Option<String>,
    /// 请求刷新表格数据
    pub refresh_requested: bool,
    /// 请求焦点转移
    pub focus_transfer: Option<FocusTransfer>,
    /// 表格被点击，请求获取焦点
    pub request_focus: bool,
    /// 请求打开左侧栏筛选面板
    pub open_filter_panel: bool,
    /// 请求将当前行滚动到屏幕中央 (zz/zc)
    pub scroll_to_center: bool,
    /// 请求将当前行滚动到屏幕顶部 (zt)
    pub scroll_to_top: bool,
    /// 请求将当前行滚动到屏幕底部 (zb)
    pub scroll_to_bottom: bool,
    /// 请求切换到指定的查询Tab (1-indexed)
    pub switch_to_tab: Option<usize>,
}

/// SQL 危险保留字（可能被用于注入攻击）
const SQL_DANGEROUS_KEYWORDS: &[&str] = &[
    "DROP", "DELETE", "TRUNCATE", "ALTER", "CREATE", "INSERT", "UPDATE", "EXEC", "EXECUTE",
    "UNION", "SELECT", "FROM", "WHERE", "OR", "AND", "--", "/*", "*/", "GRANT", "REVOKE",
    "SHUTDOWN", "KILL",
];

/// 验证 SQL 标识符（表名、列名）
///
/// 防止 SQL 注入攻击，禁止危险字符和保留字
/// 返回经过验证的原始标识符（不加引号）
pub fn escape_identifier(name: &str) -> Result<String, String> {
    if name.is_empty() {
        return Err("标识符不能为空".to_string());
    }

    // 限制长度（PostgreSQL 63 字符，MySQL 64 字符，取最小值）
    if name.len() > 63 {
        return Err(format!("标识符过长 (最大63字符): {}", name));
    }

    // 禁止包含危险字符：引号、分号、注释符等
    let dangerous_chars = ['"', '\'', ';', '/', '*', '\\', '\n', '\r', '\0', '`', '-'];
    for c in name.chars() {
        if dangerous_chars.contains(&c) {
            return Err(format!("标识符 '{}' 包含非法字符 '{}'", name, c));
        }
    }

    // 检查是否为危险保留字（仅当整个标识符是保留字时拒绝）
    let upper = name.to_uppercase();
    for keyword in SQL_DANGEROUS_KEYWORDS {
        if upper == *keyword {
            return Err(format!("标识符 '{}' 是 SQL 保留字", name));
        }
    }

    // 返回经过验证的原始标识符
    Ok(name.to_string())
}

/// 为 SQL 查询引用标识符（根据数据库类型使用不同的引号）
///
/// - MySQL: 使用反引号 `table`
/// - PostgreSQL/SQLite: 使用双引号 "table"
pub fn quote_identifier(name: &str, use_backticks: bool) -> Result<String, String> {
    // 先验证标识符
    let validated = escape_identifier(name)?;

    if use_backticks {
        // MySQL 使用反引号
        Ok(format!("`{}`", validated.replace('`', "``")))
    } else {
        // PostgreSQL/SQLite 使用双引号
        Ok(format!("\"{}\"", validated.replace('"', "\"\"")))
    }
}

/// 转义 SQL 字符串值
///
/// 处理单引号转义，防止 SQL 注入
pub fn escape_value(value: &str) -> String {
    // 转义单引号为两个单引号
    format!("'{}'", value.replace("'", "''"))
}

/// 确认执行待确认的参数化变异批次。
pub(crate) fn confirm_pending_mutations(state: &mut DataGridState, actions: &mut DataGridActions) {
    if let Some(batch) = state.pending_mutation_batch.take() {
        actions.message = Some(format!("执行 {} 项变更", batch.len()));
        actions.mutation_batch = Some(batch);
    }
    state.show_save_confirm = false;
}

/// 取消待确认的变异批次，保留用户编辑状态。
pub(crate) fn cancel_pending_mutations(state: &mut DataGridState) {
    state.pending_mutation_batch = None;
    state.show_save_confirm = false;
}

use crate::domain::mutation::{
    ColumnRef, ExpectedRows, InputValue, Mutation, MutationBatch, RowIdentity,
};
use crate::domain::result::ResultSet;
use crate::domain::value::{DbTypeInfo, DbValue};
use std::collections::{BTreeMap, BTreeSet};

/// 从网格编辑状态构建参数化、原子执行的变异批次。
pub(crate) fn build_mutation_batch(
    result: &ResultSet,
    state: &DataGridState,
    table_name: &str,
) -> Result<MutationBatch, String> {
    let primary_key_indices = collect_primary_key_indices(result, state)?;
    let deleted_rows: BTreeSet<usize> = state.rows_to_delete.iter().copied().collect();
    let table = ColumnRef {
        name: table_name.to_string(),
    };
    let mut batch = MutationBatch::new();

    append_update_mutations(
        &mut batch,
        result,
        state,
        &table,
        &primary_key_indices,
        &deleted_rows,
    )?;
    append_delete_mutations(
        &mut batch,
        result,
        &table,
        &primary_key_indices,
        &deleted_rows,
    )?;
    append_insert_mutations(&mut batch, result, state, &table)?;

    if batch.is_empty() {
        return Err("没有需要保存的修改".to_string());
    }
    Ok(batch)
}

fn collect_primary_key_indices(
    result: &ResultSet,
    state: &DataGridState,
) -> Result<Vec<usize>, String> {
    let tm = state
        .table_metadata
        .as_ref()
        .ok_or_else(|| "表元数据未加载，请先选择表并等待 schema 加载完成".to_string())?;
    let pk = tm
        .primary_key
        .as_ref()
        .ok_or_else(|| "该表没有主键，无法安全编辑".to_string())?;
    let indices: Vec<usize> = pk
        .columns
        .iter()
        .filter_map(|col_name| result.columns.iter().position(|c| c.name == *col_name))
        .collect();
    if indices.len() != pk.columns.len() {
        return Err("主键列与结果集列不匹配，请刷新后重试。".to_string());
    }
    Ok(indices)
}

fn append_update_mutations(
    batch: &mut MutationBatch,
    result: &ResultSet,
    state: &DataGridState,
    table: &ColumnRef,
    primary_key_indices: &[usize],
    deleted_rows: &BTreeSet<usize>,
) -> Result<(), String> {
    let mut changes_by_row: BTreeMap<usize, Vec<(usize, &String)>> = BTreeMap::new();
    for ((row_index, column_index), value) in &state.modified_cells {
        if !deleted_rows.contains(row_index) {
            changes_by_row
                .entry(*row_index)
                .or_default()
                .push((*column_index, value));
        }
    }

    for (row_index, changes) in changes_by_row {
        let identity = build_row_identity(result, row_index, primary_key_indices)?;
        let changes = build_column_changes(result, changes)?;
        batch.mutations.push(Mutation::Update {
            table: table.clone(),
            identity,
            changes,
            expected_rows: ExpectedRows::Exactly(1),
        });
    }
    Ok(())
}

fn append_delete_mutations(
    batch: &mut MutationBatch,
    result: &ResultSet,
    table: &ColumnRef,
    primary_key_indices: &[usize],
    deleted_rows: &BTreeSet<usize>,
) -> Result<(), String> {
    for row_index in deleted_rows {
        batch.mutations.push(Mutation::Delete {
            table: table.clone(),
            identity: build_row_identity(result, *row_index, primary_key_indices)?,
            expected_rows: ExpectedRows::Exactly(1),
        });
    }
    Ok(())
}

fn append_insert_mutations(
    batch: &mut MutationBatch,
    result: &ResultSet,
    state: &DataGridState,
    table: &ColumnRef,
) -> Result<(), String> {
    for row in &state.new_rows {
        if row.len() != result.column_count() {
            return Err("新增行列数与结果集不一致".to_string());
        }
        let values = row
            .iter()
            .zip(result.columns.iter())
            .map(|(value, column)| input_value_from_text(value, &column.type_info))
            .collect();
        let columns = result
            .columns
            .iter()
            .map(|column| ColumnRef {
                name: column.name.clone(),
            })
            .collect();
        batch.mutations.push(Mutation::Insert {
            table: table.clone(),
            columns,
            values,
        });
    }
    Ok(())
}

fn build_row_identity(
    result: &ResultSet,
    row_index: usize,
    primary_key_indices: &[usize],
) -> Result<RowIdentity, String> {
    if row_index >= result.row_count {
        return Err(format!("修改的行 {} 已不在当前结果集中", row_index + 1));
    }
    let columns = primary_key_indices
        .iter()
        .map(|column_index| {
            let column = result
                .columns
                .get(*column_index)
                .ok_or_else(|| "主键列索引超出范围".to_string())?;
            let value = result.cell(row_index, *column_index);
            if matches!(value, DbValue::Null) {
                return Err(format!("第 {} 行主键为 NULL，无法安全保存", row_index + 1));
            }
            Ok((
                ColumnRef {
                    name: column.name.clone(),
                },
                value.clone(),
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(RowIdentity::PrimaryKey(columns))
}

fn build_column_changes(
    result: &ResultSet,
    mut changes: Vec<(usize, &String)>,
) -> Result<Vec<(ColumnRef, InputValue)>, String> {
    changes.sort_by_key(|(column_index, _)| *column_index);
    changes
        .into_iter()
        .map(|(column_index, value)| {
            let column = result
                .columns
                .get(column_index)
                .ok_or_else(|| "修改的列已不在当前结果集中".to_string())?;
            Ok((
                ColumnRef {
                    name: column.name.clone(),
                },
                input_value_from_text(value, &column.type_info),
            ))
        })
        .collect()
}

fn input_value_from_text(value: &str, type_info: &DbTypeInfo) -> InputValue {
    if value.is_empty() {
        return InputValue::Value(DbValue::Text(String::new()));
    }
    InputValue::Value(crate::data::infer_value(value, type_info))
}
#[cfg(test)]
mod tests {
    use super::{
        DataGridActions, build_mutation_batch, cancel_pending_mutations, confirm_pending_mutations,
    };
    use crate::domain::metadata::{KeyMetadata, TableMetadata};
    use crate::domain::mutation::{InputValue, Mutation, RowIdentity};
    use crate::domain::result::{ResultColumn, ResultCompleteness, ResultSet};
    use crate::domain::value::{DbTypeFamily, DbTypeInfo, DbValue};
    use crate::ui::DataGridState;
    use std::sync::Arc;

    fn sample_result() -> ResultSet {
        ResultSet {
            columns: std::sync::Arc::new([
                ResultColumn {
                    name: "id".into(),
                    type_info: DbTypeInfo {
                        family: DbTypeFamily::Integer,
                        native_name: "INTEGER".into(),
                        nullable: Some(false),
                    },
                },
                ResultColumn {
                    name: "name".into(),
                    type_info: DbTypeInfo {
                        family: DbTypeFamily::Text,
                        native_name: "TEXT".into(),
                        nullable: Some(false),
                    },
                },
            ]),
            cells: vec![DbValue::Int(1), DbValue::Text("alice".into())],
            row_count: 1,
            completeness: ResultCompleteness::Complete,
        }
    }

    fn single_pk_metadata() -> Arc<TableMetadata> {
        Arc::new(TableMetadata {
            name: "users".to_string(),
            schema: None,
            columns: vec![],
            primary_key: Some(KeyMetadata {
                name: None,
                columns: vec!["id".to_string()],
            }),
            unique_keys: vec![],
            foreign_keys: vec![],
        })
    }

    fn composite_pk_metadata() -> Arc<TableMetadata> {
        Arc::new(TableMetadata {
            name: "users".to_string(),
            schema: None,
            columns: vec![],
            primary_key: Some(KeyMetadata {
                name: None,
                columns: vec!["tenant_id".to_string(), "user_id".to_string()],
            }),
            unique_keys: vec![],
            foreign_keys: vec![],
        })
    }

    #[test]
    fn build_mutation_batch_update_keeps_edit_state() {
        let result = sample_result();
        let mut state = DataGridState {
            table_metadata: Some(single_pk_metadata()),
            ..Default::default()
        };
        state.modified_cells.insert((0, 1), "bob".to_string());

        let batch = build_mutation_batch(&result, &state, "users").expect("batch");

        assert_eq!(batch.len(), 1);
        assert_eq!(state.modified_cells.get(&(0, 1)), Some(&"bob".to_string()));
        assert!(state.has_changes());
    }

    #[test]
    fn build_mutation_batch_empty_string_is_text_not_null() {
        let result = sample_result();
        let mut state = DataGridState {
            table_metadata: Some(single_pk_metadata()),
            ..Default::default()
        };
        state.modified_cells.insert((0, 1), String::new());

        let batch = build_mutation_batch(&result, &state, "users").expect("batch");

        let Mutation::Update { changes, .. } = &batch.mutations[0] else {
            panic!("expected update mutation");
        };
        assert!(matches!(
            changes[0].1,
            InputValue::Value(DbValue::Text(ref value)) if value.is_empty()
        ));
    }

    #[test]
    fn build_mutation_batch_composite_primary_key_uses_all_key_columns() {
        let result = ResultSet {
            columns: std::sync::Arc::new([
                ResultColumn {
                    name: "tenant_id".into(),
                    type_info: DbTypeInfo {
                        family: DbTypeFamily::Integer,
                        native_name: "INTEGER".into(),
                        nullable: Some(false),
                    },
                },
                ResultColumn {
                    name: "user_id".into(),
                    type_info: DbTypeInfo {
                        family: DbTypeFamily::Integer,
                        native_name: "INTEGER".into(),
                        nullable: Some(false),
                    },
                },
                ResultColumn {
                    name: "name".into(),
                    type_info: DbTypeInfo {
                        family: DbTypeFamily::Text,
                        native_name: "TEXT".into(),
                        nullable: Some(false),
                    },
                },
            ]),
            cells: vec![
                DbValue::Int(7),
                DbValue::Int(9),
                DbValue::Text("alice".into()),
            ],
            row_count: 1,
            completeness: ResultCompleteness::Complete,
        };
        let mut state = DataGridState {
            table_metadata: Some(composite_pk_metadata()),
            ..Default::default()
        };
        state.modified_cells.insert((0, 2), "bob".to_string());

        let batch = build_mutation_batch(&result, &state, "users").expect("batch");

        let Mutation::Update { identity, .. } = &batch.mutations[0] else {
            panic!("expected update mutation");
        };
        let RowIdentity::PrimaryKey(columns) = identity else {
            panic!("expected primary-key identity");
        };
        assert_eq!(columns.len(), 2);
    }

    #[test]
    fn build_mutation_batch_missing_row_returns_error() {
        let result = sample_result();
        let mut state = DataGridState {
            table_metadata: Some(single_pk_metadata()),
            ..Default::default()
        };
        state.modified_cells.insert((1, 1), "bob".to_string());

        let error = build_mutation_batch(&result, &state, "users").expect_err("missing row");

        assert!(error.contains("不在当前结果集中"));
    }

    #[test]
    fn confirm_pending_mutations_keeps_edits_until_execution_result_returns() {
        let result = sample_result();
        let mut state = DataGridState {
            table_metadata: Some(single_pk_metadata()),
            ..Default::default()
        };
        state.rows_to_delete.push(0);
        state.pending_mutation_batch =
            Some(build_mutation_batch(&result, &state, "users").expect("batch"));
        state.show_save_confirm = true;
        let mut actions = DataGridActions::default();

        confirm_pending_mutations(&mut state, &mut actions);

        assert!(!state.show_save_confirm);
        assert!(state.pending_mutation_batch.is_none());
        assert!(actions.mutation_batch.is_some());
        assert_eq!(state.rows_to_delete, vec![0]);
        assert!(state.has_changes());
    }

    #[test]
    fn cancel_pending_mutations_discards_confirmation_queue_only() {
        let result = sample_result();
        let mut state = DataGridState {
            table_metadata: Some(single_pk_metadata()),
            ..Default::default()
        };
        state.rows_to_delete.push(0);
        state.pending_mutation_batch =
            Some(build_mutation_batch(&result, &state, "users").expect("batch"));
        state.show_save_confirm = true;

        cancel_pending_mutations(&mut state);

        assert!(!state.show_save_confirm);
        assert!(state.pending_mutation_batch.is_none());
        assert_eq!(state.rows_to_delete, vec![0]);
        assert!(state.has_changes());
    }
}
