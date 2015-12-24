//! 冻结运行时定义契约：全体层共同语言。
//!
//! adaptor 填充；engine / renderer / desktop / testing / net 只消费。
//! 对局创建后不可变；不含 ECS、实体、资金、tick、路径或 GPU 句柄。
//! 禁止在引擎内按外部内容名（如 GAPOWR）做玩法分支。

mod animation;
mod capability;
mod deployable;
mod fingerprint;
mod locomotor;
mod production;
mod runtime;
mod sound;
mod structure;
mod tech_tree;
mod techno;
mod type_definition;
mod warhead;
mod weapon;

pub use animation::AnimationDefinitions;
pub use capability::{BuiltinCapability, CapabilitySet};
pub use deployable::{DeployableDefinition, DeployableDefinitions, DeploymentPlacement};
pub use fingerprint::ContentFingerprint;
pub use locomotor::LocomotorDefinitions;
pub use production::{ProductionCategory, ProductionDefinitions, ProductionProfile};
pub use runtime::RuntimeDefinitions;
pub use sound::SoundDefinitions;
pub use structure::{HouseDefinitions, PowerProfile, StructureDefinition, StructureDefinitions};
pub use tech_tree::PrerequisiteGroups;
pub use techno::{TechnoClass, TechnoDefinition, TechnoDefinitions};
pub use type_definition::TypeDefinitionId;
pub use warhead::{WarheadDefinition, WarheadDefinitions};
pub use weapon::WeaponDefinitions;
