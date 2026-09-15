//! rules `[AI]` / `[IQ]` 冻结控制表：AI 基建候选与门闩。
//!
//! 引擎只按 `TypeId` 消费；候选内容由 adaptor 从 rules 姓名单绑定，禁止写死外部类型名。

use crate::id::TypeId;

/// 带比例 / 数量上限的 AI 建造类别（矿场、兵营、车厂等）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiBuildCategory {
    /// 候选建筑稳定 id（rules 列表顺序）。
    pub candidates: Vec<TypeId>,
    /// 相对己方建筑总数的目标比例（千分比，`ratio * 1000`）。
    pub ratio_millis: u32,
    /// 同时拥有上限；`0` 表示不按数量封顶（仍受比例约束）。
    pub limit: u32,
}

/// 冻结 AI 基建控制（来自 rules `[AI]` + `[IQ]`）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AiControls {
    /// `[AI] BuildConst`：建造场候选。
    pub build_const: Vec<TypeId>,
    /// `[AI] BuildPower`：电厂候选。
    pub build_power: Vec<TypeId>,
    /// `[AI] PowerSurplus`：目标电力富余。
    pub power_surplus: i32,
    /// `[AI] BuildRefinery`。
    pub build_refinery: AiBuildCategory,
    /// `[AI] BuildBarracks`。
    pub build_barracks: AiBuildCategory,
    /// `[AI] BuildWeapons`（战车工厂）。
    pub build_weapons: AiBuildCategory,
    /// `[AI] BuildRadar`。
    pub build_radar: Vec<TypeId>,
    /// `[AI] BuildTech`。
    pub build_tech: Vec<TypeId>,
    /// `[AI] BuildNavalYard`。
    pub build_naval_yard: Vec<TypeId>,
    /// `[AI] BuildHelipad`。
    pub build_helipad: AiBuildCategory,
    /// `[AI] BuildDefense`。
    pub build_defense: AiBuildCategory,
    /// `[AI] BuildAA`。
    pub build_aa: AiBuildCategory,
    /// `[AI] BuildDummy`。
    pub build_dummy: Vec<TypeId>,
    /// `[AI] AIBaseSpacing`（与 `RuntimeDefinitions.ai_base_spacing` 同值快照）。
    pub base_spacing: u32,
    /// `[AI] BaseSizeAdd`。
    pub base_size_add: u32,
    /// `[IQ] MaxIQLevels`。
    pub max_iq_levels: i32,
    /// `[IQ] Production`：达到该 IQ 才推进工厂类扩展。
    pub iq_production: i32,
}
