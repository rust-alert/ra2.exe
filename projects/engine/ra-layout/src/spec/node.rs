// reshape-layout-components:skeleton
//! 布局树节点。

use crate::policy::LayoutRules;

/// 布局节点标识（稳定字符串标签，后续可换成数值 Id）。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LayoutId(pub String);

/// 布局树节点（骨架）。
#[derive(Debug, Clone)]
pub struct LayoutNode {
    /// 标识。
    pub id: LayoutId,
    /// 规则。
    pub rules: LayoutRules,
    /// 子节点。
    pub children: Vec<LayoutNode>,
}

impl LayoutNode {
    /// 叶子节点。
    pub fn leaf(id: impl Into<String>, rules: LayoutRules) -> Self {
        Self { id: LayoutId(id.into()), rules, children: Vec::new() }
    }
}
