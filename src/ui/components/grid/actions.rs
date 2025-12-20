//! 表格操作和 SQL 生成

use super::state::DataGridState;
use crate::database::QueryResult;

/// 焦点转移方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTransfer {
    /// 转移到侧边栏
    ToSidebar,
    /// 转移到 SQL 编辑器
    ToSqlEditor,
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
}

/// SQL 危险保留字（可能被用于注入攻击）
const SQL_DANGEROUS_KEYWORDS: &[&str] = &[
    "DROP", "DELETE", "TRUNCATE", "ALTER", "CREATE", "INSERT", "UPDATE",
    "EXEC", "EXECUTE", "UNION", "SELECT", "FROM", "WHERE", "OR", "AND",
    "--", "/*", "*/", "GRANT", "REVOKE", "SHUTDOWN", "KILL",
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
    if value == "NULL" {
        return "NULL".to_string();
    }
    // 转义单引号为两个单引号
    format!("'{}'", value.replace('\'', "''"))
}

/// 生成保存修改的 SQL（带确认）
pub fn generate_save_sql(
    result: &QueryResult,
    state: &mut DataGridState,
    table_name: &str,
    actions: &mut DataGridActions,
) {
    // 验证表名
    let safe_table_name = match escape_identifier(table_name) {
        Ok(name) => name,
        Err(e) => {
            actions.message = Some(format!("表名无效: {}", e));
            return;
        }
    };

    // 验证所有列名
    let mut safe_columns: Vec<String> = Vec::new();
    for col in &result.columns {
        match escape_identifier(col) {
            Ok(name) => safe_columns.push(name),
            Err(e) => {
                actions.message = Some(format!("列名无效: {}", e));
                return;
            }
        }
    }

    let mut sql_statements = Vec::new();
    let has_deletes = !state.rows_to_delete.is_empty();

    // 获取主键列索引
    // 优先使用已设置的主键，其次尝试查找 "id" 列
    let pk_idx = match state.primary_key_column {
        Some(idx) => idx,
        None => {
            // 尝试查找名为 "id" 的列
            match result.columns.iter().position(|c| c.eq_ignore_ascii_case("id")) {
                Some(idx) => idx,
                None => {
                    // 无法确定主键，拒绝执行修改操作以防止数据损坏
                    actions.message = Some(
                        "无法确定主键列：未找到 'id' 列。\n\
                         请确保表有主键列，或使用自定义 SQL 进行修改。".to_string()
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
        if let Some(row) = result.rows.get(*row_idx) {
            if let Some(pk_value) = row.get(pk_idx) {
                if let Some(col_name) = safe_columns.get(*col_idx) {
                    let safe_value = if new_value.is_empty() || new_value.eq_ignore_ascii_case("null") {
                        "NULL".to_string()
                    } else {
                        escape_value(new_value)
                    };
                    let safe_pk_value = escape_value(pk_value);

                    let sql = format!(
                        "UPDATE {} SET {} = {} WHERE {} = {};",
                        safe_table_name, col_name, safe_value, pk_col, safe_pk_value
                    );
                    sql_statements.push(sql);
                }
            }
        }
    }

    // 生成 DELETE 语句
    for row_idx in &state.rows_to_delete {
        if let Some(row) = result.rows.get(*row_idx) {
            if let Some(pk_value) = row.get(pk_idx) {
                let safe_pk_value = escape_value(pk_value);
                let sql = format!(
                    "DELETE FROM {} WHERE {} = {};",
                    safe_table_name, pk_col, safe_pk_value
                );
                sql_statements.push(sql);
            }
        }
    }

    // 生成 INSERT 语句
    for new_row in &state.new_rows {
        if new_row.iter().any(|v| !v.is_empty()) {
            let cols = safe_columns.join(", ");
            let vals: Vec<String> = new_row
                .iter()
                .map(|v| {
                    if v.is_empty() || v.eq_ignore_ascii_case("null") {
                        "NULL".to_string()
                    } else {
                        escape_value(v)
                    }
                })
                .collect();
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

    // 如果包含删除操作，需要确认
    if has_deletes {
        state.pending_sql = sql_statements;
        state.show_save_confirm = true;
        actions.message = Some(format!(
            "包含 {} 条删除操作，请确认",
            state.rows_to_delete.len()
        ));
    } else {
        // 没有删除操作，直接执行
        actions.sql_to_execute = sql_statements;
        actions.message = Some(format!(
            "将执行 {} 条 SQL 语句",
            actions.sql_to_execute.len()
        ));
        state.clear_edits();
    }
}

/// 确认执行待确认的 SQL
pub fn confirm_pending_sql(state: &mut DataGridState, actions: &mut DataGridActions) {
    if !state.pending_sql.is_empty() {
        actions.sql_to_execute = std::mem::take(&mut state.pending_sql);
        actions.message = Some(format!("执行 {} 条 SQL 语句", actions.sql_to_execute.len()));
        state.clear_edits();
    }
    state.show_save_confirm = false;
}

/// 取消待确认的 SQL
pub fn cancel_pending_sql(state: &mut DataGridState) {
    state.pending_sql.clear();
    state.show_save_confirm = false;
}

