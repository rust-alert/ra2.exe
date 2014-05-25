//! 内容指纹：确认双方对局材料一致。

/// 规则 / 地图 / 能力组合等材料的稳定摘要。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ContentFingerprint {
    /// 当前实现用 `u64` 承载；算法与混入集合后续冻结。
    pub hash: u64,
}
