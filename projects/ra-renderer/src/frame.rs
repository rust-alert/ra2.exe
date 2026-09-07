//! 帧构建：引擎投影 → [`crate::world::RenderWorld`]（R1）。

use std::collections::HashSet;

use ra_engine::RenderSnapshot;
use ra_types::EntityId;

use crate::world::{RenderUnit, RenderWorld};

/// 将只读投影应用到可复用渲染世界。
///
/// 全量同步仍用于原型 marker 路径；同时维护可复用槽位，供后续脏集增量更新。
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
            let slot = RenderUnit {
                id: u.id,
                screen_x: u.screen_x,
                screen_y: u.screen_y,
                is_structure: u.is_structure(),
                dead: u.dead,
                selected: selected.contains(&key),
            };
            world.units.insert(key, slot);
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
        units: &[ra_engine::SnapshotUnit],
        selected: &[EntityId],
    ) {
        world.source_tick = source_tick;
        let selected: HashSet<u64> = selected.iter().map(|id| id.0).collect();
        let mut provided: HashSet<u64> = HashSet::with_capacity(units.len());
        let mut updated = 0u32;
        for u in units {
            let key = u.id.0;
            provided.insert(key);
            world.units.insert(
                key,
                RenderUnit {
                    id: u.id,
                    screen_x: u.screen_x,
                    screen_y: u.screen_y,
                    is_structure: u.is_structure(),
                    dead: u.dead,
                    selected: selected.contains(&key),
                },
            );
            updated += 1;
        }
        for id in dirty {
            if !provided.contains(&id.0) {
                world.units.remove(&id.0);
            }
        }
        world.dirty_count = updated;
    }
}
