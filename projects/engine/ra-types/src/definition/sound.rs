//! 音效定义表（逻辑名 / ID，不含混音句柄）。

/// 音效定义集合（骨架）。
#[derive(Debug, Clone, Default)]
pub struct SoundDefinitions {
    /// 条目数占位。
    pub count: u32,
}
