//! 全局样式常量

#![allow(dead_code)] // 公开 API

use egui::Color32;

// 颜色常量
pub const SUCCESS: Color32 = Color32::from_rgb(82, 196, 106);
pub const DANGER: Color32 = Color32::from_rgb(235, 87, 87);
pub const GRAY: Color32 = Color32::from_rgb(140, 140, 150);
pub const MUTED: Color32 = Color32::from_rgb(100, 100, 110);

// 间距常量 (f32 用于 add_space 等)
pub const SPACING_SM: f32 = 4.0;
pub const SPACING_MD: f32 = 8.0;
pub const SPACING_LG: f32 = 12.0;

// 间距常量 (i8 用于 Margin)
pub const MARGIN_SM: i8 = 4;
pub const MARGIN_MD: i8 = 8;
pub const MARGIN_LG: i8 = 12;
