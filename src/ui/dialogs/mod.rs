//! 对话框组件
//!
//! 交互快捷键由各对话框自己的局部 action 层负责，
//! 避免所有面板共享一套过于粗糙的全局对话框导航。

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
mod keybindings_dialog;
mod picker_shell;

pub use about_dialog::AboutDialog;
#[allow(unused_imports)] // 公开 API，供未来使用
pub use common::{
    DialogContent, DialogFooter, DialogHeader, DialogShortcutContext, DialogStatus, DialogStyle,
    DialogWindow, FooterResult,
};
pub use confirm_dialog::ConfirmDialog;
pub use connection_dialog::ConnectionDialog;
pub use create_db_dialog::{
    CreateDatabaseRequest, CreateDbDialog, CreateDbDialogResult, CreateDbDialogState,
};
pub use create_user_dialog::{CreateUserDialog, CreateUserDialogResult, CreateUserDialogState};
pub use ddl_dialog::{ColumnDefinition, ColumnType, DdlDialog, DdlDialogState, TableDefinition};
#[allow(unused_imports)] // 公开 API，供未来使用
pub use dialog_trait::{
    DataDialogState, DialogButtons, DialogResult, DialogSize, DialogState, SimpleDialogState,
};
pub use export_dialog::{ExportConfig, ExportDialog};
pub use help_dialog::{
    HelpAction, HelpContext, HelpDialog, HelpOnboardingStep, HelpState, HelpTab, LearningTopic,
};
pub use import_dialog::{
    ImportAction, ImportDialog, ImportFormat, ImportPreview, ImportState, parse_sql_file,
};
pub use keybindings_dialog::{KeyBindingsDialog, KeyBindingsDialogState};
