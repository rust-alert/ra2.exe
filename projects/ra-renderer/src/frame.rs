//! 帧构建：引擎投影 → [`crate::world::RenderWorld`]（R1）。

use std::collections::HashSet;

use ra_engine::{AnimState, RenderSnapshot, SnapshotUnit};
use ra_types::EntityId;

use crate::world::{RenderUnit, RenderWorld};

/// 将只读投影应用到可复用渲染世界。
///
/// 全量同步仍用于原型路径；同时维护可复用槽位，供脏集增量与 marker 读取。
#[derive(Debug, Default)]
pub struct FrameBuilder;

impl FrameBuilder {
    /// 用完整快照同步世界槽位（复用 `HashMap` 容量；移除快照中已不存在的 ID）。
    pub fn apply_full_snapshot(world: &mut RenderWorld, snap: &RenderSnapshot) {
        world.source_tick = snap.tick;
        let selected: HashSet<u64> = snap.selected.iter().map(|id| id.0).collect();
        let mut seen: HashSet<u64> = HashSet::with_capacity(snap.units.len());
        let mut updated = 0u32;
        for u in &snap.units {
            let key = u.id.0;
            seen.insert(key);
            world.units.insert(key, render_unit_from_snapshot(u, selected.contains(&key)));
            updated += 1;
        }
        world.units.retain(|k, _| seen.contains(k));
        world.dirty_count = updated;
    }

    /// 仅更新脏 ID 对应槽位；`units` 应为这些 ID 的投影切片。
    ///
    /// 未知 ID 的投影会被插入；脏集中有但 `units` 未给出的 ID 会从世界移除（视为销毁）。
    pub fn apply_dirty_units(
        world: &mut RenderWorld,
        source_tick: u64,
        dirty: &[EntityId],
        units: &[SnapshotUnit],
        selected: &[EntityId],
    ) {
        world.source_tick = source_tick;
        let selected: HashSet<u64> = selected.iter().map(|id| id.0).collect();
        let mut provided: HashSet<u64> = HashSet::with_capacity(units.len());
        let mut updated = 0u32;
        for u in units {
            let key = u.id.0;
            provided.insert(key);
            world.units.insert(key, render_unit_from_snapshot(u, selected.contains(&key)));
            updated += 1;
        }
        for id in dirty {
            if !provided.contains(&id.0) {
                world.units.remove(&id.0);
            }
        }
        // 选中集合变化时刷新未出现在 dirty 投影中的槽位选中标记。
        for unit in world.units.values_mut() {
            unit.selected = selected.contains(&unit.id.0);
        }
        world.dirty_count = updated;
    }
}

fn render_unit_from_snapshot(u: &SnapshotUnit, selected: bool) -> RenderUnit {
    RenderUnit {
        id: u.id,
        screen_x: u.screen_x,
        screen_y: u.screen_y,
        is_structure: u.is_structure(),
        dead: u.dead,
        selected,
        color: anim_tint(owner_color(u.owner.as_ref()), u.anim_state),
        health: u.health,
        max_health: u.max_health,
    }
}

fn owner_color(owner: &str) -> [f32; 4] {
    let mut h: u32 = 2166136261;
    for b in owner.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(16777619);
    }
    let r = ((h >> 16) & 0xff) as f32 / 255.0;
    let g = ((h >> 8) & 0xff) as f32 / 255.0;
    let b = (h & 0xff) as f32 / 255.0;
    [0.35 + r * 0.55, 0.35 + g * 0.55, 0.35 + b * 0.55, 0.92]
}

fn anim_tint(base: [f32; 4], state: AnimState) -> [f32; 4] {
    let (r, g, b, a) = (base[0], base[1], base[2], base[3]);
    match state {
        AnimState::Idle => base,
        AnimState::Move => [r * 0.85 + 0.15, g * 0.85 + 0.15, b * 0.7, a],
        AnimState::Attack => [r * 0.55 + 0.45, g * 0.45, b * 0.35, a],
        AnimState::TakeDamage => [0.95, 0.95, 0.95, a],
        AnimState::Produce => [r * 0.55, g * 0.55 + 0.4, b * 0.7 + 0.25, a],
        AnimState::Die => [0.2, 0.2, 0.2, 0.55],
    }
}
