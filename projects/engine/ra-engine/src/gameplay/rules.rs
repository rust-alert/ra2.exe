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

/// Verses 索敌／开火资格模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VersesTargetingMode {
    /// 自动索敌 / 攻击移动（`P` 或倍率 > 0）。
    PassiveAcquire,
    /// 玩家强制攻击（`F` 或倍率 > 0）。
    ForceFire,
    /// 受击反击（`R` 或倍率 > 0）；反击接线切片消费。
    #[allow(dead_code)]
    Retaliate,
}

/// 按模式判断攻击方弹头是否允许以该护甲为目标。
pub(crate) fn target_allowed_by_verses(
    defs: &RuntimeDefinitions,
    attacker_warhead: Option<WarheadId>,
    target_armor_index: usize,
    mode: VersesTargetingMode,
) -> bool {
    let entry = verses_entry_for(defs, attacker_warhead, target_armor_index);
    match mode {
        VersesTargetingMode::PassiveAcquire => entry.allows_passive_acquire(),
        VersesTargetingMode::ForceFire => entry.allows_force_fire(),
        VersesTargetingMode::Retaliate => entry.allows_retaliate(),
    }
}

/// 攻击方主武器弹头；缺武器时回落 techno 绑定弹头。
pub(crate) fn attacker_primary_warhead(defs: &RuntimeDefinitions, type_id: ra_types::TypeId) -> Option<WarheadId> {
    let techno = defs.techno.get_by_id(type_id)?;
    if let Some(weapon_id) = techno.primary_id {
        if let Some(weapon) = defs.weapons.get_by_id(weapon_id) {
            if weapon.warhead_id.is_some() {
                return weapon.warhead_id;
            }
        }
    }
    techno.warhead_id
}
