//! SQL 格式化测试

use gridix::core::format_sql;

#[test]
fn test_simple_select() {
    let sql = "select * from users where id = 1";
    let formatted = format_sql(sql);
    assert!(formatted.contains("SELECT"));
    assert!(formatted.contains("FROM"));
    assert!(formatted.contains("WHERE"));
}

#[test]
fn test_multicolumn_select() {
    let sql = "select id, name, email from users";
    let formatted = format_sql(sql);
    assert!(formatted.contains("SELECT"));
}
