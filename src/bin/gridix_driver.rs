//! gridix-driver — headless driver for the Gridix egui desktop app.
//! Uses X11 through xdotool for deterministic input and ImageMagick for screenshots.
//!
//! Usage:
//!   cargo run --bin gridix-driver -- launch
//!   cargo run --bin gridix-driver -- key Ctrl+N
//!   cargo run --bin gridix-driver -- type "SELECT 1"
//!   cargo run --bin gridix-driver -- click 640 480
//!   cargo run --bin gridix-driver -- ss shot
//!   cargo run --bin gridix-driver -- quit

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, Output};

use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};

// ── config ────────────────────────────────────────────────────────────

const XVFB_DISPLAY: &str = ":99";
const SHOT_DIR: &str = "/tmp/shots";
const WINDOW_TIMEOUT: u64 = 30;
const XVFB_READY_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WAIT_SECONDS: u64 = 300;
const MAX_ARTIFACT_BYTES: u64 = 16 * 1024 * 1024;
const TERMINATION_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(100);
const AUTH_COOKIE_BYTES: usize = 16;
const AUTH_DIR_SUFFIX_BYTES: usize = 8;
const DRIVER_STATE_FILE: &str = "gridix-driver-state.json";
const WINDOW_NOT_FOUND: &str = "window not found";

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

fn state_path() -> PathBuf {
    std::env::var_os("GRIDIX_DRIVER_STATE")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join(DRIVER_STATE_FILE))
}

#[derive(Debug, Serialize, Deserialize)]
struct DriverState {
    display: String,
    gridix_pid: u32,
    xvfb_pid: Option<u32>,
    gridix_bin: PathBuf,
    #[serde(default)]
    xauth_path: Option<PathBuf>,
}

fn read_state() -> Result<Option<DriverState>, String> {
    match std::fs::read(state_path()) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|e| format!("invalid driver state: {e}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("read driver state: {error}")),
    }
}

fn write_state(state: &DriverState) -> Result<(), String> {
    let path = state_path();
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("create driver state directory: {error}"))?;
    }

    let temporary_path = path.with_extension("tmp");
    let bytes =
        serde_json::to_vec(state).map_err(|error| format!("serialize driver state: {error}"))?;
    std::fs::write(&temporary_path, bytes)
        .map_err(|error| format!("write driver state: {error}"))?;
    if let Err(error) = std::fs::rename(&temporary_path, &path) {
        let _ = std::fs::remove_file(&temporary_path);
        return Err(format!("publish driver state: {error}"));
    }
    Ok(())
}

fn remove_state() -> Result<(), String> {
    match std::fs::remove_file(state_path()) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("remove driver state: {error}")),
    }
}

fn environment_display(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|display| !display.is_empty())
}

fn active_display() -> String {
    if let Some(display) = environment_display("GRIDIX_DISPLAY") {
        return display;
    }
    if let Ok(Some(state)) = read_state() {
        return state.display;
    }
    environment_display("DISPLAY").unwrap_or_else(|| XVFB_DISPLAY.into())
}

fn launch_display() -> Result<String, String> {
    if std::env::var("XVFB_MANAGED").unwrap_or_default() == "1" {
        return environment_display("GRIDIX_DISPLAY")
            .or_else(|| environment_display("DISPLAY"))
            .ok_or_else(|| "XVFB_MANAGED=1 requires DISPLAY or GRIDIX_DISPLAY".into());
    }
    Ok(environment_display("GRIDIX_DISPLAY").unwrap_or_else(|| XVFB_DISPLAY.into()))
}

fn environment_xauthority() -> Option<PathBuf> {
    std::env::var_os("XAUTHORITY")
        .filter(|path| !path.is_empty())
        .map(PathBuf::from)
}

fn active_xauthority() -> Option<PathBuf> {
    read_state()
        .ok()
        .flatten()
        .and_then(|state| state.xauth_path)
        .or_else(environment_xauthority)
}

fn configure_x11_command(command: &mut Command, display: &str, xauth_path: Option<&Path>) {
    command.env("DISPLAY", display);
    if let Some(xauth_path) = xauth_path {
        command.env("XAUTHORITY", xauth_path);
    }
}

// ── process ownership ─────────────────────────────────────────────────

fn process_cmdline(pid: u32) -> Option<Vec<u8>> {
    std::fs::read(format!("/proc/{pid}/cmdline")).ok()
}

fn process_matches(pid: u32, expected_binary: &Path) -> bool {
    let Some(cmdline) = process_cmdline(pid) else {
        return false;
    };
    let Some(first_argument) = cmdline.split(|byte| *byte == 0).next() else {
        return false;
    };
    let first_argument = String::from_utf8_lossy(first_argument);
    if expected_binary.file_name().and_then(|name| name.to_str()) == Some("Xvfb") {
        return Path::new(first_argument.as_ref())
            .file_name()
            .and_then(|name| name.to_str())
            == Some("Xvfb");
    }
    first_argument == expected_binary.to_string_lossy()
}

fn tracked_session_is_running(state: &DriverState) -> bool {
    process_matches(state.gridix_pid, &state.gridix_bin)
        || state
            .xvfb_pid
            .is_some_and(|pid| process_matches(pid, Path::new("Xvfb")))
}

fn ensure_session_available() -> Result<(), String> {
    let Some(state) = read_state()? else {
        return Ok(());
    };
    if tracked_session_is_running(&state) {
        return Err(format!(
            "driver session already running (state: {})",
            state_path().display()
        ));
    }
    if let Some(xauth_path) = state.xauth_path.as_deref() {
        remove_private_xauth(xauth_path)?;
    }
    remove_state()
}

fn send_signal(pid: u32, signal: &str, expected_binary: &Path) -> Result<(), String> {
    if !process_matches(pid, expected_binary) {
        return Ok(());
    }

    let output = Command::new("kill")
        .args([format!("-{signal}"), pid.to_string()])
        .output()
        .map_err(|error| format!("kill process {pid}: {error}"))?;
    if output.status.success() || !process_matches(pid, expected_binary) {
        return Ok(());
    }
    Err(format!(
        "kill process {pid} failed: {}",
        command_stderr(&output)
    ))
}

fn terminate_tracked_process(pid: u32, expected_binary: &Path) -> Result<bool, String> {
    if !process_matches(pid, expected_binary) {
        return Ok(false);
    }
    send_signal(pid, "TERM", expected_binary)?;
    let started = Instant::now();
    while process_matches(pid, expected_binary) {
        if started.elapsed() >= TERMINATION_TIMEOUT {
            send_signal(pid, "KILL", expected_binary)?;
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    if process_matches(pid, expected_binary) {
        return Err(format!("process {pid} did not exit after termination"));
    }
    Ok(true)
}

fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn stop_children(xvfb: &mut Option<ManagedXvfb>, gridix: &mut Child) {
    stop_child(gridix);
    if let Some(xvfb) = xvfb {
        stop_child(&mut xvfb.child);
    }
}

// ── X11 window find ───────────────────────────────────────────────────

fn command_stderr(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        "no diagnostic output".into()
    } else {
        stderr
    }
}

fn find_window_on(display: &str, xauth_path: Option<&Path>) -> Result<String, String> {
    let mut command = Command::new("xdotool");
    configure_x11_command(&mut command, display, xauth_path);
    let output = command
        .args(["search", "--onlyvisible", "--name", "Gridix"])
        .output()
        .map_err(|error| format!("xdotool not found: {error}. Install xdotool"))?;

    if !output.status.success() {
        if output.stderr.is_empty() {
            return Err(WINDOW_NOT_FOUND.into());
        }
        let error = command_stderr(&output);
        if error.contains("Failed creating new xdo instance") {
            return Err(WINDOW_NOT_FOUND.into());
        }
        return Err(format!("xdotool search failed: {error}"));
    }

    let wid = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    if wid.is_empty() || !wid.chars().all(|character| character.is_ascii_digit()) {
        return Err(WINDOW_NOT_FOUND.into());
    }
    Ok(wid)
}

fn find_window() -> Result<String, String> {
    let display = active_display();
    let xauth_path = active_xauthority();
    find_window_on(&display, xauth_path.as_deref())
}

fn wait_for_window(
    display: &str,
    timeout_secs: u64,
    xauth_path: Option<&Path>,
) -> Result<String, String> {
    let started = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        match find_window_on(display, xauth_path) {
            Ok(window_id) => return Ok(window_id),
            Err(error) if error == WINDOW_NOT_FOUND => {}
            Err(error) => return Err(error),
        }
        if started.elapsed() >= timeout {
            return Err(format!("window not found after {timeout_secs}s"));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn wait_for_file(path: &Path, timeout_secs: u64) -> Result<(), String> {
    let started = Instant::now();
    let timeout = Duration::from_secs(timeout_secs);
    loop {
        if path.is_file()
            && path
                .metadata()
                .map(|metadata| metadata.len() > 0)
                .unwrap_or(false)
        {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(format!(
                "non-empty file not found after {timeout_secs}s: {}",
                path.display()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

// ── process management ────────────────────────────────────────────────

struct ManagedXvfb {
    child: Child,
    xauth_path: PathBuf,
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn remove_private_xauth(path: &Path) -> Result<(), String> {
    if path.file_name().and_then(|name| name.to_str()) != Some(".Xauthority") {
        return Err(format!(
            "refusing to remove unexpected Xauthority path: {}",
            path.display()
        ));
    }
    let Some(directory) = path.parent() else {
        return Err(format!("Xauthority path has no parent: {}", path.display()));
    };
    let directory_name = directory.file_name().and_then(|name| name.to_str());
    if directory_name.is_none_or(|name| !name.starts_with("gridix-driver-xauth-"))
        || directory.parent() != Some(std::env::temp_dir().as_path())
    {
        return Err(format!(
            "refusing to remove Xauthority outside the private temp directory: {}",
            path.display()
        ));
    }

    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("remove Xauthority file: {error}")),
    }
    match std::fs::remove_dir(directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("remove Xauthority directory: {error}")),
    }
}

fn combine_cleanup_error(primary: String, cleanup: Result<(), String>) -> String {
    match cleanup {
        Ok(()) => primary,
        Err(error) => format!("{primary}; cleanup failed: {error}"),
    }
}

fn create_private_directory(path: &Path) -> Result<(), std::io::Error> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;

        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder.create(path)
    }
    #[cfg(not(unix))]
    std::fs::create_dir(path)
}

fn create_xauth_file(display: &str) -> Result<PathBuf, String> {
    let mut random = [0_u8; AUTH_COOKIE_BYTES + AUTH_DIR_SUFFIX_BYTES];
    File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut random))
        .map_err(|error| format!("read secure Xauthority cookie bytes: {error}"))?;
    let cookie = hex_encode(&random[..AUTH_COOKIE_BYTES]);
    let directory = std::env::temp_dir().join(format!(
        "gridix-driver-xauth-{}-{}",
        std::process::id(),
        hex_encode(&random[AUTH_COOKIE_BYTES..])
    ));
    create_private_directory(&directory)
        .map_err(|error| format!("create private Xauthority directory: {error}"))?;
    #[cfg(unix)]
    if let Err(error) = std::fs::set_permissions(
        &directory,
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    ) {
        return Err(combine_cleanup_error(
            format!("protect Xauthority directory: {error}"),
            remove_private_xauth(&directory.join(".Xauthority")),
        ));
    }

    let path = directory.join(".Xauthority");
    let output = Command::new("xauth")
        .arg("-f")
        .arg(&path)
        .arg("add")
        .arg(display)
        .arg(".")
        .arg(&cookie)
        .output()
        .map_err(|error| {
            combine_cleanup_error(
                format!("xauth not found: {error}. Install xauth"),
                remove_private_xauth(&path),
            )
        })?;
    if !output.status.success() {
        return Err(combine_cleanup_error(
            format!("xauth failed: {}", command_stderr(&output)),
            remove_private_xauth(&path),
        ));
    }
    #[cfg(unix)]
    if let Err(error) =
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
    {
        return Err(combine_cleanup_error(
            format!("protect Xauthority file: {error}"),
            remove_private_xauth(&path),
        ));
    }
    Ok(path)
}

fn display_is_ready(display: &str, xauth_path: &Path) -> Result<bool, String> {
    let mut command = Command::new("xdotool");
    configure_x11_command(&mut command, display, Some(xauth_path));
    let output = command
        .arg("getdisplaygeometry")
        .output()
        .map_err(|error| format!("xdotool not found: {error}. Install xdotool"))?;
    Ok(output.status.success())
}

fn wait_for_display_ready(
    child: &mut Child,
    display: &str,
    xauth_path: &Path,
) -> Result<(), String> {
    let started = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("check Xvfb process: {error}"))?
        {
            return Err(format!("Xvfb exited before display became ready: {status}"));
        }
        if display_is_ready(display, xauth_path)? {
            return Ok(());
        }
        if started.elapsed() >= XVFB_READY_TIMEOUT {
            return Err(format!(
                "Xvfb display {display} was not ready after {}s",
                XVFB_READY_TIMEOUT.as_secs()
            ));
        }
        thread::sleep(POLL_INTERVAL);
    }
}

fn start_xvfb(display: &str) -> Result<Option<ManagedXvfb>, String> {
    if std::env::var("XVFB_MANAGED").unwrap_or_default() == "1" {
        println!("using external Xvfb on {display}");
        return Ok(None);
    }

    let xauth_path = create_xauth_file(display)?;
    let mut child = match Command::new("Xvfb")
        .arg(display)
        .arg("-auth")
        .arg(&xauth_path)
        .arg("-screen")
        .arg("0")
        .arg("1920x1080x24")
        .arg("-nolisten")
        .arg("tcp")
        .arg("+extension")
        .arg("RANDR")
        .arg("+extension")
        .arg("XTEST")
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return Err(combine_cleanup_error(
                format!(
                    "Xvfb not found: {error}\nUbuntu: sudo apt-get install xvfb xauth\nArch: sudo pacman -S xorg-server-xvfb xorg-xauth"
                ),
                remove_private_xauth(&xauth_path),
            ));
        }
    };

    if let Err(error) = wait_for_display_ready(&mut child, display, &xauth_path) {
        stop_child(&mut child);
        return Err(combine_cleanup_error(
            error,
            remove_private_xauth(&xauth_path),
        ));
    }
    println!("Xvfb started on {display} (pid={})", child.id());
    Ok(Some(ManagedXvfb { child, xauth_path }))
}
fn cleanup_xvfb_auth(xvfb: &mut Option<ManagedXvfb>) -> Result<(), String> {
    let Some(xvfb) = xvfb.take() else {
        return Ok(());
    };
    remove_private_xauth(&xvfb.xauth_path)
}

fn cleanup_session_state(xvfb: &mut Option<ManagedXvfb>) -> Result<(), String> {
    let auth_result = cleanup_xvfb_auth(xvfb);
    let state_result = remove_state();
    match (auth_result, state_result) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(auth), Ok(())) => Err(format!("cleanup Xauthority: {auth}")),
        (Ok(()), Err(state)) => Err(format!("remove driver state: {state}")),
        (Err(auth), Err(state)) => Err(format!(
            "cleanup Xauthority: {auth}; remove driver state: {state}"
        )),
    }
}

fn cmd_launch() -> Result<(Option<ManagedXvfb>, Child, String), String> {
    ensure_session_available()?;
    let display = launch_display()?;
    let bin = executable_path()?;
    let mut xvfb = start_xvfb(&display)?;
    let xauth_path = xvfb.as_ref().map(|xvfb| xvfb.xauth_path.clone());
    let mut gridix = match start_gridix(&bin, &display, xauth_path.as_deref()) {
        Ok(child) => child,
        Err(error) => {
            return Err(combine_cleanup_error(error, cleanup_xvfb_auth(&mut xvfb)));
        }
    };

    println!("waiting for window (up to {WINDOW_TIMEOUT}s)...");
    let window_id = match wait_for_window(&display, WINDOW_TIMEOUT, xauth_path.as_deref()) {
        Ok(window_id) => window_id,
        Err(error) => {
            stop_children(&mut xvfb, &mut gridix);
            return Err(combine_cleanup_error(error, cleanup_xvfb_auth(&mut xvfb)));
        }
    };

    let state = DriverState {
        display,
        gridix_pid: gridix.id(),
        xvfb_pid: xvfb.as_ref().map(|xvfb| xvfb.child.id()),
        gridix_bin: bin,
        xauth_path,
    };
    if let Err(error) = write_state(&state) {
        stop_children(&mut xvfb, &mut gridix);
        return Err(combine_cleanup_error(error, cleanup_xvfb_auth(&mut xvfb)));
    }

    println!("window found: {window_id}");
    println!("ready.");
    Ok((xvfb, gridix, window_id))
}

fn executable_path() -> Result<PathBuf, String> {
    let path = gridix_bin();
    if !path.is_file() {
        return Err(format!(
            "gridix binary not found at {}\nBuild: cargo build --release",
            path.display()
        ));
    }
    std::fs::canonicalize(&path).map_err(|error| format!("resolve gridix binary: {error}"))
}

fn start_gridix(bin: &Path, display: &str, xauth_path: Option<&Path>) -> Result<Child, String> {
    let mut command = Command::new(bin);
    command.env("WINIT_UNIX_BACKEND", "x11");
    command.env_remove("WAYLAND_DISPLAY");
    configure_x11_command(&mut command, display, xauth_path);
    let child = command
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|error| format!("failed to start gridix: {error}"))?;
    println!("gridix started (pid={})", child.id());
    Ok(child)
}

// ── xdotool actions ───────────────────────────────────────────────────

fn run_xdotool(args: &[String]) -> Result<(), String> {
    let display = active_display();
    let xauth_path = active_xauthority();
    let mut command = Command::new("xdotool");
    configure_x11_command(&mut command, &display, xauth_path.as_deref());
    let output = command
        .args(args)
        .output()
        .map_err(|error| format!("xdotool not found: {error}. Install xdotool"))?;
    if !output.status.success() {
        return Err(format!("xdotool failed: {}", command_stderr(&output)));
    }
    Ok(())
}

/// 激活窗口：优先使用 `_NET_ACTIVE_WINDOW`，无窗口管理器时退化为直接设置输入焦点。
///
/// 驱动自己启动的 Xvfb 不带窗口管理器，`windowactivate` 在这种情况下会失败，
/// 而 `windowfocus`（XSetInputFocus）不需要窗口管理器支持。
fn activate_window(window_id: &str) -> Result<(), String> {
    match run_xdotool(&["windowactivate".into(), "--sync".into(), window_id.into()]) {
        Ok(()) => Ok(()),
        Err(activation_error) => {
            // 退化路径写 stderr：成功时不改变 stdout 的输出约定，但日志里能看出
            // 本次运行没有走 EWMH 激活。
            eprintln!(
                "windowactivate {window_id} unavailable ({activation_error}); falling back to windowfocus"
            );
            run_xdotool(&["windowfocus".into(), window_id.into()]).map_err(|focus_error| {
                format!(
                    "windowactivate {window_id} failed: {activation_error}; windowfocus {window_id} failed: {focus_error}"
                )
            })
        }
    }
}

fn parse_launch_args(args: &[String]) -> Result<bool, String> {
    match args {
        [] => Ok(false),
        [flag] if flag == "--detach" => Ok(true),
        _ => Err("usage: launch [--detach]".into()),
    }
}

fn wait_for_operator() -> Result<bool, String> {
    let mut input = String::new();
    match std::io::stdin().read_line(&mut input) {
        Ok(0) => {
            println!("stdin closed; session remains running; run `gridix-driver quit` to stop");
            Ok(false)
        }
        Ok(_) => Ok(true),
        Err(error) => Err(format!("wait for stop input: {error}")),
    }
}

fn run_launch(args: &[String]) -> ExitCode {
    let detached = match parse_launch_args(args) {
        Ok(detached) => detached,
        Err(error) => {
            eprintln!("ERROR: {error}");
            return ExitCode::from(1);
        }
    };

    match cmd_launch() {
        Ok((mut xvfb, mut gridix, window_id)) => {
            println!("GRIDIX_WID={window_id}");
            if detached {
                println!("detached; run `gridix-driver quit` when finished");
                return ExitCode::SUCCESS;
            }

            println!("Press Ctrl+C to stop");

            match wait_for_operator() {
                Ok(true) => {
                    stop_children(&mut xvfb, &mut gridix);
                    match cleanup_session_state(&mut xvfb) {
                        Ok(()) => {
                            println!("stopped");
                            ExitCode::SUCCESS
                        }
                        Err(error) => {
                            eprintln!("ERROR: {error}");
                            ExitCode::from(1)
                        }
                    }
                }
                Ok(false) => ExitCode::SUCCESS,
                Err(error) => {
                    eprintln!("ERROR: {error}");
                    stop_children(&mut xvfb, &mut gridix);
                    if let Err(cleanup_error) = cleanup_session_state(&mut xvfb) {
                        eprintln!("ERROR: {cleanup_error}");
                    }
                    ExitCode::from(1)
                }
            }
        }
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::from(1)
        }
    }
}

fn cmd_key(window_id: &str, keys: &[String]) -> Result<(), String> {
    if keys.is_empty() {
        return Err("at least one key is required".into());
    }
    activate_window(window_id)?;
    let mut args = vec!["key".into()];
    args.extend(keys.iter().cloned());
    run_xdotool(&args)?;
    println!("key: {}", keys.join("+"));
    Ok(())
}

fn cmd_type(window_id: &str, text_parts: &[String]) -> Result<(), String> {
    let text = text_parts.join(" ");
    if text.is_empty() {
        return Err("text must not be empty".into());
    }
    activate_window(window_id)?;
    // 不带 `--window`：`xdotool type --window` 走 XSendEvent 合成事件，egui/winit 会忽略；
    // activate_window 已把输入焦点落在应用窗口上，直接向聚焦窗口输入才能被 UI 收到。
    run_xdotool(&[
        "type".into(),
        "--clearmodifiers".into(),
        "--delay".into(),
        "0".into(),
        text,
    ])?;
    println!("typed {} characters", text_parts.join(" ").chars().count());
    Ok(())
}

fn parse_coordinate(raw: &str, name: &str) -> Result<u32, String> {
    let value = raw
        .parse::<u32>()
        .map_err(|_| format!("{name} must be a non-negative integer"))?;
    Ok(value)
}

fn parse_button(raw: &str) -> Result<u8, String> {
    let button = raw
        .parse::<u8>()
        .map_err(|_| "button must be an integer from 1 to 5".to_string())?;
    if (1..=5).contains(&button) {
        Ok(button)
    } else {
        Err("button must be an integer from 1 to 5".into())
    }
}

fn cmd_move(window_id: &str, args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("usage: move <x> <y>".into());
    }
    let x = parse_coordinate(&args[0], "x")?;
    let y = parse_coordinate(&args[1], "y")?;
    activate_window(window_id)?;
    run_xdotool(&[
        "mousemove".into(),
        "--sync".into(),
        "--window".into(),
        window_id.into(),
        x.to_string(),
        y.to_string(),
    ])?;
    println!("pointer: ({x}, {y})");
    Ok(())
}

fn cmd_click(window_id: &str, args: &[String]) -> Result<(), String> {
    if args.len() != 2 && args.len() != 3 {
        return Err("usage: click <x> <y> [button]".into());
    }
    let button = parse_button(args.get(2).map(String::as_str).unwrap_or("1"))?;
    cmd_move(window_id, &args[..2])?;
    run_xdotool(&[
        "click".into(),
        "--repeat".into(),
        "1".into(),
        button.to_string(),
    ])?;
    println!("click: ({}, {}) button {button}", args[0], args[1]);
    Ok(())
}

fn parse_timeout(raw: Option<&String>) -> Result<u64, String> {
    let timeout = raw
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| "timeout must be an integer number of seconds".to_string())?
        .unwrap_or(WINDOW_TIMEOUT);
    if timeout <= MAX_WAIT_SECONDS {
        Ok(timeout)
    } else {
        Err(format!("timeout must be no more than {MAX_WAIT_SECONDS}s"))
    }
}

fn cmd_wait(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        Some("window") if (1..=2).contains(&args.len()) => {
            let timeout = parse_timeout(args.get(1))?;
            let display = active_display();
            let xauth_path = active_xauthority();
            let window_id = wait_for_window(&display, timeout, xauth_path.as_deref())?;
            println!("window ready: {window_id}");
            Ok(())
        }
        Some("file") if (2..=3).contains(&args.len()) => {
            let timeout = parse_timeout(args.get(2))?;
            wait_for_file(Path::new(&args[1]), timeout)?;
            println!("file ready: {}", args[1]);
            Ok(())
        }
        _ => Err("usage: wait window [timeout] | wait file <path> [timeout]".into()),
    }
}

fn cmd_wait_window(args: &[String]) -> Result<(), String> {
    if args.len() > 1 {
        return Err("usage: wait-window [timeout]".into());
    }
    let timeout = parse_timeout(args.first())?;
    let display = active_display();
    let xauth_path = active_xauthority();
    let window_id = wait_for_window(&display, timeout, xauth_path.as_deref())?;
    println!("window ready: {window_id}");
    Ok(())
}

fn cmd_wait_file(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args.len() > 2 {
        return Err("usage: wait-file <path> [timeout]".into());
    }
    let timeout = parse_timeout(args.get(1))?;
    wait_for_file(Path::new(&args[0]), timeout)?;
    println!("file ready: {}", args[0]);
    Ok(())
}

fn safe_shot_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
        return Err("screenshot name must be a simple file name".into());
    }
    Ok(())
}

fn cmd_ss(window_id: &str, name: &str) -> Result<(), String> {
    safe_shot_name(name)?;
    let dir = shot_dir();
    std::fs::create_dir_all(&dir).map_err(|error| format!("mkdir: {error}"))?;
    let path = dir.join(format!("{name}.png"));
    let display = active_display();
    let xauth_path = active_xauthority();
    let mut command = Command::new("import");
    configure_x11_command(&mut command, &display, xauth_path.as_deref());
    let output = command
        .args(["-window", window_id, &path.to_string_lossy()])
        .output()
        .map_err(|error| {
            format!(
                "import (imagemagick) not found: {error}\nInstall: sudo apt-get install imagemagick"
            )
        })?;
    if !output.status.success() {
        return Err(format!("import failed: {}", command_stderr(&output)));
    }
    println!("screenshot: {}", path.display());
    Ok(())
}

// ── acceptance assertions ─────────────────────────────────────────────

fn read_artifact(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("read artifact {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "artifact is not a regular file: {}",
            path.display()
        ));
    }
    if metadata.len() == 0 {
        return Err(format!("artifact is empty: {}", path.display()));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(format!(
            "artifact exceeds {} bytes: {}",
            MAX_ARTIFACT_BYTES,
            path.display()
        ));
    }
    let bytes = std::fs::read(path)
        .map_err(|error| format!("read artifact {}: {error}", path.display()))?;
    if bytes.is_empty() {
        return Err(format!("artifact became empty: {}", path.display()));
    }
    Ok(bytes)
}

fn cmd_assert_file(args: &[String]) -> Result<(), String> {
    if args.is_empty() || args.len() > 2 {
        return Err("usage: assert-file <path> [minimum-bytes]".into());
    }
    let minimum_bytes = args
        .get(1)
        .map(|value| value.parse::<u64>())
        .transpose()
        .map_err(|_| "minimum-bytes must be a non-negative integer".to_string())?
        .unwrap_or(1);
    let metadata =
        std::fs::metadata(&args[0]).map_err(|error| format!("read file {}: {error}", args[0]))?;
    if !metadata.is_file() || metadata.len() < minimum_bytes {
        return Err(format!(
            "file assertion failed: {} ({} bytes, expected at least {minimum_bytes})",
            args[0],
            metadata.len()
        ));
    }
    println!(
        "file assertion passed: {} ({} bytes)",
        args[0],
        metadata.len()
    );
    Ok(())
}

#[derive(Debug, Clone, Copy)]
enum ExportFormat {
    Csv,
    Json,
    Sql,
}

fn parse_export_format(raw: &str) -> Result<ExportFormat, String> {
    match raw.to_ascii_lowercase().as_str() {
        "csv" => Ok(ExportFormat::Csv),
        "json" => Ok(ExportFormat::Json),
        "sql" => Ok(ExportFormat::Sql),
        _ => Err("format must be csv, json, or sql".into()),
    }
}

fn cmd_assert_export(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        return Err("usage: assert-export <csv|json|sql> <path> [expected-text ...]".into());
    }
    let format = parse_export_format(&args[0])?;
    let bytes = read_artifact(Path::new(&args[1]))?;
    match format {
        ExportFormat::Csv => {
            let mut reader = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_reader(bytes.as_slice());
            if reader
                .records()
                .next()
                .transpose()
                .map_err(|error| format!("invalid CSV export {}: {error}", args[1]))?
                .is_none()
            {
                return Err(format!("CSV export has no records: {}", args[1]));
            }
        }
        ExportFormat::Json => {
            serde_json::from_slice::<serde_json::Value>(&bytes)
                .map_err(|error| format!("invalid JSON export {}: {error}", args[1]))?;
        }
        ExportFormat::Sql => {
            if String::from_utf8_lossy(&bytes).trim().is_empty() {
                return Err(format!("SQL export has no statements: {}", args[1]));
            }
        }
    }

    let text = String::from_utf8(bytes)
        .map_err(|error| format!("export is not UTF-8 text {}: {error}", args[1]))?;
    for expected in args.iter().skip(2) {
        if expected.is_empty() || !text.contains(expected) {
            return Err(format!(
                "export assertion failed for {}: missing expected text {:?}",
                args[1], expected
            ));
        }
    }
    println!("{} export assertion passed: {}", args[0], args[1]);
    Ok(())
}

fn sqlite_identifier(raw: &str, label: &str) -> Result<String, String> {
    let mut characters = raw.chars();
    let Some(first) = characters.next() else {
        return Err(format!("{label} must be a non-empty SQLite identifier"));
    };
    if !(first == '_' || first.is_ascii_alphabetic())
        || !characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
    {
        return Err(format!(
            "{label} must contain only ASCII letters, digits, and underscores"
        ));
    }
    Ok(format!("\"{raw}\""))
}

fn cmd_assert_reopened(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err("usage: assert-reopened <db> <table> <column> <expected|NULL>".into());
    }
    let db_path = Path::new(&args[0]);
    if !db_path.is_file() {
        return Err(format!(
            "SQLite database is not a regular file: {}",
            args[0]
        ));
    }
    let table = sqlite_identifier(&args[1], "table")?;
    let column = sqlite_identifier(&args[2], "column")?;
    let connection = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("open SQLite database {}: {error}", args[0]))?;

    let result = if args[3] == "NULL" {
        let sql = format!("SELECT 1 FROM {table} WHERE {column} IS NULL LIMIT 1");
        connection.query_row(&sql, [], |row| row.get::<_, i64>(0))
    } else {
        let sql = format!("SELECT 1 FROM {table} WHERE {column} = ?1 LIMIT 1");
        connection.query_row(&sql, rusqlite::params![args[3]], |row| row.get::<_, i64>(0))
    };
    match result {
        Ok(_) => {
            println!(
                "reopen assertion passed: {} contains {}={} ",
                args[0], args[2], args[3]
            );
            Ok(())
        }
        Err(rusqlite::Error::QueryReturnedNoRows) => Err(format!(
            "reopen assertion failed: no row in {} with {}={}",
            args[1], args[2], args[3]
        )),
        Err(error) => Err(format!("reopen assertion query failed: {error}")),
    }
}

fn cmd_quit() -> Result<(), String> {
    let Some(state) = read_state()? else {
        println!("quit: no tracked driver session");
        return Ok(());
    };
    let mut stopped = 0;
    let mut failure = None;
    match terminate_tracked_process(state.gridix_pid, &state.gridix_bin) {
        Ok(true) => stopped += 1,
        Ok(false) => {}
        Err(error) => failure = Some(error),
    }
    if let Some(xvfb_pid) = state.xvfb_pid {
        match terminate_tracked_process(xvfb_pid, Path::new("Xvfb")) {
            Ok(true) => stopped += 1,
            Ok(false) => {}
            Err(error) => {
                if failure.is_none() {
                    failure = Some(error);
                }
            }
        }
    }
    if let Some(xauth_path) = state.xauth_path.as_deref()
        && let Err(error) = remove_private_xauth(xauth_path)
        && failure.is_none()
    {
        failure = Some(error);
    }
    if let Err(error) = remove_state()
        && failure.is_none()
    {
        failure = Some(error);
    }

    if let Some(error) = failure {
        return Err(error);
    }
    println!("quit done ({stopped} tracked process(es) stopped)");
    Ok(())
}

fn print_help() {
    println!("gridix-driver — headless driver for Gridix");
    println!();
    println!("Commands:");
    println!(
        "  launch [--detach]             start private-cookie Xvfb + Gridix and wait for its window"
    );
    println!("  key <keys>                     send keystroke (e.g. 'key Ctrl+N')");
    println!("  type <text>                    type text into the active Gridix window");
    println!("  move <x> <y>                  move pointer relative to the Gridix window");
    println!("  click <x> <y> [button]        move and click button 1-5");
    println!("  wait window [timeout]          poll until the Gridix window is visible");
    println!("  wait file <path> [timeout]     poll until a non-empty file exists");
    println!("  wait-window [timeout]          shorthand for 'wait window'");
    println!("  wait-file <path> [timeout]    shorthand for 'wait file'");
    println!("  ss <name>                      screenshot → /tmp/shots/<name>.png");
    println!("  assert-file <path> [bytes]     assert a non-empty artifact");
    println!("  assert-export <format> <path> validate CSV/JSON/SQL and expected text");
    println!("  assert-reopened <db> <table> <column> <value|NULL>");
    println!("                                verify a persisted SQLite value read-only");
    println!(
        "  quit                           stop tracked Gridix/Xvfb PIDs and remove private Xauthority"
    );
    println!();
    println!("Env:");
    println!("  GRIDIX_BIN                     path to gridix binary");
    println!("  GRIDIX_SHOT_DIR               screenshot directory (default: /tmp/shots)");
    println!("  GRIDIX_DRIVER_STATE           state file for exact process cleanup");
    println!("  GRIDIX_DISPLAY                display for launch, actions, and screenshots");
    println!("  XVFB_MANAGED=1                use an externally managed DISPLAY");
    println!("  XVFB_MANAGED=0                start and own Xvfb on GRIDIX_DISPLAY (default)");
    println!();
}

fn exit_with_result(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("ERROR: {error}");
            ExitCode::from(1)
        }
    }
}

// ── main ─────────────────────────────────────────────────────────────

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let command = args.get(1).map(String::as_str);
    match command {
        Some("launch") => run_launch(&args[2..]),
        Some("key") => {
            let keys: Vec<String> = args.iter().skip(2).cloned().collect();
            let result = find_window().and_then(|window_id| cmd_key(&window_id, &keys));
            exit_with_result(result)
        }
        Some("type") => {
            let text: Vec<String> = args.iter().skip(2).cloned().collect();
            let result = find_window().and_then(|window_id| cmd_type(&window_id, &text));
            exit_with_result(result)
        }
        Some("move") => {
            let action_args: Vec<String> = args.iter().skip(2).cloned().collect();
            let result = find_window().and_then(|window_id| cmd_move(&window_id, &action_args));
            exit_with_result(result)
        }
        Some("click") => {
            let action_args: Vec<String> = args.iter().skip(2).cloned().collect();
            let result = find_window().and_then(|window_id| cmd_click(&window_id, &action_args));
            exit_with_result(result)
        }
        Some("wait") => exit_with_result(cmd_wait(&args[2..])),
        Some("wait-window") => exit_with_result(cmd_wait_window(&args[2..])),
        Some("wait-file") => exit_with_result(cmd_wait_file(&args[2..])),
        Some("ss") => {
            let name = args.get(2).map(String::as_str).unwrap_or("screenshot");
            let result = find_window().and_then(|window_id| cmd_ss(&window_id, name));
            exit_with_result(result)
        }
        Some("assert-file") => exit_with_result(cmd_assert_file(&args[2..])),
        Some("assert-export") => exit_with_result(cmd_assert_export(&args[2..])),
        Some("assert-reopened") => exit_with_result(cmd_assert_reopened(&args[2..])),
        Some("quit") => exit_with_result(cmd_quit()),
        _ => {
            print_help();
            ExitCode::SUCCESS
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeout_is_bounded() {
        assert_eq!(parse_timeout(Some(&"0".to_string())).unwrap(), 0);
        assert!(parse_timeout(Some(&(MAX_WAIT_SECONDS + 1).to_string())).is_err());
    }

    #[test]
    fn launch_args_accept_detach_mode() {
        assert!(!parse_launch_args(&[]).unwrap());
        assert!(parse_launch_args(&["--detach".into()]).unwrap());
        assert!(parse_launch_args(&["--unknown".into()]).is_err());
    }

    #[test]
    fn sqlite_identifier_rejects_sql_fragments() {
        assert_eq!(sqlite_identifier("users", "table").unwrap(), "\"users\"");
        assert!(sqlite_identifier("users; DROP TABLE users", "table").is_err());
    }

    #[test]
    fn screenshot_name_cannot_escape_directory() {
        assert!(safe_shot_name("acceptance").is_ok());
        assert!(safe_shot_name("../acceptance").is_err());
        assert!(safe_shot_name("nested/name").is_err());
    }
    #[test]
    fn acceptance_assertions_validate_export_and_reopened_data() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("acceptance.db");
        let connection = Connection::open(&database).unwrap();
        connection
            .execute("CREATE TABLE items (name TEXT, note TEXT)", [])
            .unwrap();
        connection
            .execute(
                "INSERT INTO items (name, note) VALUES (?1, NULL)",
                ["after"],
            )
            .unwrap();
        drop(connection);

        let reopened_args = vec![
            database.display().to_string(),
            "items".into(),
            "name".into(),
            "after".into(),
        ];
        cmd_assert_reopened(&reopened_args).unwrap();

        let csv = directory.path().join("acceptance.csv");
        std::fs::write(&csv, "name,note\nafter,\n").unwrap();
        cmd_assert_export(&["csv".into(), csv.display().to_string(), "after".into()]).unwrap();

        let json = directory.path().join("acceptance.json");
        std::fs::write(&json, r#"[{"name":"after","note":null}]"#).unwrap();
        cmd_assert_export(&[
            "json".into(),
            json.display().to_string(),
            r#""name":"after""#.into(),
        ])
        .unwrap();

        let sql = directory.path().join("acceptance.sql");
        std::fs::write(
            &sql,
            "INSERT INTO items (name, note) VALUES ('after', NULL);\n",
        )
        .unwrap();
        cmd_assert_export(&[
            "sql".into(),
            sql.display().to_string(),
            "'after'".into(),
            "NULL".into(),
        ])
        .unwrap();
    }
}
