//! 建筑与阵营定义表。

/// 阵营 / 房屋定义集合（骨架）。
#[derive(Debug, Clone, Default)]
pub struct HouseDefinitions {
    /// 条目数占位。
    pub count: u32,
}

/// 建筑占格与静态属性定义集合（骨架）。
#[derive(Debug, Clone, Default)]
pub struct StructureDefinitions {
    /// 条目数占位。
    pub count: u32,
}
