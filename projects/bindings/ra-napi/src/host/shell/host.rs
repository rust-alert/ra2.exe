//! 平台宿主边界：窗口、事件循环与呈现入口。
//!
//! Host 负责平台副作用；Shell 负责壳层 UI 会话状态。

use ra_types::RaResult;

/// 平台中性宿主入口（desktop / 未来 wasm 共用语义）。
pub struct Host;

impl Host {
    /// 初始化并进入壳层事件循环。
    pub fn run() -> RaResult<()> {
        super::launch::run_shell()
    }
}
