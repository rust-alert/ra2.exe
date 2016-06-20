//! 侧栏扳手持续修理：按 `[General]` 库存默认步进回血扣费。
//!
//! 原版键：`RepairPercent=15`、`RepairRate=.016`（分钟）、`RepairStep=8`。
//! 脉冲间隔取 `ftol(RepairRate * 900)`（15Hz 下约 14 tick），与自愈脉冲同一换算。

use ra_map::MapEntityKind;
use ra_types::EntityId;

use crate::state::{
    components::{Health, Identity, Owner, Repairing},
    BattleState,
};

/// 原版 `[General] RepairPercent` 库存默认（完全修好相对造价的百分比）。
pub(crate) const STOCK_REPAIR_PERCENT: u32 = 15;
/// 原版 `[General] RepairStep` 库存默认（每脉冲回复生命）。
pub(crate) const STOCK_REPAIR_STEP: u32 = 8;
/// 原版 `[General] RepairRate=.016` → `ftol(0.016 * 900)`。
pub(crate) const STOCK_REPAIR_INTERVAL_TICKS: u64 = 14;

/// 本 tick 若落在修理脉冲上，则对所有挂着 [`Repairing`] 的建筑步进一次。
pub(crate) fn tick_repairs(world: &mut BattleState) {
    if world.tick == 0 || world.tick % STOCK_REPAIR_INTERVAL_TICKS != 0 {
        return;
    }

    let mut jobs: Vec<RepairJob> = Vec::new();
    let mut stop: Vec<EntityId> = Vec::new();
    for (handle, _) in world.ecs.world().iter::<Repairing>() {
        let Some(identity) = world.ecs.world().get::<Identity>(handle).cloned()
        else {
            continue;
        };
        let id = identity.entity_id;
        if identity.kind != MapEntityKind::Structure {
            stop.push(id);
            continue;
        }
        let Some(health) = world.ecs.world().get::<Health>(handle).copied()
        else {
            stop.push(id);
            continue;
        };
        if health.dead || health.current >= health.maximum || health.maximum == 0 {
            stop.push(id);
            continue;
        }
        let Some(house) = world.ecs.world().get::<Owner>(handle).map(|o| o.house.as_ref().to_string())
        else {
            stop.push(id);
            continue;
        };
        jobs.push(RepairJob {
            id,
            house,
            type_id: identity.type_id.as_ref().to_string(),
            current: health.current,
            maximum: health.maximum,
        });
    }

    for job in jobs {
        let Some(player_index) = world.players.iter().position(|p| p.house.as_ref() == job.house)
        else {
            stop.push(job.id);
            continue;
        };
        let cost = world
            .definitions
            .techno
            .get(job.type_id.as_str())
            .map(|tt| tt.cost.max(0) as u32)
            .unwrap_or(0);
        let heal = STOCK_REPAIR_STEP.min(job.maximum.saturating_sub(job.current));
        if heal == 0 {
            stop.push(job.id);
            continue;
        }
        // 完全修好费用 = Cost * RepairPercent / 100，再按本脉冲回复量分摊。
        let fee = if cost == 0 {
            0
        } else {
            ((cost as u64) * (STOCK_REPAIR_PERCENT as u64) * (heal as u64)
                / (100u64 * (job.maximum as u64).max(1))) as i32
        };
        if fee > 0 && world.players[player_index].funds < fee {
            stop.push(job.id);
            continue;
        }
        if fee > 0 {
            world.players[player_index].funds -= fee;
            world.players[player_index].funds_spent =
                world.players[player_index].funds_spent.saturating_add(fee);
        }
        let _ = world.with_health_mut(job.id, |h| {
            h.current = (h.current + heal).min(h.maximum);
        });
        world.mark_entity_dirty(job.id);
        if world.ecs_get::<Health>(job.id).is_some_and(|h| h.current >= h.maximum) {
            stop.push(job.id);
        }
    }

    stop.sort_unstable_by_key(|id| id.0);
    stop.dedup();
    for id in stop {
        if let Some(handle) = world.ecs.resolve(id) {
            let _ = world.ecs.world_mut().remove::<Repairing>(handle);
            world.mark_entity_dirty(id);
        }
    }
}

struct RepairJob {
    id: EntityId,
    house: String,
    type_id: String,
    current: u32,
    maximum: u32,
}
