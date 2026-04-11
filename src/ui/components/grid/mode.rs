//! 编辑器模式定义

use egui::Color32;

/// 编辑器模式（类似 Helix/Vim）
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum GridMode {
    #[default]
    Normal, // 普通模式：导航、命令
    Insert, // 插入模式：编辑单元格
    Select, // 选择模式：扩展选择
}

impl GridMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Select => "SELECT",
        }
    }

    pub fn color(&self) -> Color32 {
        match self {
            Self::Normal => Color32::from_rgb(130, 170, 255),
            Self::Insert => Color32::from_rgb(150, 220, 130),
            Self::Select => Color32::from_rgb(200, 150, 255),
        }
    }
}
