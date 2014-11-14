//! 原生窗口、输入与事件循环实现。
//!
//! 只给 `ra-napi` 用。对外产品入口是 npm 包 `@game-gpt/red-alert2`，不是本 crate。

#![allow(missing_docs)]

pub mod audio;
pub mod boot;
pub mod config;
pub mod extract;
pub mod fs_source;
pub mod load_job;
pub mod local_player;
pub mod match_ctrl;
pub mod menu_action;
pub mod options_dialog;
pub mod preview_job;
pub mod screen;
pub mod screenshot;
pub mod shell;
pub mod shell_slide;
pub mod skirmish_setup;
pub mod startup_splash;
#[cfg(feature = "test-harness")]
pub mod test_boot;
pub mod ui_assets;
pub mod ui_compose;
pub mod ui_decode;
pub mod ui_hit;
pub mod ui_layout;
pub mod ui_movie;
pub mod ui_page;
pub mod ui_present;
pub mod ui_resolve;
pub mod ui_slots;
pub mod ui_text;
pub mod ui_typewriter;

use ra_types::RaResult;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() -> WorkerGuard {
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::never("logs", "ra-desktop.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = fmt::layer().with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);
    tracing_subscriber::registry().with(filter).with(stderr_layer).with(file_layer).init();
    guard
}

/// 初始化日志并进入 GUI 事件循环（供 N-API / 示例调用）。
pub fn run() -> RaResult<()> {
    let _log_guard = init_tracing();
    tracing::info!("ra-desktop 启动");
    shell::run_shell()
}
