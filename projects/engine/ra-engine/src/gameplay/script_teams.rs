//! 剧本小队：按 TaskForce / TeamType 在航点生成增援，并按 ScriptTypes 最小步进。

use ra_types::{EntityId, HouseId, MapWaypoint, PlayerId, PreparedTaskForce, PreparedTeamType, ScriptTypeId, TeamTypeId};

use crate::{game::GameCommand, gameplay::houses_are_allied, state::BattleState};

/// 原版 `[ScriptTypes]` 步骤动作码：攻击航点附近敌方（`argument` = 航点编号）。
const SCRIPT_ACTION_ATTACK_WAYPOINT: i32 = 1;
/// 原版 `[ScriptTypes]` 步骤动作码：移动到航点（`argument` = 航点编号）。
const SCRIPT_ACTION_MOVE_TO_WAYPOINT: i32 = 3;
/// 原版 `[ScriptTypes]` 步骤动作码：部署（`argument` 通常未用；对可部署单位下发 `Deploy`）。
const SCRIPT_ACTION_DEPLOY: i32 = 6;
/// 原版 `[ScriptTypes]` 步骤动作码：驻守区域（清移动目的地与攻击目标，就地警戒）。
const SCRIPT_ACTION_GUARD_AREA: i32 = 7;
/// 原版 `[ScriptTypes]` 步骤动作码：跳转到步骤行（`argument` = 0-based 步骤下标）。
const SCRIPT_ACTION_JUMP_TO_LINE: i32 = 8;

/// 攻击航点时，在航点曼哈顿距离内搜敌的半径（格）。
const ATTACK_WAYPOINT_SEARCH_RADIUS: u32 = 8;

/// 待创建的 TeamType 产队请求。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PendingTeamSpawn {
    /// TeamType 稳定 id。
    pub team_id: TeamTypeId,
    /// 产队房主覆盖；`None` = 用 TeamType.House。
    pub house_override: Option<HouseId>,
    /// 航点编号覆盖；`None` = 用 TeamType.Waypoint。
    pub waypoint_override: Option<i32>,
}

/// 已生成、仍在执行 Script 的小队。
#[derive(Debug, Clone)]
struct ActiveScriptTeam {
    team_type_id: TeamTypeId,
    members: Vec<EntityId>,
    script_id: Option<ScriptTypeId>,
    step_idx: usize,
}

/// 小队 Script 运行时。
#[derive(Debug, Default, Clone)]
pub struct ScriptTeamRuntime {
    active: Vec<ActiveScriptTeam>,
}

impl ScriptTeamRuntime {
    /// 统计指定 TeamType 当前仍登记的活跃小队数（供 `Max=`）。
    pub fn count_active_of_type(&self, team_type_id: TeamTypeId) -> usize {
        self.active.iter().filter(|t| t.team_type_id == team_type_id).count()
    }
}

/// 若未达 `Max=` 且未重复排队，则将 TeamType 排入 `pending_team_spawns`。
///
/// 返回是否成功入队。
pub(crate) fn try_enqueue_team_spawn(
    world: &mut BattleState,
    team_id: TeamTypeId,
    house_override: Option<HouseId>,
    waypoint_override: Option<i32>,
) -> bool {
    if world.trigger_runtime.pending_team_spawns.iter().any(|p| {
        p.team_id == team_id && p.house_override == house_override && p.waypoint_override == waypoint_override
    }) {
        return false;
    }
    if let Some(team) = world.prepared.team_types.iter().find(|t| t.id == team_id) {
        if team.max > 0 {
            let active = world.script_team_runtime.count_active_of_type(team.id);
            let pending = world.trigger_runtime.pending_team_spawns.iter().filter(|p| p.team_id == team_id).count();
            if active.saturating_add(pending) >= team.max as usize {
                return false;
            }
        }
    }
    world.trigger_runtime.pending_team_spawns.push(PendingTeamSpawn { team_id, house_override, waypoint_override });
    true
}

/// `Autocreate=yes` 的 TeamType：房主已开始生产且未达 `Max=` 时自动排队。
pub fn tick_autocreate_teams(world: &mut BattleState) {
    let teams = world.prepared.team_types.clone();
    for team in teams {
        if !team.autocreate {
            continue;
        }
        let Some(house) = team.house
        else {
            // `<all>` 通配不自动产队，避免对每个对手刷队。
            continue;
        };
        let Some(house_key) = world.definitions.houses.get_by_id(house).map(|h| h.type_key.as_str().to_string())
        else {
            continue;
        };
        if crate::gameplay::ai::is_ambient_house(&house_key) {
            continue;
        }
        world.ensure_house(&house_key);
        if !world.house_production_begun(&house_key) {
            continue;
        }
        if !crate::gameplay::ai::iq_allows(world, &house_key, world.definitions.ai_controls.iq_production) {
            continue;
        }
        let _ = try_enqueue_team_spawn(world, team.id, None, None);
    }
}

/// 消费 `pending_team_spawns`，按 TaskForce / TeamType 在航点生成单位。
pub fn flush_pending_team_spawns(world: &mut BattleState) {
    let pending = std::mem::take(&mut world.trigger_runtime.pending_team_spawns);
    if pending.is_empty() {
        return;
    }
    let teams = world.prepared.team_types.clone();
    let forces = world.prepared.task_forces.clone();
    let waypoints = world.prepared.definition.waypoints.clone();
    for spawn in pending {
        let Some(team) = teams.iter().find(|t| t.id == spawn.team_id)
        else {
            continue;
        };
        spawn_team_type(world, team, &forces, &waypoints, spawn.house_override, spawn.waypoint_override);
    }
}

/// 推进已生成小队的 ScriptTypes 步骤（竖切：攻击/移动到航点）。
pub fn tick_script_teams(world: &mut BattleState) {
    if world.script_team_runtime.active.is_empty() {
        return;
    }
    let scripts = world.prepared.script_types.clone();
    let waypoints = world.prepared.definition.waypoints.clone();

    // 先快照本 tick 要执行的步骤，避免与 `ecs_*` / `push_player_command` 争用 `active` 借用。
    let plan: Vec<(usize, Option<(i32, i32)>, Vec<EntityId>, usize)> = world
        .script_team_runtime
        .active
        .iter()
        .enumerate()
        .map(|(idx, team)| {
            // 无 Script 的队仅驻留供 Destroy Team 回收，不推进。
            let Some(script_id) = team.script_id
            else {
                return (idx, Some((i32::MIN, 0)), Vec::new(), 0);
            };
            let Some(script) = scripts.iter().find(|s| s.id == script_id)
            else {
                return (idx, None, Vec::new(), team.step_idx);
            };
            if team.step_idx >= script.steps.len() {
                return (idx, None, Vec::new(), team.step_idx);
            }
            let step = &script.steps[team.step_idx];
            (idx, Some((step.action, step.argument)), team.members.clone(), script.steps.len())
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
            i32::MIN => {
                // 无 Script 驻留队：本 tick 不推进。
                continue;
            }
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
                        let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house.as_ref()))
                        else {
                            continue;
                        };
                        if let Some(target) = nearest_hostile_near(world, house.as_ref(), wp.x, wp.y, ATTACK_WAYPOINT_SEARCH_RADIUS) {
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
                        let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house.as_ref()))
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
                    let Some(player) = world.players.iter().find(|p| p.house.eq_ignore_ascii_case(house.as_ref()))
                    else {
                        continue;
                    };
                    deploy_orders.push((player.id, id));
                }
            }
            SCRIPT_ACTION_GUARD_AREA => {
                // 就地驻守：清空移动与攻击目标，并写入 `mission=Guard`（本 tick 立即生效）。
                for id in members {
                    let Some((_, _, dead)) = world.ecs_health(id)
                    else {
                        continue;
                    };
                    if dead {
                        continue;
                    }
                    let _ = world.clear_ecs_movement(id);
                    let _ = world.with_attack_mut(id, |attack| {
                        attack.target = None;
                        attack.infiltrate_target = None;
                        attack.capture_target = None;
                    });
                    let _ = world.with_identity_mut(id, |identity| {
                        identity.mission = Some(ra_types::MissionKind::Guard);
                    });
                }
            }
            SCRIPT_ACTION_JUMP_TO_LINE => {
                // `argument` = 目标步骤下标；越界则结束小队脚本。
                let target = argument.max(0) as usize;
                if target >= step_count {
                    remove.push(idx);
                }
                else {
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
fn nearest_hostile_near(world: &BattleState, house: &str, cx: u16, cy: u16, radius: u32) -> Option<EntityId> {
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
    team: &PreparedTeamType,
    forces: &[PreparedTaskForce],
    waypoints: &[MapWaypoint],
    house_override: Option<ra_types::HouseId>,
    waypoint_override: Option<i32>,
) {
    let Some(force) = forces.iter().find(|f| f.id == team.task_force)
    else {
        return;
    };
    let Some(house) = house_override.or(team.house)
    else {
        return;
    };
    let Some(house_key) = world.definitions.houses.get_by_id(house).map(|h| h.type_key.as_str().to_string())
    else {
        return;
    };
    world.ensure_house(&house_key);
    // 产队格：动作航点覆盖 > TeamType.Waypoint= > index 0。
    let waypoint_index = waypoint_override.filter(|n| *n >= 0).unwrap_or(team.waypoint);
    let spawn_wp = if waypoint_index >= 0 { waypoints.iter().find(|w| w.index as i32 == waypoint_index) } else { None };
    let (wx, wy) = spawn_wp
        .or_else(|| waypoints.iter().find(|w| w.index == 0))
        .map(|w| (w.x, w.y))
        .or_else(|| waypoints.first().map(|w| (w.x, w.y)))
        .unwrap_or((1, 1));

    let tag_id = team.tag;

    let mut members = Vec::new();
    let mut ox = 0i32;
    let mut oy = 0i32;
    for entry in &force.entries {
        let Some(tt) = world.definitions.techno.get_by_id(entry.definition_id)
        else {
            continue;
        };
        if tt.class == ra_types::TechnoClass::Building {
            continue;
        }
        for _ in 0..entry.count.max(1) {
            let x = (i32::from(wx) + ox).clamp(0, i32::from(u16::MAX)) as u16;
            let y = (i32::from(wy) + oy).clamp(0, i32::from(u16::MAX)) as u16;
            let spawned = match world.spawn_unit_at_ids(house, entry.definition_id, x, y) {
                Ok(id) => Some(id),
                Err(_) => {
                    // 格占用时尝试邻格。
                    world.spawn_unit_at_ids(house, entry.definition_id, x.saturating_add(1), y).ok()
                }
            };
            if let Some(id) = spawned {
                if let Some(tag) = tag_id {
                    let _ = world.with_identity_mut(id, |identity| {
                        identity.tag = Some(tag);
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

    if !members.is_empty() {
        world.script_team_runtime.active.push(ActiveScriptTeam { team_type_id: team.id, members, script_id: team.script, step_idx: 0 });
    }
}

/// 销毁指定 `TeamType`：取消排队产队，并击杀已生成实例、移出脚本队表。
pub(crate) fn destroy_team_type(world: &mut BattleState, team_id: TeamTypeId) {
    world.trigger_runtime.pending_team_spawns.retain(|p| p.team_id != team_id);
    let mut kill = Vec::new();
    world.script_team_runtime.active.retain(|team| {
        let matched = team.team_type_id == team_id;
        if matched {
            kill.extend(team.members.iter().copied());
            false
        }
        else {
            true
        }
    });
    for id in kill {
        if world.ecs_health(id).map(|(_, _, d)| d).unwrap_or(true) {
            continue;
        }
        let max = world.ecs_health(id).map(|(_, m, _)| m).unwrap_or(1).max(1);
        let _ = world.set_ecs_health(id, 0, max, true);
    }
}
