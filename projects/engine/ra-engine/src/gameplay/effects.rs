//! 持续效果与状态修正（铁幕无敌、护盾、揭示、超时空等）。

use ra_map::MapEntityKind;
use ra_types::EntityId;

use crate::state::{
    BattleState,
    components::{AttackState, Health, Identity, MovementState, Owner, TimedInvulnerability, Transform},
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
/// `force_americans`：`AmerParaDrop` 强制走美军表。
pub(crate) fn apply_paradrop_at(world: &mut BattleState, house: &str, x: u16, y: u16, force_americans: bool) {
    let payload = select_paradrop_payload(world, house, force_americans);
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

fn select_paradrop_payload(world: &BattleState, house: &str, force_americans: bool) -> Vec<ra_types::TypeId> {
    let rules = &world.definitions.paradrop;
    if force_americans {
        if !rules.americans.is_empty() {
            return rules.americans.clone();
        }
        return rules.payload.clone();
    }
    let key = house.trim().to_ascii_uppercase();
    if key == "AMERICANS" && !rules.americans.is_empty() {
        return rules.americans.clone();
    }
    if let Some(h) = world.definitions.houses.get(&key) {
        match h.stolen_tech {
            Some(ra_types::StolenTechKind::Allied) if !rules.allies.is_empty() => return rules.allies.clone(),
            Some(ra_types::StolenTechKind::Soviet) if !rules.soviets.is_empty() => return rules.soviets.clone(),
            _ => {}
        }
        let side = h.side.as_str().to_ascii_uppercase();
        if side == "GDI" && !rules.allies.is_empty() {
            return rules.allies.clone();
        }
        if side == "NOD" && !rules.soviets.is_empty() {
            return rules.soviets.clone();
        }
    }
    rules.payload.clone()
}

/// 揭示：为行动方标记圆盘格并推送雷达事件。
pub(crate) fn apply_reveal_at(world: &mut BattleState, house: &str, x: u16, y: u16) {
    let radius = world.definitions.reveal.radius_cells;
    apply_reveal_disk_at(world, house, x, y, radius);
}

/// 间谍飞机竖切：较大航空揭示半径 + 雷达事件（暂无飞越实体）。
pub(crate) fn apply_spy_plane_at(world: &mut BattleState, house: &str, x: u16, y: u16) {
    let radius = world.definitions.reveal.aircraft_radius_cells;
    apply_reveal_disk_at(world, house, x, y, radius);
}

fn apply_reveal_disk_at(world: &mut BattleState, house: &str, x: u16, y: u16, radius: u32) {
    let (mw, mh) = (world.map.width, world.map.height);
    world.house_reveal.reveal_disk(house, x, y, radius, mw, mh);
    world.push_radar_event(house, x, y);
}

/// 超时空：将源点半径内友军机动单位传送到目标邻域。
pub(crate) fn apply_chronosphere_warp(world: &mut BattleState, house: &str, sx: u16, sy: u16, dx: u16, dy: u16) {
    let rules = world.definitions.chrono_sphere.clone();
    let source_r = rules.source_radius_cells as i32;
    let dest_r = rules.dest_search_radius_cells as i32;
    let house_id = crate::gameplay::house_id_of(&world.definitions, house);
    let mut movers = Vec::new();
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if !matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft) {
            continue;
        }
        if world.ecs_get::<Owner>(id).map(|o| Some(o.house) != house_id).unwrap_or(true) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        let ox = (xf.x as i32 - sx as i32).abs();
        let oy = (xf.y as i32 - sy as i32).abs();
        if ox.max(oy) <= source_r {
            movers.push(id);
        }
    }
    let mut dest_offsets: Vec<(i32, i32)> = Vec::new();
    for r in 0i32..=dest_r {
        for oy in -r..=r {
            for ox in -r..=r {
                if ox.abs().max(oy.abs()) != r {
                    continue;
                }
                dest_offsets.push((ox, oy));
            }
        }
    }
    let mut slot = 0usize;
    for id in movers {
        let mut placed = false;
        for _ in 0..dest_offsets.len().max(1) {
            let (ox, oy) = dest_offsets[slot % dest_offsets.len().max(1)];
            slot += 1;
            let nx = dx as i32 + ox;
            let ny = dy as i32 + oy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let (ux, uy) = (nx as u16, ny as u16);
            if !world.pass_grid.in_bounds(ux, uy) || !world.pass_grid.is_passable(ux, uy) {
                continue;
            }
            if world.cell_blocked_by_entity(ux, uy) {
                continue;
            }
            let _ = world.with_transform_mut(id, |xf| {
                xf.x = ux;
                xf.y = uy;
            });
            let _ = world.with_movement_mut(id, |m: &mut MovementState| {
                m.destination_x = None;
                m.destination_y = None;
                m.waypoints.clear();
                m.path.clear();
                m.move_accum = 0;
            });
            let _ = world.with_attack_mut(id, |a: &mut AttackState| {
                a.target = None;
            });
            world.mark_entity_dirty(id);
            placed = true;
            break;
        }
        if !placed {
            // 落点全满则跳过该单位，不阻断其余传送。
            continue;
        }
    }
}
