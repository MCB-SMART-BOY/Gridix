use clap::Parser;

/// Gridix — 跨平台数据库管理工具，支持 SQLite / PostgreSQL / MySQL。
///
/// 默认行为：无参数启动图形界面。
#[derive(Parser)]
#[command(
    name = "gridix",
    version,
    about = "跨平台数据库管理工具",
    long_about = "Gridix — 键盘驱动的跨平台数据库管理工具。\n支持 SQLite、PostgreSQL、MySQL，内置 Helix/Vim 风格快捷键。"
)]
struct Cli {
    /// 测试 CI 检查（打印成功消息后退出）
    #[arg(long, hide = true)]
    ci_check: bool,
}

fn main() {
    let cli = Cli::parse();

    if cli.ci_check {
        println!("Gridix CI check passed.");
        return;
    }

    // 默认：启动图形界面
    if let Err(err) = gridix::bootstrap::run() {
        eprintln!("Gridix 启动失败: {err}");
        std::process::exit(1);
    }
}
