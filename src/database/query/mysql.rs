//! MySQL 查询实现

use mysql_async::prelude::*;
use crate::database::{ConnectionConfig, DbError, QueryResult, DatabaseType, POOL_MANAGER};
use super::{query_result, exec_result, empty_result, is_query_statement, TriggerInfo, ForeignKeyInfo, ColumnInfo, RoutineInfo, RoutineType};

/// 获取 MySQL 数据库列表
pub async fn get_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let databases: Vec<String> = conn
        .query("SHOW DATABASES")
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    // 过滤系统数据库
    Ok(databases
        .into_iter()
        .filter(|db| !matches!(db.as_str(), "information_schema" | "mysql" | "performance_schema" | "sys"))
        .collect())
}

/// 获取 MySQL 指定数据库的表列表
pub async fn get_tables(config: &ConnectionConfig, database: &str) -> Result<Vec<String>, DbError> {
    // 创建一个临时配置，连接到指定数据库
    let mut db_config = config.clone();
    db_config.database = database.to_string();
    
    let pool = POOL_MANAGER.get_mysql_pool(&db_config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let tables: Vec<String> = conn
        .query("SHOW TABLES")
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(tables)
}

/// 获取 MySQL 表的主键列名
pub async fn get_primary_key(config: &ConnectionConfig, table: &str) -> Result<Option<String>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;
    
    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;
    
    // 使用 SHOW KEYS 查询主键列
    let escaped_table = table.replace('`', "``").replace('.', "_");
    let sql = format!(
        "SHOW KEYS FROM `{}` WHERE Key_name = 'PRIMARY'",
        escaped_table
    );
    
    let result: Vec<mysql_async::Row> = conn
        .query(&sql)
        .await
        .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;
    
    // Column_name 是第 5 列（索引 4）
    if let Some(row) = result.first() {
        let col_name: Option<String> = row.get(4);
        return Ok(col_name);
    }
    
    Ok(None)
}

/// 执行 MySQL 查询
pub async fn execute(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    if is_query_statement(sql, &DatabaseType::MySQL) {
        let result: Vec<mysql_async::Row> = conn
            .query(sql)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        if result.is_empty() {
            return Ok(empty_result());
        }

        let columns: Vec<String> = result[0]
            .columns_ref()
            .iter()
            .map(|c| c.name_str().into_owned())
            .collect();

        let data: Vec<Vec<String>> = result
            .iter()
            .map(|row| row_to_strings(row, columns.len()))
            .collect();

        Ok(query_result(columns, data))
    } else {
        // 使用 query_iter 来获取影响行数
        let result = conn
            .query_iter(sql)
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        let affected = result.affected_rows();
        // 需要消耗结果
        drop(result);

        Ok(exec_result(affected))
    }
}

/// 将 MySQL 行转换为字符串向量
fn row_to_strings(row: &mysql_async::Row, col_count: usize) -> Vec<String> {
    (0..col_count)
        .map(|i| {
            row.get::<mysql_async::Value, _>(i)
                .map(value_to_string)
                .unwrap_or_else(|| String::from("NULL"))
        })
        .collect()
}

/// 将 MySQL Value 转换为字符串
fn value_to_string(val: mysql_async::Value) -> String {
    use mysql_async::Value;
    match val {
        Value::NULL => String::from("NULL"),
        Value::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
        Value::Int(i) => i.to_string(),
        Value::UInt(u) => u.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Double(d) => d.to_string(),
        Value::Date(y, m, d, h, mi, s, us) => {
            if h == 0 && mi == 0 && s == 0 && us == 0 {
                format!("{:04}-{:02}-{:02}", y, m, d)
            } else if us == 0 {
                format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, h, mi, s)
            } else {
                format!(
                    "{:04}-{:02}-{:02} {:02}:{:02}:{:02}.{:06}",
                    y, m, d, h, mi, s, us
                )
            }
        }
        Value::Time(neg, d, h, m, s, us) => {
            let sign = if neg { "-" } else { "" };
            if d > 0 {
                format!("{}{}d {:02}:{:02}:{:02}", sign, d, h, m, s)
            } else if us > 0 {
                format!("{}{:02}:{:02}:{:02}.{:06}", sign, h, m, s, us)
            } else {
                format!("{}{:02}:{:02}:{:02}", sign, h, m, s)
            }
        }
    }
}

/// 获取 MySQL 触发器
pub async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = r#"
        SELECT 
            TRIGGER_NAME,
            EVENT_OBJECT_TABLE,
            ACTION_TIMING,
            EVENT_MANIPULATION,
            ACTION_STATEMENT
        FROM INFORMATION_SCHEMA.TRIGGERS
        WHERE TRIGGER_SCHEMA = DATABASE()
        ORDER BY TRIGGER_NAME
    "#;

    let result: Vec<mysql_async::Row> = conn
        .query(sql)
        .await
        .map_err(|e| DbError::Query(format!("查询触发器失败: {}", e)))?;

    let triggers: Vec<TriggerInfo> = result
        .iter()
        .map(|row| {
            let name: String = row.get(0).unwrap_or_default();
            let table_name: String = row.get(1).unwrap_or_default();
            let timing: String = row.get(2).unwrap_or_default();
            let event: String = row.get(3).unwrap_or_default();
            let action: String = row.get(4).unwrap_or_default();

            // 构造完整的触发器定义
            let definition = format!(
                "CREATE TRIGGER {} {} {} ON {} FOR EACH ROW {}",
                name, timing, event, table_name, action
            );

            TriggerInfo {
                name,
                table_name,
                event,
                timing,
                definition,
            }
        })
        .collect();

    Ok(triggers)
}

/// 获取 MySQL 外键
pub async fn get_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = r#"
        SELECT 
            TABLE_NAME,
            COLUMN_NAME,
            REFERENCED_TABLE_NAME,
            REFERENCED_COLUMN_NAME
        FROM INFORMATION_SCHEMA.KEY_COLUMN_USAGE
        WHERE TABLE_SCHEMA = DATABASE()
          AND REFERENCED_TABLE_NAME IS NOT NULL
        ORDER BY TABLE_NAME, COLUMN_NAME
    "#;

    let result: Vec<mysql_async::Row> = conn
        .query(sql)
        .await
        .map_err(|e| DbError::Query(format!("查询外键失败: {}", e)))?;

    let foreign_keys: Vec<ForeignKeyInfo> = result
        .iter()
        .map(|row| ForeignKeyInfo {
            from_table: row.get(0).unwrap_or_default(),
            from_column: row.get(1).unwrap_or_default(),
            to_table: row.get(2).unwrap_or_default(),
            to_column: row.get(3).unwrap_or_default(),
        })
        .collect();

    Ok(foreign_keys)
}

/// 获取 MySQL 表的列信息
pub async fn get_columns(config: &ConnectionConfig, table: &str) -> Result<Vec<ColumnInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = format!(
        r#"
        SELECT 
            c.COLUMN_NAME,
            c.DATA_TYPE,
            CASE WHEN c.COLUMN_KEY = 'PRI' THEN 1 ELSE 0 END AS is_primary_key,
            CASE WHEN c.IS_NULLABLE = 'YES' THEN 1 ELSE 0 END AS is_nullable,
            c.COLUMN_DEFAULT
        FROM INFORMATION_SCHEMA.COLUMNS c
        WHERE c.TABLE_SCHEMA = DATABASE()
          AND c.TABLE_NAME = '{}'
        ORDER BY c.ORDINAL_POSITION
        "#,
        table.replace('\'', "''")
    );

    let result: Vec<mysql_async::Row> = conn
        .query(&sql)
        .await
        .map_err(|e| DbError::Query(format!("查询列信息失败: {}", e)))?;

    let columns: Vec<ColumnInfo> = result
        .iter()
        .map(|row| {
            let is_pk: i32 = row.get(2).unwrap_or(0);
            let is_null: i32 = row.get(3).unwrap_or(0);
            let default_val: Option<String> = row.get(4).unwrap_or(None);
            ColumnInfo {
                name: row.get(0).unwrap_or_default(),
                data_type: row.get(1).unwrap_or_default(),
                is_primary_key: is_pk == 1,
                is_nullable: is_null == 1,
                default_value: default_val,
            }
        })
        .collect();

    Ok(columns)
}

/// 获取 MySQL 存储过程和函数
pub async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
    let pool = POOL_MANAGER.get_mysql_pool(config).await?;

    let mut conn = pool
        .get_conn()
        .await
        .map_err(|e| DbError::Connection(format!("MySQL 获取连接失败: {}", e)))?;

    let sql = r#"
        SELECT 
            ROUTINE_NAME,
            ROUTINE_TYPE,
            ROUTINE_DEFINITION,
            DTD_IDENTIFIER
        FROM INFORMATION_SCHEMA.ROUTINES
        WHERE ROUTINE_SCHEMA = DATABASE()
        ORDER BY ROUTINE_TYPE, ROUTINE_NAME
    "#;

    let result: Vec<mysql_async::Row> = conn
        .query(sql)
        .await
        .map_err(|e| DbError::Query(format!("查询存储过程失败: {}", e)))?;

    // 获取参数信息
    let params_sql = r#"
        SELECT 
            SPECIFIC_NAME,
            PARAMETER_MODE,
            PARAMETER_NAME,
            DATA_TYPE
        FROM INFORMATION_SCHEMA.PARAMETERS
        WHERE SPECIFIC_SCHEMA = DATABASE()
        ORDER BY SPECIFIC_NAME, ORDINAL_POSITION
    "#;

    let params_result: Vec<mysql_async::Row> = conn
        .query(params_sql)
        .await
        .unwrap_or_default();

    // 构建参数映射
    let mut params_map: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for row in &params_result {
        let routine_name: String = row.get(0).unwrap_or_default();
        let mode: Option<String> = row.get(1).unwrap_or(None);
        let param_name: Option<String> = row.get(2).unwrap_or(None);
        let data_type: String = row.get(3).unwrap_or_default();
        
        // 跳过返回值参数（PARAMETER_NAME 为 NULL 且 PARAMETER_MODE 为 NULL）
        if let Some(name) = param_name {
            let param_str = if let Some(m) = mode {
                format!("{} {} {}", m, name, data_type)
            } else {
                format!("{} {}", name, data_type)
            };
            params_map.entry(routine_name).or_default().push(param_str);
        }
    }

    let routines: Vec<RoutineInfo> = result
        .iter()
        .map(|row| {
            let name: String = row.get(0).unwrap_or_default();
            let type_str: String = row.get(1).unwrap_or_default();
            let definition: Option<String> = row.get(2).unwrap_or(None);
            let return_type: Option<String> = row.get(3).unwrap_or(None);

            let routine_type = if type_str == "FUNCTION" {
                RoutineType::Function
            } else {
                RoutineType::Procedure
            };

            let parameters = params_map
                .get(&name)
                .map(|p| p.join(", "))
                .unwrap_or_default();

            RoutineInfo {
                name,
                routine_type,
                parameters,
                return_type,
                definition: definition.unwrap_or_else(|| "(定义不可见)".to_string()),
            }
        })
        .collect();

    Ok(routines)
}
