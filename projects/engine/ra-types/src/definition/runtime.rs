//! 对局级冻结定义集。

use super::{
    AiControls, AnimationDefinitions, CapabilityGapReport, CapabilitySet, ContentFingerprint, CrateRules, DeployableDefinitions,
    HouseDefinitions, HouseStolenTechMap, InfiltrationRules, LightningStormRules, LocomotorDefinitions, OverlayTypeRegistry,
    PrerequisiteGroups, ProductionDefinitions, ProjectileDefinitions, SoundDefinitions, StructureDefinitions, SuperWeaponDefinitions,
    TechnoDefinitions, TerrainSpawnerDefinitions, WarheadDefinitions, WeaponDefinitions,
};
use crate::id::TypeId;

/// 全体层共享的冻结运行时定义。
///
/// - 对局创建后不可变、可共享、可稳定指纹化；
/// - 不含 ECS、当前实体、资金、tick、文件路径、MIX/GPU/窗口句柄；
/// - 消费者不必知道原始 INI 文本；
/// - 引擎按 `type_key` / `TypeId` 查询，不得硬编码外部内容名。
#[derive(Debug, Clone)]
pub struct RuntimeDefinitions {
    /// 内容指纹。
    pub content_fingerprint: ContentFingerprint,
    /// 能力集。
    pub capabilities: CapabilitySet,
    /// 阵营。
    pub houses: HouseDefinitions,
    /// Techno。
    pub techno: TechnoDefinitions,
    /// 武器。
    pub weapons: WeaponDefinitions,
    /// 弹头。
    pub warheads: WarheadDefinitions,
    /// 抛射体。
    pub projectiles: ProjectileDefinitions,
    /// 移动器。
    pub locomotors: LocomotorDefinitions,
    /// 建筑。
    pub structures: StructureDefinitions,
    /// 部署关系。
    pub deployables: DeployableDefinitions,
    /// 生产。
    pub production: ProductionDefinitions,
    /// 动画。
    pub animations: AnimationDefinitions,
    /// 音效。
    pub sounds: SoundDefinitions,
    /// `[SuperWeaponTypes]` 超级武器表。
    pub super_weapons: SuperWeaponDefinitions,
    /// `[General]` 通用前置组。
    pub prerequisite_groups: PrerequisiteGroups,
    /// house → 渗透其科技建筑时授予的偷取科技。
    pub stolen_tech_by_house: HouseStolenTechMap,
    /// 遭遇战 / 多人默认科技上限（`[MultiplayerDialogSettings] TechLevel`，缺省 10）。
    pub default_tech_level: i32,
    /// `[General] RepairPercent`：完全修好相对造价的百分比（缺省 15）。
    pub repair_percent: u32,
    /// `[General] RefundPercent`：出售时相对造价的百分比（缺省 50；`Soylent` 优先）。
    pub refund_percent: u32,
    /// `[General] RepairStep`：每修理脉冲回复生命（缺省 8）。
    pub repair_step: u32,
    /// `[General] RepairRate`（分钟）换算的脉冲间隔 tick：`ftol(rate * 900)`（缺省 14）。
    pub repair_interval_ticks: u64,
    /// `[AudioVisual]` / `[General] SpeakDelay`（分钟）× 900 → 逻辑 tick；0 表示关闭。
    pub speak_delay_ticks: u32,
    /// `[AudioVisual] SavourDelay`（分钟）× 900 → 逻辑 tick；0 表示立即锁定胜负。
    pub savour_delay_ticks: u32,
    /// `SpawnsTiberium` 动画地形产矿表。
    pub terrain_spawners: TerrainSpawnerDefinitions,
    /// `[OverlayTypes]` 声明序类型表（含可采标记）。
    pub overlays: OverlayTypeRegistry,
    /// `[General] BaseUnit`：短局保活载具稳定 id。
    pub base_units: Vec<TypeId>,
    /// `[AI] AIBaseSpacing`：AI 建筑之间最少空隙格数（零售缺省 1）。
    pub ai_base_spacing: u32,
    /// `[General] AINavalYardAdjacency`：AI 船厂相对建造场的最大切比雪夫距离（零售缺省 20）。
    pub ai_naval_yard_adjacency: u32,
    /// rules `[AI]` / `[IQ]` 冻结基建控制表。
    pub ai_controls: AiControls,
    /// 间谍渗透数值。
    pub infiltration: InfiltrationRules,
    /// 箱子奖励表。
    pub crate_rules: CrateRules,
    /// 闪电风暴执行参数。
    pub lightning_storm: LightningStormRules,
    /// 装载期发现的能力缺口（含「有定义无执行器」）。
    pub capability_gaps: Vec<CapabilityGapReport>,
}

impl Default for RuntimeDefinitions {
    fn default() -> Self {
        Self {
            content_fingerprint: ContentFingerprint::default(),
            capabilities: CapabilitySet::default(),
            houses: HouseDefinitions::default(),
            techno: TechnoDefinitions::default(),
            weapons: WeaponDefinitions::default(),
            warheads: WarheadDefinitions::default(),
            projectiles: ProjectileDefinitions::default(),
            locomotors: LocomotorDefinitions::default(),
            structures: StructureDefinitions::default(),
            deployables: DeployableDefinitions::default(),
            production: ProductionDefinitions::default(),
            animations: AnimationDefinitions::default(),
            sounds: SoundDefinitions::default(),
            super_weapons: SuperWeaponDefinitions::default(),
            prerequisite_groups: PrerequisiteGroups::default(),
            stolen_tech_by_house: HouseStolenTechMap::default(),
            default_tech_level: 10,
            repair_percent: 15,
            refund_percent: 50,
            repair_step: 8,
            repair_interval_ticks: 14,
            speak_delay_ticks: 0,
            savour_delay_ticks: 0,
            terrain_spawners: TerrainSpawnerDefinitions::default(),
            overlays: OverlayTypeRegistry::default(),
            base_units: Vec::new(),
            ai_base_spacing: super::structure::DEFAULT_AI_BASE_SPACING,
            ai_naval_yard_adjacency: super::structure::DEFAULT_AI_NAVAL_YARD_ADJACENCY,
            ai_controls: AiControls::default(),
            infiltration: InfiltrationRules::default(),
            crate_rules: CrateRules::default(),
            lightning_storm: LightningStormRules::default(),
            capability_gaps: Vec::new(),
        }
    }
}
