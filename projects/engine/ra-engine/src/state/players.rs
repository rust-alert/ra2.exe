//! 玩家侧仿真状态（资金、电力等）。

use std::sync::Arc;

use ra_types::PlayerId;

/// 一名玩家在世界中的可哈希状态。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerState {
    /// 稳定玩家编号。
    pub id: PlayerId,
    /// 阵营 / 房主名称（与地图放置段 `owner` 对齐，`Arc` 共享）。
    pub house: Arc<str>,
    /// 当前资金。
    pub funds: i32,
    /// 供电量。
    pub power_output: i32,
    /// 耗电量。
    pub power_drain: i32,
    /// 累计花费（建造与生产扣款之和，不含采矿收入）。
    pub funds_spent: i32,
    /// 间谍渗透电厂后的断电剩余 tick；>0 时有效供电视为 0。
    pub power_blackout_ticks: u32,
    /// 渗透步兵厂后，新产步兵享受简易晋升级。
    pub promoted_infantry: bool,
    /// 渗透车厂后，新产载具享受简易晋升级。
    pub promoted_vehicle: bool,
}

impl PlayerState {
    /// 构造默认经济字段的玩家。
    pub fn new(id: PlayerId, house: impl AsRef<str>) -> Self {
        Self {
            id,
            house: Arc::<str>::from(house.as_ref()),
            funds: 0,
            power_output: 0,
            power_drain: 0,
            funds_spent: 0,
            power_blackout_ticks: 0,
            promoted_infantry: false,
            promoted_vehicle: false,
        }
    }

    /// 断电期间有效供电为 0。
    pub fn effective_power_output(&self) -> i32 {
        if self.power_blackout_ticks > 0 {
            0
        } else {
            self.power_output
        }
    }

    /// 是否处于低电（耗电大于有效供电）。
    pub fn low_power(&self) -> bool {
        self.power_drain > self.effective_power_output()
    }
}
