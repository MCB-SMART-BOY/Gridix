//! 对话框组件
//!
//! 所有对话框统一支持 Helix 风格键盘导航：
//! - `Esc` - 关闭/取消
//! - `Enter` - 确认/提交
//! - `j/k` - 列表上下导航
//! - `h/l` - 选项左右切换
//! - `Space` - 切换选中状态
//! - `g/G` - 跳到开头/结尾
//! - `1-9` - 数字键快速选择

mod about_dialog;
mod common;
mod confirm_dialog;
mod connection_dialog;
mod create_db_dialog;
mod create_user_dialog;
mod ddl_dialog;
mod dialog_trait;
mod export_dialog;
mod help_dialog;
mod import_dialog;
pub mod keyboard;

pub use about_dialog::AboutDialog;
pub use confirm_dialog::ConfirmDialog;
pub use connection_dialog::ConnectionDialog;
pub use create_db_dialog::{CreateDbDialog, CreateDbDialogResult, CreateDbDialogState};
pub use create_user_dialog::{CreateUserDialog, CreateUserDialogResult, CreateUserDialogState};
pub use ddl_dialog::{ColumnDefinition, ColumnType, DdlDialog, DdlDialogState, TableDefinition};
pub use export_dialog::{ExportConfig, ExportDialog};
pub use help_dialog::HelpDialog;
pub use import_dialog::{
    parse_sql_file, ImportAction, ImportDialog, ImportFormat, ImportPreview, ImportState,
};
#[allow(unused_imports)] // 公开 API，供未来使用
pub use dialog_trait::{
    DataDialogState, DialogButtons, DialogResult, DialogSize, DialogState, SimpleDialogState,
};
#[allow(unused_imports)] // 公开 API，供未来使用
pub use common::{
    DialogContent, DialogFooter, DialogHeader, DialogStatus, DialogStyle, DialogWindow, FooterResult,
};
// keyboard 模块的类型通过子模块直接使用，无需在此重导出
