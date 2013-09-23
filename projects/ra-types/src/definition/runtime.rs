//! 对局级冻结定义集。

use super::{
    AnimationDefinitions, CapabilitySet, ContentFingerprint, HouseDefinitions, LocomotorDefinitions, ProductionDefinitions,
    SoundDefinitions, StructureDefinitions, TechnoDefinitions, WarheadDefinitions, WeaponDefinitions,
};

/// 全体层共享的冻结运行时定义。
///
/// - 对局创建后不可变、可共享、可稳定指纹化；
/// - 不含 ECS、当前实体、资金、tick、文件路径、MIX/GPU/窗口句柄；
/// - 消费者不必知道原始 INI 文本。
#[derive(Debug, Clone, Default)]
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
    /// 生产。
    pub production: ProductionDefinitions,
    /// 动画。
    pub animations: AnimationDefinitions,
    /// 音效。
    pub sounds: SoundDefinitions,
}
