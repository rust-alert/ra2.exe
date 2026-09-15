//! 持续效果与状态修正（铁幕无敌、护盾等）。

use ra_map::MapEntityKind;
use ra_types::EntityId;

use crate::state::{
    BattleState,
    components::{Health, Identity, Owner, TimedInvulnerability, Transform},
};

/// 推进所有定时无敌计时；归零则摘除组件。
pub(crate) fn tick_timed_invulnerability(world: &mut BattleState) {
    let mut expired: Vec<EntityId> = Vec::new();
    let mut dirty: Vec<EntityId> = Vec::new();
    for (handle, buff) in world.ecs.world().iter::<TimedInvulnerability>() {
        let Some(identity) = world.ecs.world().get::<Identity>(handle)
        else {
            continue;
        };
        let id = identity.entity_id;
        if buff.remaining_ticks <= 1 {
            expired.push(id);
        }
        else {
            dirty.push(id);
        }
    }
    for id in dirty {
        if let Some(handle) = world.ecs.resolve(id) {
            if let Some(buff) = world.ecs.world_mut().get_mut::<TimedInvulnerability>(handle) {
                buff.remaining_ticks = buff.remaining_ticks.saturating_sub(1);
            }
        }
        world.mark_entity_dirty(id);
    }
    for id in expired {
        if let Some(handle) = world.ecs.resolve(id) {
            let _ = world.ecs.world_mut().remove::<TimedInvulnerability>(handle);
        }
        world.mark_entity_dirty(id);
    }
}

/// 实体当前是否处于定时无敌。
pub(crate) fn is_invulnerable(world: &BattleState, id: EntityId) -> bool {
    world.ecs_get::<TimedInvulnerability>(id).is_some_and(|b| b.remaining_ticks > 0)
}

/// 给实体挂上或刷新定时无敌。
pub(crate) fn grant_invulnerability(world: &mut BattleState, id: EntityId, duration_ticks: u32) {
    if duration_ticks == 0 {
        return;
    }
    let Some(handle) = world.ecs.resolve(id)
    else {
        return;
    };
    if let Some(buff) = world.ecs.world_mut().get_mut::<TimedInvulnerability>(handle) {
        buff.remaining_ticks = buff.remaining_ticks.max(duration_ticks);
    }
    else {
        let _ = world.ecs.world_mut().insert(handle, TimedInvulnerability { remaining_ticks: duration_ticks });
    }
    world.mark_entity_dirty(id);
}

/// 铁幕：友军机动单位；力场：友军建筑。半径内授予无敌。
pub(crate) fn apply_iron_curtain_at(world: &mut BattleState, house: &str, x: u16, y: u16, structures_only: bool) {
    let rules = &world.definitions.iron_curtain;
    let duration = rules.duration_ticks;
    let radius = rules.radius_cells as i32;
    if duration == 0 {
        return;
    }
    let house_id = crate::gameplay::house_id_of(&world.definitions, house);
    let mut targets = Vec::new();
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        let is_structure = identity.kind == MapEntityKind::Structure;
        if structures_only {
            if !is_structure {
                continue;
            }
        }
        else if !matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
            continue;
        }
        if world.ecs_get::<Owner>(id).map(|o| Some(o.house) != house_id).unwrap_or(true) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        let dx = (xf.x as i32 - x as i32).abs();
        let dy = (xf.y as i32 - y as i32).abs();
        if dx.max(dy) <= radius {
            targets.push(id);
        }
    }
    for id in targets {
        grant_invulnerability(world, id, duration);
    }
}

/// 空降：在目标格邻域为行动方刷出冻结载荷单位。
pub(crate) fn apply_paradrop_at(world: &mut BattleState, house: &str, x: u16, y: u16) {
    let payload = world.definitions.paradrop.payload.clone();
    if payload.is_empty() {
        return;
    }
    let offsets: &[(i32, i32)] = &[
        (0, 0),
        (1, 0),
        (0, 1),
        (-1, 0),
        (0, -1),
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
        (2, 0),
        (0, 2),
        (-2, 0),
        (0, -2),
    ];
    let mut slot = 0usize;
    for type_id in payload {
        let mut placed = false;
        for _ in 0..offsets.len() {
            let (dx, dy) = offsets[slot % offsets.len()];
            slot += 1;
            let nx = x as i32 + dx;
            let ny = y as i32 + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let (ux, uy) = (nx as u16, ny as u16);
            if world.spawn_unit_at_type(house, type_id, ux, uy).is_ok() {
                placed = true;
                break;
            }
        }
        if !placed {
            // 邻域放不下则跳过该载荷项，不阻断后续。
            continue;
        }
    }
}
