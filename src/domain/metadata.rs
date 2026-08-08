//! 统一 Schema 元数据目录
//!
//! `SchemaCatalog` 是数据库 schema 的唯一权威来源，
//! 替代分散在各处的独立 metadata fetch（triggers/routines/columns/FK）。
//!
//! 通过 `SchemaRevision` 实现 stale-guard：任何 schema 变更（DDL、连接切换）
//! 都会递增 revision，过期的异步回包被丢弃。

use super::ids::SchemaRevision;
use super::value::DbTypeInfo;

/// 完整的数据库 schema 快照
#[derive(Debug, Clone)]
pub struct SchemaCatalog {
    pub revision: SchemaRevision,
    pub tables: Vec<TableMetadata>,
}

/// 单表元数据
#[derive(Debug, Clone)]
pub struct TableMetadata {
    pub name: String,
    /// 所在的 schema（PostgreSQL 的 public 等，SQLite 为空）
    pub schema: Option<String>,
    pub columns: Vec<ColumnMetadata>,
    pub primary_key: Option<KeyMetadata>,
    pub unique_keys: Vec<KeyMetadata>,
    pub foreign_keys: Vec<ForeignKeyMetadata>,
}

/// 列元数据
#[derive(Debug, Clone)]
pub struct ColumnMetadata {
    pub name: String,
    /// 列序号（1-based）
    pub position: usize,
    pub type_info: DbTypeInfo,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub default_value: Option<String>,
}

/// 键（主键或唯一键）
#[derive(Debug, Clone)]
pub struct KeyMetadata {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// 外键
#[derive(Debug, Clone)]
pub struct ForeignKeyMetadata {
    pub name: Option<String>,
    pub from_columns: Vec<String>,
    pub ref_table: String,
    pub ref_columns: Vec<String>,
}

impl SchemaCatalog {
    pub fn empty(revision: SchemaRevision) -> Self {
        Self {
            revision,
            tables: Vec::new(),
        }
    }

    /// 按名称查找表
    pub fn table(&self, name: &str) -> Option<&TableMetadata> {
        self.tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    /// 获取所有表名
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.iter().map(|t| t.name.as_str()).collect()
    }

    /// 获取某个表的所有列名
    pub fn column_names(&self, table_name: &str) -> Vec<&str> {
        self.table(table_name)
            .map(|t| t.columns.iter().map(|c| c.name.as_str()).collect())
            .unwrap_or_default()
    }
}
