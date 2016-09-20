//! 全体 ra-* crate 共享的基类型与冻结定义契约。
//!
//! 定义归本 crate；适配归 `ra-adaptor`；执行归 `ra-engine`。
//! 不存在独立的 `ra-definition` / `ra-rules` crate。

#![deny(missing_docs)]

mod asset_source;
mod command;
pub mod definition;
mod display_mode;
mod edition;
mod error;
mod id;
mod math;
mod present_feel;
mod time;
pub mod ui_profile;
mod vga_expand;

pub use asset_source::AssetSource;
pub use command::{CommandBody, CommandId, CommandKind, CommandTarget, ScheduledCommand};
pub use definition::{
    ARMOR_ORDER, AnimationDefinitions, ArmorKind, BuildCat, BuiltinCapability, CapabilitySet, ContentFingerprint, DeployableDefinition,
    DeployableDefinitions, DeploymentPlacement, Foundation, HouseAllowList, HouseDefinition, HouseDefinitions, HouseName, HouseStolenTechMap,
    ImageName, LocomotorDefinitions, MapAction, MapActionCommand, MapAiTrigger, MapCellTag, MapDefinition, MapEvent, MapEventCondition,
    MapHouse, MapIsoCell, MapLighting, MapLocalSize, MapOverlayCell, MapPlacedEntity, MapPlacedEntityKind, MapScriptStep, MapScriptType, MapTag,
    MapTaskForce, MapTaskForceEntry, MapTeamType, MapTerrainObject, MapTrigger, MapWaypoint, OverlayTypeRegistry, PowerProfile,
    PrerequisiteGroupKind, PrerequisiteGroups, PrerequisiteList, PrerequisiteToken, PreparedMap, ProductionCategory, ProductionDefinitions,
    ProductionProfile, ProjectileDefinition, ProjectileDefinitions, ProjectileName, RuntimeDefinitions, SoundDefinitions, StolenTechKind,
    StructureDefinition, StructureDefinitions, StructureLightProfile, SuperWeaponActionName, SuperWeaponDefinition, SuperWeaponDefinitions,
    SuperWeaponKindName, SuperWeaponName, TechnoCategory, TechnoClass, TechnoDefinition, TechnoDefinitions, TechnoName,
    TerrainSpawnerDefinition, TerrainSpawnerDefinitions, TypeDefinitionId, UiName, WarheadDefinition, WarheadDefinitions, WarheadName,
    WarheadVerses, WeaponDefinition, WeaponDefinitions, WeaponName, armor_index, deserialize_optional_factory,
};
pub use display_mode::DisplayMode;
pub use edition::GameEdition;
pub use error::{RaError, RaResult};
pub use id::{EntityId, HouseId, LocomotorId, PlayerId, ProjectileId, SessionId, TypeId, WarheadId, WeaponId};
pub use math::{Cell, Distance, Facing, Fixed, Fixed16, SubCell};
pub use present_feel::{PresentFeel, PresentMode, PresentQuantize};
pub use time::{DurationTicks, Tick, TickRate};
pub use ui_profile::{
    AssetRole, ControlId, ControlPlacement, DialogControlDesc, DialogTemplate, RuntimeUiProfile, TextKey, UiCapabilities, dialog_template_0x6b,
    dialog_template_0x102,
};
pub use vga_expand::VgaExpandMode;
