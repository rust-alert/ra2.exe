//! 简易文件日志器：追加写入 `logs/`，可选同步到 stderr。
//!
//! 桌面主路径用；Wasm 目标可后续接 `console`，本 crate 不强制 `std::fs` 以外的平台。

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ra_types::{RaError, RaResult};

static LOGGER: OnceLock<Mutex<FileLogger>> = OnceLock::new();

/// 日志级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Error = 1,
    Warn = 2,
    Info = 3,
    Debug = 4,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Error => "ERROR",
            Level::Warn => "WARN",
            Level::Info => "INFO",
            Level::Debug => "DEBUG",
        }
    }
}

struct FileLogger {
    file: File,
    path: PathBuf,
    echo_stderr: bool,
    max_level: Level,
}

impl FileLogger {
    fn write_line(&mut self, level: Level, msg: &str) {
        if level > self.max_level {
            return;
        }
        let ts = unix_millis();
        let line = format!("[{ts}] {} {msg}\n", level.as_str());
        let _ = self.file.write_all(line.as_bytes());
        let _ = self.file.flush();
        if self.echo_stderr {
            eprint!("{line}");
        }
    }
}

/// 初始化全局日志。目录不存在则创建；文件为 `logs/ra2.log`（追加）。
///
/// `echo_stderr`：同时打到 stderr（启动诊断仍可见）。
/// 重复调用时复用已打开的文件。
pub fn init(logs_dir: impl AsRef<Path>, echo_stderr: bool) -> RaResult<PathBuf> {
    if let Some(p) = path() {
        return Ok(p);
    }
    let dir = logs_dir.as_ref();
    fs::create_dir_all(dir).map_err(|e| RaError::Io(format!("{}: {e}", dir.display())))?;
    let path = dir.join("ra2.log");
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| RaError::Io(format!("{}: {e}", path.display())))?;
    let logger = FileLogger {
        file,
        path: path.clone(),
        echo_stderr,
        max_level: Level::Debug,
    };
    match LOGGER.set(Mutex::new(logger)) {
        Ok(()) => {
            info(&format!("日志已打开 {}", path.display()));
            Ok(path)
        }
        Err(_) => path_or(path),
    }
}

fn path_or(fallback: PathBuf) -> RaResult<PathBuf> {
    Ok(path().unwrap_or(fallback))
}

/// 工作目录下的 `logs/`。
pub fn init_default(echo_stderr: bool) -> RaResult<PathBuf> {
    init("logs", echo_stderr)
}

fn with_logger(f: impl FnOnce(&mut FileLogger)) {
    let Some(lock) = LOGGER.get() else {
        return;
    };
    if let Ok(mut g) = lock.lock() {
        f(&mut g);
    }
}

pub fn log(level: Level, msg: impl AsRef<str>) {
    let msg = msg.as_ref();
    with_logger(|l| l.write_line(level, msg));
}

pub fn error(msg: impl AsRef<str>) {
    log(Level::Error, msg);
}

pub fn warn(msg: impl AsRef<str>) {
    log(Level::Warn, msg);
}

pub fn info(msg: impl AsRef<str>) {
    log(Level::Info, msg);
}

pub fn debug(msg: impl AsRef<str>) {
    log(Level::Debug, msg);
}

/// 当前日志文件路径（若已 init）。
pub fn path() -> Option<PathBuf> {
    LOGGER
        .get()
        .and_then(|l| l.lock().ok().map(|g| g.path.clone()))
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn writes_append_file() {
        let dir = std::env::temp_dir().join(format!("ra-logger-test-{}", unix_millis()));
        let _ = fs::remove_dir_all(&dir);
        let path = init(&dir, false).unwrap();
        info("hello");
        warn("careful");
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("INFO hello"));
        assert!(text.contains("WARN careful"));
        let _ = fs::remove_dir_all(&dir);
    }
}
