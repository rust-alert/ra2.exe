//! 地图触发与脚本钩子：评估 Events、执行 Actions（Win/Lose/建队等）。

use std::collections::{HashMap, HashSet};

use ra_map::{MapActionKind, MapEntityKind, MapEventCondition, MapEventKind};
use ra_types::{EntityId, PreparedAction, PreparedActionCommand, PreparedEvent, PreparedTrigger, TagId, TriggerId};

use crate::{
    game::{BattleOutcome, GameCommand},
    gameplay::{ai::is_ambient_house, houses_are_allied, script_teams::try_enqueue_team_spawn},
    state::{
        BattleState,
        components::{Health, Identity, Owner, Transform},
    },
};

/// 地图触发事件/动作类型见 [`MapEventKind`] / [`MapActionKind`]（`ra-map`）。

/// 单条触发器运行时状态。
#[derive(Debug, Clone)]
struct TriggerRuntimeState {
    id: TriggerId,
    disabled: bool,
    fired: bool,
    /// `MapEventKind::TimeElapse` 倒计时（tick）；`None` 表示本触发无计时条件。
    timer_remaining: Option<u32>,
    /// 计时器是否暂停（`Timer Stop` / `Timer Start`）。
    timer_paused: bool,
}

/// 局内触发运行时（由 [`PreparedTrigger`] / [`PreparedEvent`] / [`PreparedAction`] 播种）。
#[derive(Debug, Clone, Default)]
pub struct TriggerRuntime {
    states: Vec<TriggerRuntimeState>,
    /// 未实现动作码（去重后供能力缺口报告）。
    pub unsupported_actions: Vec<i32>,
    /// 待创建的 TeamType（可选产队房主 / 航点覆盖）。
    pub pending_team_spawns: Vec<super::script_teams::PendingTeamSpawn>,
    /// 本 tick 请求的剧本胜负（由 BattleSession 消费）。
    pub pending_outcome: Option<BattleOutcome>,
    /// 胜利阻塞层数：开局等于含 `Allow Win` 动作的触发条数；归零后 `Win` 才生效。
    win_blockers: u32,
    /// `Win` 在阻塞未清时记下的获胜 house；阻塞归零后写入 `pending_outcome`。
    deferred_victory_house: Option<String>,
    /// 剧本「Lock input」：为真时 host 应吞掉对局操作（暂停/Esc 仍可用）。
    pub script_input_locked: bool,
    /// 剧本动作刷出的可拾取箱（竖切：踩格领固定资金）。
    pub script_crates: Vec<ScriptCrate>,
}

/// 剧本 `CreateCrate` 刷出的箱子。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptCrate {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 原版箱子类型参数（竖切暂不区分效果）。
    pub crate_type: String,
}

/// 剧本箱踩格领取的资金（竖切固定值，完整 Powerups 表后置）。
pub const SCRIPT_CRATE_CREDITS: i32 = 2_000;

impl TriggerRuntime {
    /// 从已绑定的 trigger / event / action 表播种。
    ///
    /// 无触发则空运行时。`win_blockers` 按已绑定 `[Actions]` 中含 `Allow Win` 的条数统计。
    pub fn from_prepared(triggers: &[PreparedTrigger], events: &[PreparedEvent], actions: &[PreparedAction]) -> Self {
        let events_by_tid: HashMap<_, &_> = events.iter().map(|e| (e.trigger_id, e)).collect();
        let mut states = Vec::with_capacity(triggers.len());
        for tr in triggers {
            let timer_remaining = events_by_tid.get(&tr.id).and_then(|ev| {
                ev.conditions
                    .iter()
                    .find(|c| MapEventKind::from_code(c.kind_code) == MapEventKind::TimeElapse)
                    .map(|c| c.params.first().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0))
            });
            states.push(TriggerRuntimeState { id: tr.id, disabled: tr.disabled, fired: false, timer_remaining, timer_paused: false });
        }
        Self {
            states,
            unsupported_actions: Vec::new(),
            pending_team_spawns: Vec::new(),
            pending_outcome: None,
            win_blockers: count_allow_win_actions(actions),
            deferred_victory_house: None,
            script_input_locked: false,
            script_crates: Vec::new(),
        }
    }

    fn record_unsupported(&mut self, kind: MapActionKind) {
        let code = kind.code();
        if !self.unsupported_actions.contains(&code) {
            self.unsupported_actions.push(code);
        }
    }
}

/// 每个逻辑 tick 评估并执行已满足条件的触发器。
pub fn tick_triggers(world: &mut BattleState) {
    if world.trigger_runtime.pending_outcome.is_some() {
        return;
    }
    if world.trigger_runtime.states.is_empty() {
        return;
    }

    let events_by_id: HashMap<TriggerId, Vec<MapEventCondition>> = world
        .prepared
        .events
        .iter()
        .map(|e| {
            (
                e.trigger_id,
                e.conditions
                    .iter()
                    .map(|c| MapEventCondition { kind: MapEventKind::from_code(c.kind_code), params: c.params.clone() })
                    .collect(),
            )
        })
        .collect();
    let actions_by_id: HashMap<TriggerId, Vec<PreparedActionCommand>> =
        world.prepared.actions.iter().map(|a| (a.trigger_id, a.commands.clone())).collect();
    let local_house = world.players.iter().find(|p| p.id == world.local_player).map(|p| p.house.clone()).unwrap_or_default();

    // 先推进计时器（暂停中的不扣减）。
    for st in &mut world.trigger_runtime.states {
        if st.disabled || st.fired || st.timer_paused {
            continue;
        }
        if let Some(rem) = st.timer_remaining.as_mut() {
            if *rem > 0 {
                *rem = rem.saturating_sub(1);
            }
        }
    }

    let mut to_fire: Vec<TriggerId> = Vec::new();
    for st in &world.trigger_runtime.states {
        if st.disabled || st.fired {
            continue;
        }
        let Some(conditions) = events_by_id.get(&st.id)
        else {
            continue;
        };
        if event_conditions_met(world, st, conditions, &local_house) {
            to_fire.push(st.id);
        }
    }

    for id in to_fire {
        if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == id) {
            st.fired = true;
        }
        let Some(commands) = actions_by_id.get(&id)
        else {
            continue;
        };
        for cmd in commands {
            apply_action(world, id, cmd, &local_house);
            if world.trigger_runtime.pending_outcome.is_some() {
                return;
            }
        }
    }
}

fn event_conditions_met(world: &BattleState, st: &TriggerRuntimeState, conditions: &[MapEventCondition], local_house: &str) -> bool {
    if conditions.is_empty() {
        return false;
    }
    conditions.iter().all(|c| condition_met(world, st, c, local_house))
}

fn condition_met(world: &BattleState, st: &TriggerRuntimeState, c: &MapEventCondition, local_house: &str) -> bool {
    match c.kind {
        MapEventKind::AnyEvent => true,
        MapEventKind::TimeElapse => st.timer_remaining == Some(0),
        MapEventKind::DestroyedByAnybody | MapEventKind::DestroyedByAnything => {
            let bound_tags = tags_for_trigger(world, st.id);
            if bound_tags.is_empty() {
                return false;
            }
            !any_living_with_tags(world, &bound_tags)
        }
        MapEventKind::DestroyedUnitsAll => {
            let house = event_house_param(c).unwrap_or_else(|| local_house.to_string());
            !any_living_of_house(world, &house, HouseAliveFilter::LandUnits)
        }
        MapEventKind::DestroyedBuildingsAll => {
            let house = event_house_param(c).unwrap_or_else(|| local_house.to_string());
            !any_living_of_house(world, &house, HouseAliveFilter::Buildings)
        }
        MapEventKind::DestroyedAll => {
            let house = event_house_param(c).unwrap_or_else(|| local_house.to_string());
            !any_living_of_house(world, &house, HouseAliveFilter::All)
        }
        MapEventKind::EnteredBy => {
            let bound_tags = tags_for_trigger(world, st.id);
            if bound_tags.is_empty() {
                return false;
            }
            cell_entered_by_house(world, &bound_tags, local_house)
        }
        MapEventKind::CreditsExceed => {
            let Some(threshold) = event_numeric_param(c)
            else {
                return false;
            };
            let house = trigger_owner_house(world, st.id).unwrap_or_else(|| local_house.to_string());
            house_funds(world, &house) >= threshold as i32
        }
        MapEventKind::CreditsBelow => {
            let Some(threshold) = event_numeric_param(c)
            else {
                return false;
            };
            let house = trigger_owner_house(world, st.id).unwrap_or_else(|| local_house.to_string());
            house_funds(world, &house) < threshold as i32
        }
        MapEventKind::LowPower => {
            let house = event_house_param(c).unwrap_or_else(|| local_house.to_string());
            world.players.iter().find(|p| p.house.as_ref().eq_ignore_ascii_case(&house)).map(|p| p.low_power()).unwrap_or(false)
        }
        MapEventKind::Unknown(_) => false,
    }
}

/// 事件条件中的数值参数（Credits 阈值等，常在 `params[1]`）。
fn event_numeric_param(c: &MapEventCondition) -> Option<u32> {
    for p in c.params.iter().rev() {
        let t = p.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(n) = t.parse::<u32>() {
            return Some(n);
        }
    }
    None
}

fn house_funds(world: &BattleState, house: &str) -> i32 {
    world.players.iter().find(|p| p.house.as_ref().eq_ignore_ascii_case(house)).map(|p| p.funds).unwrap_or(0)
}

/// 事件条件中的 house 参数（常见在 `params[1]`）。
fn event_house_param(c: &MapEventCondition) -> Option<String> {
    for p in c.params.iter().rev() {
        let t = p.trim();
        if t.is_empty() || t == "0" || t == "-1" {
            continue;
        }
        if t.parse::<i32>().is_ok() {
            continue;
        }
        return Some(t.to_string());
    }
    None
}

/// 阵营存活过滤（事件 9/10/11）。
#[derive(Clone, Copy)]
enum HouseAliveFilter {
    All,
    Buildings,
    LandUnits,
}

fn any_living_of_house(world: &BattleState, house: &str, filter: HouseAliveFilter) -> bool {
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        let matched = match filter {
            HouseAliveFilter::All => true,
            HouseAliveFilter::Buildings => identity.kind == MapEntityKind::Structure,
            HouseAliveFilter::LandUnits => {
                matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry)
            }
        };
        if matched {
            return true;
        }
    }
    false
}

fn tags_for_trigger(world: &BattleState, trigger_id: TriggerId) -> HashSet<TagId> {
    world.prepared.tags.iter().filter(|t| t.trigger_id == trigger_id).map(|t| t.id).collect()
}

/// 统计已绑定动作表中含 `Allow Win` 的触发条数（每条贡献一层胜利阻塞）。
fn count_allow_win_actions(actions: &[PreparedAction]) -> u32 {
    actions.iter().filter(|a| a.commands.iter().any(|c| MapActionKind::from_code(c.kind_code) == MapActionKind::AllowWin)).count() as u32
}

/// 查找触发器所属 house（优先 [`PreparedTrigger.house`](PreparedTrigger) 稳定 id）。
fn trigger_owner_house(world: &BattleState, trigger_id: TriggerId) -> Option<String> {
    let house_id = world.prepared.triggers.iter().find(|t| t.id == trigger_id).map(|t| t.house)?;
    world.definitions.houses.get_by_id(house_id).map(|h| h.type_key.as_str().to_string()).filter(|h| !h.is_empty())
}

/// 双向结盟或解盟：写入双方 `PlayerState.allies`。
fn set_houses_allied(world: &mut BattleState, a: &str, b: &str, allied: bool) {
    if a.eq_ignore_ascii_case(b) {
        return;
    }
    world.ensure_house(a);
    world.ensure_house(b);
    for (house, other) in [(a, b), (b, a)] {
        let Some(player) = world.players.iter_mut().find(|p| p.house.as_ref().eq_ignore_ascii_case(house))
        else {
            continue;
        };
        if allied {
            if !player.allies.iter().any(|x| x.eq_ignore_ascii_case(other)) {
                player.allies.push(other.to_string());
            }
        }
        else {
            player.allies.retain(|x| !x.eq_ignore_ascii_case(other));
        }
    }
    world.rehash();
}

fn any_living_with_tags(world: &BattleState, tags: &HashSet<TagId>) -> bool {
    for e in &world.entities {
        let id = e.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(identity) = world.ecs_get::<Identity>(id)
        else {
            continue;
        };
        if identity.tag.is_some_and(|tag| tags.contains(&tag)) {
            return true;
        }
    }
    false
}

fn cell_entered_by_house(world: &BattleState, bound_tags: &HashSet<TagId>, house: &str) -> bool {
    let cells: Vec<(u16, u16)> = world.prepared.cell_tags.iter().filter(|c| bound_tags.contains(&c.tag)).map(|c| (c.x, c.y)).collect();
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
        if crate::gameplay::house_id_of(&world.definitions, house) != Some(owner.house) {
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

fn apply_action(world: &mut BattleState, trigger_id: TriggerId, cmd: &PreparedActionCommand, local_house: &str) {
    let kind = MapActionKind::from_code(cmd.kind_code);
    match kind {
        MapActionKind::Unknown(_) => {
            world.trigger_runtime.record_unsupported(kind);
        }
        MapActionKind::Win => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string()).trim().to_ascii_uppercase();
            if world.trigger_runtime.win_blockers > 0 {
                // 仍有 Allow Win 阻塞：延后胜利，待阻塞清零。
                world.trigger_runtime.deferred_victory_house = Some(house);
                return;
            }
            world.trigger_runtime.pending_outcome = Some(BattleOutcome::Victory { owner: house });
        }
        MapActionKind::Lose => {
            let reason = action_house_param(cmd).unwrap_or_default();
            world.trigger_runtime.pending_outcome = Some(BattleOutcome::Defeat { reason });
        }
        MapActionKind::ProductionBegins => {
            let house = action_house_param(cmd).or_else(|| trigger_owner_house(world, trigger_id)).unwrap_or_else(|| local_house.to_string());
            if !world.begin_house_production(&house) {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::LockInput => {
            world.trigger_runtime.script_input_locked = true;
        }
        MapActionKind::UnlockInput => {
            world.trigger_runtime.script_input_locked = false;
        }
        MapActionKind::Apply100Damage => {
            if !apply_100_damage_at_action_waypoint(world, cmd) {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::CreateCrate => {
            if !spawn_script_crate_at_action(world, cmd) {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::AllowWin => {
            world.trigger_runtime.win_blockers = world.trigger_runtime.win_blockers.saturating_sub(1);
            if world.trigger_runtime.win_blockers == 0 {
                if let Some(house) = world.trigger_runtime.deferred_victory_house.take() {
                    world.trigger_runtime.pending_outcome = Some(BattleOutcome::Victory { owner: house });
                }
            }
        }
        MapActionKind::CreateTeam | MapActionKind::Reinforcement => {
            if let Some(team_id) = cmd.team_id {
                let _ = try_enqueue_team_spawn(world, team_id, None, None);
            }
            else {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::ReinforcementAtWaypoint => {
            if let Some(team_id) = cmd.team_id {
                let wp = action_waypoint_index_param(cmd).map(|n| n as i32);
                let _ = try_enqueue_team_spawn(world, team_id, None, wp);
            }
            else {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::DestroyTeam => {
            if let Some(team_id) = cmd.team_id {
                super::script_teams::destroy_team_type(world, team_id);
            }
            else {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::DestroyAttachedObjects => {
            destroy_attached_objects(world, trigger_id);
        }
        MapActionKind::DestroyTag => {
            let Some(tag_id) = cmd.tag_id
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            destroy_entities_with_tag(world, tag_id);
        }
        MapActionKind::AllToHunt => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            all_house_units_hunt(world, &house);
        }
        MapActionKind::DestroyTrigger => {
            if let Some(id) = cmd.target_trigger_id {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == id) {
                    st.disabled = true;
                    st.fired = true;
                }
            }
            else {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::ChangeHouse => {
            let Some(new_house) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            change_attached_objects_house(world, trigger_id, &new_house);
        }
        MapActionKind::MakeAlly => {
            let Some(other) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            let owner = trigger_owner_house(world, trigger_id).unwrap_or_else(|| local_house.to_string());
            set_houses_allied(world, &owner, &other, true);
        }
        MapActionKind::MakeEnemy => {
            let Some(other) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            let owner = trigger_owner_house(world, trigger_id).unwrap_or_else(|| local_house.to_string());
            set_houses_allied(world, &owner, &other, false);
        }
        MapActionKind::AllChangeHouse => {
            let Some(new_house) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            let from = trigger_owner_house(world, trigger_id).unwrap_or_else(|| local_house.to_string());
            change_all_house_entities(world, &from, &new_house);
        }
        MapActionKind::DestroyAllOf => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            destroy_house_entities(world, &house, DestroyHouseFilter::All);
        }
        MapActionKind::DestroyAllBuildingsOf => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            destroy_house_entities(world, &house, DestroyHouseFilter::Buildings);
        }
        MapActionKind::DestroyAllLandUnitsOf => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            destroy_house_entities(world, &house, DestroyHouseFilter::LandUnits);
        }
        MapActionKind::ForceTrigger => {
            if let Some(id) = cmd.target_trigger_id {
                force_fire_trigger(world, id, local_house);
            }
            else {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::TimerStart => {
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                st.timer_paused = false;
            }
        }
        MapActionKind::TimerStop => {
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                st.timer_paused = true;
            }
        }
        MapActionKind::TimerExtend => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                let cur = st.timer_remaining.unwrap_or(0);
                st.timer_remaining = Some(cur.saturating_add(ticks));
                st.fired = false;
                st.disabled = false;
            }
        }
        MapActionKind::TimerShorten => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                let cur = st.timer_remaining.unwrap_or(0);
                st.timer_remaining = Some(cur.saturating_sub(ticks));
                st.fired = false;
                st.disabled = false;
            }
        }
        MapActionKind::TimerSet => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(kind);
                return;
            };
            // 目标触发：参数中的 Trigger id；缺省则作用于本触发。
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                st.timer_remaining = Some(ticks);
                st.timer_paused = false;
                st.fired = false;
                st.disabled = false;
            }
        }
        MapActionKind::EnableTrigger => {
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                st.disabled = false;
            }
        }
        MapActionKind::DisableTrigger => {
            let target = cmd.target_trigger_id.unwrap_or(trigger_id);
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == target) {
                st.disabled = true;
            }
        }
        MapActionKind::AiTriggersBegin => {
            let house = action_house_param(cmd);
            super::ai_triggers::set_ai_triggers_for_house(world, house.as_deref(), true);
            world.ai_trigger_runtime.enabled = true;
        }
        MapActionKind::AiTriggersStop => {
            let house = action_house_param(cmd);
            if house.is_none() {
                world.ai_trigger_runtime.enabled = false;
            }
            else {
                super::ai_triggers::set_ai_triggers_for_house(world, house.as_deref(), false);
            }
        }
        MapActionKind::CreateRadarEvent => {
            if let Some(wp_idx) = action_waypoint_index_param(cmd) {
                if let Some(wp) = world.map.waypoints.iter().find(|w| w.index == wp_idx).copied() {
                    let house =
                        action_house_param(cmd).or_else(|| trigger_owner_house(world, trigger_id)).unwrap_or_else(|| local_house.to_string());
                    world.push_radar_event(&house, wp.x, wp.y);
                }
                else {
                    world.trigger_runtime.record_unsupported(kind);
                }
            }
            else {
                world.trigger_runtime.record_unsupported(kind);
            }
        }
        MapActionKind::None
        | MapActionKind::DropZoneFlare
        | MapActionKind::PlayMovie
        | MapActionKind::TextTrigger
        | MapActionKind::RevealAllMap
        | MapActionKind::RevealAroundWaypoint
        | MapActionKind::RevealWaypointZone
        | MapActionKind::PlaySound
        | MapActionKind::PlayTheme
        | MapActionKind::PlaySpeech
        | MapActionKind::PlayAnimAt
        | MapActionKind::CenterCameraAtWaypoint
        | MapActionKind::GrowShroud
        | MapActionKind::ReshroudMap
        | MapActionKind::PlaySoundEffect
        | MapActionKind::PlaySoundEffectAt
        | MapActionKind::PlayIngameMovie
        | MapActionKind::ReshroudMapAt
        | MapActionKind::TimerText
        | MapActionKind::FlashTeam
        | MapActionKind::MakeHouseCheer
        | MapActionKind::SetSidebarTab
        | MapActionKind::FlashCameo
        | MapActionKind::StopSoundsAt => {}
    }
}

/// 将绑定到本触发 Tag 的存活实体改属 `new_house`。
fn change_attached_objects_house(world: &mut BattleState, trigger_id: TriggerId, new_house: &str) {
    world.ensure_house(new_house);
    let bound = tags_for_trigger(world, trigger_id);
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
            world.ecs_get::<Identity>(id).map(|identity| identity.tag.is_some_and(|tag| bound.contains(&tag))).unwrap_or(false)
        })
        .collect();
    let Some(new_house_id) = crate::gameplay::house_id_of(&world.definitions, new_house)
    else {
        return;
    };
    for id in ids {
        let _ = world.with_owner_mut(id, |owner| {
            owner.house = new_house_id;
        });
    }
}

/// 将 `from_house` 名下全部存活实体改属 `new_house`。
fn change_all_house_entities(world: &mut BattleState, from_house: &str, new_house: &str) {
    if from_house.eq_ignore_ascii_case(new_house) {
        return;
    }
    world.ensure_house(new_house);
    let Some(from_house_id) = crate::gameplay::house_id_of(&world.definitions, from_house)
    else {
        return;
    };
    let Some(new_house_id) = crate::gameplay::house_id_of(&world.definitions, new_house)
    else {
        return;
    };
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            world.ecs_get::<Owner>(id).map(|o| o.house == from_house_id).unwrap_or(false)
        })
        .collect();
    for id in ids {
        let _ = world.with_owner_mut(id, |owner| {
            owner.house = new_house_id;
        });
    }
}

/// 按种类过滤摧毁指定 house 实体。
#[derive(Clone, Copy)]
enum DestroyHouseFilter {
    All,
    Buildings,
    LandUnits,
}

fn destroy_house_entities(world: &mut BattleState, house: &str, filter: DestroyHouseFilter) {
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_id_of(&world.definitions, house) == Some(o.house)).unwrap_or(false) {
                return false;
            }
            let Some(identity) = world.ecs_get::<Identity>(id)
            else {
                return false;
            };
            match filter {
                DestroyHouseFilter::All => true,
                DestroyHouseFilter::Buildings => identity.kind == MapEntityKind::Structure,
                DestroyHouseFilter::LandUnits => {
                    matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry)
                }
            }
        })
        .collect();
    for id in ids {
        let max = world.ecs_health(id).map(|(_, m, _)| m).unwrap_or(1).max(1);
        let _ = world.set_ecs_health(id, 0, max, true);
    }
}

/// 摧毁绑定到本触发 Tag 的存活实体。
fn destroy_attached_objects(world: &mut BattleState, trigger_id: TriggerId) {
    let bound = tags_for_trigger(world, trigger_id);
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
            world.ecs_get::<Identity>(id).map(|identity| identity.tag.is_some_and(|tag| bound.contains(&tag))).unwrap_or(false)
        })
        .collect();
    for id in ids {
        let max = world.ecs_health(id).map(|(_, m, _)| m).unwrap_or(1).max(1);
        let _ = world.set_ecs_health(id, 0, max, true);
    }
}

/// 摧毁带有指定 Tag 稳定 id 的全部存活实体。
fn destroy_entities_with_tag(world: &mut BattleState, tag_id: TagId) {
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            world.ecs_get::<Identity>(id).map(|identity| identity.tag == Some(tag_id)).unwrap_or(false)
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
            if crate::gameplay::house_id_of(&world.definitions, house) != Some(owner.house) {
                return false;
            }
            world
                .ecs_get::<Identity>(id)
                .map(|identity| matches!(identity.kind, MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft))
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
        if houses_are_allied(world, house, crate::gameplay::house_key_of(&world.definitions, owner.house)) {
            continue;
        }
        if is_ambient_house(crate::gameplay::house_key_of(&world.definitions, owner.house)) {
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
fn force_fire_trigger(world: &mut BattleState, id: TriggerId, local_house: &str) {
    let already = world.trigger_runtime.states.iter().find(|s| s.id == id).map(|s| s.fired).unwrap_or(true);
    if already {
        return;
    }
    if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id == id) {
        st.disabled = false;
        st.fired = true;
    }
    let commands: Vec<PreparedActionCommand> =
        world.prepared.actions.iter().find(|a| a.trigger_id == id).map(|a| a.commands.clone()).unwrap_or_default();
    for cmd in &commands {
        apply_action(world, id, cmd, local_house);
        if world.trigger_runtime.pending_outcome.is_some() {
            return;
        }
    }
}

fn action_waypoint_index_param(cmd: &PreparedActionCommand) -> Option<u32> {
    // 原版布局：`kind,0,<Waypoint#>,…` → 航点在 `params[1]`。
    if let Some(n) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok()) {
        return Some(n);
    }
    cmd.params.first().map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

/// 在动作指定航点格造成 100 点伤害（覆盖该格上的机动单位与 Foundation 含该格的建筑）。
fn apply_100_damage_at_action_waypoint(world: &mut BattleState, cmd: &PreparedActionCommand) -> bool {
    let Some(wp_idx) = action_waypoint_index_param(cmd)
    else {
        return false;
    };
    let Some(wp) = world.map.waypoints.iter().find(|w| w.index == wp_idx).copied()
    else {
        return false;
    };
    let mut hit_indices = Vec::new();
    for (index, entity) in world.entities.iter().enumerate() {
        let id = entity.id;
        if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
            continue;
        }
        let Some(xf) = world.ecs_get::<Transform>(id)
        else {
            continue;
        };
        let is_structure = world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false);
        let covers = if is_structure {
            let type_id = world.ecs_get::<Identity>(id).map(|i| i.type_id);
            let foundation =
                type_id.and_then(|tid| world.definitions.structures.get_by_id(tid)).map(|s| s.foundation.clone()).unwrap_or_default();
            let fw = foundation.width.max(1);
            let fh = foundation.height.max(1);
            wp.x >= xf.x && wp.y >= xf.y && wp.x < xf.x.saturating_add(fw) && wp.y < xf.y.saturating_add(fh)
        }
        else {
            xf.x == wp.x && xf.y == wp.y
        };
        if covers {
            hit_indices.push(index);
        }
    }
    for index in hit_indices {
        world.apply_damage(index, 100);
    }
    true
}

/// 在航点刷出剧本箱。参数：`params[1]`=类型，`params[6]`=航点号（缺航点则失败并记 unsupported）。
fn spawn_script_crate_at_action(world: &mut BattleState, cmd: &PreparedActionCommand) -> bool {
    let crate_type = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()).unwrap_or("0").to_string();
    // Create Crate：类型在 P2（params[1]），航点在 P7（params[6]）；禁止把类型误当航点。
    let Some(wp_idx) = cmd.params.get(6).map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse::<u32>().ok())
    else {
        return false;
    };
    let Some(wp) = world.map.waypoints.iter().find(|w| w.index == wp_idx).copied()
    else {
        return false;
    };
    // 奖励暂固定竖切，尚未按 Powerups 类型表解析。
    world.trigger_runtime.script_crates.push(ScriptCrate { x: wp.x, y: wp.y, crate_type });
    true
}

/// 机动单位踩到剧本箱时领取固定资金并移除该箱。
pub fn tick_script_crates(world: &mut BattleState) {
    if world.trigger_runtime.script_crates.is_empty() {
        return;
    }
    let mut collected = Vec::new();
    for (ci, crate_spawn) in world.trigger_runtime.script_crates.iter().enumerate() {
        for entity in &world.entities {
            let id = entity.id;
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                continue;
            }
            if world.ecs_get::<Identity>(id).map(|i| i.kind == MapEntityKind::Structure).unwrap_or(false) {
                continue;
            }
            let Some(xf) = world.ecs_get::<Transform>(id)
            else {
                continue;
            };
            if xf.x != crate_spawn.x || xf.y != crate_spawn.y {
                continue;
            }
            let Some(house) = world.ecs_get::<Owner>(id).map(|o| crate::gameplay::house_key_of(&world.definitions, o.house).to_string())
            else {
                continue;
            };
            collected.push((ci, house));
            break;
        }
    }
    // 从后往前删，避免下标错位；同一 tick 多箱可被不同单位领。
    collected.sort_by(|a, b| b.0.cmp(&a.0));
    let mut seen = std::collections::HashSet::new();
    for (ci, house) in collected {
        if !seen.insert(ci) {
            continue;
        }
        if ci >= world.trigger_runtime.script_crates.len() {
            continue;
        }
        world.trigger_runtime.script_crates.remove(ci);
        let _ = world.set_house_funds(&house, world.house_funds(&house).unwrap_or(0).saturating_add(SCRIPT_CRATE_CREDITS));
    }
}

/// 从动作参数中取 Timer Set 的 tick 数。
///
/// 常见布局：`kind,0,<TriggerId>,<ticks>,…`（ticks 在 `params[2]`）；
/// 或 `kind,0,<ticks>,…`（无目标 id 时 ticks 在 `params[1]`）。
fn action_timer_ticks_param(cmd: &PreparedActionCommand) -> Option<u32> {
    if let Some(n) = cmd.params.get(2).map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok()) {
        return Some(n);
    }
    if let Some(n) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok()) {
        return Some(n);
    }
    cmd.params.first().map(|s| s.trim()).filter(|s| !s.is_empty()).and_then(|s| s.parse().ok())
}

fn action_house_param(cmd: &PreparedActionCommand) -> Option<String> {
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
