//! 数据导出功能
//!
//! 提供应用层的导出入口，并将 UI 导出配置适配为统一传输管线。

use crate::core::{plan_export_transfer, write_transfer_plan};
use crate::database::{DatabaseType, QueryResult};
use crate::ui::ExportConfig;
use std::path::Path;

/// 执行导出操作
pub fn execute_export(
    result: &QueryResult,
    table_name: &str,
    path: &Path,
    config: &ExportConfig,
    db_type: DatabaseType,
) -> Result<String, String> {
    let session = config.to_transfer_session(result, table_name, db_type);
    let plan = plan_export_transfer(result, &session)?;
    let exported_rows = plan.total_rows;

    write_transfer_plan(path, &plan)?;

    Ok(format!("已导出 {} 行到 {}", exported_rows, path.display()))
}
