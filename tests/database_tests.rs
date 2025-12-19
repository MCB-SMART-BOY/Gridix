//! 数据库模块集成测试
//!
//! 测试 SQLite、PostgreSQL、MySQL 的连接和查询功能。

use gridix::database::{ConnectionConfig, DatabaseType, QueryResult};

/// 创建 SQLite 测试配置
fn sqlite_test_config() -> ConnectionConfig {
    ConnectionConfig {
        name: "test_sqlite".to_string(),
        db_type: DatabaseType::SQLite,
        database: ":memory:".to_string(),
        ..Default::default()
    }
}

#[cfg(test)]
mod sqlite_tests {
    use super::*;

    #[test]
    fn test_sqlite_config_creation() {
        let config = sqlite_test_config();
        assert_eq!(config.name, "test_sqlite");
        assert_eq!(config.db_type, DatabaseType::SQLite);
        assert_eq!(config.database, ":memory:");
    }

    #[test]
    fn test_connection_string() {
        let config = sqlite_test_config();
        assert_eq!(config.connection_string(), ":memory:");
    }

    #[test]
    fn test_pool_key() {
        let config = sqlite_test_config();
        let key = config.pool_key();
        assert!(key.starts_with("sqlite:"));
    }
}

#[cfg(test)]
mod postgresql_tests {
    use super::*;

    fn pg_test_config() -> ConnectionConfig {
        ConnectionConfig {
            name: "test_pg".to_string(),
            db_type: DatabaseType::PostgreSQL,
            host: "localhost".to_string(),
            port: 5432,
            username: "test".to_string(),
            password: "test".to_string(),
            database: "testdb".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_pg_config_creation() {
        let config = pg_test_config();
        assert_eq!(config.db_type, DatabaseType::PostgreSQL);
        assert_eq!(config.port, 5432);
    }

    #[test]
    fn test_pg_connection_string() {
        let config = pg_test_config();
        let conn_str = config.connection_string();
        assert!(conn_str.contains("host=localhost"));
        assert!(conn_str.contains("port=5432"));
        assert!(conn_str.contains("dbname=testdb"));
    }

    #[test]
    fn test_pg_pool_key() {
        let config = pg_test_config();
        let key = config.pool_key();
        assert!(key.starts_with("pg:"));
        assert!(key.contains("localhost"));
    }
}

#[cfg(test)]
mod mysql_tests {
    use super::*;

    fn mysql_test_config() -> ConnectionConfig {
        ConnectionConfig {
            name: "test_mysql".to_string(),
            db_type: DatabaseType::MySQL,
            host: "localhost".to_string(),
            port: 3306,
            username: "root".to_string(),
            password: "password".to_string(),
            database: "testdb".to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_mysql_config_creation() {
        let config = mysql_test_config();
        assert_eq!(config.db_type, DatabaseType::MySQL);
        assert_eq!(config.port, 3306);
    }

    #[test]
    fn test_mysql_connection_string() {
        let config = mysql_test_config();
        let conn_str = config.connection_string();
        assert!(conn_str.starts_with("mysql://"));
        assert!(conn_str.contains("localhost:3306"));
    }

    #[test]
    fn test_mysql_pool_key() {
        let config = mysql_test_config();
        let key = config.pool_key();
        assert!(key.starts_with("mysql:"));
    }
}

#[cfg(test)]
mod query_result_tests {
    use super::*;

    #[test]
    fn test_empty_query_result() {
        let result = QueryResult::default();
        assert!(result.columns.is_empty());
        assert!(result.rows.is_empty());
        assert_eq!(result.affected_rows, 0);
    }

    #[test]
    fn test_query_result_with_data() {
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![
                vec!["1".to_string(), "Alice".to_string()],
                vec!["2".to_string(), "Bob".to_string()],
            ],
            affected_rows: 0,
        };
        
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.rows.len(), 2);
        assert_eq!(result.rows[0][1], "Alice");
    }
}

#[cfg(test)]
mod database_type_tests {
    use super::*;

    #[test]
    fn test_database_type_display_name() {
        assert_eq!(DatabaseType::SQLite.display_name(), "SQLite");
        assert_eq!(DatabaseType::PostgreSQL.display_name(), "PostgreSQL");
        assert_eq!(DatabaseType::MySQL.display_name(), "MySQL");
    }

    #[test]
    fn test_default_ports() {
        assert_eq!(DatabaseType::SQLite.default_port(), 0);
        assert_eq!(DatabaseType::PostgreSQL.default_port(), 5432);
        assert_eq!(DatabaseType::MySQL.default_port(), 3306);
    }

    #[test]
    fn test_all_types() {
        let all = DatabaseType::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&DatabaseType::SQLite));
        assert!(all.contains(&DatabaseType::PostgreSQL));
        assert!(all.contains(&DatabaseType::MySQL));
    }
}
