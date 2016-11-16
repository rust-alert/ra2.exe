//! 地图触发与脚本钩子：评估 Events、执行 Actions（Win/Lose/建队等）。

use std::collections::{HashMap, HashSet};

use ra_map::{MapActionCommand, MapActionKind, MapEntityKind, MapEventCondition, MapEventKind, MapScripting};
use ra_types::EntityId;

use crate::{
    game::{BattleOutcome, GameCommand},
    gameplay::{ai::is_ambient_house, houses_are_allied},
    state::{
        BattleState,
        components::{Health, Identity, Owner, Transform},
    },
};

/// 地图触发事件/动作类型见 [`MapEventKind`] / [`MapActionKind`]（`ra-map`）。

/// 单条触发器运行时状态。
#[derive(Debug, Clone)]
struct TriggerRuntimeState {
    id: String,
    disabled: bool,
    fired: bool,
    /// `MapEventKind::TimeElapse` 倒计时（tick）；`None` 表示本触发无计时条件。
    timer_remaining: Option<u32>,
    /// 计时器是否暂停（`Timer Stop` / `Timer Start`）。
    timer_paused: bool,
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
    /// 胜利阻塞层数：开局等于含 `Allow Win` 动作的触发条数；归零后 `Win` 才生效。
    win_blockers: u32,
    /// `Win` 在阻塞未清时记下的获胜 house；阻塞归零后写入 `pending_outcome`。
    deferred_victory_house: Option<String>,
}

impl TriggerRuntime {
    /// 从地图剧本播种；无触发则空运行时。
    pub fn from_scripting(scripting: &MapScripting) -> Self {
        let events_by_id: HashMap<&str, &ra_map::MapEvent> = scripting.events.iter().map(|e| (e.id.as_str(), e)).collect();
        let mut states = Vec::with_capacity(scripting.triggers.len());
        for tr in &scripting.triggers {
            let timer_remaining = events_by_id.get(tr.id.as_str()).and_then(|ev| {
                ev.conditions
                    .iter()
                    .find(|c| c.kind == MapEventKind::TimeElapse)
                    .map(|c| c.params.first().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0))
            });
            states.push(TriggerRuntimeState { id: tr.id.clone(), disabled: tr.disabled, fired: false, timer_remaining, timer_paused: false });
        }
        Self {
            states,
            unsupported_actions: Vec::new(),
            pending_team_spawns: Vec::new(),
            pending_outcome: None,
            win_blockers: count_allow_win_actions(scripting),
            deferred_victory_house: None,
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
    let scripting = world.map.scripting.clone();
    if scripting.triggers.is_empty() {
        return;
    }

    let events_by_id: HashMap<String, ra_map::MapEvent> = scripting.events.iter().cloned().map(|e| (e.id.clone(), e)).collect();
    let actions_by_id: HashMap<String, ra_map::MapAction> = scripting.actions.iter().cloned().map(|a| (a.id.clone(), a)).collect();
    let tags = scripting.tags.clone();
    let cell_tags = scripting.cell_tags.clone();
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
        MapEventKind::TimeElapse => st.timer_remaining == Some(0),
        MapEventKind::DestroyedByAnybody | MapEventKind::DestroyedByAnything => {
            let bound_tags = tags_for_trigger(tags, &st.id);
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
            let bound_tags = tags_for_trigger(tags, &st.id);
            if bound_tags.is_empty() {
                return false;
            }
            cell_entered_by_house(world, cell_tags, &bound_tags, local_house)
        }
        MapEventKind::CreditsExceed => {
            let Some(threshold) = event_numeric_param(c)
            else {
                return false;
            };
            let house = trigger_owner_house(world, &st.id).unwrap_or_else(|| local_house.to_string());
            house_funds(world, &house) >= threshold as i32
        }
        MapEventKind::CreditsBelow => {
            let Some(threshold) = event_numeric_param(c)
            else {
                return false;
            };
            let house = trigger_owner_house(world, &st.id).unwrap_or_else(|| local_house.to_string());
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
        if !world.ecs_get::<Owner>(id).map(|o| o.house.eq_ignore_ascii_case(house)).unwrap_or(false) {
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

fn tags_for_trigger(tags: &[ra_map::MapTag], trigger_id: &str) -> HashSet<String> {
    tags.iter().filter(|t| t.trigger_id.eq_ignore_ascii_case(trigger_id)).map(|t| t.id.to_string()).collect()
}

/// 统计地图中含 `Allow Win` 动作的触发条数（每条贡献一层胜利阻塞）。
fn count_allow_win_actions(scripting: &MapScripting) -> u32 {
    scripting.actions.iter().filter(|a| a.commands.iter().any(|c| c.kind == MapActionKind::AllowWin)).count() as u32
}

/// 查找触发器所属 house（`[Triggers]` 行首字段）。
fn trigger_owner_house(world: &BattleState, trigger_id: &str) -> Option<String> {
    world
        .map
        .scripting
        .triggers
        .iter()
        .find(|t| t.id.eq_ignore_ascii_case(trigger_id))
        .map(|t| t.house.as_str().to_string())
        .filter(|h| !h.is_empty())
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

fn cell_entered_by_house(world: &BattleState, cell_tags: &[ra_map::MapCellTag], bound_tags: &HashSet<String>, house: &str) -> bool {
    let cells: Vec<(u16, u16)> = cell_tags.iter().filter(|c| bound_tags.contains(c.tag_id.as_str())).map(|c| (c.x, c.y)).collect();
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
        MapActionKind::Unknown(_) => {
            world.trigger_runtime.record_unsupported(cmd.kind);
        }
        MapActionKind::Win => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
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
        MapActionKind::AllowWin => {
            world.trigger_runtime.win_blockers = world.trigger_runtime.win_blockers.saturating_sub(1);
            if world.trigger_runtime.win_blockers == 0 {
                if let Some(house) = world.trigger_runtime.deferred_victory_house.take() {
                    world.trigger_runtime.pending_outcome = Some(BattleOutcome::Victory { owner: house });
                }
            }
        }
        MapActionKind::CreateTeam | MapActionKind::Reinforcement | MapActionKind::ReinforcementAtWaypoint => {
            if let Some(team) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty()) {
                world.trigger_runtime.pending_team_spawns.push(team.to_string());
            }
            else if let Some(team) = cmd.params.first().map(|s| s.trim()).filter(|s| !s.is_empty() && s.parse::<i32>().is_err()) {
                world.trigger_runtime.pending_team_spawns.push(team.to_string());
            }
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        MapActionKind::DestroyTeam => {
            if let Some(team) = action_team_id_param(cmd) {
                super::script_teams::destroy_team_type(world, &team);
            }
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        MapActionKind::DestroyAttachedObjects => {
            destroy_attached_objects(world, trigger_id);
        }
        MapActionKind::DestroyTag => {
            let Some(tag_id) = action_tag_id_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            destroy_entities_with_tag(world, &tag_id);
        }
        MapActionKind::AllToHunt => {
            let house = action_house_param(cmd).unwrap_or_else(|| local_house.to_string());
            all_house_units_hunt(world, &house);
        }
        MapActionKind::DestroyTrigger => {
            if let Some(id) = action_trigger_id_param(cmd) {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&id)) {
                    st.disabled = true;
                    st.fired = true;
                }
            }
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        MapActionKind::ChangeHouse => {
            let Some(new_house) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            change_attached_objects_house(world, trigger_id, &new_house);
        }
        MapActionKind::MakeAlly => {
            let Some(other) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            let owner = trigger_owner_house(world, trigger_id).unwrap_or_else(|| local_house.to_string());
            set_houses_allied(world, &owner, &other, true);
        }
        MapActionKind::MakeEnemy => {
            let Some(other) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            let owner = trigger_owner_house(world, trigger_id).unwrap_or_else(|| local_house.to_string());
            set_houses_allied(world, &owner, &other, false);
        }
        MapActionKind::AllChangeHouse => {
            let Some(new_house) = action_house_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
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
            if let Some(id) = action_trigger_id_param(cmd) {
                force_fire_trigger(world, &id, local_house);
            }
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        MapActionKind::TimerStart => {
            let target = action_trigger_id_param(cmd).unwrap_or_else(|| trigger_id.to_string());
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&target)) {
                st.timer_paused = false;
            }
        }
        MapActionKind::TimerStop => {
            let target = action_trigger_id_param(cmd).unwrap_or_else(|| trigger_id.to_string());
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&target)) {
                st.timer_paused = true;
            }
        }
        MapActionKind::TimerExtend => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            let target = action_trigger_id_param(cmd).unwrap_or_else(|| trigger_id.to_string());
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&target)) {
                let cur = st.timer_remaining.unwrap_or(0);
                st.timer_remaining = Some(cur.saturating_add(ticks));
                st.fired = false;
                st.disabled = false;
            }
        }
        MapActionKind::TimerShorten => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            let target = action_trigger_id_param(cmd).unwrap_or_else(|| trigger_id.to_string());
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&target)) {
                let cur = st.timer_remaining.unwrap_or(0);
                st.timer_remaining = Some(cur.saturating_sub(ticks));
                st.fired = false;
                st.disabled = false;
            }
        }
        MapActionKind::TimerSet => {
            let Some(ticks) = action_timer_ticks_param(cmd)
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
                return;
            };
            // 目标触发：参数中的 Trigger id；缺省则作用于本触发。
            let target = action_trigger_id_param(cmd).unwrap_or_else(|| trigger_id.to_string());
            if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&target)) {
                st.timer_remaining = Some(ticks);
                st.timer_paused = false;
                st.fired = false;
                st.disabled = false;
            }
        }
        MapActionKind::EnableTrigger => {
            if let Some(id) = action_trigger_id_param(cmd) {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&id)) {
                    st.disabled = false;
                }
            }
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
            }
        }
        MapActionKind::DisableTrigger => {
            if let Some(id) = action_trigger_id_param(cmd) {
                if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&id)) {
                    st.disabled = true;
                }
            }
            else {
                world.trigger_runtime.record_unsupported(cmd.kind);
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
        | MapActionKind::GrowShroud
        | MapActionKind::ReshroudMap
        | MapActionKind::PlaySoundEffect
        | MapActionKind::ReshroudMapAt
        | MapActionKind::TimerText => {}
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
            world.ecs_get::<Identity>(id).map(|identity| !identity.tag.is_empty() && bound.contains(&identity.tag)).unwrap_or(false)
        })
        .collect();
    for id in ids {
        let _ = world.with_owner_mut(id, |owner| {
            owner.house = std::sync::Arc::<str>::from(new_house);
        });
    }
}

/// 将 `from_house` 名下全部存活实体改属 `new_house`。
fn change_all_house_entities(world: &mut BattleState, from_house: &str, new_house: &str) {
    if from_house.eq_ignore_ascii_case(new_house) {
        return;
    }
    world.ensure_house(new_house);
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            world.ecs_get::<Owner>(id).map(|o| o.house.eq_ignore_ascii_case(from_house)).unwrap_or(false)
        })
        .collect();
    for id in ids {
        let _ = world.with_owner_mut(id, |owner| {
            owner.house = std::sync::Arc::<str>::from(new_house);
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
            if !world.ecs_get::<Owner>(id).map(|o| o.house.eq_ignore_ascii_case(house)).unwrap_or(false) {
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
            world.ecs_get::<Identity>(id).map(|identity| !identity.tag.is_empty() && bound.contains(&identity.tag)).unwrap_or(false)
        })
        .collect();
    for id in ids {
        let max = world.ecs_health(id).map(|(_, m, _)| m).unwrap_or(1).max(1);
        let _ = world.set_ecs_health(id, 0, max, true);
    }
}

/// 摧毁带有指定 Tag id 的全部存活实体。
fn destroy_entities_with_tag(world: &mut BattleState, tag_id: &str) {
    let ids: Vec<_> = world
        .entities
        .iter()
        .map(|e| e.id)
        .filter(|&id| {
            if world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            world.ecs_get::<Identity>(id).map(|identity| identity.tag.eq_ignore_ascii_case(tag_id)).unwrap_or(false)
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
    let already = world.trigger_runtime.states.iter().find(|s| s.id.eq_ignore_ascii_case(id)).map(|s| s.fired).unwrap_or(true);
    if already {
        return;
    }
    if let Some(st) = world.trigger_runtime.states.iter_mut().find(|s| s.id.eq_ignore_ascii_case(id)) {
        st.disabled = false;
        st.fired = true;
    }
    let commands = world.map.scripting.actions.iter().find(|a| a.id.eq_ignore_ascii_case(id)).map(|a| a.commands.clone()).unwrap_or_default();
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

/// Create Team / Destroy Team / Reinforcement：TeamType id（通常在 `params[1]`）。
fn action_team_id_param(cmd: &MapActionCommand) -> Option<String> {
    if let Some(id) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty() && s.parse::<i32>().is_err()) {
        return Some(id.to_string());
    }
    cmd.params.first().map(|s| s.trim()).filter(|s| !s.is_empty() && s.parse::<i32>().is_err()).map(|s| s.to_string())
}

/// Destroy Tag：Tag id（通常在 `params[1]`）。
fn action_tag_id_param(cmd: &MapActionCommand) -> Option<String> {
    action_team_id_param(cmd)
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
