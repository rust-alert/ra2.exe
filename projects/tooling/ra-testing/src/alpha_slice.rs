//! Alpha 竖切冻结数据（合成夹具，不读安装目录）。

/// 冻结竖切标识。
pub const ALPHA_SKIRMISH_SLICE_ID: &str = "alpha-skirmish-v1";

/// 建筑在竖切中的职能。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceBuildingRole {
    /// 建造场。
    ConstructionYard,
    /// 供电。
    Power,
    /// 兵营。
    Barracks,
    /// 战车工厂。
    WarFactory,
    /// 矿场。
    Refinery,
}

/// 单位在竖切中的职能。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliceUnitRole {
    /// 步兵。
    Infantry,
    /// 载具。
    Vehicle,
    /// 机动建造车。
    Mcv,
}

/// 一条冻结建筑定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceBuilding {
    /// 规则类型 ID。
    pub type_id: &'static str,
    /// `allied` 或 `soviet`。
    pub side: &'static str,
    /// 职能。
    pub role: SliceBuildingRole,
    /// 造价。
    pub cost: i32,
    /// 供电贡献（正）或耗电（负）。
    pub power: i32,
}

/// 一条冻结单位定义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SliceUnit {
    /// 规则类型 ID。
    pub type_id: &'static str,
    /// `allied` 或 `soviet`。
    pub side: &'static str,
    /// 职能。
    pub role: SliceUnitRole,
    /// 造价。
    pub cost: i32,
}

/// Alpha 首版竖切内容清单。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlphaSkirmishSlice {
    /// 切片 ID。
    pub slice_id: &'static str,
    /// 基础环境。
    pub edition: &'static str,
    /// 固定地图 ID。
    pub map_id: &'static str,
    /// 人类阵营 house。
    pub human_house: &'static str,
    /// AI 阵营 house。
    pub ai_house: &'static str,
    /// 开局资金。
    pub starting_funds: i32,
    /// 冻结随机种子（仅 headless / 自动化复现；产品大厅不钉死）。
    pub match_seed: u64,
    /// 矿场每趟采矿入账。
    pub ore_income_per_trip: i32,
    /// 盟军 MCV 类型。
    pub allied_mcv: &'static str,
    /// 苏军 MCV 类型。
    pub soviet_mcv: &'static str,
    /// 建筑表。
    pub buildings: &'static [SliceBuilding],
    /// 单位表。
    pub units: &'static [SliceUnit],
}

/// 返回 `alpha-skirmish-v1` 冻结清单。
pub fn alpha_skirmish_v1() -> AlphaSkirmishSlice {
    AlphaSkirmishSlice {
        slice_id: ALPHA_SKIRMISH_SLICE_ID,
        edition: "ra2",
        map_id: "mp03t4",
        human_house: "Americans",
        ai_house: "Russians",
        starting_funds: 10_000,
        match_seed: 0xA1_0A_5EED_0000_0001,
        ore_income_per_trip: 700,
        allied_mcv: "AMCV",
        soviet_mcv: "SMCV",
        buildings: &[
            SliceBuilding { type_id: "GACNST", side: "allied", role: SliceBuildingRole::ConstructionYard, cost: 2500, power: 0 },
            SliceBuilding { type_id: "NACNST", side: "soviet", role: SliceBuildingRole::ConstructionYard, cost: 2500, power: 0 },
            SliceBuilding { type_id: "GAPOWR", side: "allied", role: SliceBuildingRole::Power, cost: 600, power: 200 },
            SliceBuilding { type_id: "NAPOWR", side: "soviet", role: SliceBuildingRole::Power, cost: 600, power: 200 },
            SliceBuilding { type_id: "GAPILE", side: "allied", role: SliceBuildingRole::Barracks, cost: 500, power: -20 },
            SliceBuilding { type_id: "NAHAND", side: "soviet", role: SliceBuildingRole::Barracks, cost: 500, power: -20 },
            SliceBuilding { type_id: "GAWEAP", side: "allied", role: SliceBuildingRole::WarFactory, cost: 2000, power: -30 },
            SliceBuilding { type_id: "NAWEAP", side: "soviet", role: SliceBuildingRole::WarFactory, cost: 2000, power: -30 },
            SliceBuilding { type_id: "GAREFN", side: "allied", role: SliceBuildingRole::Refinery, cost: 2000, power: -50 },
            SliceBuilding { type_id: "NAREFN", side: "soviet", role: SliceBuildingRole::Refinery, cost: 2000, power: -50 },
        ],
        units: &[
            SliceUnit { type_id: "E1", side: "allied", role: SliceUnitRole::Infantry, cost: 200 },
            SliceUnit { type_id: "E2", side: "soviet", role: SliceUnitRole::Infantry, cost: 200 },
            SliceUnit { type_id: "MTNK", side: "allied", role: SliceUnitRole::Vehicle, cost: 800 },
            SliceUnit { type_id: "HTNK", side: "soviet", role: SliceUnitRole::Vehicle, cost: 900 },
            SliceUnit { type_id: "AMCV", side: "allied", role: SliceUnitRole::Mcv, cost: 2500 },
            SliceUnit { type_id: "SMCV", side: "soviet", role: SliceUnitRole::Mcv, cost: 2500 },
        ],
    }
}
