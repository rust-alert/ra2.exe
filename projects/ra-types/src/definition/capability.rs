//! 内容能力集（基础环境 + 扩展能力声明）。

/// 本份定义启用的能力开关集合（骨架：后续改为稳定位或表）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    /// 占位：能力标识列表（稳定字符串或后续改为 ID）。
    pub tags: Vec<String>,
}
