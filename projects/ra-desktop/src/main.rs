//! 原生 GUI 入口：二进制名 `ra2`（Windows 上为 `ra2.exe`）。
//!
//! 不是命令行工具——启动配置来自 exe/工作目录旁的 `config.toml`，然后由窗口接管进程。
//!
//! 页面状态机见 [`shell::AppShell`] / [`screen::OriginalScreen`]；对局输入见 [`match_ctrl::MatchController`]。
//! 产品路径对齐原版主 UI 流程，禁止启动后自动开局。

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod boot;
mod config;
mod fs_source;
mod load_job;
mod local_player;
mod match_ctrl;
mod menu_action;
mod preview_job;
mod screen;
mod screenshot;
mod shell;
mod skirmish_setup;
mod ui_assets;
mod ui_hit;
mod ui_page;
mod ui_resolve;
mod ui_slots;
#[cfg(feature = "test-harness")]
mod test_boot;

use ra_types::RaResult;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() -> WorkerGuard {
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::never("logs", "ra2.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = fmt::layer().with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);
    tracing_subscriber::registry().with(filter).with(stderr_layer).with(file_layer).init();
    guard
}

fn main() {
    if let Err(e) = run() {
        eprintln!("ra2 错误: {e}");
        tracing::error!("致命错误: {e}");
        std::process::exit(1);
    }
}

fn run() -> RaResult<()> {
    let _log_guard = init_tracing();
    tracing::info!("ra2 启动");
    shell::run_shell()
}
