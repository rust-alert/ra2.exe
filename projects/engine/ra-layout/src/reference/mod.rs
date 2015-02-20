//! 模板与遗留参照。

mod dialog_0x102;
mod dialog_0x6b;
mod dlu;
mod legacy;

pub use dialog_0x102::{dialog_0x102_layout_tree, resolve_dialog_0x102};
pub use dialog_0x6b::{dialog_0x6b_layout_tree, resolve_dialog_0x6b, shell_design_size};
pub use dlu::{mul_div_round, DluRect, FontBaseUnits, MS_SANS_SERIF_8PT};
pub use legacy::{LegacyReference, LegacyRole, LegacySource};
