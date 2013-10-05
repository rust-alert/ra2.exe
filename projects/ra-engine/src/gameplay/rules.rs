//! 弹头 Verses 辅助（无外部类型名硬编码）。

use ra_types::RuntimeDefinitions;

pub(crate) fn full_verses() -> [u32; 11] {
    [100; 11]
}

pub(crate) fn verses_for(defs: &RuntimeDefinitions, warhead: &str) -> [u32; 11] {
    if warhead.is_empty() {
        return full_verses();
    }
    defs.warheads.get(warhead).map(|w| w.verses).unwrap_or_else(full_verses)
}
