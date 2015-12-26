//! 对局级冻结定义集。

use super::{
    AnimationDefinitions, CapabilitySet, ContentFingerprint, DeployableDefinitions, HouseDefinitions, HouseStolenTechMap,
    LocomotorDefinitions, PrerequisiteGroups, ProductionDefinitions, SoundDefinitions, StructureDefinitions,
    TechnoDefinitions, WarheadDefinitions, WeaponDefinitions,
};

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
    /// `[General]` 通用前置组。
    pub prerequisite_groups: PrerequisiteGroups,
    /// house → 渗透其科技建筑时授予的偷取科技。
    pub stolen_tech_by_house: HouseStolenTechMap,
    /// 遭遇战 / 多人默认科技上限（`[MultiplayerDialogSettings] TechLevel`，缺省 10）。
    pub default_tech_level: i32,
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
            locomotors: LocomotorDefinitions::default(),
            structures: StructureDefinitions::default(),
            deployables: DeployableDefinitions::default(),
            production: ProductionDefinitions::default(),
            animations: AnimationDefinitions::default(),
            sounds: SoundDefinitions::default(),
            prerequisite_groups: PrerequisiteGroups::default(),
            stolen_tech_by_house: HouseStolenTechMap::default(),
            default_tech_level: 10,
        }
    }
}
