//! 表格操作和 SQL 生成

use super::state::DataGridState;
use crate::data::QueryResult;

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
    /// 需要执行的 SQL 语句列表
    pub sql_to_execute: Vec<String>,
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

fn escape_result_cell(value: &str, is_null: bool) -> String {
    if is_null {
        "NULL".to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
}

fn escape_editor_input(value: &str) -> String {
    // Only truly empty input means NULL. The literal string "null" is a valid value.
    // Users can type \N or use the right-click menu to explicitly set NULL.
    if value.is_empty() {
        "NULL".to_string()
    } else {
        escape_value(value)
    }
}

/// 生成保存修改的 SQL（带确认）
pub(crate) fn generate_save_sql(
    result: &QueryResult,
    state: &mut DataGridState,
    table_name: &str,
    actions: &mut DataGridActions,
    db_type: Option<crate::data::DatabaseType>,
) {
    // Determine quoting style: backticks for MySQL, double-quotes for PG/SQLite.
    // SQLite supports both; PostgreSQL requires double-quotes; MySQL requires backticks.
    let use_backticks = matches!(db_type, Some(crate::data::DatabaseType::MySQL));

    // 验证并引用表名
    let safe_table_name = match quote_identifier(table_name, use_backticks) {
        Ok(name) => name,
        Err(e) => {
            actions.message = Some(format!("表名无效: {}", e));
            return;
        }
    };

    // 验证并引用所有列名
    let mut safe_columns: Vec<String> = Vec::new();
    for col in &result.columns {
        match quote_identifier(col, use_backticks) {
            Ok(name) => safe_columns.push(name),
            Err(e) => {
                actions.message = Some(format!("列名无效: {}", e));
                return;
            }
        }
    }

    let mut sql_statements = Vec::new();
    let has_deletes = !state.rows_to_delete.is_empty();
    // 统计被静默转成 NULL 的空单元格，保存时提示用户（审计 G5）。
    let mut null_coercions: usize = 0;

    // 获取主键列索引
    // 优先使用已设置的主键，其次尝试查找 "id" 列
    let pk_idx = match state.primary_key_column {
        Some(idx) => idx,
        None => {
            // 尝试查找名为 "id" 的列
            match result
                .columns
                .iter()
                .position(|c| c.eq_ignore_ascii_case("id"))
            {
                Some(idx) => idx,
                None => {
                    // 无法确定主键，拒绝执行修改操作以防止数据损坏
                    actions.message = Some(
                        "无法确定主键列：未找到 'id' 列。\n\
                         请确保表有主键列，或使用自定义 SQL 进行修改。"
                            .to_string(),
                    );
                    return;
                }
            }
        }
    };

    let pk_col = match safe_columns.get(pk_idx) {
        Some(col) => col.clone(),
        None => {
            actions.message = Some(format!("主键列索引 {} 超出范围", pk_idx));
            return;
        }
    };

    // 生成 UPDATE 语句
    for ((row_idx, col_idx), new_value) in &state.modified_cells {
        if let Some(row) = result.rows.get(*row_idx)
            && let Some(pk_value) = row.get(pk_idx)
            && let Some(col_name) = safe_columns.get(*col_idx)
        {
            let safe_value = escape_editor_input(new_value);
            if new_value.is_empty() {
                null_coercions += 1;
            }
            let safe_pk_value = escape_result_cell(pk_value, result.is_null(*row_idx, pk_idx));

            let sql = format!(
                "UPDATE {} SET {} = {} WHERE {} = {};",
                safe_table_name, col_name, safe_value, pk_col, safe_pk_value
            );
            sql_statements.push(sql);
        }
    }

    // 生成 DELETE 语句
    for row_idx in &state.rows_to_delete {
        if let Some(row) = result.rows.get(*row_idx)
            && let Some(pk_value) = row.get(pk_idx)
        {
            let safe_pk_value = escape_result_cell(pk_value, result.is_null(*row_idx, pk_idx));
            let sql = format!(
                "DELETE FROM {} WHERE {} = {};",
                safe_table_name, pk_col, safe_pk_value
            );
            sql_statements.push(sql);
        }
    }

    // 生成 INSERT 语句
    for new_row in &state.new_rows {
        if new_row.iter().any(|v| !v.is_empty()) {
            null_coercions += new_row.iter().filter(|v| v.is_empty()).count();
            let cols = safe_columns.join(", ");
            let vals: Vec<String> = new_row.iter().map(|v| escape_editor_input(v)).collect();
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({});",
                safe_table_name,
                cols,
                vals.join(", ")
            );
            sql_statements.push(sql);
        }
    }

    if sql_statements.is_empty() {
        actions.message = Some("没有需要保存的修改".to_string());
        return;
    }

    // 空单元格会被保存为 NULL；提示用户以免在 NOT NULL 列上误存（审计 G5）。
    let null_hint = if null_coercions > 0 {
        format!("（其中 {} 个空单元格将保存为 NULL）", null_coercions)
    } else {
        String::new()
    };

    // 如果包含删除操作，需要确认
    if has_deletes {
        state.pending_sql = sql_statements;
        state.show_save_confirm = true;
        actions.message = Some(format!(
            "包含 {} 条删除操作，请确认{}",
            state.rows_to_delete.len(),
            null_hint
        ));
    } else {
        // 没有删除操作，直接执行
        actions.sql_to_execute = sql_statements;
        actions.message = Some(format!(
            "将执行 {} 条 SQL 语句{}",
            actions.sql_to_execute.len(),
            null_hint
        ));
    }
}

/// 确认执行待确认的 SQL
pub(crate) fn confirm_pending_sql(state: &mut DataGridState, actions: &mut DataGridActions) {
    if !state.pending_sql.is_empty() {
        actions.sql_to_execute = std::mem::take(&mut state.pending_sql);
        actions.message = Some(format!("执行 {} 条 SQL 语句", actions.sql_to_execute.len()));
    }
    state.show_save_confirm = false;
}

/// 取消待确认的 SQL
pub(crate) fn cancel_pending_sql(state: &mut DataGridState) {
    state.pending_sql.clear();
    state.show_save_confirm = false;
}

#[cfg(test)]
mod tests {
    use super::{DataGridActions, cancel_pending_sql, confirm_pending_sql, generate_save_sql};
    use crate::data::QueryResult;
    use crate::ui::DataGridState;

    fn sample_result() -> QueryResult {
        QueryResult::with_rows(
            vec!["id".to_string(), "name".to_string()],
            vec![vec!["1".to_string(), "alice".to_string()]],
        )
    }

    #[test]
    fn generate_save_sql_keeps_edit_state_until_save_completes() {
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();

        state.modified_cells.insert((0, 1), "bob".to_string());

        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        assert_eq!(actions.sql_to_execute.len(), 1);
        assert_eq!(state.modified_cells.get(&(0, 1)), Some(&"bob".to_string()));
        assert!(state.has_changes());
    }

    #[test]
    fn generate_save_sql_warns_when_empty_cell_coerced_to_null() {
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();

        // 把 name 编辑成空字符串 → 会被保存为 NULL。
        state.modified_cells.insert((0, 1), String::new());

        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        let msg = actions.message.expect("应有保存提示");
        assert!(msg.contains("NULL"), "提示应说明空单元格转为 NULL: {msg}");
        assert!(msg.contains('1'), "提示应包含被转换的数量: {msg}");
        // SQL 本身仍写 NULL（行为不变）。
        assert_eq!(actions.sql_to_execute.len(), 1);
        assert!(actions.sql_to_execute[0].contains("NULL"));
    }

    #[test]
    fn generate_save_sql_no_null_hint_for_normal_edit() {
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();

        state.modified_cells.insert((0, 1), "bob".to_string());

        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        let msg = actions.message.expect("应有保存提示");
        assert!(!msg.contains("NULL"), "正常编辑不应出现 NULL 提示: {msg}");
    }

    #[test]
    fn confirm_pending_sql_keeps_edits_until_execution_result_returns() {
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();

        state.rows_to_delete.push(0);
        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        assert!(state.show_save_confirm);
        assert_eq!(state.pending_sql.len(), 1);

        confirm_pending_sql(&mut state, &mut actions);

        assert!(!state.show_save_confirm);
        assert_eq!(actions.sql_to_execute.len(), 1);
        assert_eq!(state.rows_to_delete, vec![0]);
        assert!(state.has_changes());
    }

    #[test]
    fn cancel_pending_sql_discards_confirmation_queue_only() {
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();

        state.rows_to_delete.push(0);
        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        cancel_pending_sql(&mut state);

        assert!(!state.show_save_confirm);
        assert!(state.pending_sql.is_empty());
        assert_eq!(state.rows_to_delete, vec![0]);
        assert!(state.has_changes());
    }

    #[test]
    fn generate_save_sql_uses_backticks_for_mysql() {
        use crate::data::DatabaseType;
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();

        state.modified_cells.insert((0, 1), "bob".to_string());

        generate_save_sql(
            &result,
            &mut state,
            "users",
            &mut actions,
            Some(DatabaseType::MySQL),
        );

        assert_eq!(actions.sql_to_execute.len(), 1);
        let sql = &actions.sql_to_execute[0];
        // MySQL 必须用反引号引用标识符，禁止出现双引号标识符（修复 B3）
        assert!(sql.contains("`users`"), "expected backtick table: {sql}");
        assert!(sql.contains("`name`"), "expected backtick column: {sql}");
        assert!(
            !sql.contains("\"users\"") && !sql.contains("\"name\""),
            "MySQL save must not use double-quoted identifiers: {sql}"
        );
    }

    #[test]
    fn generate_save_sql_uses_double_quotes_for_postgres_and_sqlite() {
        use crate::data::DatabaseType;
        for db_type in [DatabaseType::PostgreSQL, DatabaseType::SQLite] {
            let result = sample_result();
            let mut state = DataGridState::default();
            let mut actions = DataGridActions::default();

            state.modified_cells.insert((0, 1), "bob".to_string());

            generate_save_sql(&result, &mut state, "users", &mut actions, Some(db_type));

            assert_eq!(actions.sql_to_execute.len(), 1);
            let sql = &actions.sql_to_execute[0];
            assert!(
                sql.contains("\"users\"") && sql.contains("\"name\""),
                "{db_type:?} save must use double-quoted identifiers: {sql}"
            );
            assert!(
                !sql.contains('`'),
                "{db_type:?} save must not use backticks: {sql}"
            );
        }
    }
}
