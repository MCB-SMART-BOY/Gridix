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

use gridix::data::{
    ConnectionConfig, DatabaseType, apply_mutations, execute_typed, load_schema_catalog,
};
use gridix::domain::execution::{ExecutionOutcome, StatementOutcome};
use gridix::domain::ids::SchemaRevision;
use gridix::domain::mutation::{
    ColumnRef, ExpectedRows, InputValue, Mutation, MutationBatch, RowIdentity,
};
use gridix::domain::result::{ResultCompleteness, ResultSet};
use gridix::domain::value::DbValue;

// ── helpers ──

/// 从 GRIDIX_TEST_MYSQL_URL 环境变量解析 MySQL 连接配置。
/// 格式：mysql://user:password@host:port/database
fn mysql_config() -> Option<ConnectionConfig> {
    let url = std::env::var("GRIDIX_TEST_MYSQL_URL").ok()?;

    let rest = url.strip_prefix("mysql://")?;

    let (user_info, rest) = rest.split_once('@')?;
    let (user, password) = user_info.split_once(':').unwrap_or((user_info, ""));

    let (host_port, database) = rest.split_once('/')?;
    let (host, port_str) = host_port.split_once(':').unwrap_or((host_port, "3306"));
    let port: u16 = port_str.parse().ok()?;

    Some(ConnectionConfig {
        db_type: DatabaseType::MySQL,
        host: host.to_string(),
        port,
        username: user.to_string(),
        password: password.to_string(),
        database: database.to_string(),
        ..Default::default()
    })
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
         created_at TEXT DEFAULT '2024-01-01')",
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
    assert_eq!(price_col.default_value, Some("0.0".to_string()));

    // ── Verify "logs" table (no PK) ──
    let logs = catalog.table("logs").expect("logs table must exist");
    assert!(logs.primary_key.is_none(), "logs has no PK");

    // Cleanup
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS orders").await;
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS products").await;
    let _ = execute_typed(&config, "DROP TABLE IF EXISTS logs").await;
}
