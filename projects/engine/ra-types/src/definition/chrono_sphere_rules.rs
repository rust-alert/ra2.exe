//! 超时空传送全局规则。

/// 超时空传送执行参数（装载期冻结；竖切缺省）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChronoSphereRules {
    /// 第一次点击选取友军机动单位的半径（切比雪夫格数，含中心）。
    pub source_radius_cells: u32,
    /// 第二次点击落点允许的邻域半径（切比雪夫格数）。
    pub dest_search_radius_cells: u32,
}

impl Default for ChronoSphereRules {
    fn default() -> Self {
        Self {
            source_radius_cells: 3,
            dest_search_radius_cells: 2,
        }
    }
}
