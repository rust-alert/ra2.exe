//! 玩家侧仿真状态（资金、电力等）。

use ra_types::PlayerId;

/// 一名玩家在世界中的可哈希状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    /// 稳定玩家编号。
    pub id: PlayerId,
    /// 阵营 / 房主名称（与地图放置段 `owner` 对齐）。
    pub house: String,
    /// 当前资金。
    pub funds: i32,
    /// 供电量。
    pub power_output: i32,
    /// 耗电量。
    pub power_drain: i32,
}

impl PlayerState {
    /// 构造默认经济字段的玩家。
    pub fn new(id: PlayerId, house: impl Into<String>) -> Self {
        Self {
            id,
            house: house.into(),
            funds: 0,
            power_output: 0,
            power_drain: 0,
        }
    }

    /// 是否处于低电（耗电大于供电）。
    pub fn low_power(&self) -> bool {
        self.power_drain > self.power_output
    }
}
