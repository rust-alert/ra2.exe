//! 地图触发与脚本钩子：评估 Events、执行 Actions（Win/Lose/建队等）。

use std::collections::{HashMap, HashSet};

use ra_map::{MapActionCommand, MapEntityKind, MapEventCondition, MapScripting};
use ra_types::EntityId;

use crate::game::{BattleOutcome, GameCommand};
use crate::gameplay::{ai::is_ambient_house, houses_are_allied};
use crate::state::BattleState;
use crate::state::components::{Health, Identity, Owner, Transform};

/// 零售地图 `[Events]` 条件类型码（竖切已接线子集）。
///
/// 编号与原版 RA2 触发事件表一致；未列出的 kind 在 `condition_met` 中视为未满足。
const EVENT_ENTERED_BY: i32 = 1; // 进入绑定 CellTag 的格子（本竖切按本地玩家 house 判定）
const EVENT_DESTROYED: i32 = 8; // 绑定 Tag 的对象被摧毁（任一）
const EVENT_ALL_DESTROYED: i32 = 11; // 绑定 Tag 的对象全部摧毁
const EVENT_TIME_ELAPSE: i32 = 13; // 计时结束（params[0] 为初始 tick 数）

/// 零售地图 `[Actions]` 动作类型码（竖切已接线子集）。
///
/// 编号与原版 RA2 触发动作表一致；未列出的 kind 记入 `unsupported_actions`。
const ACTION_NONE: i32 = 0; // 无操作
const ACTION_WIN: i32 = 1; // 指定 house 胜利
const ACTION_LOSE: i32 = 2; // 失败（可带原因/house 参数）
const ACTION_CREATE_TEAM: i32 = 4; // 创建 TeamType（排队生成 TaskForce）
const ACTION_DESTROY_ATTACHED_OBJECTS: i32 = 5; // 摧毁绑定本触发 Tag 的存活实体
const ACTION_ALL_TO_HUNT: i32 = 6; // 指定 house 全部机动单位攻击最近敌方
const ACTION_DESTROY_TRIGGER: i32 = 12; // 销毁触发器（目标禁用且视为已触发）
const ACTION_CHANGE_HOUSE: i32 = 14; // 绑定本触发 Tag 的存活实体改属指定 house
const ACTION_FORCE_TRIGGER: i32 = 40; // 强制执行另一触发器的 Actions（跳过 Events）
const ACTION_TIMER_SET: i32 = 45; // 将目标触发器的计时器设为指定 tick（并可再次触发）
const ACTION_ENABLE_TRIGGER: i32 = 53; // 启用（解除 disabled）另一触发器
const ACTION_DISABLE_TRIGGER: i32 = 54; // 禁用另一触发器
const ACTION_REINFORCEMENT_TEAM: i32 = 80; // 增援 TeamType（与 Create Team 同路径产队）

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
            apply_action(world, &id, cmd, &local_house);
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

fn apply_action(world: &mut BattleState, trigger_id: &str, cmd: &MapActionCommand, local_house: &str) {
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
        ACTION_DESTROY_ATTACHED_OBJECTS => {
            destroy_attached_objects(world, trigger_id);
        }
        ACTION_ALL_TO_HUNT => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            all_house_units_hunt(world, &house);
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
        ACTION_CHANGE_HOUSE => {
            let Some(new_house) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            change_attached_objects_house(world, trigger_id, &new_house);
        }
        ACTION_FORCE_TRIGGER => {
            if let Some(id) = action_trigger_id_param(cmd) {
                force_fire_trigger(world, &id, local_house);
            } else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        ACTION_TIMER_SET => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            // 目标触发：参数中的 Trigger id；缺省则作用于本触发。
            let target = action_trigger_id_param(cmd).unwrap_or_else(|| trigger_id.to_string());
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&target)) {
                st.timer_remaining = Some(ticks);
                st.fired = false;
                st.disabled = false;
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
        ACTION_NONE => {}
        other => world.trigger_runtime.record_unsupported(other),
    }
}

/// 将绑定到本触发 Tag 的存活实体改属 `new_house`。
fn change_attached_objects_house(world: &mut BattleState, trigger_id: &str, new_house: &str) {
    world.ensure_house(new_house);
    let bound = tags_for_trigger(&world.map.scripting.tags, trigger_id);
    if bound.is_empty() {
        return;
    }
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            world
                .ecs_get::<Identity>(id)
                .map(|identity| !identity.tag.is_empty() && bound.contains(&identity.tag))
                .unwrap_or(false)
        })
        .collect();
    for id in ids {
        let _ = world.with_owner_mut(id, |owner| {
            owner.house = std::sync::Arc::<str>::from(new_house);
        });
    }
}

/// 摧毁绑定到本触发 Tag 的存活实体。
fn destroy_attached_objects(world: &mut BattleState, trigger_id: &str) {
    let bound = tags_for_trigger(&world.map.scripting.tags, trigger_id);
    if bound.is_empty() {
        return;
    }
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            world
                .ecs_get::<Identity>(id)
                .map(|identity| !identity.tag.is_empty() && bound.contains(&identity.tag))
                .unwrap_or(false)
        })
        .collect();
    for id in ids {
        let max = world.ecs_health(id).map(|(_, m, _)| m).unwrap_or(1).max(1);
        let _ = world.set_ecs_health(id, 0, max, true);
    }
}

/// 指定 house 的全部机动单位攻击各自最近的敌对目标。
fn all_house_units_hunt(world: &mut BattleState, house: &str) {
    let Some(player) = world.players.iter().find(|p| p.house.as_ref().eq_ignore_ascii_case(house)).map(|p| p.id)
    else {
        return;
    };
    let hunters: Vec<EntityId> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            let Some(owner) = world.ecs_get::<Owner>(id)
            else {
                return false;
            };
            if !owner.house.eq_ignore_ascii_case(house) {
                return false;
            }
            world
                .ecs_get::<Identity>(id)
                .map(|identity| {
                    matches!(
                        identity.kind,
                        MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                    )
                })
                .unwrap_or(false)
        })
        .collect();
    let mut orders = Vec::new();
    for hunter in hunters {
        let Some((hx, hy, _)) = world.ecs_transform(hunter)
        else {
            continue;
        };
        if let Some(target) = nearest_hostile_from(world, house, hx, hy) {
            orders.push((hunter, target));
        }
    }
    for (attacker, target) in orders {
        world.push_player_command(player, GameCommand::Attack { attacker, target });
    }
}

/// 以 `(cx,cy)` 为原点找最近敌对存活实体。
fn nearest_hostile_from(world: &BattleState, house: &str, cx: u16, cy: u16) -> Option<EntityId> {
    let mut best: Option<(u32, EntityId)> = None;
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(owner) = world.ecs_get::<Owner>(id)
        else {
            continue;
        };
        if houses_are_allied(world, house, owner.house.as_ref()) {
            continue;
        }
        if is_ambient_house(owner.house.as_ref()) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        let dist = (i32::from(cx) - i32::from(xf.x)).unsigned_abs() + (i32::from(cy) - i32::from(xf.y)).unsigned_abs();
        if best.map(|(d, _)| dist < d).unwrap_or(true) {
            best = Some((dist, id));
        }
    }
    best.map(|(_, id)| id)
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
        apply_action(world, id, cmd, local_house);
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

/// 从动作参数中取 Timer Set 的 tick 数。
///
/// 常见布局：`kind,0,<TriggerId>,<ticks>,…`（ticks 在 `params[2]`）；
/// 或 `kind,0,<ticks>,…`（无目标 id 时 ticks 在 `params[1]`）。
fn action_timer_ticks_param(cmd: &MapActionCommand) -> Option<u32> {
    if let Some(n) = cmd.params.get(2).map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok()) {
        return Some(n);
    }
    if let Some(n) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok()) {
        return Some(n);
    }
    cmd.params.first().map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
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
