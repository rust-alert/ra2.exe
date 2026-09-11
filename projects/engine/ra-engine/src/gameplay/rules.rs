//! 弹头 Verses 辅助（按装载期绑定的 `WarheadId` 查找）。

use ra_types::{RuntimeDefinitions, WarheadId};

pub(crate) fn full_verses() -> [u32; 11] {
    [100; 11]
}

pub(crate) fn verses_for(defs: &RuntimeDefinitions, warhead_id: WarheadId) -> [u32; 11] {
    if warhead_id == WarheadId(0) {
        return full_verses();
    }
    defs.warheads.get_by_id(warhead_id).map(|w| *w.verses).unwrap_or_else(full_verses)
}
