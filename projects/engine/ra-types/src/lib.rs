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
    ARMOR_ORDER, AiTriggerName, AnimationDefinitions, ArmorKind, BuildCat, BuiltinCapability, CampaignName, CapabilitySet, ColorName,
    ContentFingerprint, DeployableDefinition, DeployableDefinitions, DeploymentPlacement, Foundation, GameModeName, HouseAllowList,
    HouseIdAllowList, HouseDefinition, HouseDefinitions, HouseName, HouseStolenTechMap, ImageName, LandType, LocomotorDefinitions, MapAction, MapActionCommand,
    MapAiTrigger, MapCellTag, MapDefinition, MapEdge, MapEvent, MapEventCondition, MapFileName, MapHouse, MapIsoCell, MapLighting,
    MapLocalSize, MapOverlayCell, MapPlacedEntity, MapPlacedEntityKind, MapScriptStep, MapScriptType, MapSmudge, MapTag, MapTaskForce,
    MapTaskForceEntry, MapTeamType, MapTerrainObject, MapTrigger, MapWaypoint, MapWeatherKind, MissionKind, MissionName, OverlayName,
    OverlayTypeRegistry, PowerProfile, PreparedCellTag, PreparedHouse, PreparedMap, PreparedPlacement, PreparedTag, PreparedTrigger,
    PrerequisiteGroupKind, PrerequisiteGroups, PrerequisiteList, PrerequisiteToken, ProductionCategory, ProductionDefinitions,
    ProductionProfile, ProjectileDefinition, ProjectileDefinitions, ProjectileName, RuntimeDefinitions, ScriptTypeName, SideName, SmudgeName,
    SoundDefinitions, StolenTechKind, StructureDefinition, StructureDefinitions, StructureLightProfile, SuperWeaponActionName,
    SuperWeaponDefinition, SuperWeaponDefinitions, SuperWeaponKindName, SuperWeaponName, TMP_TERRAIN_TO_LAND, TagName, TaskForceName,
    TeamTypeName, TechnoCategory, TechnoClass, TechnoDefinition, TechnoDefinitions, TechnoName, TerrainName, TerrainSpawnerDefinition,
    TerrainSpawnerDefinitions, Theater, TriggerName, TypeDefinitionId, UiName, WarheadDefinition, WarheadDefinitions, WarheadName,
    WarheadVerses, WeaponDefinition, WeaponDefinitions, WeaponName, armor_index, bind_map_cell_tags, bind_map_houses, bind_map_placements,
    bind_map_tags, bind_map_triggers, bind_prepared_map_placements, deserialize_optional_factory, ground_passable, land_passable,
    occupancy_kind, tmp_terrain_to_land_type,
};
pub use display_mode::DisplayMode;
pub use edition::GameEdition;
pub use error::{RaError, RaResult};
pub use id::{EntityId, HouseId, LocomotorId, PlayerId, ProjectileId, SessionId, TagId, TriggerId, TypeId, WarheadId, WeaponId};
pub use math::{Cell, Distance, Facing, Fixed, Fixed16, SubCell};
pub use present_feel::{PresentFeel, PresentMode, PresentQuantize};
pub use time::{DurationTicks, Tick, TickRate};
pub use ui_profile::{
    AssetRole, ControlId, ControlPlacement, DialogControlDesc, DialogTemplate, RuntimeUiProfile, TextKey, UiCapabilities, dialog_template_0x6b,
    dialog_template_0x102, dialog_template_0xd5,
};
pub use vga_expand::VgaExpandMode;
