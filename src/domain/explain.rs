//! Explain 查询的最小领域状态。
//!
//! Explain 输出仍然使用通用 [`ResultSet`]，这里只负责识别 Explain SQL
//! 以及保存最近一次 Explain 的结果或错误；不引入跨数据库执行计划 AST。

use std::sync::Arc;

use super::result::ResultSet;

/// 当前工作区最近一次 Explain 执行的状态。
#[derive(Debug, Clone, Default)]
pub struct ExplainState {
    /// 产生该状态的查询 Tab。
    pub query_tab_id: Option<String>,
    /// 实际发送到数据库的 Explain SQL。
    pub sql: Option<String>,
    /// 最近一次成功返回的结果集。
    pub result: Option<Arc<ResultSet>>,
    /// 最近一次 Explain 错误。
    pub error: Option<String>,
    /// 最近一次 Explain 的耗时。
    pub elapsed_ms: Option<u64>,
    /// 是否仍在等待数据库回包。
    pub is_running: bool,
    /// Explain surface 是否应作为当前 Tab 的最近输出显示。
    pub visible: bool,
}

impl ExplainState {
    /// 标记一次新的 Explain 执行。
    pub fn begin(&mut self, query_tab_id: String, sql: String) {
        self.query_tab_id = Some(query_tab_id);
        self.sql = Some(sql);
        self.result = None;
        self.error = None;
        self.elapsed_ms = None;
        self.is_running = true;
        self.visible = true;
    }

    /// 保存 Explain 成功结果。
    pub fn record_success(
        &mut self,
        query_tab_id: String,
        sql: String,
        result: Option<Arc<ResultSet>>,
        elapsed_ms: u64,
    ) {
        self.query_tab_id = Some(query_tab_id);
        self.sql = Some(sql);
        self.result = result;
        self.error = None;
        self.elapsed_ms = Some(elapsed_ms);
        self.is_running = false;
        self.visible = true;
    }

    /// 保存 Explain 错误。
    pub fn record_error(
        &mut self,
        query_tab_id: String,
        sql: String,
        error: String,
        elapsed_ms: u64,
    ) {
        self.query_tab_id = Some(query_tab_id);
        self.sql = Some(sql);
        self.result = None;
        self.error = Some(error);
        self.elapsed_ms = Some(elapsed_ms);
        self.is_running = false;
        self.visible = true;
    }

    /// 普通查询开始后隐藏 Explain surface，但保留最近的 Explain 数据。
    pub fn hide_for_tab(&mut self, query_tab_id: &str) {
        if self.query_tab_id.as_deref() == Some(query_tab_id) {
            self.is_running = false;
            self.visible = false;
        }
    }

    /// 判断该 Explain 状态是否属于指定 Tab 且应被显示。
    pub fn should_show_for_tab(&self, query_tab_id: &str) -> bool {
        self.visible && self.query_tab_id.as_deref() == Some(query_tab_id)
    }
}

/// 判断 SQL 的首个有效关键字是否为 `EXPLAIN`。
///
/// 前导空白、行注释和块注释会被忽略，大小写不敏感；字符串中的
/// `EXPLAIN` 不会被误判。具体计划格式仍由各数据库后端决定。
pub fn is_explain_sql(sql: &str) -> bool {
    let mut index = skip_leading_ws_and_comments(sql, 0);
    read_keyword(sql, &mut index).is_some_and(|keyword| keyword.eq_ignore_ascii_case("explain"))
}

fn skip_leading_ws_and_comments(sql: &str, mut index: usize) -> usize {
    let bytes = sql.as_bytes();
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if index + 1 < bytes.len() && bytes[index] == b'-' && bytes[index + 1] == b'-' {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if index < bytes.len() && bytes[index] == b'#' {
            index += 1;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if index + 1 < bytes.len() && bytes[index] == b'/' && bytes[index + 1] == b'*' {
            index += 2;
            while index + 1 < bytes.len() {
                if bytes[index] == b'*' && bytes[index + 1] == b'/' {
                    index += 2;
                    break;
                }
                index += 1;
            }
            continue;
        }
        return index;
    }
}

fn read_keyword<'a>(sql: &'a str, index: &mut usize) -> Option<&'a str> {
    let start = *index;
    let bytes = sql.as_bytes();
    while *index < bytes.len() && bytes[*index].is_ascii_alphabetic() {
        *index += 1;
    }
    (*index > start).then(|| &sql[start..*index])
}

#[cfg(test)]
mod tests {
    use super::is_explain_sql;

    #[test]
    fn explain_classifier_accepts_comments_and_options() {
        assert!(is_explain_sql(" /* hint */\n explain (analyze) SELECT 1"));
        assert!(is_explain_sql("EXPLAIN QUERY PLAN SELECT 1"));
        assert!(is_explain_sql("# mysql comment\nEXPLAIN SELECT 1"));
        assert!(is_explain_sql("-- note\n# note\nEXPLAIN SELECT 1"));
    }

    #[test]
    fn explain_classifier_rejects_non_explain_statements() {
        assert!(!is_explain_sql("SELECT 'EXPLAIN'"));
        assert!(!is_explain_sql(
            "WITH explain AS (SELECT 1) SELECT * FROM explain"
        ));
        assert!(!is_explain_sql(""));
    }
}
