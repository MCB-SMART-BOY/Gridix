//! 导入相关类型定义

use std::path::PathBuf;

/// 导入格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportFormat {
    #[default]
    Sql,
    Csv,
    Tsv,
    Json,
}

impl ImportFormat {
    pub fn previous(self) -> Self {
        match self {
            ImportFormat::Sql => ImportFormat::Json,
            ImportFormat::Csv => ImportFormat::Sql,
            ImportFormat::Tsv => ImportFormat::Csv,
            ImportFormat::Json => ImportFormat::Tsv,
        }
    }

    pub fn next(self) -> Self {
        match self {
            ImportFormat::Sql => ImportFormat::Csv,
            ImportFormat::Csv => ImportFormat::Tsv,
            ImportFormat::Tsv => ImportFormat::Json,
            ImportFormat::Json => ImportFormat::Sql,
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "csv" => ImportFormat::Csv,
            "tsv" | "tab" => ImportFormat::Tsv,
            "json" => ImportFormat::Json,
            _ => ImportFormat::Sql,
        }
    }

    pub fn default_delimiter(self) -> Option<char> {
        match self {
            ImportFormat::Csv => Some(','),
            ImportFormat::Tsv => Some('\t'),
            _ => None,
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ImportFormat::Sql => "📝",
            ImportFormat::Csv => "📊",
            ImportFormat::Tsv => "↹",
            ImportFormat::Json => "🔧",
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ImportFormat::Sql => "SQL",
            ImportFormat::Csv => "CSV",
            ImportFormat::Tsv => "TSV",
            ImportFormat::Json => "JSON",
        }
    }
}

/// 导入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImportMode {
    /// 直接执行 SQL（逐条执行）
    #[default]
    Execute,
    /// 复制到编辑器
    CopyToEditor,
}

/// SQL 导入配置
#[derive(Debug, Clone)]
pub struct SqlImportConfig {
    /// 忽略注释（-- 和 /* */）
    pub strip_comments: bool,
    /// 忽略空行
    pub strip_empty_lines: bool,
    /// 遇到错误时停止
    pub stop_on_error: bool,
    /// 使用事务包装
    pub use_transaction: bool,
}

impl Default for SqlImportConfig {
    fn default() -> Self {
        Self {
            strip_comments: true,
            strip_empty_lines: true,
            stop_on_error: false,
            use_transaction: false,
        }
    }
}

/// CSV 导入配置
#[derive(Debug, Clone)]
pub struct CsvImportConfig {
    /// 分隔符
    pub delimiter: char,
    /// 跳过的行数（表头之前）
    pub skip_rows: usize,
    /// 第一行是否为表头
    pub has_header: bool,
    /// 目标表名
    pub table_name: String,
    /// 文本引用字符
    pub quote_char: char,
    /// 文件编码（预留功能，用于未来支持非 UTF-8 编码）
    #[allow(dead_code)]
    pub encoding: String,
}

impl Default for CsvImportConfig {
    fn default() -> Self {
        Self {
            delimiter: ',',
            skip_rows: 0,
            has_header: true,
            table_name: String::new(),
            quote_char: '"',
            encoding: "UTF-8".to_string(),
        }
    }
}

/// JSON 导入配置
#[derive(Debug, Clone, Default)]
pub struct JsonImportConfig {
    /// JSON 路径（如 "data.items"）
    pub json_path: String,
    /// 目标表名
    pub table_name: String,
    /// 是否扁平化嵌套对象
    pub flatten_nested: bool,
}

/// 导入预览数据
#[derive(Debug, Clone, Default)]
pub struct ImportPreview {
    /// 列名
    pub columns: Vec<String>,
    /// 预览行数据（最多显示 10 行）
    pub preview_rows: Vec<Vec<String>>,
    /// 总行数
    pub total_rows: usize,
    /// SQL 语句数（仅 SQL 格式）
    pub statement_count: usize,
    /// 警告信息
    pub warnings: Vec<String>,
    /// 解析出的 SQL 语句（SQL 格式）
    pub sql_statements: Vec<String>,
}

/// 导入状态
#[derive(Debug, Clone, Default)]
pub struct ImportState {
    /// 文件路径
    pub file_path: Option<PathBuf>,
    /// 检测到的格式
    pub format: ImportFormat,
    /// 导入模式
    pub mode: ImportMode,
    /// SQL 配置
    pub sql_config: SqlImportConfig,
    /// CSV 配置
    pub csv_config: CsvImportConfig,
    /// JSON 配置
    pub json_config: JsonImportConfig,
    /// 预览数据
    pub preview: Option<ImportPreview>,
    /// 是否正在加载
    pub loading: bool,
    /// 错误信息
    pub error: Option<String>,
}

impl ImportState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_file(&mut self, path: PathBuf) {
        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("sql");

        self.format = ImportFormat::from_extension(ext);

        // 从文件名推断表名
        let table_name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("imported_data")
            .to_string();

        self.csv_config.table_name = table_name.clone();
        self.json_config.table_name = table_name;
        if let Some(delimiter) = self.format.default_delimiter() {
            self.csv_config.delimiter = delimiter;
        }
        self.file_path = Some(path);
        self.preview = None;
        self.error = None;
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::ImportFormat;

    #[test]
    fn import_format_previous_cycles_backward() {
        assert_eq!(ImportFormat::Sql.previous(), ImportFormat::Json);
        assert_eq!(ImportFormat::Csv.previous(), ImportFormat::Sql);
        assert_eq!(ImportFormat::Tsv.previous(), ImportFormat::Csv);
        assert_eq!(ImportFormat::Json.previous(), ImportFormat::Tsv);
    }

    #[test]
    fn import_format_next_cycles_forward() {
        assert_eq!(ImportFormat::Sql.next(), ImportFormat::Csv);
        assert_eq!(ImportFormat::Csv.next(), ImportFormat::Tsv);
        assert_eq!(ImportFormat::Tsv.next(), ImportFormat::Json);
        assert_eq!(ImportFormat::Json.next(), ImportFormat::Sql);
    }

    #[test]
    fn import_format_detects_tsv_extension() {
        assert_eq!(ImportFormat::from_extension("tsv"), ImportFormat::Tsv);
        assert_eq!(ImportFormat::from_extension("TAB"), ImportFormat::Tsv);
        assert_eq!(ImportFormat::from_extension("csv"), ImportFormat::Csv);
    }
}

/// 导入操作结果
#[derive(Debug, Clone, Default)]
pub enum ImportAction {
    /// 无操作
    #[default]
    None,
    /// 选择文件
    SelectFile,
    /// 刷新预览
    RefreshPreview,
    /// 执行导入
    Execute,
    /// 复制到编辑器
    CopyToEditor(String),
    /// 关闭对话框
    Close,
}
