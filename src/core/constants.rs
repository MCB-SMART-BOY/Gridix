//! 应用程序常量定义
//!
//! 集中管理所有魔法数字和配置常量

#![allow(dead_code)] // 预留常量供将来使用

/// 历史记录相关常量
pub mod history {
    /// 每个连接的最大命令历史记录数
    pub const MAX_COMMAND_HISTORY_PER_CONNECTION: usize = 100;
    /// 全局查询历史最大条数
    pub const MAX_QUERY_HISTORY: usize = 100;
}

/// 数据库相关常量
pub mod database {
    /// 连接超时时间（秒）
    pub const CONNECTION_TIMEOUT_SECS: u64 = 30;
    /// SSH 隧道建立超时时间（秒）
    pub const SSH_TUNNEL_TIMEOUT_SECS: u64 = 30;
    /// 查询超时时间（秒）
    pub const QUERY_TIMEOUT_SECS: u64 = 300;
    /// 默认查询限制行数
    pub const DEFAULT_QUERY_LIMIT: usize = 100;
    /// 大结果集警告阈值
    pub const LARGE_RESULT_SET_WARNING_THRESHOLD: usize = 10000;
    /// 最大结果集行数限制（防止内存溢出）
    pub const MAX_RESULT_SET_ROWS: usize = 100000;
}

/// UI 相关常量
pub mod ui {
    /// UI 缩放最小值
    pub const UI_SCALE_MIN: f32 = 0.5;
    /// UI 缩放最大值
    pub const UI_SCALE_MAX: f32 = 2.0;
    /// 连接名称最大长度
    pub const CONNECTION_NAME_MAX_LENGTH: usize = 64;
    /// 用户名最大长度
    pub const USERNAME_MAX_LENGTH: usize = 128;
    /// 主机地址最大长度
    pub const HOST_MAX_LENGTH: usize = 255;
    
    /// 侧边栏默认宽度比例
    pub const SIDEBAR_DEFAULT_WIDTH_RATIO: f32 = 0.18;
    /// 侧边栏最小宽度比例
    pub const SIDEBAR_MIN_WIDTH_RATIO: f32 = 0.12;
    /// 侧边栏最大宽度比例
    pub const SIDEBAR_MAX_WIDTH_RATIO: f32 = 0.25;
    /// 侧边栏最小宽度（像素）
    pub const SIDEBAR_MIN_WIDTH_PX: f32 = 180.0;
    /// 侧边栏最大宽度（像素）
    pub const SIDEBAR_MAX_WIDTH_PX: f32 = 380.0;
}

/// 自动补全相关常量
pub mod autocomplete {
    /// 最大补全建议数量
    pub const MAX_COMPLETIONS: usize = 15;
    /// 最大缓存表数量
    pub const MAX_CACHED_TABLES: usize = 500;
    /// 每个表最大缓存列数量
    pub const MAX_CACHED_COLUMNS_PER_TABLE: usize = 200;
}

/// 显示相关常量
pub mod display {
    /// SQL 错误预览最大长度
    pub const SQL_ERROR_PREVIEW_MAX_LENGTH: usize = 200;
    /// 单元格内容截断长度
    pub const CELL_CONTENT_TRUNCATE_LENGTH: usize = 50;
}

/// 数据表格相关常量
pub mod grid {
    /// 文本高度
    pub const TEXT_HEIGHT: f32 = 20.0;
    /// 行高度
    pub const ROW_HEIGHT: f32 = TEXT_HEIGHT + 8.0;
    /// 表头高度
    pub const HEADER_HEIGHT: f32 = 28.0;
    /// 列最小宽度
    pub const MIN_COL_WIDTH: f32 = 60.0;
    /// 列最大宽度
    pub const MAX_COL_WIDTH: f32 = 300.0;
    /// 字符宽度估算
    pub const CHAR_WIDTH: f32 = 8.0;
    /// 单元格内容截断长度
    pub const CELL_TRUNCATE_LEN: usize = 50;
}

/// 错误消息常量（统一中文错误提示）
pub mod messages {
    // 连接相关
    pub const CONNECTION_TIMEOUT: &str = "连接超时";
    pub const CONNECTION_FAILED: &str = "连接失败";
    pub const QUERY_TIMEOUT: &str = "查询超时";
    pub const NO_DATABASE_CONNECTED: &str = "请先连接数据库";
    
    // 操作相关
    pub const NO_CHANGES_TO_SAVE: &str = "没有需要保存的修改";
    pub const CHANGES_DISCARDED: &str = "已放弃所有修改";
    
    // 验证相关
    pub const IDENTIFIER_EMPTY: &str = "标识符不能为空";
    pub const IDENTIFIER_INVALID_CHAR: &str = "标识符包含非法字符";
    
    // 通道相关
    pub const CHANNEL_CLOSED: &str = "接收端已关闭";
}
