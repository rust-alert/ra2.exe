//! 侧栏扳手持续修理：按冻结定义中的 `[General]` 修理键步进回血扣费。
//!
//! 原版键：`RepairPercent`、`RepairRate`（分钟）、`RepairStep`。
//! 脉冲间隔取 `ftol(RepairRate * 900)`（15Hz 下库存 `.016` → 14 tick）。

use ra_map::MapEntityKind;
use ra_types::EntityId;

use crate::state::{
    BattleState,
    components::{Health, Identity, Owner, Repairing},
};

/// 本 tick 若落在修理脉冲上，则对所有挂着 [`Repairing`] 的建筑步进一次。
pub(crate) fn tick_repairs(world: &mut BattleState) {
    let interval = world.definitions.repair_interval_ticks.max(1);
    if world.tick == 0 || world.tick % interval != 0 {
        return;
    }
    let repair_step = world.definitions.repair_step.max(1);
    let repair_percent = world.definitions.repair_percent;

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
        jobs.push(RepairJob { id, house, type_id: identity.type_id.as_ref().to_string(), current: health.current, maximum: health.maximum });
    }

    for job in jobs {
        let Some(player_index) = world.players.iter().position(|p| p.house.as_ref() == job.house)
        else {
            stop.push(job.id);
            continue;
        };
        let cost = world.definitions.techno.get(job.type_id.as_str()).map(|tt| tt.cost.max(0) as u32).unwrap_or(0);
        let heal = repair_step.min(job.maximum.saturating_sub(job.current));
        if heal == 0 {
            stop.push(job.id);
            continue;
        }
        // 完全修好费用 = Cost * RepairPercent / 100，再按本脉冲回复量分摊。
        let fee = if cost == 0 || repair_percent == 0 {
            0
        }
        else {
            ((cost as u64) * (repair_percent as u64) * (heal as u64) / (100u64 * (job.maximum as u64).max(1))) as i32
        };
        if fee > 0 && world.players[player_index].funds < fee {
            stop.push(job.id);
            continue;
        }
        if fee > 0 {
            world.players[player_index].funds -= fee;
            world.players[player_index].funds_spent = world.players[player_index].funds_spent.saturating_add(fee);
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
