//! 弹头 Verses 辅助（无外部类型名硬编码）。

use ra_assets::WarheadRegistry;

pub(crate) fn full_verses() -> [u32; 11] {
    [100; 11]
}

pub(crate) fn verses_for(warheads: &WarheadRegistry, warhead: &str) -> [u32; 11] {
    warheads.get(warhead).map(|w| w.verses).unwrap_or_else(full_verses)
}
