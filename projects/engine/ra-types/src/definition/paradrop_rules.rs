//! 空降超武载荷冻结表。

use crate::id::TypeId;

/// 空降类超武刷出的单位载荷（装载期冻结；缺省由 adaptor 软绑定零售步兵名）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParaDropRules {
    /// 通用回落载荷（规则缺阵营键或未知阵营时使用）。
    pub payload: Vec<TypeId>,
    /// `[General] AmerParaDropInf/Num`：美军专用载荷。
    pub americans: Vec<TypeId>,
    /// `[General] AllyParaDropInf/Num`：盟军（`Side=GDI`）载荷。
    pub allies: Vec<TypeId>,
    /// `[General] SovParaDropInf/Num`：苏军（`Side=Nod`）载荷。
    pub soviets: Vec<TypeId>,
}
