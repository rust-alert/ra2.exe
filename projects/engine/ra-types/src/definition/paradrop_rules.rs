//! 空降超武载荷冻结表。

use crate::id::TypeId;

/// 空降类超武刷出的单位载荷（装载期冻结；缺省由 adaptor 软绑定零售步兵名）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParaDropRules {
    /// 按顺序刷出的单位稳定 id（可重复）。
    pub payload: Vec<TypeId>,
}
