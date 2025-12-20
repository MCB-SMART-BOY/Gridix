//! 数据库驱动抽象
//!
//! 提供统一的数据库操作接口，便于支持多种数据库类型和未来扩展。

#![allow(dead_code)] // 公开 API，供未来使用

use async_trait::async_trait;
use super::{ConnectionConfig, DatabaseType, DbError, QueryResult, TriggerInfo, ForeignKeyInfo};

/// 列信息
#[derive(Debug, Clone)]
pub struct ColumnMeta {
    /// 列名
    pub name: String,
    /// 数据类型
    pub data_type: String,
    /// 是否可为空
    pub nullable: bool,
    /// 是否为主键
    pub is_primary_key: bool,
    /// 默认值
    pub default_value: Option<String>,
}

/// 表信息
#[derive(Debug, Clone)]
pub struct TableMeta {
    /// 表名
    pub name: String,
    /// 列信息
    pub columns: Vec<ColumnMeta>,
    /// 主键列名
    pub primary_key: Option<String>,
    /// 行数估计（可能不精确）
    pub row_count_estimate: Option<u64>,
}

/// 数据库连接结果
#[derive(Debug)]
pub enum ConnectResultType {
    /// SQLite 模式：直接返回表列表
    Tables(Vec<String>),
    /// MySQL/PostgreSQL 模式：返回数据库列表
    Databases(Vec<String>),
}

/// 数据库驱动 trait
///
/// 定义统一的数据库操作接口，所有数据库驱动都应实现此 trait。
#[async_trait]
pub trait DatabaseDriver: Send + Sync {
    /// 获取数据库类型
    fn db_type(&self) -> DatabaseType;

    /// 连接数据库
    ///
    /// 根据数据库类型，返回表列表（SQLite）或数据库列表（MySQL/PostgreSQL）
    async fn connect(&self, config: &ConnectionConfig) -> Result<ConnectResultType, DbError>;

    /// 断开连接
    async fn disconnect(&self, config: &ConnectionConfig) -> Result<(), DbError>;

    /// 执行查询
    async fn execute(&self, config: &ConnectionConfig, sql: &str) -> Result<QueryResult, DbError>;

    /// 获取数据库列表（MySQL/PostgreSQL）
    async fn list_databases(&self, config: &ConnectionConfig) -> Result<Vec<String>, DbError>;

    /// 获取表列表
    async fn list_tables(&self, config: &ConnectionConfig, database: Option<&str>) -> Result<Vec<String>, DbError>;

    /// 获取表结构
    async fn describe_table(&self, config: &ConnectionConfig, table: &str) -> Result<TableMeta, DbError>;

    /// 获取主键列名
    async fn get_primary_key(&self, config: &ConnectionConfig, table: &str) -> Result<Option<String>, DbError>;

    /// 获取外键关系
    async fn get_foreign_keys(&self, config: &ConnectionConfig) -> Result<Vec<ForeignKeyInfo>, DbError>;

    /// 获取触发器列表
    async fn get_triggers(&self, config: &ConnectionConfig) -> Result<Vec<TriggerInfo>, DbError>;

    /// 引用标识符（表名、列名等）
    ///
    /// 不同数据库使用不同的引用字符：
    /// - SQLite/MySQL: `backtick`
    /// - PostgreSQL: "双引号"
    fn quote_identifier(&self, name: &str) -> String;

    /// 获取限制查询的 SQL 语法
    ///
    /// 用于生成 "SELECT ... LIMIT n" 类型的查询
    fn limit_clause(&self, limit: usize) -> String {
        format!("LIMIT {}", limit)
    }

    /// 获取分页查询的 SQL 语法
    fn offset_clause(&self, offset: usize, limit: usize) -> String {
        format!("LIMIT {} OFFSET {}", limit, offset)
    }

    /// 检查是否支持事务
    fn supports_transactions(&self) -> bool {
        true
    }

    /// 检查是否支持用户管理
    fn supports_user_management(&self) -> bool {
        !matches!(self.db_type(), DatabaseType::SQLite)
    }

    /// 获取 NULL 值的显示文本
    fn null_display(&self) -> &'static str {
        "NULL"
    }

    /// 转义字符串值（用于 SQL 构建）
    fn escape_string(&self, value: &str) -> String {
        // 基本的 SQL 字符串转义
        value.replace('\'', "''")
    }

    /// 获取数据库版本查询语句
    fn version_query(&self) -> &'static str {
        match self.db_type() {
            DatabaseType::SQLite => "SELECT sqlite_version()",
            DatabaseType::PostgreSQL => "SELECT version()",
            DatabaseType::MySQL => "SELECT version()",
        }
    }
}

/// 驱动注册表
///
/// 用于管理和查找数据库驱动实例。
pub struct DriverRegistry {
    drivers: Vec<Box<dyn DatabaseDriver>>,
}

impl DriverRegistry {
    /// 创建新的驱动注册表
    pub fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    /// 注册驱动
    pub fn register(&mut self, driver: Box<dyn DatabaseDriver>) {
        self.drivers.push(driver);
    }

    /// 根据数据库类型获取驱动
    pub fn get(&self, db_type: DatabaseType) -> Option<&dyn DatabaseDriver> {
        self.drivers
            .iter()
            .find(|d| d.db_type() == db_type)
            .map(|d| d.as_ref())
    }

    /// 获取所有已注册的驱动类型
    pub fn registered_types(&self) -> Vec<DatabaseType> {
        self.drivers.iter().map(|d| d.db_type()).collect()
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// 驱动信息
///
/// 提供驱动的元数据信息。
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// 驱动名称
    pub name: String,
    /// 驱动版本
    pub version: String,
    /// 支持的数据库类型
    pub db_type: DatabaseType,
    /// 驱动描述
    pub description: String,
}

impl DriverInfo {
    /// 创建新的驱动信息
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        db_type: DatabaseType,
        description: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
            db_type,
            description: description.into(),
        }
    }
}

/// 驱动能力标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DriverCapabilities {
    /// 支持事务
    pub transactions: bool,
    /// 支持存储过程
    pub stored_procedures: bool,
    /// 支持触发器
    pub triggers: bool,
    /// 支持视图
    pub views: bool,
    /// 支持外键
    pub foreign_keys: bool,
    /// 支持用户管理
    pub user_management: bool,
    /// 支持数据库创建
    pub database_creation: bool,
    /// 支持批量插入
    pub batch_insert: bool,
}

impl DriverCapabilities {
    /// SQLite 的默认能力
    pub const SQLITE: Self = Self {
        transactions: true,
        stored_procedures: false,
        triggers: true,
        views: true,
        foreign_keys: true,
        user_management: false,
        database_creation: false,
        batch_insert: true,
    };

    /// PostgreSQL 的默认能力
    pub const POSTGRESQL: Self = Self {
        transactions: true,
        stored_procedures: true,
        triggers: true,
        views: true,
        foreign_keys: true,
        user_management: true,
        database_creation: true,
        batch_insert: true,
    };

    /// MySQL 的默认能力
    pub const MYSQL: Self = Self {
        transactions: true,
        stored_procedures: true,
        triggers: true,
        views: true,
        foreign_keys: true,
        user_management: true,
        database_creation: true,
        batch_insert: true,
    };

    /// 根据数据库类型获取默认能力
    pub fn for_db_type(db_type: DatabaseType) -> Self {
        match db_type {
            DatabaseType::SQLite => Self::SQLITE,
            DatabaseType::PostgreSQL => Self::POSTGRESQL,
            DatabaseType::MySQL => Self::MYSQL,
        }
    }
}

impl Default for DriverCapabilities {
    fn default() -> Self {
        Self::SQLITE
    }
}

