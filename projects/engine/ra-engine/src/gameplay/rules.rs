//! 弹头 Verses 辅助（按装载期绑定的 `WarheadId` 查找）。

use ra_types::{RuntimeDefinitions, VersesEntry, WarheadId, WarheadVerses};

pub(crate) fn full_verses() -> [u32; 11] {
    [100; 11]
}

pub(crate) fn verses_for(defs: &RuntimeDefinitions, warhead_id: Option<WarheadId>) -> [u32; 11] {
    verses_table(defs, warhead_id).multipliers()
}

/// 完整 Verses 表（倍率 + F/R/P）；缺弹头时全满额无标志。
pub(crate) fn verses_table(defs: &RuntimeDefinitions, warhead_id: Option<WarheadId>) -> WarheadVerses {
    let Some(warhead_id) = warhead_id
    else {
        return WarheadVerses::all_full();
    };
    defs.warheads.get_by_id(warhead_id).map(|w| w.verses).unwrap_or_else(WarheadVerses::all_full)
}

/// 目标护甲对应的 Verses 条目（供索敌 / 强制攻击资格查询）。
pub(crate) fn verses_entry_for(defs: &RuntimeDefinitions, warhead_id: Option<WarheadId>, armor_index: usize) -> VersesEntry {
    verses_table(defs, warhead_id).entry(armor_index)
}
