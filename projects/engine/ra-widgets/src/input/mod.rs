//! 输入命中。

pub mod hit;
pub mod snapshot_hit;

pub use hit::*;
pub use snapshot_hit::{hit_element_at, hit_id_at};
