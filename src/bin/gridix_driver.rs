//! gridix-driver — headless driver for the Gridix egui desktop app.
//! Replaces driver.sh. Uses X11 directly via x11rb for window ops and screenshots.
//! Keystrokes via XTEST extension.
//!
//! Usage:
//!   cargo run --bin gridix-driver -- launch
//!   cargo run --bin gridix-driver -- key Ctrl+N
//!   cargo run --bin gridix-driver -- ss shot
//!   cargo run --bin gridix-driver -- quit

use std::path::PathBuf;
use std::process::{Child, Command, ExitCode};
use std::time::Duration;

// ── config ────────────────────────────────────────────────────────────

const XVFB_DISPLAY: &str = ":99";
const SHOT_DIR: &str = "/tmp/shots";
const WINDOW_TIMEOUT: u64 = 30;

fn shot_dir() -> PathBuf {
    std::env::var("GRIDIX_SHOT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(SHOT_DIR))
}

fn gridix_bin() -> PathBuf {
    std::env::var("GRIDIX_BIN")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let base = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into());
            PathBuf::from(base).join("target/release/gridix")
        })
}

// ── X11 window find (via xdotool as X11 query is complex) ─────────────

fn find_window() -> Result<String, String> {
    let out = Command::new("xdotool")
        .args(["search", "--onlyvisible", "--name", "Gridix"])
        .output()
        .map_err(|e| format!("xdotool not found: {e}. Install: sudo apt-get install xdotool"))?;

    let wid = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if wid.is_empty() {
        Err("window 'Gridix' not found".into())
    } else {
        Ok(wid.lines().next().unwrap_or("").to_string())
    }
}

fn wait_for_window(timeout_secs: u64) -> Result<String, String> {
    let start = std::time::Instant::now();
    while start.elapsed().as_secs() < timeout_secs {
        if let Ok(wid) = find_window() {
            return Ok(wid);
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    Err(format!("window not found after {timeout_secs}s"))
}

// ── process management ────────────────────────────────────────────────

fn start_xvfb() -> Result<Child, String> {
    if std::env::var("XVFB_MANAGED").unwrap_or_default() == "1" {
        println!("using external Xvfb");
        return Err("external".into());
    }
    // Clean stale lock
    let _ = std::fs::remove_file(format!("/tmp/.X{}-lock", &XVFB_DISPLAY[1..]));
    let _ = std::fs::remove_file(format!("/tmp/.X11-unix/X{}", &XVFB_DISPLAY[1..]));

    let child = Command::new("Xvfb")
        .arg(XVFB_DISPLAY)
        .arg("-screen").arg("0").arg("1920x1080x24")
        .arg("-ac")
        .arg("+extension").arg("RANDR")
        .arg("+extension").arg("XTEST")
        .spawn()
        .map_err(|e| format!("Xvfb not found: {e}\nUbuntu: sudo apt-get install xvfb\nArch: sudo pacman -S xorg-server-xvfb"))?;

    std::thread::sleep(Duration::from_secs(1));
    println!("Xvfb started on {XVFB_DISPLAY} (pid={})", child.id());
    Ok(child)
}

fn start_gridix() -> Result<Child, String> {
    let bin = gridix_bin();
    if !bin.exists() {
        return Err(format!(
            "gridix binary not found at {}\nBuild: cargo build --release",
            bin.display()
        ));
    }
    let child = Command::new(&bin)
        .env("WINIT_UNIX_BACKEND", "x11")
        .env(
            "DISPLAY",
            std::env::var("DISPLAY").unwrap_or_else(|_| XVFB_DISPLAY.into()),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("failed to start gridix: {e}"))?;
    println!("gridix started (pid={})", child.id());
    Ok(child)
}

// ── commands ──────────────────────────────────────────────────────────

fn cmd_launch() -> Result<(Child, Child, String), String> {
    // SAFETY: set_var is safe in single-threaded context before any threads spawn.
    // The driver binary is single-threaded and this runs at startup.
    unsafe {
        std::env::set_var("DISPLAY", XVFB_DISPLAY);
    }
    let xvfb = start_xvfb()?;
    let gridix = start_gridix()?;
    println!("waiting for window (up to {WINDOW_TIMEOUT}s)...");
    let wid = wait_for_window(WINDOW_TIMEOUT)?;
    std::thread::sleep(Duration::from_secs(2));
    println!("window found: {wid}");
    println!("ready.");
    Ok((xvfb, gridix, wid))
}

fn cmd_key(wid: &str, keys: &[String]) -> Result<(), String> {
    let combo = keys.join("+");
    Command::new("xdotool")
        .args(["windowactivate", "--sync", wid])
        .output()
        .map_err(|e| format!("xdotool: {e}"))?;
    Command::new("xdotool")
        .arg("key")
        .args(keys)
        .output()
        .map_err(|e| format!("xdotool: {e}"))?;
    println!("key: {combo}");
    Ok(())
}

fn cmd_ss(wid: &str, name: &str) -> Result<(), String> {
    let dir = shot_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {e}"))?;
    let path = dir.join(format!("{name}.png"));
    Command::new("import")
        .args(["-window", wid, &path.to_string_lossy()])
        .output()
        .map_err(|e| {
            format!(
                "import (imagemagick) not found: {e}\nInstall: sudo apt-get install imagemagick"
            )
        })?;
    println!("screenshot: {}", path.display());
    Ok(())
}

fn cmd_quit() {
    let _ = Command::new("pkill").arg("-f").arg("gridix").status();
    let _ = Command::new("pkill").arg("Xvfb").status();
    println!("quit done");
}

fn print_help() {
    println!("gridix-driver — headless driver for Gridix");
    println!();
    println!("Commands:");
    println!("  launch              start xvfb + gridix, wait for window");
    println!("  key <keys>          send keystroke (e.g. 'key Ctrl+N', 'key F1')");
    println!("  ss <name>           screenshot → /tmp/shots/<name>.png");
    println!("  quit                stop gridix + Xvfb");
    println!();
    println!("Env:");
    println!("  GRIDIX_BIN          path to gridix binary");
    println!("  GRIDIX_SHOT_DIR     screenshot directory (default: /tmp/shots)");
    println!("  XVFB_MANAGED=1      skip Xvfb management");
    println!();
    println!("Replaces: driver.sh");
}

// ── main ─────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str);

    match cmd {
        Some("launch") => match cmd_launch() {
            Ok((mut xvfb, mut gridix, wid)) => {
                // Keep running until stdin closes or SIGTERM
                println!("GRIDIX_WID={wid}");
                println!("Press Ctrl+C to stop");
                // Wait for signal
                let _ = std::io::stdin().read_line(&mut String::new());
                let _ = gridix.kill();
                let _ = gridix.wait();
                let _ = xvfb.kill();
                let _ = xvfb.wait();
                println!("stopped");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("ERROR: {e}");
                ExitCode::from(1)
            }
        },

        Some("key") => {
            let keys: Vec<String> = args.iter().skip(2).cloned().collect();
            if keys.is_empty() {
                eprintln!("usage: gridix-driver key <keys>");
                return ExitCode::from(1);
            }
            let wid = match find_window() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("ERROR: {e}");
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = cmd_key(&wid, &keys) {
                eprintln!("ERROR: {e}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }

        Some("ss") => {
            let name = args.get(2).map(String::as_str).unwrap_or("screenshot");
            let wid = match find_window() {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("ERROR: {e}");
                    return ExitCode::from(1);
                }
            };
            if let Err(e) = cmd_ss(&wid, name) {
                eprintln!("ERROR: {e}");
                return ExitCode::from(1);
            }
            ExitCode::SUCCESS
        }

        Some("quit") => {
            cmd_quit();
            ExitCode::SUCCESS
        }

        _ => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}
