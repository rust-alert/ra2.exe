//! 地图触发与脚本钩子：评估 Events、执行 Actions（Win/Lose/建队等）。

use std::collections::{HashMap, HashSet};

use ra_map::{MapActionCommand, MapEventCondition, MapScripting};

use crate::game::BattleOutcome;
use crate::state::BattleState;
use crate::state::components::{Health, Identity, Owner, Transform};

/// 零售常用事件码（竖切子集）。
const EVENT_DESTROYED: i32 = 8;
const EVENT_ALL_DESTROYED: i32 = 11;
const EVENT_ENTERED_BY: i32 = 1;
const EVENT_TIME_ELAPSE: i32 = 13;

/// 零售常用动作码（竖切子集）。
const ACTION_WIN: i32 = 1;
const ACTION_LOSE: i32 = 2;
const ACTION_CREATE_TEAM: i32 = 4;
const ACTION_DESTROY_TRIGGER: i32 = 12;
const ACTION_FORCE_TRIGGER: i32 = 40;
const ACTION_ENABLE_TRIGGER: i32 = 53;
const ACTION_DISABLE_TRIGGER: i32 = 54;
const ACTION_REINFORCEMENT_TEAM: i32 = 80;

/// 单条触发器运行时状态。
#[derive(Debug, Clone)]
struct TriggerRuntimeState {
    id: String,
    disabled: bool,
    fired: bool,
    /// `EVENT_TIME_ELAPSE` 倒计时（tick）；`None` 表示本触发无计时条件。
    timer_remaining: Option<u32>,
}

/// 局内触发运行时（由地图 `MapScripting` 播种）。
#[derive(Debug, Clone, Default)]
pub struct TriggerRuntime {
    states: Vec<TriggerRuntimeState>,
    /// 未实现动作码（去重后供能力缺口报告）。
    pub unsupported_actions: Vec<i32>,
    /// 待创建的 TeamType id（由 Create Team 动作排队）。
    pub pending_team_spawns: Vec<String>,
    /// 本 tick 请求的剧本胜负（由 BattleSession 消费）。
    pub pending_outcome: Option<BattleOutcome>,
}

impl TriggerRuntime {
    /// 从地图剧本播种；无触发则空运行时。
    pub fn from_scripting(scripting: &MapScripting) -> Self {
        let events_by_id: HashMap<&str, &ra_map::MapEvent> =
            scripting.events.iter().map(|e| (e.id.as_str(), e)).collect();
        let mut states = Vec::with_capacity(scripting.triggers.len());
        for tr in &scripting.triggers {
            let timer_remaining = events_by_id.get(tr.id.as_str()).and_then(|ev| {
                ev.conditions.iter().find(|c| c.kind == EVENT_TIME_ELAPSE).map(|c| {
                    c.params.first().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0)
                })
            });
            states.push(TriggerRuntimeState {
                id: tr.id.clone(),
                disabled: tr.disabled,
                fired: false,
                timer_remaining,
            });
        }
        Self {
            states,
            unsupported_actions: Vec::new(),
            pending_team_spawns: Vec::new(),
            pending_outcome: None,
        }
    }

    fn record_unsupported(&mut self, kind: i32) {
        if !self.unsupported_actions.contains(&kind) {
            self.unsupported_actions.push(kind);
        }
    }
}

/// 每个逻辑 tick 评估并执行已满足条件的触发器。
pub fn tick_triggers(world: &mut BattleState) {
    if world.trigger_runtime.pending_outcome.is_some() {
        return;
    }
    let scripting = world.map.scripting.clone();
    if scripting.triggers.is_empty() {
        return;
    }

    let events_by_id: HashMap<String, ra_map::MapEvent> =
        scripting.events.iter().cloned().map(|e| (e.id.clone(), e)).collect();
    let actions_by_id: HashMap<String, ra_map::MapAction> =
        scripting.actions.iter().cloned().map(|a| (a.id.clone(), a)).collect();
    let tags = scripting.tags.clone();
    let cell_tags = scripting.cell_tags.clone();
    let local_house = world
        .players
        .iter()
        .find(|p| p.id == world.local_player)
        .map(|p| p.house.clone())
        .unwrap_or_default();

    // 先推进计时器。
    for st in &mut world.trigger_runtime.states {
        if st.disabled || st.fired {
            continue;
        }
        if let Some(rem) = st.timer_remaining.as_mut() {
            if *rem > 0 {
                *rem = rem.saturating_sub(1);
            }
        }
    }

    let mut to_fire: Vec<String> = Vec::new();
    for st in &world.trigger_runtime.states {
        if st.disabled || st.fired {
            continue;
        }
        let Some(event) = events_by_id.get(&st.id)
        else {
            continue;
        };
        if event_conditions_met(world, st, event, &tags, &cell_tags, &local_house) {
            to_fire.push(st.id.clone());
        }
    }

    for id in to_fire {
        if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == id) {
            st.fired = true;
        }
        let Some(action) = actions_by_id.get(&id)
        else {
            continue;
        };
        for cmd in &action.commands {
            apply_action(world, cmd, &local_house);
            if world.trigger_runtime.pending_outcome.is_some() {
                return;
            }
        }
    }
}

fn event_conditions_met(
    world: &BattleState,
    st: &TriggerRuntimeState,
    event: &ra_map::MapEvent,
    tags: &[ra_map::MapTag],
    cell_tags: &[ra_map::MapCellTag],
    local_house: &str,
) -> bool {
    if event.conditions.is_empty() {
        return false;
    }
    event.conditions.iter().all(|c| condition_met(world, st, c, tags, cell_tags, local_house))
}

fn condition_met(
    world: &BattleState,
    st: &TriggerRuntimeState,
    c: &MapEventCondition,
    tags: &[ra_map::MapTag],
    cell_tags: &[ra_map::MapCellTag],
    local_house: &str,
) -> bool {
    match c.kind {
        EVENT_TIME_ELAPSE => st.timer_remaining == Some(0),
        EVENT_DESTROYED | EVENT_ALL_DESTROYED => {
            let bound_tags = tags_for_trigger(tags, &st.id);
            if bound_tags.is_empty() {
                // 无 Tag 绑定时：无法判定，视为未满足（避免误触发）。
                return false;
            }
            !any_living_with_tags(world, &bound_tags)
        }
        EVENT_ENTERED_BY => {
            let bound_tags = tags_for_trigger(tags, &st.id);
            if bound_tags.is_empty() {
                return false;
            }
            cell_entered_by_house(world, cell_tags, &bound_tags, local_house)
        }
        _ => false,
    }
}

fn tags_for_trigger(tags: &[ra_map::MapTag], trigger_id: &str) -> HashSet<String> {
    tags.iter()
        .filter(|t| t.trigger_id.eq_ignore_ascii_case(trigger_id))
        .map(|t| t.id.clone())
        .collect()
}

fn any_living_with_tags(world: &BattleState, tags: &HashSet<String>) -> bool {
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if !identity.tag.is_empty() && tags.contains(&identity.tag) {
            return true;
        }
    }
    false
}

fn cell_entered_by_house(
    world: &BattleState,
    cell_tags: &[ra_map::MapCellTag],
    bound_tags: &HashSet<String>,
    house: &str,
) -> bool {
    let cells: Vec<(u16, u16)> = cell_tags
        .iter()
        .filter(|c| bound_tags.contains(&c.tag_id))
        .map(|c| (c.x, c.y))
        .collect();
    if cells.is_empty() {
        return false;
    }
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(owner) = world.ecs_get::<Owner>(id)
        else {
            continue;
        };
        if !owner.house.eq_ignore_ascii_case(house) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        if cells.iter().any(|(x, y)| *x == xf.x && *y == xf.y) {
            return true;
        }
    }
    false
}

fn apply_action(world: &mut BattleState, cmd: &MapActionCommand, local_house: &str) {
    match cmd.kind {
        ACTION_WIN => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            world.trigger_runtime.pending_outcome = Some(BattleOutcome::Victory { owner: house });
        }
        ACTION_LOSE => {
            let reason = action_house_param(cmd).unwrap_or_default();
            world.trigger_runtime.pending_outcome = Some(BattleOutcome::Defeat { reason });
        }
        ACTION_CREATE_TEAM | ACTION_REINFORCEMENT_TEAM => {
            if let Some(team) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                world.trigger_runtime.pending_team_spawns.push(team.to_string());
            } else if let Some(team) = cmd.params.first().map(|s| s.trim()).filter(|s| !s.is_empty() && s.parse::<i32>().is_err())
            {
                world.trigger_runtime.pending_team_spawns.push(team.to_string());
            } else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        ACTION_DESTROY_TRIGGER => {
            if let Some(id) = action_trigger_id_param(cmd) {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&id)) {
                    st.disabled = true;
                    st.fired = true;
                }
            } else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        ACTION_FORCE_TRIGGER => {
            if let Some(id) = action_trigger_id_param(cmd) {
                force_fire_trigger(world, &id, local_house);
            } else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        ACTION_ENABLE_TRIGGER => {
            if let Some(id) = action_trigger_id_param(cmd) {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&id)) {
                    st.disabled = false;
                }
            } else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        ACTION_DISABLE_TRIGGER => {
            if let Some(id) = action_trigger_id_param(cmd) {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&id)) {
                    st.disabled = true;
                }
            } else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        0 => {}
        other => world.trigger_runtime.record_unsupported(other),
    }
}

/// 强制执行目标触发的 Actions（跳过 Events；已触发过则忽略，避免环）。
fn force_fire_trigger(world: &mut BattleState, id: &str, local_house: &str) {
    let already = world
        .trigger_runtime
        .states
        .iter()
        .find(|s| s.id.eq_ignore_ascii_case(id))
        .map(|s| s.fired)
        .unwrap_or(true);
    if already {
        return;
    }
    if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(id)) {
        st.disabled = false;
        st.fired = true;
    }
    let commands = world
        .map
        .scripting
        .actions
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(id))
        .map(|a| a.commands.clone())
        .unwrap_or_default();
    for cmd in &commands {
        apply_action(world, cmd, local_house);
        if world.trigger_runtime.pending_outcome.is_some() {
            return;
        }
    }
}

fn action_trigger_id_param(cmd: &MapActionCommand) -> Option<String> {
    // 常见写法：params[1] 为 Trigger id（与 Create Team 槽位一致）。
    if let Some(id) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty() && s.parse::<i32>().is_err()) {
        return Some(id.to_string());
    }
    for p in &cmd.params {
        let t = p.trim();
        if t.is_empty() || t.parse::<i32>().is_ok() {
            continue;
        }
        return Some(t.to_string());
    }
    None
}

fn action_house_param(cmd: &MapActionCommand) -> Option<String> {
    // 动作参数第 7 槽常为 House 字母/名；亦接受非空首个非数字参数。
    for p in cmd.params.iter().rev() {
        let t = p.trim();
        if t.is_empty() || t == "0" {
            continue;
        }
        if t.parse::<i32>().is_ok() {
            continue;
        }
        return Some(t.to_string());
    }
    None
}
