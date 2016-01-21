//! 剧本小队：按 TaskForce / TeamType 在航点生成增援，并按 ScriptTypes 最小步进。

use ra_map::MapTeamType;
use ra_types::{EntityId, PlayerId};

use crate::game::GameCommand;
use crate::gameplay::houses_are_allied;
use crate::state::BattleState;

/// 原版 `[ScriptTypes]` 步骤动作码：攻击航点附近敌方（`argument` = 航点编号）。
const SCRIPT_ACTION_ATTACK_WAYPOINT: i32 = 1;
/// 原版 `[ScriptTypes]` 步骤动作码：移动到航点（`argument` = 航点编号）。
const SCRIPT_ACTION_MOVE_TO_WAYPOINT: i32 = 3;
/// 原版 `[ScriptTypes]` 步骤动作码：部署（`argument` 通常未用；对可部署单位下发 `Deploy`）。
const SCRIPT_ACTION_DEPLOY: i32 = 6;
/// 原版 `[ScriptTypes]` 步骤动作码：跳转到步骤行（`argument` = 0-based 步骤下标）。
const SCRIPT_ACTION_JUMP_TO_LINE: i32 = 8;

/// 攻击航点时，在航点曼哈顿距离内搜敌的半径（格）。
const ATTACK_WAYPOINT_SEARCH_RADIUS: u32 = 8;

/// 已生成、仍在执行 Script 的小队。
#[derive(Debug, Clone)]
struct ActiveScriptTeam {
    members: Vec<EntityId>,
    script_id: String,
    step_idx: usize,
}

/// 小队 Script 运行时。
#[derive(Debug, Default, Clone)]
pub struct ScriptTeamRuntime {
    active: Vec<ActiveScriptTeam>,
}

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

/// 推进已生成小队的 ScriptTypes 步骤（竖切：攻击/移动到航点）。
pub fn tick_script_teams(world: &mut BattleState) {
    if world.script_team_runtime.active.is_empty() {
        return;
    }
    let scripts = world.map.scripting.script_types.clone();
    let waypoints = world.map.waypoints.clone();

    // 先快照本 tick 要执行的步骤，避免与 `ecs_*` / `push_player_command` 争用 `active` 借用。
    let plan: Vec<(usize, Option<(i32, i32)>, Vec<EntityId>, usize)> = world
        .script_team_runtime
        .active
        .iter()
        .enumerate()
        .map(|(idx, team)| {
            let Some(script) = scripts.iter().find(|s| s.id.eq_ignore_ascii_case(&team.script_id))
            else {
                return (idx, None, Vec::new(), team.step_idx);
            };
            if team.step_idx >= script.steps.len() {
                return (idx, None, Vec::new(), team.step_idx);
            }
            let step = &script.steps[team.step_idx];
            (
                idx,
                Some((step.action, step.argument)),
                team.members.clone(),
                script.steps.len(),
            )
        })
        .collect();

    let mut move_orders: Vec<(PlayerId, EntityId, u16, u16)> = Vec::new();
    let mut attack_orders: Vec<(PlayerId, EntityId, EntityId)> = Vec::new();
    let mut deploy_orders: Vec<(PlayerId, EntityId)> = Vec::new();
    let mut remove = Vec::new();

    for (idx, step, members, step_count) in plan {
        let Some((action, argument)) = step
        else {
            remove.push(idx);
            continue;
        };
        match action {
            SCRIPT_ACTION_ATTACK_WAYPOINT => {
                // `argument` = 航点编号；对航点附近最近敌方下发 `Attack`。
                if let Some(wp) = waypoints.iter().find(|w| w.index as i32 == argument) {
                    for id in members {
                        let Some((_, _, dead)) = world.ecs_health(id)
                        else {
                            continue;
                        };
                        if dead {
                            continue;
                        }
                        let Some(house) = world.ecs_owner(id)
                        else {
                            continue;
                        };
                        let Some(player) = world.players.iter().find(|p| p.house.as_ref() == house.as_ref())
                        else {
                            continue;
                        };
                        if let Some(target) = nearest_hostile_near(world, house.as_ref(), wp.x, wp.y, ATTACK_WAYPOINT_SEARCH_RADIUS)
                        {
                            attack_orders.push((player.id, id, target));
                        }
                    }
                }
            }
            SCRIPT_ACTION_MOVE_TO_WAYPOINT => {
                // `argument` = 航点编号（`[Waypoints]` index）。
                if let Some(wp) = waypoints.iter().find(|w| w.index as i32 == argument) {
                    for id in members {
                        let Some((_, _, dead)) = world.ecs_health(id)
                        else {
                            continue;
                        };
                        if dead {
                            continue;
                        }
                        let Some(house) = world.ecs_owner(id)
                        else {
                            continue;
                        };
                        let Some(player) = world.players.iter().find(|p| p.house.as_ref() == house.as_ref())
                        else {
                            continue;
                        };
                        move_orders.push((player.id, id, wp.x, wp.y));
                    }
                }
            }
            SCRIPT_ACTION_DEPLOY => {
                // 对小队成员下发部署（不可部署单位由命令层拒绝）。
                for id in members {
                    let Some((_, _, dead)) = world.ecs_health(id)
                    else {
                        continue;
                    };
                    if dead {
                        continue;
                    }
                    let Some(house) = world.ecs_owner(id)
                    else {
                        continue;
                    };
                    let Some(player) = world.players.iter().find(|p| p.house.as_ref() == house.as_ref())
                    else {
                        continue;
                    };
                    deploy_orders.push((player.id, id));
                }
            }
            SCRIPT_ACTION_JUMP_TO_LINE => {
                // `argument` = 目标步骤下标；越界则结束小队脚本。
                let target = argument.max(0) as usize;
                if target >= step_count {
                    remove.push(idx);
                } else {
                    world.script_team_runtime.active[idx].step_idx = target;
                }
                continue;
            }
            _ => {
                // 未接线动作：跳过一步，避免卡死整队脚本。
            }
        }
        let next = world.script_team_runtime.active[idx].step_idx.saturating_add(1);
        world.script_team_runtime.active[idx].step_idx = next;
        if next >= step_count {
            remove.push(idx);
        }
    }

    for idx in remove.into_iter().rev() {
        if idx < world.script_team_runtime.active.len() {
            world.script_team_runtime.active.swap_remove(idx);
        }
    }

    for (player, entity, x, y) in move_orders {
        world.push_player_command(player, GameCommand::MoveTo { entity, x, y });
    }
    for (player, attacker, target) in attack_orders {
        world.push_player_command(player, GameCommand::Attack { attacker, target });
    }
    for (player, entity) in deploy_orders {
        world.push_player_command(player, GameCommand::Deploy { entity });
    }
}

/// 在 `(cx,cy)` 附近找距离最近的敌对存活实体（同盟 / 氛围房主除外）。
fn nearest_hostile_near(
    world: &BattleState,
    house: &str,
    cx: u16,
    cy: u16,
    radius: u32,
) -> Option<EntityId> {
    let mut best: Option<(u32, EntityId)> = None;
    for e in &world.entities {
        let id = e.id;
        if world.ecs_health(id).map(|(_, _, d)| d).unwrap_or(true) {
            continue;
        }
        let Some(owner) = world.ecs_owner(id)
        else {
            continue;
        };
        if houses_are_allied(world, house, owner.as_ref()) {
            continue;
        }
        if crate::gameplay::ai::is_ambient_house(owner.as_ref()) {
            continue;
        }
        let Some((x, y, _)) = world.ecs_transform(id)
        else {
            continue;
        };
        let dist = (i32::from(cx) - i32::from(x)).unsigned_abs() + (i32::from(cy) - i32::from(y)).unsigned_abs();
        if dist > radius {
            continue;
        }
        if best.map(|(d, _)| dist < d).unwrap_or(true) {
            best = Some((dist, id));
        }
    }
    best.map(|(_, id)| id)
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

    let mut members = Vec::new();
    let mut ox = 0i32;
    let mut oy = 0i32;
    for entry in &force.entries {
        for _ in 0..entry.count.max(1) {
            let x = (i32::from(wx) + ox).clamp(0, i32::from(u16::MAX)) as u16;
            let y = (i32::from(wy) + oy).clamp(0, i32::from(u16::MAX)) as u16;
            let spawned = match world.spawn_unit_at(house, &entry.type_id, x, y) {
                Ok(id) => Some(id),
                Err(_) => {
                    // 格占用时尝试邻格。
                    world.spawn_unit_at(house, &entry.type_id, x.saturating_add(1), y).ok()
                }
            };
            if let Some(id) = spawned {
                if !team.tag.is_empty() {
                    let _ = world.with_identity_mut(id, |identity| {
                        identity.tag = team.tag.clone();
                    });
                }
                members.push(id);
            }
            ox += 1;
            if ox > 2 {
                ox = 0;
                oy += 1;
            }
        }
    }

    if !members.is_empty() && !team.script.is_empty() {
        world.script_team_runtime.active.push(ActiveScriptTeam {
            members,
            script_id: team.script.clone(),
            step_idx: 0,
        });
    }
}
