//! # Rust 数据库管理器
//!
//! 一个跨平台的数据库管理 GUI 工具，支持 SQLite、PostgreSQL 和 MySQL。
//!
//! ## 功能特性
//!
//! - 多数据库支持：SQLite、PostgreSQL、MySQL
//! - 多行 SQL 编辑器，支持语法高亮
//! - SQL 自动补全和格式化
//! - 查询结果导出 (CSV/SQL/JSON)
//! - 19 种主题预设
//! - 查询历史记录
//!
//! ## 模块结构
//!
//! - `app`: 主应用程序逻辑
//! - `core`: 核心功能（配置、主题、导出等）
//! - `database`: 数据库连接和查询
//! - `ui`: 用户界面组件

mod app;
mod core;
mod database;
mod ui;

use app::DbManagerApp;
use eframe::egui;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// 初始化日志系统
fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("gridix=info,warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

/// 程序入口点
fn main() -> eframe::Result<()> {
    // 初始化日志系统
    init_tracing();
    tracing::info!("Gridix 启动中...");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Rust 数据库管理器"),
        ..Default::default()
    };

    eframe::run_native(
        "Rust 数据库管理器",
        options,
        Box::new(|cc| {
            // 配置字体
            setup_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(DbManagerApp::new(cc)))
        }),
    )
}

/// 内嵌的 Noto Sans SC 字体（思源黑体，支持完整 Unicode）
const EMBEDDED_NOTO_SANS_SC: &[u8] = include_bytes!("../assets/fonts/NotoSansSC-Regular.ttf");

/// 内嵌的 Noto Emoji 字体（支持 Unicode Emoji）
const EMBEDDED_NOTO_EMOJI: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// 配置字体
///
/// 使用 Noto Sans SC（思源黑体）字体，支持完整的 Unicode 字符集，
/// 包括中文、日文、韩文以及各种特殊符号。
/// 同时添加 Noto Emoji 字体作为后备，支持 Unicode Emoji 符号。
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // 使用 Noto Sans SC 字体（支持完整 Unicode）
    fonts.font_data.insert(
        "noto_sans_sc".to_owned(),
        egui::FontData::from_static(EMBEDDED_NOTO_SANS_SC).into(),
    );

    // 使用 Noto Emoji 字体（支持 Emoji）
    fonts.font_data.insert(
        "noto_emoji".to_owned(),
        egui::FontData::from_static(EMBEDDED_NOTO_EMOJI).into(),
    );

    // 配置字体优先级：Noto Sans SC 优先，Noto Emoji 作为后备，最后是默认字体
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "noto_sans_sc".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .push("noto_emoji".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(0, "noto_sans_sc".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("noto_emoji".to_owned());

    ctx.set_fonts(fonts);
}
