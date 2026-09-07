//! 引擎版本与内容兼容信息。

/// 引擎版本（骨架）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineVersion {
    /// 人类可读版本标签。
    pub label: String,
}
