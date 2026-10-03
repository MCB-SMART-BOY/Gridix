//! MySQL 类型化运行时 E2E 集成测试
//!
//! 使用 REAL 代码路径（execute_typed、apply_mutations、load_catalog）
//! 对真实 MySQL 服务器执行端到端验证。无 mocking。
//!
//! 通过 GRIDIX_TEST_MYSQL_URL 环境变量获取连接信息。
//! 格式：mysql://user:password@host:port/database
//!
//! 覆盖：
//! - 复合主键 CRUD 往返
//! - NULL / 空字符串区分
//! - 大结果集与截断语义
//! - DEFAULT 值
//! - Schema 目录加载

use gridix::core::constants;
use gridix::data::{
    ConnectionConfig, DbError, POOL_MANAGER, apply_mutations, execute_import_batch, execute_typed,
    load_schema_catalog,
};
use std::sync::Arc;
mod common;
use common::mysql_config_from_env;

use gridix::domain::execution::{ExecutionOutcome, StatementOutcome};
use gridix::domain::ids::SchemaRevision;
use gridix::domain::mutation::{
    ColumnRef, ExpectedRows, InputValue, Mutation, MutationBatch, RowIdentity,
};
use gridix::domain::result::{ResultCompleteness, ResultSet};
use gridix::domain::value::{DbDate, DbDateTime, DbTime, DbValue};

// ── helpers ──

/// 从 GRIDIX_TEST_MYSQL_URL 环境变量解析 MySQL 连接配置。
fn mysql_config() -> Option<ConnectionConfig> {
    mysql_config_from_env()
}

fn col(name: &str) -> ColumnRef {
    ColumnRef {
        name: name.to_string(),
    }
}

fn pk(cols: Vec<(&str, DbValue)>) -> RowIdentity {
    RowIdentity::PrimaryKey(cols.into_iter().map(|(n, v)| (col(n), v)).collect())
}

/// 从 ExecutionOutcome 中提取单个 ResultSet
fn single_result_set(outcome: ExecutionOutcome) -> ResultSet {
    assert_eq!(
        outcome.statements.len(),
        1,
        "expected single StatementOutcome"
    );
    match &outcome.statements[0] {
        StatementOutcome::ResultSet(rs) => rs.clone(),
        other => panic!("expected ResultSet, got {:?}", other),
    }
}

/// 断言 DDL/DML 执行返回 AffectedRows
fn assert_affected(outcome: ExecutionOutcome, expected_rows: u64) {
    assert_eq!(outcome.statements.len(), 1);
    match &outcome.statements[0] {
        StatementOutcome::AffectedRows { rows } => assert_eq!(*rows, expected_rows),
        other => panic!("expected AffectedRows, got {:?}", other),
    }
}

// ═══════════════════════════════════════════════════════════════════
// Test 1: 复合主键 CRUD 往返
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn typed_select_and_mutation_roundtrip() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    // 1. DROP + CREATE TABLE with composite PK
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS users").await;
    let outcome = execute_typed(
        &config,
        "CREATE TABLE users (\
         tenant_id INT, \
         user_id INT, \
         name TEXT, \
         email TEXT, \
         PRIMARY KEY (tenant_id, user_id))",
    )
    .await
    .unwrap();
    assert_affected(outcome, 0);

    // 2. INSERT via apply_mutations
    let insert_batch = MutationBatch {
        mutations: vec![Mutation::Insert {
            table: col("users"),
            columns: vec![col("tenant_id"), col("user_id"), col("name"), col("email")],
            values: vec![
                InputValue::Value(DbValue::Int(1)),
                InputValue::Value(DbValue::Int(100)),
                InputValue::Value(DbValue::Text("Alice".into())),
                InputValue::Value(DbValue::Text("alice@example.com".into())),
            ],
        }],
        atomic: true,
    };
    let result = apply_mutations(&config, &insert_batch).await.unwrap();
    assert!(result.all_success);
    assert_eq!(result.affected, vec![1]);

    // 3. SELECT via execute_typed
    let outcome = execute_typed(&config, "SELECT * FROM users ORDER BY user_id")
        .await
        .unwrap();
    let rs = single_result_set(outcome);

    // Verify columns
    let col_names: Vec<&str> = rs.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(col_names, vec!["tenant_id", "user_id", "name", "email"]);

    // Verify row count
    assert_eq!(rs.row_count, 1);
    assert_eq!(rs.completeness, ResultCompleteness::Complete);

    // Verify cell values
    assert_eq!(rs.cell(0, 0), &DbValue::Int(1)); // tenant_id
    assert_eq!(rs.cell(0, 1), &DbValue::Int(100)); // user_id
    assert_eq!(rs.cell(0, 2), &DbValue::Text("Alice".into())); // name
    assert_eq!(rs.cell(0, 3), &DbValue::Text("alice@example.com".into())); // email

    // Verify row access
    let row = rs.row(0);
    assert_eq!(row.len(), 4);
    assert_eq!(row[0], DbValue::Int(1));

    // 4. UPDATE via apply_mutations with composite PK
    let update_batch = MutationBatch {
        mutations: vec![Mutation::Update {
            table: col("users"),
            identity: pk(vec![
                ("tenant_id", DbValue::Int(1)),
                ("user_id", DbValue::Int(100)),
            ]),
            changes: vec![
                (
                    col("name"),
                    InputValue::Value(DbValue::Text("Alice Updated".into())),
                ),
                (
                    col("email"),
                    InputValue::Value(DbValue::Text("alice.new@example.com".into())),
                ),
            ],
            expected_rows: ExpectedRows::Exactly(1),
        }],
        atomic: true,
    };
    let result = apply_mutations(&config, &update_batch).await.unwrap();
    assert!(result.all_success);
    assert_eq!(result.affected, vec![1]);

    // 5. SELECT again — verify updated values
    let outcome = execute_typed(&config, "SELECT * FROM users ORDER BY user_id")
        .await
        .unwrap();
    let rs = single_result_set(outcome);
    assert_eq!(rs.row_count, 1);
    assert_eq!(rs.cell(0, 2), &DbValue::Text("Alice Updated".into()));
    assert_eq!(
        rs.cell(0, 3),
        &DbValue::Text("alice.new@example.com".into())
    );

    // 6. DELETE via apply_mutations
    let delete_batch = MutationBatch {
        mutations: vec![Mutation::Delete {
            table: col("users"),
            identity: pk(vec![
                ("tenant_id", DbValue::Int(1)),
                ("user_id", DbValue::Int(100)),
            ]),
            expected_rows: ExpectedRows::Exactly(1),
        }],
        atomic: true,
    };
    let result = apply_mutations(&config, &delete_batch).await.unwrap();
    assert!(result.all_success);
    assert_eq!(result.affected, vec![1]);

    // 7. SELECT — verify empty
    let outcome = execute_typed(&config, "SELECT * FROM users").await.unwrap();
    let rs = single_result_set(outcome);
    assert_eq!(rs.row_count, 0);
    assert!(rs.is_empty());

    // Cleanup
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS users").await;
}

// ═══════════════════════════════════════════════════════════════════
// Test 2: NULL 与空字符串区分
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn null_and_empty_string() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    // 1. DROP + CREATE TABLE
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS nullable_test").await;
    let outcome = execute_typed(
        &config,
        "CREATE TABLE nullable_test (id INT AUTO_INCREMENT PRIMARY KEY, val TEXT)",
    )
    .await
    .unwrap();
    assert_affected(outcome, 0);

    // 2. INSERT NULL row
    let insert_null = MutationBatch {
        mutations: vec![Mutation::Insert {
            table: col("nullable_test"),
            columns: vec![col("id"), col("val")],
            values: vec![InputValue::Value(DbValue::Int(1)), InputValue::Null],
        }],
        atomic: true,
    };
    apply_mutations(&config, &insert_null).await.unwrap();

    // 3. INSERT empty string row
    let insert_empty = MutationBatch {
        mutations: vec![Mutation::Insert {
            table: col("nullable_test"),
            columns: vec![col("id"), col("val")],
            values: vec![
                InputValue::Value(DbValue::Int(2)),
                InputValue::Value(DbValue::Text(String::new())),
            ],
        }],
        atomic: true,
    };
    apply_mutations(&config, &insert_empty).await.unwrap();

    // 4. SELECT — verify both rows
    let outcome = execute_typed(&config, "SELECT id, val FROM nullable_test ORDER BY id")
        .await
        .unwrap();
    let rs = single_result_set(outcome);

    assert_eq!(rs.row_count, 2);

    // Row 0: id=1, val=NULL
    assert_eq!(rs.cell(0, 0), &DbValue::Int(1));
    assert_eq!(rs.cell(0, 1), &DbValue::Null);
    assert!(rs.is_null(0, 1), "val column should be NULL");

    // Row 1: id=2, val="" (empty string, NOT NULL)
    assert_eq!(rs.cell(1, 0), &DbValue::Int(2));
    assert_eq!(rs.cell(1, 1), &DbValue::Text(String::new()));
    assert!(!rs.is_null(1, 1), "empty string should NOT be NULL");

    // Verify NULL ≠ empty string
    assert_ne!(rs.cell(0, 1), rs.cell(1, 1));

    // Cleanup
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS nullable_test").await;
}

// ═══════════════════════════════════════════════════════════════════
// Test 3: 大结果集与截断语义
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn large_result_set() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    // 1. DROP + CREATE TABLE
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS big").await;
    let outcome = execute_typed(&config, "CREATE TABLE big (id INT, data TEXT)")
        .await
        .unwrap();
    assert_affected(outcome, 0);

    // 2. Insert 2000 rows via a single multi-VALUES INSERT
    const N: usize = 2000;
    let mut insert_sql = String::from("INSERT INTO big (id, data) VALUES ");
    for i in 1..=N {
        if i > 1 {
            insert_sql.push(',');
        }
        insert_sql.push_str(&format!("({}, 'row{}')", i, i));
    }
    let outcome = execute_typed(&config, &insert_sql).await.unwrap();
    assert_affected(outcome, N as u64);

    // 3. SELECT * — verify row_count and completeness
    let outcome = execute_typed(&config, "SELECT id, data FROM big ORDER BY id")
        .await
        .unwrap();
    let rs = single_result_set(outcome);

    assert_eq!(rs.row_count, N);
    // MAX_RESULT_SET_ROWS = 500000, so 2000 rows fits comfortably
    assert_eq!(
        rs.completeness,
        ResultCompleteness::Complete,
        "2000 rows should be Complete (MAX_RESULT_SET_ROWS = 500000)"
    );

    // Verify row_count <= MAX_RESULT_SET_ROWS invariant
    assert!(rs.row_count <= 500_000);

    // Verify cell access across the entire range
    assert_eq!(rs.cell(0, 0), &DbValue::Int(1));
    assert_eq!(rs.cell(0, 1), &DbValue::Text("row1".into()));
    assert_eq!(rs.cell(N - 1, 0), &DbValue::Int(N as i64));
    assert_eq!(rs.cell(N - 1, 1), &DbValue::Text(format!("row{}", N)));

    // Verify column_names
    let names = rs.column_names();
    assert_eq!(names, vec!["id", "data"]);

    // Verify is_empty
    assert!(!rs.is_empty());

    // Cleanup
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS big").await;
}

// ═══════════════════════════════════════════════════════════════════
// Test 4: DEFAULT 值
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn default_value() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    // 1. DROP + CREATE TABLE with DEFAULT
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS with_default").await;
    let outcome = execute_typed(
        &config,
        "CREATE TABLE with_default (\
         id INT AUTO_INCREMENT PRIMARY KEY, \
         created_at VARCHAR(10) DEFAULT '2024-01-01')",
    )
    .await
    .unwrap();
    assert_affected(outcome, 0);

    // 2. INSERT without specifying created_at — use Unspecified
    let insert_batch = MutationBatch {
        mutations: vec![Mutation::Insert {
            table: col("with_default"),
            columns: vec![col("id")],
            values: vec![InputValue::Value(DbValue::Int(1))],
        }],
        atomic: true,
    };
    let result = apply_mutations(&config, &insert_batch).await.unwrap();
    assert!(result.all_success);

    // 3. Also INSERT using InputValue::Default explicitly
    let insert_default_batch = MutationBatch {
        mutations: vec![Mutation::Insert {
            table: col("with_default"),
            columns: vec![col("id"), col("created_at")],
            values: vec![InputValue::Value(DbValue::Int(2)), InputValue::Default],
        }],
        atomic: true,
    };
    let result = apply_mutations(&config, &insert_default_batch)
        .await
        .unwrap();
    assert!(result.all_success);

    // 4. SELECT — verify DEFAULT value appeared for both rows
    let outcome = execute_typed(
        &config,
        "SELECT id, created_at FROM with_default ORDER BY id",
    )
    .await
    .unwrap();
    let rs = single_result_set(outcome);

    assert_eq!(rs.row_count, 2);

    // Row 0 (id=1): created_at should be '2024-01-01' (via Unspecified / implicit default)
    assert_eq!(rs.cell(0, 0), &DbValue::Int(1));
    assert_eq!(rs.cell(0, 1), &DbValue::Text("2024-01-01".into()));

    // Row 1 (id=2): created_at should be '2024-01-01' (via explicit Default)
    assert_eq!(rs.cell(1, 0), &DbValue::Int(2));
    assert_eq!(rs.cell(1, 1), &DbValue::Text("2024-01-01".into()));

    // Cleanup
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS with_default").await;
}

// ═══════════════════════════════════════════════════════════════════
// Test 5: Temporal values retain their MySQL semantics
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn temporal_roundtrip_and_extended_time_decode() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    let _ = execute_typed(&config, "DROP TABLE IF EXISTS temporal_values").await;
    execute_typed(
        &config,
        "CREATE TABLE temporal_values (\
         id INT PRIMARY KEY, \
         date_value DATE NOT NULL, \
         datetime_value DATETIME(6) NOT NULL, \
         time_value TIME(6) NOT NULL)",
    )
    .await
    .unwrap();

    let date = DbDate {
        year: 2024,
        month: 2,
        day: 29,
    };
    let datetime = DbDateTime {
        date,
        time: DbTime {
            hour: 23,
            minute: 45,
            second: 6,
            nanos: 789_123_000,
        },
    };
    let time = DbTime {
        hour: 25,
        minute: 2,
        second: 3,
        nanos: 456_000_000,
    };
    let midnight = DbDateTime {
        date: DbDate {
            year: 2024,
            month: 1,
            day: 1,
        },
        time: DbTime {
            hour: 0,
            minute: 0,
            second: 0,
            nanos: 0,
        },
    };
    let insert = MutationBatch {
        mutations: vec![Mutation::Insert {
            table: col("temporal_values"),
            columns: vec![
                col("id"),
                col("date_value"),
                col("datetime_value"),
                col("time_value"),
            ],
            values: vec![
                InputValue::Value(DbValue::Int(1)),
                InputValue::Value(DbValue::Date(date)),
                InputValue::Value(DbValue::DateTime(datetime)),
                InputValue::Value(DbValue::Time(time)),
            ],
        }],
        atomic: true,
    };
    apply_mutations(&config, &insert).await.unwrap();

    execute_typed(
        &config,
        "INSERT INTO temporal_values VALUES \
         (2, '2024-01-01', '2024-01-01 00:00:00', '-01:02:03'), \
         (3, '2024-01-01', '2024-01-01 00:00:00', '838:59:59')",
    )
    .await
    .unwrap();

    let result = single_result_set(
        execute_typed(
            &config,
            "SELECT id, date_value, datetime_value, time_value \
             FROM temporal_values ORDER BY id",
        )
        .await
        .unwrap(),
    );
    assert_eq!(result.cell(0, 1), &DbValue::Date(date));
    assert_eq!(result.cell(0, 2), &DbValue::DateTime(datetime));
    assert_eq!(result.cell(0, 3), &DbValue::Time(time));
    assert_eq!(result.cell(1, 2), &DbValue::DateTime(midnight));
    assert_eq!(
        result.cell(1, 3),
        &DbValue::Other {
            native_type: "TIME".to_string(),
            display: "-01:02:03.000000".to_string(),
        }
    );

    assert_eq!(
        result.cell(2, 3),
        &DbValue::Other {
            native_type: "TIME".to_string(),
            display: "838:59:59.000000".to_string(),
        }
    );

    let _ = execute_typed(&config, "DROP TABLE IF EXISTS temporal_values").await;
}

#[tokio::test]
async fn decimal_select_preserves_exact_decimal_value() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    let result = single_result_set(
        execute_typed(
            &config,
            "SELECT CAST('123.4500' AS DECIMAL(10, 4)) AS amount",
        )
        .await
        .unwrap(),
    );

    assert_eq!(result.cell(0, 0), &DbValue::Decimal("123.4500".to_string()));
}

// ═══════════════════════════════════════════════════════════════════
// Test 5: Schema 目录加载
// ═══════════════════════════════════════════════════════════════════

#[tokio::test]
async fn catalog_load() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };

    // Cleanup any leftover tables
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS orders").await;
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS products").await;
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS logs").await;

    // Create tables with various schemas
    // Composite PK table
    execute_typed(
        &config,
        "CREATE TABLE orders (\
         tenant_id INT, \
         order_id INT, \
         amount REAL, \
         note TEXT, \
         PRIMARY KEY (tenant_id, order_id))",
    )
    .await
    .unwrap();

    // Simple PK table with AUTO_INCREMENT
    execute_typed(
        &config,
        "CREATE TABLE products (id INT AUTO_INCREMENT PRIMARY KEY, name TEXT, price REAL DEFAULT 0.0)",
    )
    .await
    .unwrap();

    // No PK table
    execute_typed(&config, "CREATE TABLE logs (event TEXT, ts TEXT)")
        .await
        .unwrap();

    // Load catalog
    let revision = SchemaRevision(1);
    let catalog = load_schema_catalog(&config, revision).await.unwrap();

    assert_eq!(catalog.revision, revision);
    assert_eq!(catalog.tables.len(), 3);

    // ── Verify "orders" table (composite PK) ──
    let orders = catalog.table("orders").expect("orders table must exist");
    assert_eq!(orders.name, "orders");

    // Verify columns
    let col_names: Vec<&str> = orders.columns.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(col_names, vec!["tenant_id", "order_id", "amount", "note"]);

    // Verify column metadata
    let tenant_col = &orders.columns[0];
    assert_eq!(tenant_col.name, "tenant_id");
    assert!(tenant_col.is_primary_key);

    let order_col = &orders.columns[1];
    assert_eq!(order_col.name, "order_id");
    assert!(order_col.is_primary_key);

    let amount_col = &orders.columns[2];
    assert_eq!(amount_col.name, "amount");
    assert!(!amount_col.is_primary_key);

    // Verify composite PK KeyMetadata
    let pk = orders
        .primary_key
        .as_ref()
        .expect("orders must have a primary key");
    assert_eq!(pk.columns.len(), 2);
    assert_eq!(pk.columns, vec!["tenant_id", "order_id"]);

    // ── Verify "products" table (simple PK) ──
    let products = catalog
        .table("products")
        .expect("products table must exist");
    assert_eq!(products.name, "products");

    let prod_pk = products
        .primary_key
        .as_ref()
        .expect("products must have a primary key");
    assert_eq!(prod_pk.columns, vec!["id"]);

    // Verify DEFAULT value in metadata
    let price_col = products
        .columns
        .iter()
        .find(|c| c.name == "price")
        .expect("price column must exist");
    assert_eq!(price_col.default_value, Some("0".to_string()));

    // ── Verify "logs" table (no PK) ──
    let logs = catalog.table("logs").expect("logs table must exist");
    assert!(logs.primary_key.is_none(), "logs has no PK");

    // Cleanup
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS orders").await;
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS products").await;
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS logs").await;
}

#[tokio::test]
async fn catalog_composite_constraints_preserve_pairing_and_unique_keys() {
    let Some(config) = mysql_config() else {
        return;
    };
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_meta_child_mysql")
        .await
        .unwrap();
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_meta_parent_mysql")
        .await
        .unwrap();
    execute_typed(
        &config,
        "CREATE TABLE gridix_meta_parent_mysql (a INT, b INT, UNIQUE KEY parent_pair (b, a))",
    )
    .await
    .unwrap();
    execute_typed(
        &config,
        "CREATE TABLE gridix_meta_child_mysql (x INT, y INT, z VARCHAR(30), \
         UNIQUE KEY child_pair (y, x), UNIQUE KEY prefix_only (z(5)), \
         CONSTRAINT child_pair_fk FOREIGN KEY (x, y) REFERENCES gridix_meta_parent_mysql (b, a), \
         CONSTRAINT child_pair_fk_other FOREIGN KEY (y, x) \
           REFERENCES gridix_meta_parent_mysql (b, a))",
    )
    .await
    .unwrap();

    let catalog = load_schema_catalog(&config, SchemaRevision(1))
        .await
        .unwrap();
    let parent = catalog.table("gridix_meta_parent_mysql").unwrap();
    assert_eq!(parent.unique_keys.len(), 1);
    assert_eq!(parent.unique_keys[0].columns, ["b", "a"]);
    let child = catalog.table("gridix_meta_child_mysql").unwrap();
    assert_eq!(child.unique_keys.len(), 1);
    assert_eq!(child.unique_keys[0].name.as_deref(), Some("child_pair"));
    assert_eq!(child.unique_keys[0].columns, ["y", "x"]);
    assert_eq!(child.foreign_keys.len(), 2);
    assert!(
        child
            .foreign_keys
            .iter()
            .any(|fk| fk.name.as_deref() == Some("child_pair_fk")
                && fk.from_columns == ["x", "y"]
                && fk.ref_table == "gridix_meta_parent_mysql"
                && fk.ref_columns == ["b", "a"])
    );
    assert!(
        child
            .foreign_keys
            .iter()
            .any(|fk| fk.name.as_deref() == Some("child_pair_fk_other")
                && fk.from_columns == ["y", "x"]
                && fk.ref_table == "gridix_meta_parent_mysql"
                && fk.ref_columns == ["b", "a"])
    );

    execute_typed(&config, "DROP TABLE gridix_meta_child_mysql")
        .await
        .unwrap();
    execute_typed(&config, "DROP TABLE gridix_meta_parent_mysql")
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_pool_reuses_handle_and_evicts_oldest_under_pressure() {
    let Some(config) = mysql_config() else {
        eprintln!("SKIP: GRIDIX_TEST_MYSQL_URL not set");
        return;
    };
    POOL_MANAGER.clear_all().await;

    let first = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool fixture must connect");
    let reused = POOL_MANAGER
        .get_mysql_pool(&config)
        .await
        .expect("MySQL pool fixture must reuse its handle");
    assert!(Arc::ptr_eq(&first.metrics(), &reused.metrics()));

    for index in 0..constants::database::pool::MAX_MYSQL_POOLS {
        let mut pressure_config = config.clone();
        pressure_config.ssl_ca_cert = format!("gridix-mysql-pressure-{index}");
        POOL_MANAGER
            .get_mysql_pool(&pressure_config)
            .await
            .expect("MySQL pressure fixture must connect");
    }

    assert!(
        first.get_conn().await.is_err(),
        "the oldest MySQL pool must be disconnected after eviction"
    );
    POOL_MANAGER.clear_all().await;
}

#[tokio::test]
async fn mysql_varbinary_identity_preserves_non_utf8_bytes() {
    let Some(config) = mysql_config() else { return };
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_binary_identity_mysql")
        .await
        .unwrap();
    execute_typed(&config, "CREATE TABLE gridix_binary_identity_mysql (id VARBINARY(2) PRIMARY KEY, v INT) ENGINE=InnoDB").await.unwrap();
    execute_typed(
        &config,
        "INSERT INTO gridix_binary_identity_mysql VALUES (X'FF00', 1)",
    )
    .await
    .unwrap();
    let rows = single_result_set(
        execute_typed(&config, "SELECT id, v FROM gridix_binary_identity_mysql")
            .await
            .unwrap(),
    );
    assert_eq!(rows.cell(0, 0), &DbValue::Bytes(Arc::from([255u8, 0])));
    let batch = MutationBatch {
        mutations: vec![Mutation::Update {
            table: col("gridix_binary_identity_mysql"),
            identity: pk(vec![("id", rows.cell(0, 0).clone())]),
            changes: vec![(col("v"), InputValue::Value(DbValue::Int(2)))],
            expected_rows: ExpectedRows::Exactly(1),
        }],
        atomic: true,
    };
    apply_mutations(&config, &batch).await.unwrap();
    let updated = single_result_set(
        execute_typed(&config, "SELECT v FROM gridix_binary_identity_mysql")
            .await
            .unwrap(),
    );
    assert_eq!(updated.cell(0, 0), &DbValue::Int(2));
    execute_typed(&config, "DROP TABLE gridix_binary_identity_mysql")
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_at_least_mutation_failure_rolls_back_prior_insert() {
    let Some(config) = mysql_config() else { return };
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_expected_rows_mysql")
        .await
        .unwrap();
    execute_typed(
        &config,
        "CREATE TABLE gridix_expected_rows_mysql (id INT PRIMARY KEY) ENGINE=InnoDB",
    )
    .await
    .unwrap();
    let batch = MutationBatch {
        mutations: vec![
            Mutation::Insert {
                table: col("gridix_expected_rows_mysql"),
                columns: vec![col("id")],
                values: vec![InputValue::Value(DbValue::Int(1))],
            },
            Mutation::Delete {
                table: col("gridix_expected_rows_mysql"),
                identity: pk(vec![("id", DbValue::Int(999))]),
                expected_rows: ExpectedRows::AtLeast(1),
            },
        ],
        atomic: true,
    };
    assert!(matches!(
        apply_mutations(&config, &batch).await,
        Err(DbError::Query(_))
    ));
    let rows = single_result_set(
        execute_typed(&config, "SELECT id FROM gridix_expected_rows_mysql")
            .await
            .unwrap(),
    );
    assert_eq!(rows.row_count, 0);
    execute_typed(&config, "DROP TABLE gridix_expected_rows_mysql")
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_myisam_mutation_failed_batch_reports_unknown_and_preserves_insert() {
    let Some(config) = mysql_config() else { return };
    let table = "gridix_myisam_mutation_rollback";
    execute_typed(&config, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    execute_typed(
        &config,
        &format!("CREATE TABLE {table} (id INT PRIMARY KEY) ENGINE=MyISAM"),
    )
    .await
    .unwrap();
    let batch = MutationBatch {
        mutations: vec![
            Mutation::Insert {
                table: col(table),
                columns: vec![col("id")],
                values: vec![InputValue::Value(DbValue::Int(1))],
            },
            Mutation::Insert {
                table: col(table),
                columns: vec![col("id")],
                values: vec![InputValue::Value(DbValue::Int(1))],
            },
        ],
        atomic: true,
    };
    let error = apply_mutations(&config, &batch).await.unwrap_err();
    assert!(matches!(
        &error,
        DbError::MutationOutcomeUnknown {
            operation: "ROLLBACK",
            ..
        }
    ));
    assert!(error.to_string().contains("non-transactional"), "{error}");
    let rows = single_result_set(
        execute_typed(&config, &format!("SELECT id FROM {table}"))
            .await
            .unwrap(),
    );
    assert_eq!(rows.row_count, 1);
    assert_eq!(rows.cell(0, 0), &DbValue::Int(1));
    execute_typed(&config, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_import_consumes_all_procedure_results_before_success() {
    let Some(config) = mysql_config() else { return };
    execute_import_batch(
        &config,
        vec!["DROP PROCEDURE IF EXISTS gridix_late_error_mysql".into()],
        false,
        true,
    )
    .await
    .unwrap();
    execute_import_batch(&config, vec!["CREATE PROCEDURE gridix_late_error_mysql() BEGIN SELECT 1; SIGNAL SQLSTATE '45000' SET MESSAGE_TEXT='late failure'; END".into()], false, true).await.unwrap();
    let statements = vec!["CALL gridix_late_error_mysql()".into()];
    let report = execute_import_batch(&config, statements.clone(), false, false)
        .await
        .unwrap();
    assert_eq!((report.succeeded, report.failed), (0, 1));
    assert!(
        execute_import_batch(&config, statements, false, true)
            .await
            .is_err()
    );
    execute_import_batch(
        &config,
        vec!["DROP PROCEDURE gridix_late_error_mysql".into()],
        false,
        true,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn mysql_wrapped_import_rejects_control_before_writing() {
    let Some(config) = mysql_config() else { return };
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_wrapped_import_mysql")
        .await
        .unwrap();
    execute_typed(
        &config,
        "CREATE TABLE gridix_wrapped_import_mysql (id INT PRIMARY KEY) ENGINE=InnoDB",
    )
    .await
    .unwrap();
    for statement in [
        "COMMIT/**/WORK".to_string(),
        "CHECK TABLE gridix_wrapped_import_mysql".to_string(),
        "PREPARE gridix_escape FROM 'COMMIT'".to_string(),
        "EXECUTE gridix_escape".to_string(),
        "SELECT 1 --\u{0001}'\n; COMMIT; -- '\nSELECT * FROM missing_table;".to_string(),
        "SELECT 1--\u{00a0}2; COMMIT".to_string(),
        "SELECT 1 AS -- '\nDELIMITER ;\nCOMMIT; -- '\nINSERT INTO missing_table_xyz VALUES (2);"
            .to_string(),
        "SELECT 1 AS\nDELIMITER ;\nCOMMIT;".to_string(),
    ] {
        let statements = vec![
            "INSERT INTO gridix_wrapped_import_mysql VALUES (1)".into(),
            statement,
        ];
        let result = execute_import_batch(&config, statements, true, true).await;
        assert!(
            matches!(&result, Err(DbError::Query(error)) if error.contains("事务导入拒绝执行")),
            "{result:?}"
        );
        let rows = single_result_set(
            execute_typed(&config, "SELECT id FROM gridix_wrapped_import_mysql")
                .await
                .unwrap(),
        );
        assert_eq!(rows.row_count, 0);
    }
    execute_typed(&config, "DROP TABLE gridix_wrapped_import_mysql")
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_wrapped_import_bare_cr_comments_never_execute_delete() {
    use gridix::core::{
        SqlDialect, TransferDirection, TransferFormat, TransferFormatOptions, TransferSession,
        TransferSqlOptions, plan_sql_transfer_content,
    };
    let Some(config) = mysql_config() else { return };
    let table = "gridix_import_cr_comments_mysql";
    execute_typed(&config, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .expect("remove prior fixture");
    execute_typed(
        &config,
        &format!("CREATE TABLE {table} (id INT PRIMARY KEY) ENGINE=InnoDB"),
    )
    .await
    .expect("create isolated import fixture");
    let session = TransferSession {
        direction: TransferDirection::Import,
        format: TransferFormat::Sql,
        sql_dialect: SqlDialect::MySql,
        options: TransferFormatOptions::Sql(TransferSqlOptions::default()),
        ..Default::default()
    };

    for comment in ["-- note", "# note"] {
        execute_typed(&config, &format!("DELETE FROM {table}"))
            .await
            .expect("clear fixture");
        execute_typed(&config, &format!("INSERT INTO {table} VALUES (1)"))
            .await
            .expect("seed row that a stray DELETE would remove");
        let sql = format!(
            "INSERT INTO {table} VALUES (2); {comment}\rDELETE FROM {table};\nINSERT INTO {table} VALUES (3);"
        );
        let statements = plan_sql_transfer_content(&sql, &session)
            .expect("plan MySQL import")
            .into_sql_statements()
            .expect("SQL statements");
        let report = execute_import_batch(&config, statements, true, true)
            .await
            .expect("execute imports without the commented DELETE");
        assert_eq!(report.succeeded, 2, "{comment}");
        let rows = single_result_set(
            execute_typed(&config, &format!("SELECT id FROM {table} ORDER BY id"))
                .await
                .expect("inspect persisted rows"),
        );
        assert_eq!(rows.row_count, 3, "{comment}");
        for (index, id) in [1, 2, 3].into_iter().enumerate() {
            assert_eq!(rows.cell(index, 0), &DbValue::Int(id), "{comment}");
        }
    }
    execute_typed(&config, &format!("DROP TABLE {table}"))
        .await
        .expect("drop isolated import fixture");
}

#[tokio::test]
async fn mysql_wrapped_import_non_ascii_dashes_preserve_delete_predicate() {
    use gridix::core::{
        SqlDialect, TransferDirection, TransferFormat, TransferFormatOptions, TransferSession,
        TransferSqlOptions, plan_sql_transfer_content,
    };
    let Some(config) = mysql_config() else { return };
    let table = "gridix_import_non_ascii_comment_mysql";
    execute_typed(&config, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .expect("remove prior fixture");
    execute_typed(
        &config,
        &format!("CREATE TABLE {table} (id INT PRIMARY KEY, `\u{00a0}x` INT) ENGINE=InnoDB"),
    )
    .await
    .expect("create isolated import fixture");
    execute_typed(&config, &format!("INSERT INTO {table} VALUES (1, -1)"))
        .await
        .expect("seed row whose predicate is false");
    let sql = format!("DELETE FROM {table} WHERE 1--\u{00a0}x;");
    let session = TransferSession {
        direction: TransferDirection::Import,
        format: TransferFormat::Sql,
        sql_dialect: SqlDialect::MySql,
        options: TransferFormatOptions::Sql(TransferSqlOptions::default()),
        ..Default::default()
    };
    let statements = plan_sql_transfer_content(&sql, &session)
        .expect("plan MySQL import without removing the predicate")
        .into_sql_statements()
        .expect("SQL statements");
    assert_eq!(statements, [sql.trim_end_matches(';')]);
    let report = execute_import_batch(&config, statements, true, true)
        .await
        .expect("execute import with intact predicate");
    assert_eq!(report.succeeded, 1);
    let rows = single_result_set(
        execute_typed(&config, &format!("SELECT id FROM {table}"))
            .await
            .expect("inspect persisted row"),
    );
    assert_eq!(rows.row_count, 1);
    assert_eq!(rows.cell(0, 0), &DbValue::Int(1));
    execute_typed(&config, &format!("DROP TABLE {table}"))
        .await
        .expect("drop isolated fixture");
}

#[tokio::test]
async fn mysql_wrapped_myisam_import_failed_statement_reports_unknown_and_keeps_write() {
    let Some(config) = mysql_config() else { return };
    let table = "gridix_myisam_import_rollback";
    execute_typed(&config, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    execute_typed(
        &config,
        &format!("CREATE TABLE {table} (id INT PRIMARY KEY) ENGINE=MyISAM"),
    )
    .await
    .unwrap();
    let statements = vec![
        format!("INSERT INTO {table} VALUES (1)"),
        format!("INSERT INTO {table} VALUES (1)"),
    ];
    let error = execute_import_batch(&config, statements, true, true)
        .await
        .unwrap_err();
    assert!(matches!(
        &error,
        DbError::ImportOutcomeUnknown {
            operation: "ROLLBACK",
            ..
        }
    ));
    assert!(error.to_string().contains("第 2 条"), "{error}");
    assert!(error.to_string().contains("non-transactional"), "{error}");
    let rows = single_result_set(
        execute_typed(&config, &format!("SELECT id FROM {table}"))
            .await
            .unwrap(),
    );
    assert_eq!(rows.row_count, 1);
    assert_eq!(rows.cell(0, 0), &DbValue::Int(1));
    execute_typed(&config, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_wrapped_innodb_import_failed_statement_confirms_rollback() {
    let Some(config) = mysql_config() else { return };
    let table = "gridix_innodb_import_rollback";
    execute_typed(&config, &format!("DROP TABLE IF EXISTS {table}"))
        .await
        .unwrap();
    execute_typed(
        &config,
        &format!("CREATE TABLE {table} (id INT PRIMARY KEY) ENGINE=InnoDB"),
    )
    .await
    .unwrap();
    let statements = vec![
        format!("INSERT INTO {table} VALUES (1)"),
        format!("INSERT INTO {table} VALUES (1)"),
    ];
    let error = execute_import_batch(&config, statements, true, true)
        .await
        .unwrap_err();
    assert!(matches!(error, DbError::Query(_)));
    let rows = single_result_set(
        execute_typed(&config, &format!("SELECT id FROM {table}"))
            .await
            .unwrap(),
    );
    assert_eq!(rows.row_count, 0);
    execute_typed(&config, &format!("DROP TABLE {table}"))
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_sql_export_import_preserves_microseconds() {
    use gridix::core::{
        SqlDialect, TransferDirection, TransferFormat, TransferFormatOptions, TransferSchema,
        TransferSession, TransferSqlOptions, plan_export_transfer,
    };
    let Some(config) = mysql_config() else { return };
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_fraction_export_mysql")
        .await
        .unwrap();
    execute_typed(&config, "CREATE TABLE gridix_fraction_export_mysql (dt DATETIME(6), t TIME(6), txt VARCHAR(40)) ENGINE=InnoDB").await.unwrap();
    let source = single_result_set(execute_typed(&config,
        "SELECT CAST('2024-01-02 01:02:03.123456' AS DATETIME(6)) AS dt, CAST('01:02:03.123456' AS TIME(6)) AS t, CONVERT(0x615C62 USING utf8mb4) AS txt"
    ).await.unwrap());
    let session = TransferSession {
        direction: TransferDirection::Export,
        format: TransferFormat::Sql,
        sql_dialect: SqlDialect::MySql,
        schema: TransferSchema {
            target_name: Some("gridix_fraction_export_mysql".into()),
            ..Default::default()
        },
        options: TransferFormatOptions::Sql(TransferSqlOptions {
            use_transaction: false,
            batch_size: 0,
            ..Default::default()
        }),
        ..Default::default()
    };
    let sql = plan_export_transfer(&source, &session)
        .unwrap()
        .into_rendered_text()
        .unwrap();
    execute_import_batch(&config, vec![sql], false, true)
        .await
        .unwrap();
    let persisted = single_result_set(
        execute_typed(
            &config,
            "SELECT dt, t, txt FROM gridix_fraction_export_mysql",
        )
        .await
        .unwrap(),
    );
    for index in 0..3 {
        assert_eq!(persisted.cell(0, index), source.cell(0, index));
    }
    execute_typed(&config, "DROP TABLE gridix_fraction_export_mysql")
        .await
        .unwrap();
}

#[tokio::test]
async fn mysql_csv_json_import_uses_backtick_identifiers() {
    use gridix::core::{
        SqlDialect, TransferDirection, TransferFormat, TransferFormatOptions, TransferSchema,
        TransferSession, plan_import_transfer,
    };
    let Some(config) = mysql_config() else { return };
    execute_typed(&config, "DROP TABLE IF EXISTS gridix_dialect_import_mysql")
        .await
        .unwrap();
    execute_typed(
        &config,
        "CREATE TABLE gridix_dialect_import_mysql (id INT PRIMARY KEY) ENGINE=InnoDB",
    )
    .await
    .unwrap();
    for (format, content) in [
        (TransferFormat::Csv, "id\n1\n"),
        (TransferFormat::Json, "[{\"id\":2}]"),
    ] {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), content).unwrap();
        let session = TransferSession {
            direction: TransferDirection::Import,
            format,
            sql_dialect: SqlDialect::MySql,
            schema: TransferSchema {
                target_name: Some("gridix_dialect_import_mysql".into()),
                ..Default::default()
            },
            options: match format {
                TransferFormat::Csv => TransferFormatOptions::Delimited(Default::default()),
                _ => TransferFormatOptions::Json(Default::default()),
            },
            ..Default::default()
        };
        let statements = plan_import_transfer(file.path(), &session)
            .unwrap()
            .into_sql_statements()
            .unwrap();
        assert!(statements[0].starts_with("INSERT INTO `gridix_dialect_import_mysql` (`id`)"));
        let report = execute_import_batch(&config, statements, true, true)
            .await
            .unwrap();
        assert_eq!(report.failed, 0);
    }
    let result = single_result_set(
        execute_typed(
            &config,
            "SELECT id FROM gridix_dialect_import_mysql ORDER BY id",
        )
        .await
        .unwrap(),
    );
    assert_eq!(
        (result.cell(0, 0), result.cell(1, 0)),
        (&DbValue::Int(1), &DbValue::Int(2))
    );
    execute_typed(&config, "DROP TABLE gridix_dialect_import_mysql")
        .await
        .unwrap();
}
