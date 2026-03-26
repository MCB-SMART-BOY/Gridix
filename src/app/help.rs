//! 帮助面板交互动作
//!
//! 将“数据库知识学习指南”里的按钮动作落到真实应用行为上，
//! 复用现有连接、查询、ER 图渲染逻辑。

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection as SqliteConn;

use crate::core::AppConfig;
use crate::database::{ConnectionConfig, DatabaseType};
use crate::ui;

use super::DbManagerApp;

const LEARNING_CONNECTION_NAME: &str = "Gridix 学习示例";
const LEARNING_DB_FILENAME: &str = "learning.sqlite3";

impl DbManagerApp {
    pub(super) fn handle_help_action(&mut self, action: ui::HelpAction) {
        match action {
            ui::HelpAction::OpenConnectionDialog => {
                self.show_connection_dialog = true;
            }
            ui::HelpAction::EnsureLearningSample { reset } => {
                if let Err(error) = self.ensure_learning_connection(reset, true) {
                    self.notifications.error(error);
                }
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
                self.focus_sql_editor = false;
                self.focus_area = ui::FocusArea::DataGrid;
                self.grid_state.focused = true;
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
        }
    }

    fn ensure_learning_connection(&mut self, reset: bool, notify: bool) -> Result<(), String> {
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
        conn.execute_batch(mutation_sql)
            .map_err(|e| format!("执行学习演示失败: {}", e))?;

        self.notifications.success(success_message);
        self.show_sidebar = true;
        self.sidebar_panel_state.show_connections = true;
        self.sidebar_section = ui::SidebarSection::Tables;
        self.show_sql_editor = true;
        self.focus_sql_editor = false;
        self.focus_area = ui::FocusArea::DataGrid;
        self.grid_state.focused = true;
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
    let table_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type = 'table' AND name IN ('customers', 'products', 'orders', 'order_items')",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查学习示例库失败: {}", e))?;
    let customer_email_column: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('customers') WHERE name = 'email'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查学习示例库列定义失败: {}", e))?;
    let customer_created_at_column: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('customers') WHERE name = 'created_at'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查学习示例库列定义失败: {}", e))?;
    let order_shipped_at_column: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM pragma_table_info('orders') WHERE name = 'shipped_at'",
            [],
            |row| row.get(0),
        )
        .map_err(|e| format!("检查学习示例库列定义失败: {}", e))?;

    Ok(table_count == 4
        && customer_email_column == 1
        && customer_created_at_column == 1
        && order_shipped_at_column == 1)
}

fn seed_learning_database(path: &Path) -> Result<(), String> {
    let conn = SqliteConn::open(path).map_err(|e| format!("创建学习示例库失败: {}", e))?;

    conn.execute_batch(LEARNING_DB_SQL)
        .map_err(|e| format!("初始化学习示例数据失败: {}", e))
}

const LEARNING_DB_SQL: &str = r#"
PRAGMA foreign_keys = ON;

DROP TABLE IF EXISTS order_items;
DROP TABLE IF EXISTS orders;
DROP TABLE IF EXISTS products;
DROP TABLE IF EXISTS customers;

CREATE TABLE customers (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    city TEXT NOT NULL,
    level TEXT NOT NULL,
    email TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE products (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    price REAL NOT NULL
);

CREATE TABLE orders (
    id INTEGER PRIMARY KEY,
    customer_id INTEGER NOT NULL,
    order_date TEXT NOT NULL,
    status TEXT NOT NULL,
    total_amount REAL NOT NULL,
    shipped_at TEXT,
    FOREIGN KEY (customer_id) REFERENCES customers(id)
);

CREATE TABLE order_items (
    id INTEGER PRIMARY KEY,
    order_id INTEGER NOT NULL,
    product_id INTEGER NOT NULL,
    quantity INTEGER NOT NULL,
    unit_price REAL NOT NULL,
    FOREIGN KEY (order_id) REFERENCES orders(id),
    FOREIGN KEY (product_id) REFERENCES products(id)
);

INSERT INTO customers (id, name, city, level, email, created_at) VALUES
    (1, 'Alice Zhang', 'Shanghai', 'Gold', 'alice.zhang@example.com', '2026-01-12 09:30:00'),
    (2, 'Bob Chen', 'Beijing', 'Silver', NULL, '2026-01-18 15:20:00'),
    (3, 'Carol Lin', 'Hangzhou', 'Gold', 'carol.lin@example.com', '2026-02-02 10:10:00'),
    (4, 'David Wu', 'Shenzhen', 'Bronze', NULL, '2026-02-11 18:45:00'),
    (5, 'Eva Sun', 'Guangzhou', 'Gold', 'eva.sun@example.com', '2026-02-20 08:00:00'),
    (6, 'Frank Liu', 'Nanjing', 'Silver', NULL, '2026-03-01 11:05:00');

INSERT INTO products (id, name, category, price) VALUES
    (1, 'Mechanical Keyboard', 'Peripheral', 129.00),
    (2, 'Wireless Mouse', 'Peripheral', 89.00),
    (3, 'USB-C Dock', 'Accessory', 159.00),
    (4, '27-inch Monitor', 'Display', 999.00),
    (5, 'Laptop Stand', 'Accessory', 79.00),
    (6, 'Noise-canceling Headset', 'Audio', 249.00);

INSERT INTO orders (id, customer_id, order_date, status, total_amount, shipped_at) VALUES
    (1001, 1, '2026-03-01', 'PAID', 218.00, NULL),
    (1002, 2, '2026-03-02', 'PAID', 999.00, NULL),
    (1003, 3, '2026-03-03', 'SHIPPED', 408.00, '2026-03-04 14:00:00'),
    (1004, 1, '2026-03-04', 'CREATED', 79.00, NULL),
    (1005, 4, '2026-03-05', 'PAID', 249.00, NULL),
    (1006, 5, '2026-03-06', 'CANCELLED', 89.00, NULL),
    (1007, 6, '2026-03-07', 'SHIPPED', 288.00, '2026-03-08 09:30:00'),
    (1008, 5, '2026-03-08', 'PAID', 1248.00, NULL);

INSERT INTO order_items (id, order_id, product_id, quantity, unit_price) VALUES
    (1, 1001, 1, 1, 129.00),
    (2, 1001, 2, 1, 89.00),
    (3, 1002, 4, 1, 999.00),
    (4, 1003, 2, 1, 89.00),
    (5, 1003, 3, 2, 159.50),
    (6, 1004, 5, 1, 79.00),
    (7, 1005, 6, 1, 249.00),
    (8, 1006, 2, 1, 89.00),
    (9, 1007, 1, 1, 129.00),
    (10, 1007, 5, 2, 79.50),
    (11, 1008, 4, 1, 999.00),
    (12, 1008, 6, 1, 249.00);
"#;
