//! 筛选 UI 组件
//!
//! 提供筛选状态栏的 UI 渲染功能。

use super::condition::ColumnFilter;
use crate::database::QueryResult;
use crate::ui::styles::GRAY;
use egui::{self, Color32, RichText};

/// 筛选栏状态
#[allow(dead_code)] // 公开 API，供外部使用
pub struct FilterBarState {
    pub filters: Vec<ColumnFilter>,
}

/// 显示筛选状态栏（简洁版，只显示筛选数量）
/// 
/// 返回是否有修改（用于使缓存失效）
pub fn show_filter_bar(
    ui: &mut egui::Ui,
    _result: &QueryResult,
    filters: &mut Vec<ColumnFilter>,
) -> bool {
    if filters.is_empty() {
        return false;
    }
    
    let initial_count = filters.len();
    let enabled_count = filters.iter().filter(|f| f.enabled).count();
    
    ui.horizontal(|ui| {
        // 筛选状态
        let status_text = if enabled_count == filters.len() {
            format!("筛选: {} 条", filters.len())
        } else {
            format!("筛选: {}/{} 条", enabled_count, filters.len())
        };
        ui.label(RichText::new(status_text).size(12.0).color(Color32::from_rgb(130, 160, 200)));
        
        // 清空按钮
        if ui
            .add(egui::Label::new(RichText::new("清空").size(11.0).color(GRAY)).sense(egui::Sense::click()))
            .on_hover_text("清空所有筛选条件")
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            filters.clear();
        }
    });
    
    filters.len() != initial_count
}


