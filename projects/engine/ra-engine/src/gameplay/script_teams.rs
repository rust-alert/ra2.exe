//! 剧本小队：按 TaskForce / TeamType 在航点生成增援。

use ra_map::MapTeamType;

use crate::state::BattleState;

/// 消费 `pending_team_spawns`，按 TeamType + TaskForce 在航点生成单位。
pub fn flush_pending_team_spawns(world: &mut BattleState) {
    let pending = std::mem::take(&mut world.trigger_runtime.pending_team_spawns);
    if pending.is_empty() {
        return;
    }
    let teams = world.map.scripting.team_types.clone();
    let forces = world.map.scripting.task_forces.clone();
    let waypoints = world.map.waypoints.clone();
    for team_id in pending {
        let Some(team) = teams.iter().find(|t| t.id.eq_ignore_ascii_case(&team_id))
        else {
            continue;
        };
        spawn_team_type(world, team, &forces, &waypoints);
    }
}

fn spawn_team_type(
    world: &mut BattleState,
    team: &MapTeamType,
    forces: &[ra_map::MapTaskForce],
    waypoints: &[ra_map::Waypoint],
) {
    let Some(force) = forces.iter().find(|f| f.id.eq_ignore_ascii_case(&team.task_force))
    else {
        return;
    };
    let house = if team.house.is_empty() { "Neutral" } else { team.house.as_str() };
    world.ensure_house(house);
    // 航点：优先用脚本参数缺失时的默认 0；TeamType 未建模 waypoint 字段时用 index 0。
    let (wx, wy) = waypoints
        .iter()
        .find(|w| w.index == 0)
        .map(|w| (w.x, w.y))
        .or_else(|| waypoints.first().map(|w| (w.x, w.y)))
        .unwrap_or((1, 1));

    let mut ox = 0i32;
    let mut oy = 0i32;
    for entry in &force.entries {
        for _ in 0..entry.count.max(1) {
            let x = (i32::from(wx) + ox).clamp(0, i32::from(u16::MAX)) as u16;
            let y = (i32::from(wy) + oy).clamp(0, i32::from(u16::MAX)) as u16;
            match world.spawn_unit_at(house, &entry.type_id, x, y) {
                Ok(id) => {
                    if !team.tag.is_empty() {
                        let _ = world.with_identity_mut(id, |identity| {
                            identity.tag = team.tag.clone();
                        });
                    }
                }
                Err(_) => {
                    // 格占用时尝试邻格。
                    let _ = world.spawn_unit_at(house, &entry.type_id, x.saturating_add(1), y);
                }
            }
            ox += 1;
            if ox > 2 {
                ox = 0;
                oy += 1;
            }
        }
    }
}
