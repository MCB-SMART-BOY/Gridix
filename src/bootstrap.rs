//! 应用启动引导
//!
//! 将日志、panic hook、窗口参数、字体配置集中到一个入口，
//! 避免 main/lib 双份启动逻辑分散。

use crate::app::DbManagerApp;
use eframe::egui;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// 内嵌的 Noto Sans SC 字体（思源黑体，支持完整 Unicode）
const EMBEDDED_NOTO_SANS_SC: &[u8] = include_bytes!("../assets/fonts/NotoSansSC-Regular.ttf");

/// 内嵌的 Noto Emoji 字体（支持 Unicode Emoji）
const EMBEDDED_NOTO_EMOJI: &[u8] = include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf");

/// 内嵌的应用图标。
const EMBEDDED_APP_ICON: &[u8] = include_bytes!("../assets/branding/gridix-icon.png");

/// 启动 Gridix GUI
pub fn run() -> eframe::Result<()> {
    init_tracing();
    install_panic_hook();

    tracing::info!("Gridix 启动中...");
    let options = native_options();

    eframe::run_native(
        "Gridix",
        options,
        Box::new(|cc| {
            setup_fonts(&cc.egui_ctx);
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(DbManagerApp::new(cc)))
        }),
    )
}

fn native_options() -> eframe::NativeOptions {
    eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Gridix")
            .with_icon(load_app_icon()),
        ..Default::default()
    }
}

fn load_app_icon() -> egui::IconData {
    let image = image::load_from_memory(EMBEDDED_APP_ICON)
        .expect("embedded Gridix icon must be a valid PNG")
        .into_rgba8();
    let (width, height) = image.dimensions();

    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

/// 初始化日志系统。
///
/// 使用 `try_init` 以避免在外部已安装 subscriber 时 panic。
fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("gridix=info,warn"));

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .try_init();
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "未知位置".to_string());
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "未知错误".to_string());
        tracing::error!(location = %location, message = %message, "程序崩溃 (panic)");
        eprintln!(
            "\n程序崩溃!\n位置: {}\n信息: {}\n\n请检查日志获取更多信息。",
            location, message
        );
    }));
}

/// 配置字体。
///
/// 使用 Noto Sans SC（思源黑体）字体，支持完整的 Unicode 字符集，
/// 包括中文、日文、韩文以及各种特殊符号。
/// 同时添加 Noto Emoji 字体作为后备，支持 Unicode Emoji 符号。
fn setup_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "noto_sans_sc".to_owned(),
        egui::FontData::from_static(EMBEDDED_NOTO_SANS_SC).into(),
    );
    fonts.font_data.insert(
        "noto_emoji".to_owned(),
        egui::FontData::from_static(EMBEDDED_NOTO_EMOJI).into(),
    );

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
