//! 冻结定义层能力缺口（Parsed 但未 Consumed / 未接线执行器）。

/// 一条机器可读的能力缺口报告（装载期写入，运行时只读）。
///
/// 与 adaptor 栈探测报告同形，供 boot / diagnose / 兼容性清单共用。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityGapReport {
    /// 机器可读码（如 `rules.superweapon.NUKE deferred`）。
    pub code: String,
    /// 面向日志或诊断 UI 的说明。
    pub message: String,
}

impl CapabilityGapReport {
    /// 构造一条报告。
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self { code: code.into(), message: message.into() }
    }

    /// 码是否表示「已定义但执行暂缓」（非阻断装载）。
    pub fn is_deferred(&self) -> bool {
        self.code.contains("deferred")
    }
}
