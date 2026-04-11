//! 帮助面板交互动作
//!
//! 将“数据库知识学习指南”里的按钮动作落到真实应用行为上，
//! 复用现有连接、查询、ER 图渲染逻辑。

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use rusqlite::{Connection as SqliteConn, OptionalExtension, Transaction, params};

use crate::core::AppConfig;
use crate::database::{ConnectionConfig, DatabaseType};
use crate::ui;

use super::DbManagerApp;
use super::action_system::AppAction;

const LEARNING_CONNECTION_NAME: &str = "Gridix 学习示例";
const LEARNING_DB_FILENAME: &str = "learning.sqlite3";
const LEARNING_DB_SCHEMA_VERSION: i64 = 2;
const LEARNING_MIN_ROWS_PER_TABLE: i64 = 100;
const LEARNING_REQUIRED_TABLES: &[&str] = &[
    "customers",
    "customer_addresses",
    "suppliers",
    "product_categories",
    "products",
    "orders",
    "order_items",
    "payments",
];

impl DbManagerApp {
    pub(in crate::app) fn handle_help_action(
        &mut self,
        ctx: &egui::Context,
        action: ui::HelpAction,
    ) {
        match action {
            ui::HelpAction::OpenConnectionDialog => {
                self.dispatch_app_action(
                    ctx,
                    AppAction::OpenConnectionDialogFor(self.onboarding_target_db_type()),
                );
            }
            ui::HelpAction::EnsureLearningSample { reset } => {
                self.dispatch_app_action(
                    ctx,
                    AppAction::EnsureLearningSample {
                        reset,
                        notify: true,
                    },
                );
            }
            ui::HelpAction::RunLearningQuery {
                table,
                sql,
                open_er_diagram,
            } => {
                if let Err(error) = self.ensure_learning_connection(false, false) {
                    self.notifications.error(error);
                    return;
                }

                self.show_sidebar = true;
                self.sidebar_panel_state.show_connections = true;
                self.sidebar_section = ui::SidebarSection::Tables;
                self.show_sql_editor = true;
                self.set_focus_area(ui::FocusArea::DataGrid);
                self.sql = sql.clone();

                if let Some(table_name) = table {
                    self.selected_table = Some(table_name.clone());
                    self.grid_state.primary_key_column = None;
                    self.fetch_primary_key(&table_name);
                }

                if open_er_diagram {
                    self.show_er_diagram = true;
                    self.load_er_diagram_data();
                }

                let _ = self.execute(sql);
            }
            ui::HelpAction::RunLearningMutationDemo {
                reset,
                mutation_sql,
                preview_table,
                preview_sql,
                success_message,
            } => {
                if let Err(error) = self.run_learning_mutation_demo(
                    reset,
                    &mutation_sql,
                    preview_table,
                    &preview_sql,
                    &success_message,
                ) {
                    self.notifications.error(error);
                }
            }
            ui::HelpAction::ShowLearningErDiagram => {
                if let Err(error) = self.ensure_learning_connection(false, false) {
                    self.notifications.error(error);
                    return;
                }

                self.show_sidebar = true;
                self.sidebar_panel_state.show_connections = true;
                self.sidebar_section = ui::SidebarSection::Tables;
                self.show_er_diagram = true;
                self.load_er_diagram_data();
                self.notifications.info("学习示例库的 ER 图已打开");
            }
            ui::HelpAction::ContinueOnboarding(step) => {
                let mapped = match step {
                    ui::HelpOnboardingStep::EnvironmentCheck => {
                        ui::WelcomeOnboardingStep::EnvironmentCheck
                    }
                    ui::HelpOnboardingStep::CreateConnection => {
                        ui::WelcomeOnboardingStep::CreateConnection
                    }
                    ui::HelpOnboardingStep::InitializeDatabase => {
                        ui::WelcomeOnboardingStep::InitializeDatabase
                    }
                    ui::HelpOnboardingStep::CreateUser => ui::WelcomeOnboardingStep::CreateUser,
                    ui::HelpOnboardingStep::RunFirstQuery => {
                        ui::WelcomeOnboardingStep::RunFirstQuery
                    }
                };
                self.handle_onboarding_step(mapped);
            }
        }
    }

    pub(in crate::app) fn ensure_learning_connection(
        &mut self,
        reset: bool,
        notify: bool,
    ) -> Result<(), String> {
        let path = learning_database_path()?;

        if reset {
            if self
                .manager
                .connections
                .contains_key(LEARNING_CONNECTION_NAME)
            {
                self.disconnect(LEARNING_CONNECTION_NAME.to_string());
                self.manager.connections.remove(LEARNING_CONNECTION_NAME);
            }

            if path.exists() {
                fs::remove_file(&path).map_err(|e| format!("删除旧学习示例库失败: {}", e))?;
            }
        }

        if !learning_database_ready(&path)? {
            if path.exists() {
                fs::remove_file(&path).map_err(|e| format!("清理学习示例库失败: {}", e))?;
            }
            seed_learning_database(&path)?;
        }

        let mut config = ConnectionConfig::new(LEARNING_CONNECTION_NAME, DatabaseType::SQLite);
        config.database = path.to_string_lossy().into_owned();

        self.manager.add(config);
        self.save_config();
        self.connect(LEARNING_CONNECTION_NAME.to_string());

        if notify {
            if reset {
                self.notifications.success("学习示例库已重置");
            } else {
                self.notifications.info("学习示例库已打开");
            }
        }

        Ok(())
    }

    fn run_learning_mutation_demo(
        &mut self,
        reset: bool,
        mutation_sql: &str,
        preview_table: Option<String>,
        preview_sql: &str,
        success_message: &str,
    ) -> Result<(), String> {
        self.ensure_learning_connection(reset, false)?;

        let path = learning_database_path()?;
        let conn = SqliteConn::open(&path).map_err(|e| format!("打开学习示例库失败: {}", e))?;
        conn.execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(|e| format!("启用学习示例库外键失败: {}", e))?;
        conn.execute_batch(mutation_sql)
            .map_err(|e| format!("执行学习演示失败: {}", e))?;

        self.notifications.success(success_message);
        self.show_sidebar = true;
        self.sidebar_panel_state.show_connections = true;
        self.sidebar_section = ui::SidebarSection::Tables;
        self.show_sql_editor = true;
        self.set_focus_area(ui::FocusArea::DataGrid);
        self.sql = format!(
            "{}\n\n-- 查看变更结果\n{}",
            mutation_sql.trim(),
            preview_sql.trim()
        );

        if let Some(table_name) = preview_table {
            self.selected_table = Some(table_name.clone());
            self.grid_state.primary_key_column = None;
            self.fetch_primary_key(&table_name);
        }

        let _ = self.execute(preview_sql.to_string());
        Ok(())
    }
}

fn learning_database_path() -> Result<PathBuf, String> {
    let dir = AppConfig::config_dir().ok_or("无法找到配置目录")?;
    fs::create_dir_all(&dir).map_err(|e| format!("创建学习示例目录失败: {}", e))?;
    Ok(dir.join(LEARNING_DB_FILENAME))
}

fn learning_database_ready(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }

    let conn = SqliteConn::open(path).map_err(|e| format!("打开学习示例库失败: {}", e))?;
    let meta_table_exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'gridix_learning_meta'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查学习示例库元数据表失败: {}", e))?;
    if meta_table_exists != 1 {
        return Ok(false);
    }

    let schema_version = conn
        .query_row(
            "SELECT schema_version FROM gridix_learning_meta LIMIT 1",
            [],
            |row| row.get::<_, i64>(0),
        )
        .optional()
        .map_err(|e| format!("检查学习示例库版本失败: {}", e))?;
    if schema_version != Some(LEARNING_DB_SCHEMA_VERSION) {
        return Ok(false);
    }

    for table in LEARNING_REQUIRED_TABLES {
        let table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .map_err(|e| format!("检查学习示例库表失败: {}", e))?;
        if table_exists != 1 {
            return Ok(false);
        }

        let column_count: i64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM pragma_table_info('{table}')"),
                [],
                |row| row.get(0),
            )
            .map_err(|e| format!("检查学习示例库列定义失败: {}", e))?;
        if column_count < 15 {
            return Ok(false);
        }

        let row_count: i64 = conn
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .map_err(|e| format!("检查学习示例库数据规模失败: {}", e))?;
        if row_count < LEARNING_MIN_ROWS_PER_TABLE {
            return Ok(false);
        }
    }

    let fk_errors: i64 = conn
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .map_err(|e| format!("检查学习示例库外键失败: {}", e))?;

    Ok(fk_errors == 0)
}

fn seed_learning_database(path: &Path) -> Result<(), String> {
    let mut conn = SqliteConn::open(path).map_err(|e| format!("创建学习示例库失败: {}", e))?;
    conn.execute_batch(LEARNING_DB_SCHEMA_SQL)
        .map_err(|e| format!("初始化学习示例结构失败: {}", e))?;
    populate_learning_database(&mut conn)?;
    conn.execute_batch(&format!(
        "PRAGMA user_version = {};",
        LEARNING_DB_SCHEMA_VERSION
    ))
    .map_err(|e| format!("写入学习示例库版本失败: {}", e))?;
    Ok(())
}

const LEARNING_DB_SCHEMA_SQL: &str = r#"
PRAGMA foreign_keys = OFF;

DROP TABLE IF EXISTS gridix_learning_meta;
DROP TABLE IF EXISTS payments;
DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS customer_addresses;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS product_categories;
DROP TABLE IF EXISTS suppliers;
DROP TABLE IF EXISTS customers;

PRAGMA foreign_keys = ON;

CREATE TABLE gridix_learning_meta (
    schema_version INTEGER NOT NULL,
    dataset_name TEXT NOT NULL,
    row_target INTEGER NOT NULL,
    seeded_at TEXT NOT NULL,
    notes TEXT
);

CREATE TABLE customers (
    id INTEGER PRIMARY KEY,
    customer_code TEXT UNIQUE,
    name TEXT NOT NULL,
    city TEXT NOT NULL,
    province TEXT,
    country TEXT NOT NULL DEFAULT 'China',
    level TEXT NOT NULL,
    email TEXT,
    phone TEXT,
    segment TEXT NOT NULL DEFAULT 'Retail',
    preferred_language TEXT NOT NULL DEFAULT 'zh-CN',
    marketing_opt_in INTEGER NOT NULL DEFAULT 0,
    lifetime_value REAL NOT NULL DEFAULT 0,
    reward_points INTEGER NOT NULL DEFAULT 0,
    referred_by_customer_id INTEGER,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT,
    last_login_at TEXT,
    status TEXT NOT NULL DEFAULT 'ACTIVE',
    notes TEXT,
    FOREIGN KEY (referred_by_customer_id) REFERENCES customers(id)
);

CREATE TABLE customer_addresses (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    label TEXT NOT NULL,
    recipient_name TEXT NOT NULL,
    phone TEXT,
    country TEXT NOT NULL,
    province TEXT NOT NULL,
    city TEXT NOT NULL,
    district TEXT NOT NULL,
    postal_code TEXT NOT NULL,
    address_line1 TEXT NOT NULL,
    address_line2 TEXT,
    is_default_shipping INTEGER NOT NULL DEFAULT 0,
    is_default_billing INTEGER NOT NULL DEFAULT 0,
    delivery_notes TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (customer_id) REFERENCES customers(id)
);

CREATE TABLE suppliers (
    id INTEGER PRIMARY KEY,
    supplier_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    contact_name TEXT NOT NULL,
    contact_email TEXT,
    contact_phone TEXT,
    city TEXT NOT NULL,
    province TEXT NOT NULL,
    country TEXT NOT NULL,
    tier TEXT NOT NULL,
    lead_time_days INTEGER NOT NULL,
    rating REAL NOT NULL,
    active_contract INTEGER NOT NULL DEFAULT 1,
    payment_terms TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE product_categories (
    id INTEGER PRIMARY KEY,
    category_code TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    parent_category_id INTEGER,
    department TEXT NOT NULL,
    merchandising_group TEXT NOT NULL,
    tax_rate REAL NOT NULL,
    shelf_life_days INTEGER NOT NULL,
    requires_serial INTEGER NOT NULL DEFAULT 0,
    hazardous_level INTEGER NOT NULL DEFAULT 0,
    display_order INTEGER NOT NULL,
    seo_slug TEXT NOT NULL,
    description TEXT,
    active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (parent_category_id) REFERENCES product_categories(id)
);

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    sku TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    category_id INTEGER NOT NULL,
    supplier_id INTEGER NOT NULL,
    price REAL NOT NULL,
    cost REAL NOT NULL,
    stock_qty INTEGER NOT NULL,
    reorder_level INTEGER NOT NULL,
    weight_kg REAL NOT NULL,
    color TEXT,
    size_label TEXT,
    material TEXT,
    warranty_months INTEGER NOT NULL,
    discontinued INTEGER NOT NULL DEFAULT 0,
    launch_date TEXT NOT NULL,
    rating REAL NOT NULL,
    barcode TEXT NOT NULL UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (category_id) REFERENCES product_categories(id),
    FOREIGN KEY (supplier_id) REFERENCES suppliers(id)
);

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    shipping_address_id INTEGER NOT NULL,
    billing_address_id INTEGER NOT NULL,
    order_date TEXT NOT NULL,
    status TEXT NOT NULL,
    payment_status TEXT NOT NULL,
    fulfillment_status TEXT NOT NULL,
    source_channel TEXT NOT NULL,
    coupon_code TEXT,
    total_amount REAL NOT NULL,
    shipping_fee REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    discount_amount REAL NOT NULL DEFAULT 0,
    shipped_at TEXT,
    delivered_at TEXT,
    priority_level TEXT NOT NULL,
    remarks TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (customer_id) REFERENCES customers(id),
    FOREIGN KEY (shipping_address_id) REFERENCES customer_addresses(id),
    FOREIGN KEY (billing_address_id) REFERENCES customer_addresses(id)
);

CREATE TABLE order_items (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    line_number INTEGER NOT NULL,
    quantity INTEGER NOT NULL,
    unit_price REAL NOT NULL,
    discount_amount REAL NOT NULL DEFAULT 0,
    tax_amount REAL NOT NULL DEFAULT 0,
    line_total REAL NOT NULL,
    fulfillment_status TEXT NOT NULL,
    requested_ship_date TEXT NOT NULL,
    shipped_at TEXT,
    return_requested INTEGER NOT NULL DEFAULT 0,
    gift_wrap INTEGER NOT NULL DEFAULT 0,
    note TEXT,
    created_at TEXT NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (product_id) REFERENCES products(id)
);

CREATE TABLE payments (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL,
    customer_id INTEGER NOT NULL,
    payment_number TEXT NOT NULL UNIQUE,
    method TEXT NOT NULL,
    provider TEXT NOT NULL,
    currency TEXT NOT NULL,
    amount REAL NOT NULL,
    fee_amount REAL NOT NULL,
    status TEXT NOT NULL,
    paid_at TEXT,
    settled_at TEXT,
    refunded_at TEXT,
    installments INTEGER NOT NULL DEFAULT 1,
    captured_by TEXT,
    reference_code TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(id) ON DELETE CASCADE,
    FOREIGN KEY (customer_id) REFERENCES customers(id)
);

CREATE INDEX idx_addresses_customer ON customer_addresses(customer_id);
CREATE INDEX idx_products_category ON products(category_id);
CREATE INDEX idx_products_supplier ON products(supplier_id);
CREATE INDEX idx_orders_customer_status ON orders(customer_id, status);
CREATE INDEX idx_order_items_order ON order_items(order_id);
CREATE INDEX idx_order_items_product ON order_items(product_id);
CREATE INDEX idx_payments_order_status ON payments(order_id, status);
"#;

fn populate_learning_database(conn: &mut SqliteConn) -> Result<(), String> {
    let tx = conn
        .transaction()
        .map_err(|e| format!("开启学习示例事务失败: {}", e))?;

    seed_customers(&tx)?;
    seed_customer_addresses(&tx)?;
    seed_suppliers(&tx)?;
    seed_product_categories(&tx)?;
    seed_products(&tx)?;
    seed_orders_and_order_items(&tx)?;
    seed_payments(&tx)?;

    tx.execute(
        "INSERT INTO gridix_learning_meta (schema_version, dataset_name, row_target, seeded_at, notes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            LEARNING_DB_SCHEMA_VERSION,
            "Gridix Commerce Academy",
            LEARNING_MIN_ROWS_PER_TABLE,
            timestamp_for_offset(140, 10, 30),
            "8 tables, 100+ rows each, 15+ columns each, with self-references and multi-hop joins"
        ],
    )
    .map_err(|e| format!("写入学习示例元数据失败: {}", e))?;

    tx.commit()
        .map_err(|e| format!("提交学习示例数据失败: {}", e))
}

fn seed_customers(tx: &Transaction<'_>) -> Result<(), String> {
    let cities = [
        ("Shanghai", "Shanghai"),
        ("Beijing", "Beijing"),
        ("Hangzhou", "Zhejiang"),
        ("Shenzhen", "Guangdong"),
        ("Guangzhou", "Guangdong"),
        ("Nanjing", "Jiangsu"),
        ("Chongqing", "Chongqing"),
        ("Tianjin", "Tianjin"),
        ("Wuhan", "Hubei"),
        ("Suzhou", "Jiangsu"),
    ];
    let levels = ["Gold", "Silver", "Bronze", "Platinum"];
    let segments = ["Retail", "SMB", "Enterprise", "Education"];
    let languages = ["zh-CN", "en-US", "ja-JP"];
    let statuses = ["ACTIVE", "ACTIVE", "ACTIVE", "DORMANT", "VIP"];

    let mut stmt = tx
        .prepare(
            "INSERT INTO customers (
                id, customer_code, name, city, province, country, level, email, phone, segment,
                preferred_language, marketing_opt_in, lifetime_value, reward_points,
                referred_by_customer_id, created_at, updated_at, last_login_at, status, notes
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, 'China', ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16,
                ?17, ?18, ?19
            )",
        )
        .map_err(|e| format!("准备 customers 插入失败: {}", e))?;

    for id in 1..=100 {
        let (city, province) = cities[(id - 1) % cities.len()];
        let name = match id {
            1 => "Alice Zhang".to_string(),
            2 => "Bob Chen".to_string(),
            3 => "Carol Lin".to_string(),
            4 => "David Wu".to_string(),
            5 => "Eva Sun".to_string(),
            6 => "Frank Liu".to_string(),
            _ => format!("Customer {:03}", id),
        };
        let email = if id % 4 == 0 {
            None
        } else if id == 1 {
            Some("alice.zhang@example.com".to_string())
        } else if id == 3 {
            Some("carol.lin@example.com".to_string())
        } else if id == 5 {
            Some("eva.sun@example.com".to_string())
        } else {
            Some(format!("customer{:03}@gridix.dev", id))
        };
        let referred_by = if id > 6 && id % 7 == 0 {
            Some(((id - 2) % 6 + 1) as i64)
        } else {
            None
        };
        stmt.execute(params![
            id as i64,
            format!("CUST-{id:04}"),
            name,
            city,
            province,
            levels[(id - 1) % levels.len()],
            email,
            format!("138{:08}", 10000000 + id as i32),
            segments[(id - 1) % segments.len()],
            languages[(id - 1) % languages.len()],
            if id % 3 == 0 { 0 } else { 1 },
            round2(1500.0 + id as f64 * 128.75),
            (id * 45) as i64,
            referred_by,
            timestamp_for_offset(id as i32, 9, 15),
            timestamp_for_offset(id as i32 + 40, 14, 0),
            if id % 5 == 0 {
                None
            } else {
                Some(timestamp_for_offset(id as i32 + 60, 21, 30))
            },
            statuses[(id - 1) % statuses.len()],
            format!("Customer {} generated for learning joins and filters", id),
        ])
        .map_err(|e| format!("写入 customers 失败: {}", e))?;
    }

    Ok(())
}

fn seed_customer_addresses(tx: &Transaction<'_>) -> Result<(), String> {
    let districts = [
        "Huangpu", "Chaoyang", "Xihu", "Nanshan", "Tianhe", "Gulou", "Yubei", "Heping", "Wuchang",
        "Gusu",
    ];
    let mut stmt = tx
        .prepare(
            "INSERT INTO customer_addresses (
                id, customer_id, label, recipient_name, phone, country, province, city, district,
                postal_code, address_line1, address_line2, is_default_shipping, is_default_billing,
                delivery_notes, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, 'China', ?6, ?7, ?8, ?9, ?10, ?11, 1, 1, ?12, ?13, ?14
            )",
        )
        .map_err(|e| format!("准备 customer_addresses 插入失败: {}", e))?;

    for id in 1..=100 {
        let (city, province) = customer_city_and_province(id);
        stmt.execute(params![
            id as i64,
            id as i64,
            if id % 2 == 0 { "Home" } else { "Office" },
            customer_display_name(id),
            format!("139{:08}", 20000000 + id as i32),
            province,
            city,
            districts[(id - 1) % districts.len()],
            format!("{:06}", 200000 + id as i32),
            format!("{} Learning Road No. {}", city, id),
            if id % 3 == 0 {
                Some(format!("Building {}-{:02}", (id % 12) + 1, (id % 18) + 1))
            } else {
                None
            },
            format!("Leave package at service desk {}", (id % 5) + 1),
            timestamp_for_offset(id as i32, 10, 0),
            timestamp_for_offset(id as i32 + 35, 18, 15),
        ])
        .map_err(|e| format!("写入 customer_addresses 失败: {}", e))?;
    }

    Ok(())
}

fn seed_suppliers(tx: &Transaction<'_>) -> Result<(), String> {
    let categories = ["Hardware", "Accessory", "Display", "Audio", "Networking"];
    let tiers = ["Strategic", "Core", "Growth", "Backup"];
    let provinces = ["Shanghai", "Guangdong", "Zhejiang", "Jiangsu", "Beijing"];

    let mut stmt = tx
        .prepare(
            "INSERT INTO suppliers (
                id, supplier_code, name, category, contact_name, contact_email, contact_phone,
                city, province, country, tier, lead_time_days, rating, active_contract,
                payment_terms, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'China', ?10, ?11, ?12, ?13, ?14, ?15, ?16
            )",
        )
        .map_err(|e| format!("准备 suppliers 插入失败: {}", e))?;

    for id in 1..=100 {
        let province = provinces[(id - 1) % provinces.len()];
        stmt.execute(params![
            id as i64,
            format!("SUP-{id:04}"),
            format!("{} Supply {}", categories[(id - 1) % categories.len()], id),
            categories[(id - 1) % categories.len()],
            format!("Supplier Contact {}", id),
            format!("supplier{:03}@partners.gridix.dev", id),
            format!("021-6{:07}", 1000000 + id as i32),
            supplier_city(id),
            province,
            tiers[(id - 1) % tiers.len()],
            3 + (id % 12) as i64,
            round2(3.2 + (id % 18) as f64 * 0.1),
            if id % 11 == 0 { 0 } else { 1 },
            ["Net 15", "Net 30", "Net 45"][(id - 1) % 3],
            timestamp_for_offset(id as i32 - 20, 8, 45),
            timestamp_for_offset(id as i32 + 45, 16, 30),
        ])
        .map_err(|e| format!("写入 suppliers 失败: {}", e))?;
    }

    Ok(())
}

fn seed_product_categories(tx: &Transaction<'_>) -> Result<(), String> {
    let departments = [
        "Peripherals",
        "Displays",
        "Audio",
        "Accessories",
        "Storage",
        "Networking",
        "Office",
        "Mobile",
        "Gaming",
        "Components",
    ];

    let mut stmt = tx
        .prepare(
            "INSERT INTO product_categories (
                id, category_code, name, parent_category_id, department, merchandising_group,
                tax_rate, shelf_life_days, requires_serial, hazardous_level, display_order,
                seo_slug, description, active, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
            )",
        )
        .map_err(|e| format!("准备 product_categories 插入失败: {}", e))?;

    for id in 1..=100 {
        let department = departments[(id - 1) % departments.len()];
        let parent_id = if id <= 10 {
            None
        } else {
            Some(((id - 1) % 10 + 1) as i64)
        };
        let name = if id <= 10 {
            department.to_string()
        } else {
            format!("{department} Series {:02}", id - 10)
        };
        stmt.execute(params![
            id as i64,
            format!("CAT-{id:04}"),
            name,
            parent_id,
            department,
            format!("{} Group {}", department, (id % 4) + 1),
            round2(0.05 + (id % 4) as f64 * 0.02),
            365 + (id % 90) as i64,
            if id % 5 == 0 { 1 } else { 0 },
            (id % 3) as i64,
            id as i64,
            format!("{}-{:03}", department.to_lowercase(), id),
            format!(
                "Category {} for hierarchical browsing and aggregate examples",
                id
            ),
            if id % 17 == 0 { 0 } else { 1 },
            timestamp_for_offset(id as i32 - 30, 9, 0),
            timestamp_for_offset(id as i32 + 20, 11, 30),
        ])
        .map_err(|e| format!("写入 product_categories 失败: {}", e))?;
    }

    Ok(())
}

fn seed_products(tx: &Transaction<'_>) -> Result<(), String> {
    let colors = ["Black", "White", "Gray", "Blue", "Orange", "Silver"];
    let sizes = ["S", "M", "L", "XL"];
    let materials = ["ABS", "Aluminum", "Steel", "Plastic", "Carbon Fiber"];

    let mut stmt = tx
        .prepare(
            "INSERT INTO products (
                id, sku, name, category, category_id, supplier_id, price, cost, stock_qty,
                reorder_level, weight_kg, color, size_label, material, warranty_months,
                discontinued, launch_date, rating, barcode, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20, ?21
            )",
        )
        .map_err(|e| format!("准备 products 插入失败: {}", e))?;

    for id in 1..=100 {
        let category_id = id as i64;
        let supplier_id = ((id - 1) % 100 + 1) as i64;
        let (name, category, price) = legacy_or_generated_product(id);
        let cost = round2(price * (0.55 + (id % 5) as f64 * 0.04));
        stmt.execute(params![
            id as i64,
            format!("SKU-{id:05}"),
            name,
            category,
            category_id,
            supplier_id,
            price,
            cost,
            30 + (id % 170) as i64,
            10 + (id % 25) as i64,
            round2(0.35 + (id % 9) as f64 * 0.18),
            colors[(id - 1) % colors.len()],
            sizes[(id - 1) % sizes.len()],
            materials[(id - 1) % materials.len()],
            12 + (id % 24) as i64,
            if id % 29 == 0 { 1 } else { 0 },
            date_for_offset(id as i32 - 60),
            round2(3.5 + (id % 15) as f64 * 0.1),
            format!("6901234{:06}", id),
            timestamp_for_offset(id as i32 - 40, 9, 20),
            timestamp_for_offset(id as i32 + 18, 17, 10),
        ])
        .map_err(|e| format!("写入 products 失败: {}", e))?;
    }

    Ok(())
}

fn seed_orders_and_order_items(tx: &Transaction<'_>) -> Result<(), String> {
    let mut order_stmt = tx
        .prepare(
            "INSERT INTO orders (
                id, customer_id, shipping_address_id, billing_address_id, order_date, status,
                payment_status, fulfillment_status, source_channel, coupon_code, total_amount,
                shipping_fee, tax_amount, discount_amount, shipped_at, delivered_at,
                priority_level, remarks, created_at, updated_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                ?18, ?19, ?20
            )",
        )
        .map_err(|e| format!("准备 orders 插入失败: {}", e))?;
    let mut item_stmt = tx
        .prepare(
            "INSERT INTO order_items (
                id, order_id, product_id, line_number, quantity, unit_price, discount_amount,
                tax_amount, line_total, fulfillment_status, requested_ship_date, shipped_at,
                return_requested, gift_wrap, note, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
            )",
        )
        .map_err(|e| format!("准备 order_items 插入失败: {}", e))?;

    let mut item_id = 1_i64;
    for offset in 1..=100 {
        let order_id = 1000 + offset as i64;
        let customer_id = offset as i64;
        let status = order_status(offset);
        let payment_status = order_payment_status(status);
        let fulfillment_status = order_fulfillment_status(status);
        let order_date = date_for_offset(70 + offset as i32);
        let shipped_at = order_shipped_at(offset, status);
        let delivered_at = order_delivered_at(offset, status);
        let shipping_fee = round2(8.0 + (offset % 4) as f64 * 4.0);
        let order_discount = if offset % 6 == 0 { 12.0 } else { 0.0 };
        let priority = ["LOW", "NORMAL", "HIGH", "URGENT"][(offset - 1) % 4];
        let coupon = if offset % 5 == 0 {
            Some(format!("SAVE{:02}", offset % 20))
        } else {
            None
        };

        let mut pending_items = Vec::new();
        let mut tax_sum = 0.0;
        let mut line_sum = 0.0;
        for line_number in 1..=2 {
            let product_id = (((offset - 1) * 2 + (line_number - 1)) % 100 + 1) as i64;
            let quantity = ((offset + line_number) % 3 + 1) as i64;
            let unit_price = product_price(product_id as usize);
            let discount_amount = if (offset + line_number) % 7 == 0 {
                round2(5.0 * line_number as f64)
            } else {
                0.0
            };
            let taxable = (unit_price * quantity as f64 - discount_amount).max(0.0);
            let tax_amount = round2(taxable * 0.06);
            let line_total = round2(taxable + tax_amount);
            tax_sum += tax_amount;
            line_sum += line_total;
            pending_items.push((
                item_id,
                product_id,
                line_number as i64,
                quantity,
                unit_price,
                discount_amount,
                tax_amount,
                line_total,
                date_for_offset(72 + offset as i32 + line_number as i32),
                if status == "DELIVERED" && offset % 11 == 0 {
                    1
                } else {
                    0
                },
                if (offset + line_number) % 4 == 0 {
                    1
                } else {
                    0
                },
                format!(
                    "Order {} item {} for multi-table join practice",
                    order_id, line_number
                ),
                timestamp_for_offset(70 + offset as i32, 10 + line_number as i32, 5),
                shipped_at.clone(),
            ));
            item_id += 1;
        }

        let total_amount = round2(line_sum + shipping_fee - order_discount);
        order_stmt
            .execute(params![
                order_id,
                customer_id,
                customer_id,
                customer_id,
                order_date,
                status,
                payment_status,
                fulfillment_status,
                ["Web", "Mobile", "Sales", "Marketplace"][(offset - 1) % 4],
                coupon,
                total_amount,
                shipping_fee,
                round2(tax_sum),
                round2(order_discount),
                shipped_at,
                delivered_at,
                priority,
                format!(
                    "Order {} generated for UPDATE/DELETE/transaction lessons",
                    order_id
                ),
                timestamp_for_offset(70 + offset as i32, 9, 0),
                timestamp_for_offset(72 + offset as i32, 16, 45),
            ])
            .map_err(|e| format!("写入 orders 失败: {}", e))?;

        for (
            item_id,
            product_id,
            line_number,
            quantity,
            unit_price,
            discount_amount,
            tax_amount,
            line_total,
            requested_ship_date,
            return_requested,
            gift_wrap,
            note,
            created_at,
            shipped_at,
        ) in pending_items
        {
            item_stmt
                .execute(params![
                    item_id,
                    order_id,
                    product_id,
                    line_number,
                    quantity,
                    unit_price,
                    discount_amount,
                    tax_amount,
                    line_total,
                    fulfillment_status,
                    requested_ship_date,
                    shipped_at,
                    return_requested,
                    gift_wrap,
                    note,
                    created_at,
                ])
                .map_err(|e| format!("写入 order_items 失败: {}", e))?;
        }
    }

    Ok(())
}

fn seed_payments(tx: &Transaction<'_>) -> Result<(), String> {
    let methods = ["Card", "Transfer", "Wallet", "Invoice"];
    let providers = ["UnionPay", "Stripe", "Alipay", "WeChat Pay"];

    let mut stmt = tx
        .prepare(
            "INSERT INTO payments (
                id, order_id, customer_id, payment_number, method, provider, currency, amount,
                fee_amount, status, paid_at, settled_at, refunded_at, installments, captured_by,
                reference_code, created_at
            ) VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, 'CNY', ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16
            )",
        )
        .map_err(|e| format!("准备 payments 插入失败: {}", e))?;

    for offset in 1..=100 {
        let order_id = 1000 + offset as i64;
        let status = order_status(offset);
        let total_amount = generated_order_total(offset);
        let payment_status = match status {
            "CANCELLED" => "REFUNDED",
            "CREATED" | "PROCESSING" => "PENDING",
            _ => "CAPTURED",
        };
        let paid_at = if matches!(payment_status, "PENDING") {
            None
        } else {
            Some(timestamp_for_offset(71 + offset as i32, 11, 20))
        };
        let settled_at = if payment_status == "CAPTURED" {
            Some(timestamp_for_offset(72 + offset as i32, 15, 0))
        } else {
            None
        };
        let refunded_at = if payment_status == "REFUNDED" {
            Some(timestamp_for_offset(74 + offset as i32, 10, 10))
        } else {
            None
        };

        stmt.execute(params![
            offset as i64,
            order_id,
            offset as i64,
            format!("PAY-{order_id}"),
            methods[(offset - 1) % methods.len()],
            providers[(offset - 1) % providers.len()],
            total_amount,
            round2(total_amount * 0.015),
            payment_status,
            paid_at,
            settled_at,
            refunded_at,
            (offset % 3 + 1) as i64,
            format!("ops_user_{:02}", (offset % 8) + 1),
            format!("REF-{order_id}-{:03}", offset),
            timestamp_for_offset(70 + offset as i32, 9, 40),
        ])
        .map_err(|e| format!("写入 payments 失败: {}", e))?;
    }

    Ok(())
}

fn customer_display_name(id: usize) -> String {
    match id {
        1 => "Alice Zhang".to_string(),
        2 => "Bob Chen".to_string(),
        3 => "Carol Lin".to_string(),
        4 => "David Wu".to_string(),
        5 => "Eva Sun".to_string(),
        6 => "Frank Liu".to_string(),
        _ => format!("Customer {:03}", id),
    }
}

fn customer_city_and_province(id: usize) -> (&'static str, &'static str) {
    let cities = [
        ("Shanghai", "Shanghai"),
        ("Beijing", "Beijing"),
        ("Hangzhou", "Zhejiang"),
        ("Shenzhen", "Guangdong"),
        ("Guangzhou", "Guangdong"),
        ("Nanjing", "Jiangsu"),
        ("Chongqing", "Chongqing"),
        ("Tianjin", "Tianjin"),
        ("Wuhan", "Hubei"),
        ("Suzhou", "Jiangsu"),
    ];
    cities[(id - 1) % cities.len()]
}

fn supplier_city(id: usize) -> &'static str {
    ["Shanghai", "Shenzhen", "Hangzhou", "Suzhou", "Guangzhou"][(id - 1) % 5]
}

fn legacy_or_generated_product(id: usize) -> (String, String, f64) {
    match id {
        1 => (
            "Mechanical Keyboard".to_string(),
            "Peripheral".to_string(),
            129.0,
        ),
        2 => ("Wireless Mouse".to_string(), "Peripheral".to_string(), 89.0),
        3 => ("USB-C Dock".to_string(), "Accessory".to_string(), 159.0),
        4 => ("27-inch Monitor".to_string(), "Display".to_string(), 999.0),
        5 => ("Laptop Stand".to_string(), "Accessory".to_string(), 79.0),
        6 => (
            "Noise-canceling Headset".to_string(),
            "Audio".to_string(),
            249.0,
        ),
        _ => {
            let prefixes = [
                "Smart", "Portable", "Compact", "Pro", "Air", "Ultra", "Studio", "Flex",
            ];
            let nouns = [
                "Router", "Hub", "Adapter", "Camera", "Tablet", "Speaker", "Keyboard", "Dock",
            ];
            let categories = [
                "Networking",
                "Accessory",
                "Display",
                "Office",
                "Mobile",
                "Audio",
                "Peripheral",
                "Gaming",
            ];
            (
                format!(
                    "{} {} {}",
                    prefixes[(id - 1) % prefixes.len()],
                    nouns[(id - 1) % nouns.len()],
                    id
                ),
                categories[(id - 1) % categories.len()].to_string(),
                round2(59.0 + id as f64 * 7.35),
            )
        }
    }
}

fn product_price(id: usize) -> f64 {
    legacy_or_generated_product(id).2
}

fn order_status(offset: usize) -> &'static str {
    match offset {
        4 => "CREATED",
        6 => "CANCELLED",
        _ => match offset % 5 {
            0 => "DELIVERED",
            1 => "PAID",
            2 => "SHIPPED",
            3 => "PROCESSING",
            _ => "CREATED",
        },
    }
}

fn order_payment_status(status: &str) -> &'static str {
    match status {
        "CANCELLED" => "REFUNDED",
        "CREATED" | "PROCESSING" => "PENDING",
        _ => "PAID",
    }
}

fn order_fulfillment_status(status: &str) -> &'static str {
    match status {
        "CREATED" => "QUEUED",
        "PROCESSING" => "PICKING",
        "PAID" => "READY",
        "SHIPPED" => "IN_TRANSIT",
        "DELIVERED" => "DELIVERED",
        "CANCELLED" => "CANCELLED",
        _ => "QUEUED",
    }
}

fn order_shipped_at(offset: usize, status: &str) -> Option<String> {
    if matches!(status, "SHIPPED" | "DELIVERED") {
        Some(timestamp_for_offset(72 + offset as i32, 14, 0))
    } else {
        None
    }
}

fn order_delivered_at(offset: usize, status: &str) -> Option<String> {
    if status == "DELIVERED" {
        Some(timestamp_for_offset(75 + offset as i32, 18, 20))
    } else {
        None
    }
}

fn generated_order_total(offset: usize) -> f64 {
    let shipping_fee = round2(8.0 + (offset % 4) as f64 * 4.0);
    let order_discount = if offset.is_multiple_of(6) { 12.0 } else { 0.0 };
    let mut line_sum = 0.0;
    for line_number in 1..=2 {
        let product_id = ((offset - 1) * 2 + (line_number - 1)) % 100 + 1;
        let quantity = ((offset + line_number) % 3 + 1) as i64;
        let unit_price = product_price(product_id);
        let discount_amount = if (offset + line_number).is_multiple_of(7) {
            round2(5.0 * line_number as f64)
        } else {
            0.0
        };
        let taxable = (unit_price * quantity as f64 - discount_amount).max(0.0);
        let tax_amount = round2(taxable * 0.06);
        line_sum += round2(taxable + tax_amount);
    }
    round2(line_sum + shipping_fee - order_discount)
}

fn date_for_offset(offset: i32) -> String {
    let month = ((offset.max(1) - 1) / 28) % 12 + 1;
    let day = ((offset.max(1) - 1) % 28) + 1;
    format!("2026-{month:02}-{day:02}")
}

fn timestamp_for_offset(offset: i32, hour: i32, minute: i32) -> String {
    format!("{} {:02}:{:02}:00", date_for_offset(offset), hour, minute)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::{
        LEARNING_DB_SCHEMA_VERSION, LEARNING_MIN_ROWS_PER_TABLE, LEARNING_REQUIRED_TABLES,
        learning_database_ready, seed_learning_database,
    };
    use rusqlite::Connection as SqliteConn;
    use tempfile::tempdir;

    #[test]
    fn seeded_learning_database_is_versioned_and_ready() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("learning.sqlite3");

        seed_learning_database(&path).unwrap();

        assert!(learning_database_ready(&path).unwrap());

        let conn = SqliteConn::open(&path).unwrap();
        let version: i64 = conn
            .query_row(
                "SELECT schema_version FROM gridix_learning_meta LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(version, LEARNING_DB_SCHEMA_VERSION);
    }

    #[test]
    fn seeded_learning_database_has_large_relational_dataset() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("learning.sqlite3");

        seed_learning_database(&path).unwrap();

        let conn = SqliteConn::open(&path).unwrap();
        for table in LEARNING_REQUIRED_TABLES {
            let row_count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert!(
                row_count >= LEARNING_MIN_ROWS_PER_TABLE,
                "{table} should have at least {LEARNING_MIN_ROWS_PER_TABLE} rows"
            );

            let column_count: i64 = conn
                .query_row(
                    &format!("SELECT COUNT(*) FROM pragma_table_info('{table}')"),
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(
                column_count >= 15,
                "{table} should have at least 15 columns"
            );
        }

        let fk_errors: i64 = conn
            .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(fk_errors, 0);
    }

    #[test]
    fn legacy_learning_database_is_treated_as_not_ready() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("legacy-learning.sqlite3");
        let conn = SqliteConn::open(&path).unwrap();

        conn.execute_batch(
            r#"
            CREATE TABLE customers (
                id INTEGER PRIMARY KEY,
                name TEXT NOT NULL,
                city TEXT NOT NULL,
                level TEXT NOT NULL
            );
            INSERT INTO customers (id, name, city, level)
            VALUES (1, 'Alice Zhang', 'Shanghai', 'Gold');
            "#,
        )
        .unwrap();

        assert!(!learning_database_ready(&path).unwrap());
    }

    #[test]
    fn deleting_demo_order_cascades_to_learning_children() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("learning.sqlite3");

        seed_learning_database(&path).unwrap();

        let conn = SqliteConn::open(&path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        conn.execute_batch("DELETE FROM orders WHERE id = 1006;")
            .unwrap();

        let order_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders WHERE id = 1006", [], |row| {
                row.get(0)
            })
            .unwrap();
        let item_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM order_items WHERE order_id = 1006",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let payment_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM payments WHERE order_id = 1006",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(order_count, 0);
        assert_eq!(item_count, 0);
        assert_eq!(payment_count, 0);
    }
}
