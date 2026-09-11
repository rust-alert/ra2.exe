//! 冻结运行时定义契约：全体层共同语言。
//!
//! adaptor 填充；engine / renderer / desktop / testing / net 只消费。
//! 对局创建后不可变；不含 ECS、实体、资金、tick、路径或 GPU 句柄。
//! 禁止在引擎内按外部内容名（如 GAPOWR）做玩法分支。

mod animation;
mod armor;
mod capability;
mod deployable;
mod fingerprint;
pub mod foundation;
mod house_list;
mod locomotor;
mod names;
mod overlay;
mod production;
mod runtime;
mod sound;
pub mod structure;
mod super_weapon;
mod tech_tree;
mod techno;
mod terrain_spawner;
mod type_definition;
mod warhead;
mod warhead_verses;
mod weapon;

pub use animation::AnimationDefinitions;
pub use armor::{ARMOR_ORDER, ArmorKind, armor_index};
pub use capability::{BuiltinCapability, CapabilitySet};
pub use deployable::{DeployableDefinition, DeployableDefinitions, DeploymentPlacement};
pub use fingerprint::ContentFingerprint;
pub use foundation::Foundation;
pub use house_list::HouseAllowList;
pub use locomotor::LocomotorDefinitions;
pub use names::{HouseName, SuperWeaponActionName, SuperWeaponKindName, SuperWeaponName, TechnoName, WarheadName, WeaponName};
pub use overlay::OverlayTypeRegistry;
pub use production::{ProductionCategory, ProductionDefinitions, ProductionProfile, deserialize_optional_factory};
pub use runtime::RuntimeDefinitions;
pub use sound::SoundDefinitions;
pub use structure::{BuildCat, HouseDefinitions, PowerProfile, StructureDefinition, StructureDefinitions};
pub use super_weapon::{SuperWeaponDefinition, SuperWeaponDefinitions};
pub use tech_tree::{
    HouseStolenTechMap, PrerequisiteGroupKind, PrerequisiteGroups, PrerequisiteList, PrerequisiteToken, StolenTechKind,
};
pub use techno::{TechnoClass, TechnoDefinition, TechnoDefinitions};
pub use terrain_spawner::{TerrainSpawnerDefinition, TerrainSpawnerDefinitions};
pub use type_definition::TypeDefinitionId;
pub use warhead::{WarheadDefinition, WarheadDefinitions};
pub use warhead_verses::WarheadVerses;
pub use weapon::{WeaponDefinition, WeaponDefinitions};
