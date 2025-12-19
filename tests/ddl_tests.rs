//! DDL 对话框测试
//!
//! 测试表定义、列类型转换等

use gridix::database::DatabaseType;
use gridix::ui::{ColumnDefinition, ColumnType, TableDefinition};

#[test]
fn test_column_type_sql() {
    let col_type = ColumnType::Varchar(255);
    assert_eq!(col_type.to_sql(&DatabaseType::MySQL), "VARCHAR(255)");
    assert_eq!(col_type.to_sql(&DatabaseType::PostgreSQL), "VARCHAR(255)");
    assert_eq!(col_type.to_sql(&DatabaseType::SQLite), "TEXT");
}

#[test]
fn test_column_definition() {
    let col = ColumnDefinition {
        name: "id".to_string(),
        data_type: ColumnType::Integer,
        primary_key: true,
        auto_increment: true,
        nullable: false,
        ..Default::default()
    };

    let sql = col.to_sql(&DatabaseType::MySQL);
    assert!(sql.contains("PRIMARY KEY"));
    assert!(sql.contains("AUTO_INCREMENT"));
}

#[test]
fn test_table_definition() {
    let mut table = TableDefinition::new(DatabaseType::MySQL);
    table.name = "users".to_string();
    table.columns.push(ColumnDefinition {
        name: "id".to_string(),
        data_type: ColumnType::Integer,
        primary_key: true,
        auto_increment: true,
        nullable: false,
        ..Default::default()
    });
    table.columns.push(ColumnDefinition {
        name: "name".to_string(),
        data_type: ColumnType::Varchar(100),
        nullable: false,
        ..Default::default()
    });

    let sql = table.to_create_sql();
    assert!(sql.contains("CREATE TABLE"));
    assert!(sql.contains("`users`"));
    assert!(sql.contains("`id`"));
    assert!(sql.contains("`name`"));
}

#[test]
fn test_table_validation() {
    let table = TableDefinition::default();
    assert!(table.validate().is_err());

    let mut table = TableDefinition::new(DatabaseType::MySQL);
    table.name = "test".to_string();
    table.columns.push(ColumnDefinition {
        name: "id".to_string(),
        ..Default::default()
    });
    assert!(table.validate().is_ok());
}
