//! PostgreSQL 查询实现

use crate::database::{ConnectionConfig, DbError, QueryResult, DatabaseType, POOL_MANAGER};
use super::{query_result, exec_result, empty_result, is_query_statement, TriggerInfo, ForeignKeyInfo, ColumnInfo, RoutineInfo, RoutineType};

/// 获取 PostgreSQL 数据库列表
pub async fn get_databases(config: &ConnectionConfig) -> Result<Vec<String>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    let rows = client
        .query(
            "SELECT datname FROM pg_database WHERE datistemplate = false ORDER BY datname",
            &[],
        )
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get(0)).collect())
}

/// 获取 PostgreSQL 指定数据库的表列表
pub async fn get_tables(config: &ConnectionConfig, database: &str) -> Result<Vec<String>, DbError> {
    // 创建一个临时配置，连接到指定数据库
    let mut db_config = config.clone();
    db_config.database = database.to_string();
    
    let client = POOL_MANAGER.get_pg_client(&db_config).await?;

    let rows = client
        .query(
            "SELECT tablename FROM pg_tables WHERE schemaname = 'public' ORDER BY tablename",
            &[],
        )
        .await
        .map_err(|e| DbError::Query(e.to_string()))?;

    Ok(rows.iter().map(|r| r.get(0)).collect())
}

/// 获取 PostgreSQL 表的主键列名
pub async fn get_primary_key(config: &ConnectionConfig, table: &str) -> Result<Option<String>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;
    
    // 查询 information_schema 获取主键列
    let rows = client
        .query(
            "SELECT a.attname
             FROM pg_index i
             JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey)
             WHERE i.indrelid = $1::regclass
             AND i.indisprimary
             LIMIT 1",
            &[&table],
        )
        .await
        .map_err(|e| DbError::Query(format!("查询主键失败: {}", e)))?;
    
    Ok(rows.first().map(|r| r.get(0)))
}

/// 执行 PostgreSQL 查询
pub async fn execute(config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    if is_query_statement(sql, &DatabaseType::PostgreSQL) {
        let rows = client
            .query(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;

        if rows.is_empty() {
            return Ok(empty_result());
        }

        let columns: Vec<String> = rows[0]
            .columns()
            .iter()
            .map(|c| c.name().to_owned())
            .collect();

        let data: Vec<Vec<String>> = rows
            .iter()
            .map(|row| row_to_strings(row, columns.len()))
            .collect();

        Ok(query_result(columns, data))
    } else {
        let affected = client
            .execute(sql, &[])
            .await
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(exec_result(affected))
    }
}

/// 将 PostgreSQL 行转换为字符串向量
fn row_to_strings(row: &tokio_postgres::Row, col_count: usize) -> Vec<String> {
    (0..col_count)
        .map(|i| {
            // 尝试多种类型转换
            row.try_get::<_, String>(i)
                .or_else(|_| row.try_get::<_, i64>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, i32>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, i16>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, f64>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, f32>(i).map(|v| v.to_string()))
                .or_else(|_| row.try_get::<_, bool>(i).map(|v| v.to_string()))
                .or_else(|_| {
                    row.try_get::<_, chrono::NaiveDateTime>(i)
                        .map(|v| v.format("%Y-%m-%d %H:%M:%S").to_string())
                })
                .or_else(|_| {
                    row.try_get::<_, chrono::NaiveDate>(i)
                        .map(|v| v.format("%Y-%m-%d").to_string())
                })
                .or_else(|_| {
                    row.try_get::<_, chrono::NaiveTime>(i)
                        .map(|v| v.format("%H:%M:%S").to_string())
                })
                .unwrap_or_else(|_| String::from("NULL"))
        })
        .collect()
}

/// 获取 PostgreSQL 触发器
pub async fn get_triggers(config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    let sql = r#"
        SELECT 
            t.tgname AS trigger_name,
            c.relname AS table_name,
            CASE 
                WHEN t.tgtype & 2 = 2 THEN 'BEFORE'
                WHEN t.tgtype & 64 = 64 THEN 'INSTEAD OF'
                ELSE 'AFTER'
            END AS timing,
            CASE 
                WHEN t.tgtype & 4 = 4 THEN 'INSERT'
                WHEN t.tgtype & 8 = 8 THEN 'DELETE'
                WHEN t.tgtype & 16 = 16 THEN 'UPDATE'
                ELSE 'UNKNOWN'
            END AS event,
            pg_get_triggerdef(t.oid) AS definition
        FROM pg_trigger t
        JOIN pg_class c ON t.tgrelid = c.oid
        JOIN pg_namespace n ON c.relnamespace = n.oid
        WHERE NOT t.tgisinternal
          AND n.nspname = 'public'
        ORDER BY t.tgname
    "#;

    let rows = client
        .query(sql, &[])
        .await
        .map_err(|e| DbError::Query(format!("查询触发器失败: {}", e)))?;

    let triggers: Vec<TriggerInfo> = rows
        .iter()
        .map(|row| TriggerInfo {
            name: row.get(0),
            table_name: row.get(1),
            timing: row.get(2),
            event: row.get(3),
            definition: row.get(4),
        })
        .collect();

    Ok(triggers)
}

/// 获取 PostgreSQL 外键
pub async fn get_foreign_keys(config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    let sql = r#"
        SELECT 
            kcu.table_name AS from_table,
            kcu.column_name AS from_column,
            ccu.table_name AS to_table,
            ccu.column_name AS to_column
        FROM information_schema.key_column_usage kcu
        JOIN information_schema.referential_constraints rc 
            ON kcu.constraint_name = rc.constraint_name
            AND kcu.table_schema = rc.constraint_schema
        JOIN information_schema.constraint_column_usage ccu 
            ON rc.unique_constraint_name = ccu.constraint_name
            AND rc.unique_constraint_schema = ccu.table_schema
        WHERE kcu.table_schema = 'public'
        ORDER BY kcu.table_name, kcu.column_name
    "#;

    let rows = client
        .query(sql, &[])
        .await
        .map_err(|e| DbError::Query(format!("查询外键失败: {}", e)))?;

    let foreign_keys: Vec<ForeignKeyInfo> = rows
        .iter()
        .map(|row| ForeignKeyInfo {
            from_table: row.get(0),
            from_column: row.get(1),
            to_table: row.get(2),
            to_column: row.get(3),
        })
        .collect();

    Ok(foreign_keys)
}

/// 获取 PostgreSQL 表的列信息
pub async fn get_columns(config: &ConnectionConfig, table: &str) -> Result<Vec<ColumnInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    let sql = r#"
        SELECT 
            c.column_name,
            c.data_type,
            CASE WHEN pk.column_name IS NOT NULL THEN true ELSE false END AS is_primary_key,
            c.is_nullable = 'YES' AS is_nullable,
            c.column_default
        FROM information_schema.columns c
        LEFT JOIN (
            SELECT kcu.column_name
            FROM information_schema.table_constraints tc
            JOIN information_schema.key_column_usage kcu 
                ON tc.constraint_name = kcu.constraint_name
                AND tc.table_schema = kcu.table_schema
            WHERE tc.constraint_type = 'PRIMARY KEY'
              AND tc.table_name = $1
              AND tc.table_schema = 'public'
        ) pk ON c.column_name = pk.column_name
        WHERE c.table_name = $1
          AND c.table_schema = 'public'
        ORDER BY c.ordinal_position
    "#;

    let rows = client
        .query(sql, &[&table])
        .await
        .map_err(|e| DbError::Query(format!("查询列信息失败: {}", e)))?;

    let columns: Vec<ColumnInfo> = rows
        .iter()
        .map(|row| ColumnInfo {
            name: row.get(0),
            data_type: row.get(1),
            is_primary_key: row.get(2),
            is_nullable: row.get(3),
            default_value: row.get(4),
        })
        .collect();

    Ok(columns)
}

/// 获取 PostgreSQL 存储过程和函数
pub async fn get_routines(config: &ConnectionConfig) -> Result<Vec<RoutineInfo>, DbError> {
    let client = POOL_MANAGER.get_pg_client(config).await?;

    // 查询用户定义的函数和存储过程
    // prokind: 'f' = function, 'p' = procedure, 'a' = aggregate, 'w' = window
    let sql = r#"
        SELECT 
            p.proname AS name,
            CASE p.prokind 
                WHEN 'p' THEN 'PROCEDURE'
                ELSE 'FUNCTION'
            END AS routine_type,
            pg_get_function_arguments(p.oid) AS parameters,
            CASE WHEN p.prokind != 'p' THEN
                pg_catalog.format_type(p.prorettype, NULL)
            ELSE NULL END AS return_type,
            pg_get_functiondef(p.oid) AS definition
        FROM pg_proc p
        JOIN pg_namespace n ON p.pronamespace = n.oid
        WHERE n.nspname = 'public'
          AND p.prokind IN ('f', 'p')
        ORDER BY 
            CASE p.prokind WHEN 'p' THEN 0 ELSE 1 END,
            p.proname
    "#;

    let rows = client
        .query(sql, &[])
        .await
        .map_err(|e| DbError::Query(format!("查询存储过程失败: {}", e)))?;

    let routines: Vec<RoutineInfo> = rows
        .iter()
        .map(|row| {
            let name: String = row.get(0);
            let type_str: String = row.get(1);
            let parameters: String = row.get(2);
            let return_type: Option<String> = row.get(3);
            let definition: String = row.get(4);

            let routine_type = if type_str == "PROCEDURE" {
                RoutineType::Procedure
            } else {
                RoutineType::Function
            };

            RoutineInfo {
                name,
                routine_type,
                parameters,
                return_type,
                definition,
            }
        })
        .collect();

    Ok(routines)
}
