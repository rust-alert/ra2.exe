//! 模板与遗留参照。

mod dlu;
mod legacy;

pub use dlu::{mul_div_round, DluRect, FontBaseUnits, MS_SANS_SERIF_8PT};
pub use legacy::{LegacyReference, LegacyRole, LegacySource};
