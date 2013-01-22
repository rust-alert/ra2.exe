//! 全体 ra-* crate 共享的基类型。

mod asset_source;
mod edition;
mod error;
mod fixed;
mod ids;

pub use asset_source::AssetSource;
pub use edition::{detect_edition, EditionManifest, GameEdition, ResourceChain};
pub use error::{RaError, RaResult};
pub use fixed::Fixed16;
pub use ids::{EntityId, PlayerId, TypeId};
