//! 装载任务：异步推进进度（浏览器侧对标 `ra-napi` load_job）。

/// 是否有进行中的装载任务。
pub fn is_busy() -> bool {
    false
}
