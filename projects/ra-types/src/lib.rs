//! 全体 ra-* crate 共享的基类型与冻结定义契约。
//!
//! 定义归本 crate；适配归 `ra-adaptor`；执行归 `ra-engine`。
//! 不存在独立的 `ra-definition` / `ra-rules` crate。

#![deny(missing_docs)]

mod asset_source;
mod command;
pub mod definition;
mod edition;
mod error;
mod id;
mod math;
mod time;

pub use asset_source::AssetSource;
pub use command::{CommandBody, CommandId, CommandKind, CommandTarget, ScheduledCommand};
pub use definition::{
    AnimationDefinitions, BuiltinCapability, CapabilitySet, ContentFingerprint, DeployableDefinition, DeployableDefinitions,
    DeploymentPlacement, HouseDefinitions, LocomotorDefinitions, PowerProfile, ProductionCategory, ProductionDefinitions, ProductionProfile,
    RuntimeDefinitions, SoundDefinitions, StructureDefinition, StructureDefinitions, TechnoClass, TechnoDefinition, TechnoDefinitions,
    TypeDefinitionId, WarheadDefinition, WarheadDefinitions, WeaponDefinitions,
};
pub use edition::GameEdition;
pub use error::{RaError, RaResult};
pub use id::{EntityId, HouseId, LocomotorId, PlayerId, SessionId, TypeId, WarheadId, WeaponId};
pub use math::{Cell, Distance, Facing, Fixed, Fixed16, SubCell};
pub use time::{DurationTicks, Tick, TickRate};
