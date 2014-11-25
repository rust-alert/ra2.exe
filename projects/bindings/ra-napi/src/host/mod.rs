//! 原生宿主：窗口、输入、装载与对局控制。

#![allow(missing_docs)]

pub mod audio;
pub mod boot;
pub mod config;
pub mod extract;
pub mod load_job;
pub mod local_player;
pub mod match_ctrl;
pub mod preview_job;
pub mod screenshot;
pub mod shell;
#[cfg(feature = "test-harness")]
pub mod test_boot;

use ra_types::RaResult;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

fn init_tracing() -> WorkerGuard {
    let _ = std::fs::create_dir_all("logs");
    let file_appender = tracing_appender::rolling::never("logs", "ra-napi-host.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let stderr_layer = fmt::layer().with_writer(std::io::stderr);
    let file_layer = fmt::layer().with_ansi(false).with_writer(non_blocking);
    tracing_subscriber::registry().with(filter).with(stderr_layer).with(file_layer).init();
    guard
}

/// 初始化日志并进入 GUI 事件循环。
pub fn run() -> RaResult<()> {
    let _log_guard = init_tracing();
    tracing::info!("ra-napi host 启动");
    shell::run_shell()
}
