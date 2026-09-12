//! 冻结运行时定义契约：全体层共同语言。
//!
//! adaptor 填充；engine / renderer / desktop / testing / net 只消费。
//! 对局创建后不可变；不含 ECS、实体、资金、tick、路径或 GPU 句柄。
//! 禁止在引擎内按外部内容名（如 GAPOWR）做玩法分支。
//!
//! 本 crate 类型是**运行最优形状**，可随执行需求改布局；不是 INI / 地图文件的存储 schema。
//! 原版兼容在 loader / adaptor 侧完成投影。

mod animation;
mod armor;
mod capability;
mod category;
mod deployable;
mod fingerprint;
pub mod foundation;
mod house_list;
mod land;
mod locomotor;
mod map;
mod map_edge;
mod names;
mod overlay;
mod production;
mod projectile;
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
pub use category::TechnoCategory;
pub use deployable::{DeployableDefinition, DeployableDefinitions, DeploymentPlacement};
pub use fingerprint::ContentFingerprint;
pub use foundation::Foundation;
pub use house_list::HouseAllowList;
pub use land::{LandType, TMP_TERRAIN_TO_LAND, ground_passable, land_passable, tmp_terrain_to_land_type};
pub use locomotor::LocomotorDefinitions;
pub use map::{
    MapAction, MapActionCommand, MapAiTrigger, MapCellTag, MapDefinition, MapEvent, MapEventCondition, MapHouse, MapIsoCell, MapLighting,
    MapLocalSize, MapOverlayCell, MapPlacedEntity, MapPlacedEntityKind, MapScriptStep, MapScriptType, MapSmudge, MapTag, MapTaskForce,
    MapTaskForceEntry, MapTeamType, MapTerrainObject, MapTrigger, MapWaypoint, MapWeatherKind, PreparedMap, occupancy_kind,
};
pub use map_edge::MapEdge;
pub use names::{
    ColorName, HouseName, ImageName, ProjectileName, SuperWeaponActionName, SuperWeaponKindName, SuperWeaponName, TechnoName, UiName,
    WarheadName, WeaponName,
};
pub use overlay::OverlayTypeRegistry;
pub use production::{ProductionCategory, ProductionDefinitions, ProductionProfile, deserialize_optional_factory};
pub use projectile::{ProjectileDefinition, ProjectileDefinitions};
pub use runtime::RuntimeDefinitions;
pub use sound::SoundDefinitions;
pub use structure::{
    BuildCat, HouseDefinition, HouseDefinitions, PowerProfile, StructureDefinition, StructureDefinitions, StructureLightProfile,
};
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
