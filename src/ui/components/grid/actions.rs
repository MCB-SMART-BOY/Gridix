//! 表格操作和 SQL 生成

use super::state::DataGridState;
use crate::data::{ColumnInfo, QueryResult};

/// 单元格保存前校验发现的问题（审计 G6）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CellIssue {
    /// 非空列存入空值（会写成 NULL）。
    NotNull,
    /// 值无法解析为列声明的明显类型（整数/浮点/布尔）。
    TypeMismatch,
}

/// 列的"明显"基础类型，仅用于轻量客户端校验（不覆盖完整 SQL 类型系统）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SimpleType {
    Integer,
    Float,
    Boolean,
    /// 文本/日期/未知 —— 不做类型校验。
    Other,
}

fn simple_type_of(data_type: &str) -> SimpleType {
    let t = data_type.to_ascii_lowercase();
    // 先判浮点，避免 "int" 子串误伤；布尔需在整数前判（"bool" 不含 int）。
    if t.contains("bool") {
        SimpleType::Boolean
    } else if t.contains("real")
        || t.contains("doub")
        || t.contains("float")
        || t.contains("numeric")
        || t.contains("decimal")
    {
        SimpleType::Float
    } else if t.contains("int") || t.contains("serial") {
        SimpleType::Integer
    } else {
        SimpleType::Other
    }
}

/// 校验一个待保存的单元格值是否与列定义明显冲突。
///
/// 仅覆盖"明显"问题:非空列存空、整数/浮点/布尔列存无法解析的值。
/// 日期/文本/未知类型、以及空值（除非非空列）一律放行（返回 None）。
pub(crate) fn validate_cell(value: &str, col: &ColumnInfo) -> Option<CellIssue> {
    // 空值:落在非空列上才算问题(会写成 NULL)。
    if value.is_empty() {
        return (!col.is_nullable).then_some(CellIssue::NotNull);
    }

    let ok = match simple_type_of(&col.data_type) {
        SimpleType::Integer => value.trim().parse::<i64>().is_ok(),
        SimpleType::Float => value.trim().parse::<f64>().is_ok(),
        SimpleType::Boolean => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "true" | "false" | "0" | "1" | "t" | "f"
        ),
        SimpleType::Other => true,
    };
    (!ok).then_some(CellIssue::TypeMismatch)
}

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
    // 收集逐列校验问题（列名, CellIssue），保存时逐列提示（审计 G6）。
    // 仅在有列元数据时填充；无元数据则与旧行为一致。
    let mut cell_issues: Vec<(String, CellIssue)> = Vec::new();

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
            // 客户端校验（审计 G6）：列元数据与 result.columns 按位置对齐。
            if let Some(col_meta) = state.column_metadata.get(*col_idx)
                && let Some(issue) = validate_cell(new_value, col_meta)
            {
                cell_issues.push((col_meta.name.clone(), issue));
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
            // 逐单元格客户端校验（审计 G6）。
            for (col_idx, v) in new_row.iter().enumerate() {
                if let Some(col_meta) = state.column_metadata.get(col_idx)
                    && let Some(issue) = validate_cell(v, col_meta)
                {
                    cell_issues.push((col_meta.name.clone(), issue));
                }
            }
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

    // 逐列校验提示（审计 G6）：去重，最多列出前 5 项以保持可读。
    let validation_hint = build_validation_hint(&cell_issues);

    // 如果包含删除操作，需要确认
    if has_deletes {
        state.pending_sql = sql_statements;
        state.show_save_confirm = true;
        actions.message = Some(format!(
            "包含 {} 条删除操作，请确认{}{}",
            state.rows_to_delete.len(),
            null_hint,
            validation_hint
        ));
    } else {
        // 没有删除操作，直接执行
        actions.sql_to_execute = sql_statements;
        actions.message = Some(format!(
            "将执行 {} 条 SQL 语句{}{}",
            actions.sql_to_execute.len(),
            null_hint,
            validation_hint
        ));
    }
}

/// 把逐列校验问题汇总成一条人类可读提示（去重 + 截断），无问题时返回空串。
fn build_validation_hint(issues: &[(String, CellIssue)]) -> String {
    if issues.is_empty() {
        return String::new();
    }
    // 去重（同列同问题只报一次），保持出现顺序。
    let mut seen: Vec<(String, CellIssue)> = Vec::new();
    for item in issues {
        if !seen.iter().any(|s| s.0 == item.0 && s.1 == item.1) {
            seen.push(item.clone());
        }
    }
    let shown = seen.len().min(5);
    let mut parts: Vec<String> = seen
        .iter()
        .take(shown)
        .map(|(col, issue)| match issue {
            CellIssue::NotNull => format!("{} 列不可为空", col),
            CellIssue::TypeMismatch => format!("{} 列类型可能不匹配", col),
        })
        .collect();
    if seen.len() > shown {
        parts.push(format!("等 {} 处", seen.len()));
    }
    format!("。请检查:{}", parts.join("；"))
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
    use super::{
        CellIssue, DataGridActions, cancel_pending_sql, confirm_pending_sql, generate_save_sql,
        validate_cell,
    };
    use crate::data::{ColumnInfo, QueryResult};
    use crate::ui::DataGridState;

    fn col(name: &str, data_type: &str, nullable: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.to_string(),
            data_type: data_type.to_string(),
            is_primary_key: false,
            is_nullable: nullable,
            default_value: None,
        }
    }

    #[test]
    fn validate_cell_flags_not_null_and_type_mismatch() {
        // 非空列存空 → NotNull
        assert_eq!(
            validate_cell("", &col("name", "TEXT", false)),
            Some(CellIssue::NotNull)
        );
        // 可空列存空 → 放行
        assert_eq!(validate_cell("", &col("note", "TEXT", true)), None);
        // 整数列存文本 → TypeMismatch
        assert_eq!(
            validate_cell("abc", &col("age", "INTEGER", true)),
            Some(CellIssue::TypeMismatch)
        );
        // 整数列存整数 → 放行
        assert_eq!(validate_cell("42", &col("age", "INTEGER", true)), None);
        // 浮点列
        assert_eq!(validate_cell("3.14", &col("p", "REAL", true)), None);
        assert_eq!(
            validate_cell("x", &col("p", "DOUBLE", true)),
            Some(CellIssue::TypeMismatch)
        );
        // 布尔列
        assert_eq!(validate_cell("true", &col("b", "BOOLEAN", true)), None);
        assert_eq!(
            validate_cell("maybe", &col("b", "BOOLEAN", true)),
            Some(CellIssue::TypeMismatch)
        );
        // 文本/未知类型 → 不做类型校验
        assert_eq!(validate_cell("anything", &col("t", "TEXT", true)), None);
        assert_eq!(validate_cell("2026-01-01", &col("d", "DATE", true)), None);
    }

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
    fn generate_save_sql_reports_validation_issues_with_metadata() {
        // 审计 G6：有列元数据时，违规单元格使保存提示逐列指出问题。
        // 表 (id INTEGER, age INTEGER NOT NULL)
        let result = QueryResult::with_rows(
            vec!["id".to_string(), "age".to_string()],
            vec![vec!["1".to_string(), "30".to_string()]],
        );
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();
        state.column_metadata = vec![col("id", "INTEGER", false), col("age", "INTEGER", false)];
        // 把 age 改成文本 → TypeMismatch。
        state.modified_cells.insert((0, 1), "abc".to_string());

        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        let msg = actions.message.expect("应有保存提示");
        assert!(msg.contains("age"), "提示应指出 age 列: {msg}");
        assert!(msg.contains("类型"), "提示应说明类型问题: {msg}");
        // SQL 仍生成（warn-but-allow）。
        assert_eq!(actions.sql_to_execute.len(), 1);
    }

    #[test]
    fn generate_save_sql_no_validation_hint_without_metadata() {
        // 无列元数据时行为与旧版一致（回归）。
        let result = sample_result();
        let mut state = DataGridState::default();
        let mut actions = DataGridActions::default();
        // column_metadata 默认空。
        state.modified_cells.insert((0, 1), "bob".to_string());

        generate_save_sql(&result, &mut state, "users", &mut actions, None);

        let msg = actions.message.expect("应有保存提示");
        assert!(!msg.contains("请检查"), "无元数据不应出现校验提示: {msg}");
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
