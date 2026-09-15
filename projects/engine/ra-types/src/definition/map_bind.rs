//! 地图预放实体 → [`PreparedPlacement`] 规则绑定。

use std::collections::HashMap;

use crate::{
    AiTriggerConditionKind, AiTriggerId, HouseId, HouseName, MapAction, MapActionCommand, MapAiTrigger, MapBasePlan, MapCellTag, MapEvent,
    MapHouse, MapPlacedEntity, MapPlacedEntityKind, MapScriptType, MapTag, MapTaskForce, MapTeamType, MapTrigger, MissionKind, MissionName,
    PreparedAction, PreparedActionCommand, PreparedAiTrigger, PreparedBaseNode, PreparedBasePlan, PreparedCellTag, PreparedEvent,
    PreparedHouse, PreparedMap, PreparedPlacement, PreparedScriptType, PreparedTag, PreparedTaskForce, PreparedTaskForceEntry,
    PreparedTeamType, PreparedTrigger, RaError, RaResult, RuntimeDefinitions, ScriptTypeId, ScriptTypeName, StructureDefinitions,
    SuperWeaponName, TagId, TagName, TaskForceId, TaskForceName, TeamTypeId, TechnoName, TriggerId, TriggerName, TypeId, occupancy_kind,
};

/// 将 `[Houses]` 投影为稳定 [`PreparedHouse`] 表。
///
/// - 空 / 未知 `Country=` → [`RaError::UnknownReference`]
/// - `Allies=` 中空 / `NONE` / `<none>` 项跳过；非空未知 → [`RaError::UnknownReference`]
pub fn bind_map_houses(houses: &[MapHouse], defs: &RuntimeDefinitions) -> RaResult<Vec<PreparedHouse>> {
    let mut out = Vec::with_capacity(houses.len());
    for house in houses {
        let country = bind_house_id(defs, &house.country, &format!("MapHouse:{}", house.name))?;
        let mut allies = Vec::with_capacity(house.allies.len());
        for ally in &house.allies {
            if ally.is_none_sentinel() {
                continue;
            }
            allies.push(bind_house_id(defs, ally, &format!("MapHouse.allies:{}", house.name))?);
        }
        out.push(PreparedHouse {
            name: house.name.clone(),
            country,
            tech_level: house.tech_level,
            credits: house.credits,
            iq: house.iq,
            edge: house.edge,
            player_control: house.player_control,
            color: house.color.clone(),
            allies,
        });
    }
    Ok(out)
}

/// 将 `[Triggers]` 投影为稳定 [`PreparedTrigger`] 表；未知 `linked` / `house` 引用拒绝。
///
/// - 空 / `<none>` / `NONE` 的 `linked` → [`None`]
/// - 非空但找不到目标 → [`RaError::UnknownReference`]
/// - 空 / `NONE` / `<none>` 的 `house` → ambient `NEUTRAL`
/// - `ALL` / `<all>` 的 `house` → ambient `NEUTRAL`（触发所属房主通配按中立槽承载）
/// - 其它未知 `house` → [`RaError::UnknownReference`]
pub fn bind_map_triggers(triggers: &[MapTrigger], defs: &RuntimeDefinitions) -> RaResult<Vec<PreparedTrigger>> {
    let mut out = Vec::with_capacity(triggers.len());
    let mut next = 1u32;
    let mut by_name: HashMap<&str, TriggerId> = HashMap::new();
    for trigger in triggers {
        if trigger.id.is_empty() {
            continue;
        }
        let id = TriggerId(next);
        next = next.saturating_add(1);
        by_name.insert(trigger.id.as_str(), id);
        let house_name = if trigger.house.is_unrestricted_sentinel() { HouseName::parse("NEUTRAL") } else { trigger.house.clone() };
        let house = bind_house_id(defs, &house_name, &format!("MapTrigger:{}", trigger.id.as_str()))?;
        out.push(PreparedTrigger {
            id,
            name: trigger.id.clone(),
            house,
            linked: None,
            editor_name: trigger.name.clone(),
            disabled: trigger.disabled,
            easy: trigger.easy,
            normal: trigger.normal,
            hard: trigger.hard,
        });
    }
    for (i, trigger) in triggers.iter().filter(|t| !t.id.is_empty()).enumerate() {
        out[i].linked = bind_linked_trigger_id(&by_name, &trigger.linked, trigger.id.as_str())?;
    }
    Ok(out)
}

/// 将 `[Tags]` 投影为稳定 [`PreparedTag`] 表；关联 Trigger 必须可解析。
///
/// - 空 / 未知 `trigger_id` → [`RaError::UnknownReference`]
pub fn bind_map_tags(tags: &[MapTag], triggers: &[PreparedTrigger]) -> RaResult<Vec<PreparedTag>> {
    let trigger_by_name: HashMap<&str, TriggerId> = triggers.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(tags.len());
    let mut next = 1u32;
    for tag in tags {
        if tag.id.is_empty() {
            continue;
        }
        let trigger_id = bind_trigger_id(&trigger_by_name, &tag.trigger_id, tag.id.as_str())?;
        let id = TagId(next);
        next = next.saturating_add(1);
        out.push(PreparedTag { id, name: tag.id.clone(), persistence: tag.persistence, editor_name: tag.name.clone(), trigger_id });
    }
    Ok(out)
}

/// 将 [`MapPlacedEntity`] 列表绑定为稳定 id 的 [`PreparedPlacement`]。
///
/// - 未知 `type_id` → [`RaError::UnknownReference`]（techno）
/// - 未知 / 空 `owner` → [`RaError::UnknownReference`]（house）
/// - 非空且非 `NONE` 的未知 `tag` → [`RaError::UnknownReference`]（tag）
/// - 非空未知 `mission` → [`RaError::UnknownReference`]（mission）
///
/// 零售地图放置行常写 `None` 表示无 Tag；装载后为大写 `NONE`，绑定为 [`None`]。
pub fn bind_map_placements(entities: &[MapPlacedEntity], defs: &RuntimeDefinitions, tags: &[PreparedTag]) -> RaResult<Vec<PreparedPlacement>> {
    let tag_by_name: HashMap<&str, TagId> = tags.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(entities.len());
    for entity in entities {
        let definition_id = bind_techno_id(defs, &entity.type_id, "MapPlacement")?;
        let owner = bind_house_id(defs, &entity.owner, "MapPlacement")?;
        let tag = bind_tag_id(&tag_by_name, &entity.tag, "MapPlacement")?;
        out.push(PreparedPlacement {
            kind: entity.kind,
            owner,
            definition_id,
            health: entity.health,
            x: entity.x,
            y: entity.y,
            facing: entity.facing,
            sub_cell: entity.sub_cell,
            mission: bind_mission_kind(&entity.mission, "MapPlacement")?,
            tag,
        });
    }
    Ok(out)
}

/// 将 `[CellTags]` 绑定为稳定 [`PreparedCellTag`]；空 / 未知 tag → [`RaError::UnknownReference`]。
pub fn bind_map_cell_tags(cell_tags: &[MapCellTag], tags: &[PreparedTag]) -> RaResult<Vec<PreparedCellTag>> {
    let tag_by_name: HashMap<&str, TagId> = tags.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(cell_tags.len());
    for cell in cell_tags {
        let Some(tag) = bind_tag_id(&tag_by_name, &cell.tag_id, "MapCellTag")?
        else {
            return Err(RaError::UnknownReference { kind: "tag", name: String::new(), owner: "MapCellTag".to_string() });
        };
        out.push(PreparedCellTag { x: cell.x, y: cell.y, tag });
    }
    Ok(out)
}

/// 将 `[Events]` 投影为稳定 [`PreparedEvent`] 表；未知 trigger id 拒绝。
pub fn bind_map_events(events: &[MapEvent], triggers: &[PreparedTrigger]) -> RaResult<Vec<PreparedEvent>> {
    let trigger_by_name: HashMap<&str, TriggerId> = triggers.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(events.len());
    for event in events {
        if event.id.is_empty() {
            continue;
        }
        let trigger_id = bind_trigger_id(&trigger_by_name, &event.id, "MapEvent")?;
        out.push(PreparedEvent { trigger_id, conditions: event.conditions.clone() });
    }
    Ok(out)
}

/// 将 `[Actions]` 投影为稳定 [`PreparedAction`] 表；未知 trigger / team / tag / house 引用拒绝。
///
/// - 动作所属 trigger id 必须可解析
/// - CreateTeam / DestroyTeam / Reinforcement* / FlashTeam：非空 team 名必须在 `teams` 中
/// - Destroy / Force / Enable / Disable / Timer*：非空目标 trigger 名必须可解析
/// - DestroyTag：非空 tag 名必须在 `tags` 中
/// - Win / ProductionBegins / AllToHunt / ChangeHouse / MakeAlly* / DestroyAll*：非空 house 名必须可解析
pub fn bind_map_actions(
    actions: &[MapAction],
    triggers: &[PreparedTrigger],
    teams: &[PreparedTeamType],
    tags: &[PreparedTag],
    defs: &RuntimeDefinitions,
) -> RaResult<Vec<PreparedAction>> {
    let trigger_by_name: HashMap<&str, TriggerId> = triggers.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let team_by_name: HashMap<&str, TeamTypeId> = teams.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let tag_by_name: HashMap<&str, TagId> = tags.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(actions.len());
    for action in actions {
        if action.id.is_empty() {
            continue;
        }
        let trigger_id = bind_trigger_id(&trigger_by_name, &action.id, "MapAction")?;
        let mut commands = Vec::with_capacity(action.commands.len());
        for cmd in &action.commands {
            commands.push(bind_action_command(cmd, &trigger_by_name, &team_by_name, &tag_by_name, defs, action.id.as_str())?);
        }
        out.push(PreparedAction { trigger_id, commands });
    }
    Ok(out)
}

/// 原版动作码：Create Team。
const ACTION_CREATE_TEAM: i32 = 4;
/// Destroy Team。
const ACTION_DESTROY_TEAM: i32 = 5;
/// Reinforcement。
const ACTION_REINFORCEMENT: i32 = 7;
/// Destroy Trigger。
const ACTION_DESTROY_TRIGGER: i32 = 12;
/// Force Trigger。
const ACTION_FORCE_TRIGGER: i32 = 22;
/// Timer Start。
const ACTION_TIMER_START: i32 = 23;
/// Timer Stop。
const ACTION_TIMER_STOP: i32 = 24;
/// Timer Extend。
const ACTION_TIMER_EXTEND: i32 = 25;
/// Timer Shorten。
const ACTION_TIMER_SHORTEN: i32 = 26;
/// Timer Set。
const ACTION_TIMER_SET: i32 = 27;
/// Enable Trigger。
const ACTION_ENABLE_TRIGGER: i32 = 53;
/// Disable Trigger。
const ACTION_DISABLE_TRIGGER: i32 = 54;
/// Destroy Tag。
const ACTION_DESTROY_TAG: i32 = 70;
/// Reinforcement At Waypoint。
const ACTION_REINFORCEMENT_AT_WAYPOINT: i32 = 80;
/// Flash Team。
const ACTION_FLASH_TEAM: i32 = 104;
/// Win。
const ACTION_WIN: i32 = 1;
/// Production Begins。
const ACTION_PRODUCTION_BEGINS: i32 = 3;
/// All To Hunt。
const ACTION_ALL_TO_HUNT: i32 = 6;
/// Change House。
const ACTION_CHANGE_HOUSE: i32 = 14;
/// All Change House。
const ACTION_ALL_CHANGE_HOUSE: i32 = 36;
/// Make Ally。
const ACTION_MAKE_ALLY: i32 = 37;
/// Make Enemy。
const ACTION_MAKE_ENEMY: i32 = 38;
/// Destroy All Of。
const ACTION_DESTROY_ALL_OF: i32 = 119;
/// Destroy All Buildings Of。
const ACTION_DESTROY_ALL_BUILDINGS_OF: i32 = 120;
/// Destroy All Land Units Of。
const ACTION_DESTROY_ALL_LAND_UNITS_OF: i32 = 121;

fn bind_action_command(
    cmd: &MapActionCommand,
    trigger_by_name: &HashMap<&str, TriggerId>,
    team_by_name: &HashMap<&str, TeamTypeId>,
    tag_by_name: &HashMap<&str, TagId>,
    defs: &RuntimeDefinitions,
    owner: &str,
) -> RaResult<PreparedActionCommand> {
    let name = action_ref_name_param(cmd);
    let (team_id, target_trigger_id, tag_id, house_id) = match cmd.kind_code {
        ACTION_CREATE_TEAM | ACTION_DESTROY_TEAM | ACTION_REINFORCEMENT | ACTION_REINFORCEMENT_AT_WAYPOINT | ACTION_FLASH_TEAM => {
            let team_id = match name {
                Some(n) => Some(team_by_name.get(n.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
                    kind: "team_type",
                    name: n,
                    owner: format!("MapAction:{owner}"),
                })?),
                None => None,
            };
            (team_id, None, None, None)
        }
        ACTION_DESTROY_TRIGGER
        | ACTION_FORCE_TRIGGER
        | ACTION_TIMER_START
        | ACTION_TIMER_STOP
        | ACTION_TIMER_EXTEND
        | ACTION_TIMER_SHORTEN
        | ACTION_TIMER_SET
        | ACTION_ENABLE_TRIGGER
        | ACTION_DISABLE_TRIGGER => {
            let target_trigger_id = match name {
                Some(n) => Some(*trigger_by_name.get(n.as_str()).ok_or_else(|| RaError::UnknownReference {
                    kind: "trigger",
                    name: n,
                    owner: format!("MapAction:{owner}"),
                })?),
                None => None,
            };
            (None, target_trigger_id, None, None)
        }
        ACTION_DESTROY_TAG => {
            let tag_id = match name {
                Some(n) => Some(tag_by_name.get(n.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
                    kind: "tag",
                    name: n,
                    owner: format!("MapAction:{owner}"),
                })?),
                None => None,
            };
            (None, None, tag_id, None)
        }
        ACTION_WIN
        | ACTION_PRODUCTION_BEGINS
        | ACTION_ALL_TO_HUNT
        | ACTION_CHANGE_HOUSE
        | ACTION_ALL_CHANGE_HOUSE
        | ACTION_MAKE_ALLY
        | ACTION_MAKE_ENEMY
        | ACTION_DESTROY_ALL_OF
        | ACTION_DESTROY_ALL_BUILDINGS_OF
        | ACTION_DESTROY_ALL_LAND_UNITS_OF => {
            let house_id = match action_house_name_param(cmd) {
                Some(n) => Some(bind_house_id(defs, &n, &format!("MapAction:{owner}"))?),
                None => None,
            };
            (None, None, None, house_id)
        }
        _ => (None, None, None, None),
    };
    Ok(PreparedActionCommand { kind_code: cmd.kind_code, params: cmd.params.clone(), team_id, target_trigger_id, tag_id, house_id })
}

/// 与运行时一致：优先 `params[1]`，否则第一个非空且非纯数字槽。
///
/// 返回装载期大写键；空 / `NONE` / `<none>` 视为无引用。
fn action_ref_name_param(cmd: &MapActionCommand) -> Option<String> {
    let raw = if let Some(id) = cmd.params.get(1).map(|s| s.trim()).filter(|s| !s.is_empty() && s.parse::<i32>().is_err()) {
        id
    }
    else {
        cmd.params.iter().map(|s| s.trim()).find(|s| !s.is_empty() && s.parse::<i32>().is_err())?
    };
    if is_absent_name_sentinel(raw) {
        return None;
    }
    Some(raw.to_ascii_uppercase())
}

/// 与运行时 `action_house_param` 一致：自末槽向前找非空非纯数字 house 名。
fn action_house_name_param(cmd: &MapActionCommand) -> Option<HouseName> {
    for p in cmd.params.iter().rev() {
        let t = p.trim();
        if t.is_empty() || t == "0" {
            continue;
        }
        if t.parse::<i32>().is_ok() {
            continue;
        }
        if t.eq_ignore_ascii_case("NONE")
            || t.eq_ignore_ascii_case("<NONE>")
            || t.eq_ignore_ascii_case("ALL")
            || t.eq_ignore_ascii_case("<ALL>")
        {
            return None;
        }
        return Some(HouseName::parse(t));
    }
    None
}

/// 将 `[TaskForces]` 投影为稳定 [`PreparedTaskForce`]；未知 techno 成员拒绝。
pub fn bind_map_task_forces(forces: &[MapTaskForce], defs: &RuntimeDefinitions) -> RaResult<Vec<PreparedTaskForce>> {
    let mut out = Vec::with_capacity(forces.len());
    let mut next = 1u32;
    for force in forces {
        if force.id.is_empty() {
            continue;
        }
        let mut entries = Vec::with_capacity(force.entries.len());
        for entry in &force.entries {
            let definition_id = bind_techno_id(defs, &entry.type_id, &format!("MapTaskForce:{}", force.id.as_str()))?;
            entries.push(PreparedTaskForceEntry { count: entry.count, definition_id });
        }
        let id = TaskForceId(next);
        next = next.saturating_add(1);
        out.push(PreparedTaskForce { id, name: force.id.clone(), editor_name: force.name.clone(), entries, group: force.group });
    }
    Ok(out)
}

/// 将 `[ScriptTypes]` 投影为稳定 [`PreparedScriptType`]。
pub fn bind_map_script_types(scripts: &[MapScriptType]) -> RaResult<Vec<PreparedScriptType>> {
    let mut out = Vec::with_capacity(scripts.len());
    let mut next = 1u32;
    for script in scripts {
        if script.id.is_empty() {
            continue;
        }
        let id = ScriptTypeId(next);
        next = next.saturating_add(1);
        out.push(PreparedScriptType { id, name: script.id.clone(), editor_name: script.name.clone(), steps: script.steps.clone() });
    }
    Ok(out)
}

/// 将 `[TeamTypes]` 投影为稳定 [`PreparedTeamType`]；未知 house / script / task_force / tag 拒绝。
///
/// - 空 / `NONE` / `<none>` 的 `House=` → ambient `NEUTRAL`
/// - `ALL` / `<all>` 的 `House=` → [`None`]（产队时由 AI / 动作上下文覆盖）
pub fn bind_map_team_types(
    teams: &[MapTeamType],
    defs: &RuntimeDefinitions,
    scripts: &[PreparedScriptType],
    forces: &[PreparedTaskForce],
    tags: &[PreparedTag],
) -> RaResult<Vec<PreparedTeamType>> {
    let script_by_name: HashMap<&str, ScriptTypeId> = scripts.iter().map(|s| (s.name.as_str(), s.id)).collect();
    let force_by_name: HashMap<&str, TaskForceId> = forces.iter().map(|f| (f.name.as_str(), f.id)).collect();
    let tag_by_name: HashMap<&str, TagId> = tags.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(teams.len());
    let mut next = 1u32;
    for team in teams {
        if team.id.is_empty() {
            continue;
        }
        let house = if team.house.is_all_sentinel() {
            None
        }
        else {
            let house_name = if team.house.is_none_sentinel() { HouseName::parse("NEUTRAL") } else { team.house.clone() };
            Some(bind_house_id(defs, &house_name, &format!("MapTeamType:{}", team.id.as_str()))?)
        };
        let script = bind_optional_script_id(&script_by_name, &team.script, team.id.as_str())?;
        let task_force = bind_task_force_id(&force_by_name, &team.task_force, team.id.as_str())?;
        let tag = bind_tag_id(&tag_by_name, &team.tag, &format!("MapTeamType:{}", team.id.as_str()))?;
        let id = TeamTypeId(next);
        next = next.saturating_add(1);
        out.push(PreparedTeamType {
            id,
            name: team.id.clone(),
            editor_name: team.name.clone(),
            house,
            script,
            task_force,
            tag,
            waypoint: team.waypoint,
            max: team.max,
            priority: team.priority,
            veteran_level: team.veteran_level,
            autocreate: team.autocreate,
        });
    }
    Ok(out)
}

/// 将 `[AITriggerTypes]` 投影为稳定 [`PreparedAiTrigger`]；未知 team / house 拒绝。
///
/// - 空 / `NONE` / `<none>` / `ALL` / `<all>` 的 `OwnerHouse=` → [`None`]（未限定房主）
/// - 空 / `NONE` / `<none>` 的 `Team2=` → [`None`]（无第二队）
/// - 主 `Team=` 为空或 none 哨兵 → 跳过该条
pub fn bind_map_ai_triggers(
    triggers: &[MapAiTrigger],
    defs: &RuntimeDefinitions,
    teams: &[PreparedTeamType],
) -> RaResult<Vec<PreparedAiTrigger>> {
    let team_by_name: HashMap<&str, TeamTypeId> = teams.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(triggers.len());
    let mut next = 1u32;
    for trigger in triggers {
        if trigger.id.is_empty() {
            continue;
        }
        if is_absent_name_sentinel(trigger.team.as_str()) {
            continue;
        }
        let team = team_by_name.get(trigger.team.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
            kind: "team_type",
            name: trigger.team.as_str().to_string(),
            owner: format!("MapAiTrigger:{}", trigger.id.as_str()),
        })?;
        let team2 = if is_absent_name_sentinel(trigger.team2.as_str()) {
            None
        }
        else {
            Some(team_by_name.get(trigger.team2.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
                kind: "team_type",
                name: trigger.team2.as_str().to_string(),
                owner: format!("MapAiTrigger:{}:team2", trigger.id.as_str()),
            })?)
        };
        let owner_house = if trigger.owner_house.is_unrestricted_sentinel() {
            None
        }
        else {
            Some(bind_house_id(defs, &trigger.owner_house, &format!("MapAiTrigger:{}", trigger.id.as_str()))?)
        };
        let condition_object_id = bind_ai_trigger_condition_object(defs, trigger)?;
        let id = AiTriggerId(next);
        next = next.saturating_add(1);
        out.push(PreparedAiTrigger {
            id,
            name: trigger.id.clone(),
            editor_name: trigger.name.clone(),
            team,
            owner_house,
            tech_level: trigger.tech_level,
            condition: trigger.condition,
            condition_object_id,
            compare_amount: trigger.compare_amount,
            compare_op: trigger.compare_op,
            for_skirmish: trigger.for_skirmish,
            enabled_easy: trigger.enabled_easy,
            enabled_normal: trigger.enabled_normal,
            enabled_hard: trigger.enabled_hard,
            weight: trigger.weight.max(1),
            min_weight: {
                let start = trigger.weight.max(1);
                trigger.min_weight.max(1).min(start)
            },
            max_weight: {
                let start = trigger.weight.max(1);
                let min_w = trigger.min_weight.max(1).min(start);
                trigger.max_weight.max(start).max(min_w)
            },
            team2,
        });
    }
    Ok(out)
}

/// 将地图 `[Base]` 软绑定为 [`PreparedBasePlan`]。
///
/// - 缺节 / 空 `Player=` / 未知房屋 → [`None`]（不拒图）
/// - 未知建筑类型节点跳过；全部跳过后若无节点 → [`None`]
pub fn bind_map_base(plan: Option<&MapBasePlan>, defs: &RuntimeDefinitions) -> RaResult<Option<PreparedBasePlan>> {
    let Some(plan) = plan
    else {
        return Ok(None);
    };
    if plan.player.is_empty() || plan.player.is_none_sentinel() {
        return Ok(None);
    }
    let Some(house_def) = defs.houses.get_name(&plan.player)
    else {
        return Ok(None);
    };
    let mut nodes = Vec::with_capacity(plan.nodes.len());
    for node in &plan.nodes {
        if node.type_name.is_empty() {
            continue;
        }
        let Some(structure) = defs.structures.get_name(&node.type_name)
        else {
            continue;
        };
        nodes.push(PreparedBaseNode { type_id: structure.id, x: node.x, y: node.y });
    }
    if nodes.is_empty() {
        return Ok(None);
    }
    Ok(Some(PreparedBasePlan { house: house_def.id, nodes }))
}

/// 就地填充 [`PreparedMap`] 绑定表；失败时不改动已有字段。
///
/// 绑定成功后先 [`validate_placement_geometry`]（越界 / 结构足迹重叠），再按
/// [`PreparedPlacement`] + 建筑表 `Foundation=` 重写 occupancy / 结构通行封格。
pub fn bind_prepared_map_placements(prepared: &mut PreparedMap, defs: &RuntimeDefinitions) -> RaResult<()> {
    let houses = bind_map_houses(&prepared.definition.houses, defs)?;
    let triggers = bind_map_triggers(&prepared.definition.triggers, defs)?;
    let events = bind_map_events(&prepared.definition.events, &triggers)?;
    let tags = bind_map_tags(&prepared.definition.tags, &triggers)?;
    let cell_tags = bind_map_cell_tags(&prepared.definition.cell_tags, &tags)?;
    let placements = bind_map_placements(&prepared.definition.entities, defs, &tags)?;
    let task_forces = bind_map_task_forces(&prepared.definition.task_forces, defs)?;
    let script_types = bind_map_script_types(&prepared.definition.script_types)?;
    let team_types = bind_map_team_types(&prepared.definition.team_types, defs, &script_types, &task_forces, &tags)?;
    let actions = bind_map_actions(&prepared.definition.actions, &triggers, &team_types, &tags, defs)?;
    let ai_triggers = bind_map_ai_triggers(&prepared.definition.ai_triggers, defs, &team_types)?;
    let base_plan = bind_map_base(prepared.definition.base.as_ref(), defs)?;
    prepared.houses = houses;
    prepared.triggers = triggers;
    prepared.events = events;
    prepared.actions = actions;
    prepared.tags = tags;
    prepared.cell_tags = cell_tags;
    prepared.placements = placements;
    prepared.task_forces = task_forces;
    prepared.script_types = script_types;
    prepared.team_types = team_types;
    prepared.ai_triggers = ai_triggers;
    prepared.base_plan = base_plan;
    validate_placement_geometry(prepared, &defs.structures)?;
    reseal_prepared_layers_from_placements(prepared, &defs.structures);
    Ok(())
}

/// 校验已绑定放置的几何合法性。
///
/// - 任意放置锚点越出地图 → [`RaError::Msg`]
/// - 结构 `Foundation=` 足迹越出地图 → [`RaError::Msg`]
/// - 两座结构足迹重叠 → [`RaError::Msg`]
///
/// 机动单位（步兵 / 载具 / 飞行器）只校验锚点；结构按 Foundation 展开校验。
pub fn validate_placement_geometry(prepared: &PreparedMap, structures: &StructureDefinitions) -> RaResult<()> {
    let width = prepared.pass_width.max(1);
    let height = prepared.pass_height.max(1);
    let mut structure_cells: HashMap<(u16, u16), TypeId> = HashMap::new();

    for placement in &prepared.placements {
        if u32::from(placement.x) >= width || u32::from(placement.y) >= height {
            return Err(RaError::Msg(format!(
                "地图放置越界: kind={:?} type={:?} at ({},{}) map={}x{}",
                placement.kind, placement.definition_id, placement.x, placement.y, width, height
            )));
        }
        if placement.kind != MapPlacedEntityKind::Structure {
            continue;
        }
        let (fw, fh) = structures
            .get_by_id(placement.definition_id)
            .map(|def| (def.foundation.width.max(1), def.foundation.height.max(1)))
            .unwrap_or((1, 1));
        for dy in 0..fh {
            for dx in 0..fw {
                let x = placement.x.saturating_add(dx);
                let y = placement.y.saturating_add(dy);
                if u32::from(x) >= width || u32::from(y) >= height {
                    return Err(RaError::Msg(format!(
                        "地图结构 Foundation 越界: type={:?} anchor=({},{}) foundation={}x{} cell=({},{}) map={}x{}",
                        placement.definition_id, placement.x, placement.y, fw, fh, x, y, width, height
                    )));
                }
                if let Some(other) = structure_cells.insert((x, y), placement.definition_id) {
                    return Err(RaError::Msg(format!(
                        "地图结构足迹重叠: cell=({},{}) type={:?} overlaps {:?}",
                        x, y, placement.definition_id, other
                    )));
                }
            }
        }
    }
    Ok(())
}

/// 用已绑定的 [`PreparedPlacement`] 与建筑表 `Foundation=` 重写粗占格，并重封结构 / 地形通行格。
///
/// - 保留既有 [`PreparedMap::cell_heights`]
/// - 通行层先全开，再按建筑 placements + `terrain_objects` 封死（污迹只占 occupancy，与骨架一致）
/// - 不应用 overlay / TMP 陆地规则（仍由装载后序步骤处理）
/// - 未知建筑类型回退 `1x1`
pub fn reseal_prepared_layers_from_placements(prepared: &mut PreparedMap, structures: &StructureDefinitions) {
    if prepared.pass_width == 0 || prepared.pass_height == 0 {
        prepared.pass_width = prepared.definition.size_width.max(1);
        prepared.pass_height = prepared.definition.size_height.max(1);
    }
    let width = prepared.pass_width.max(1) as usize;
    let height = prepared.pass_height.max(1) as usize;
    let n = width.saturating_mul(height);
    if n == 0 {
        return;
    }

    let mut passable = vec![1u8; n];
    let mut occupancy = vec![occupancy_kind::EMPTY; n];
    let mark_occ = |occ: &mut [u8], x: u16, y: u16, kind: u8| {
        let xi = usize::from(x);
        let yi = usize::from(y);
        if xi >= width || yi >= height {
            return;
        }
        let i = yi * width + xi;
        if occ[i] == occupancy_kind::EMPTY || kind == occupancy_kind::STRUCTURE {
            occ[i] = kind;
        }
    };
    let seal = |pass: &mut [u8], x: u16, y: u16| {
        let xi = usize::from(x);
        let yi = usize::from(y);
        if xi >= width || yi >= height {
            return;
        }
        pass[yi * width + xi] = 0;
    };

    for placement in &prepared.placements {
        if placement.kind != MapPlacedEntityKind::Structure {
            continue;
        }
        let (fw, fh) = structures
            .get_by_id(placement.definition_id)
            .map(|def| (def.foundation.width.max(1), def.foundation.height.max(1)))
            .unwrap_or((1, 1));
        for dy in 0..fh {
            for dx in 0..fw {
                let x = placement.x.saturating_add(dx);
                let y = placement.y.saturating_add(dy);
                mark_occ(&mut occupancy, x, y, occupancy_kind::STRUCTURE);
                seal(&mut passable, x, y);
            }
        }
    }
    for obj in &prepared.definition.terrain_objects {
        mark_occ(&mut occupancy, obj.x, obj.y, occupancy_kind::TERRAIN);
        seal(&mut passable, obj.x, obj.y);
    }
    for smudge in &prepared.definition.smudges {
        mark_occ(&mut occupancy, smudge.x, smudge.y, occupancy_kind::SMUDGE);
    }

    if prepared.cell_heights.len() != n {
        prepared.cell_heights.resize(n, 0);
    }
    prepared.passable = passable;
    prepared.occupancy = occupancy;
}

fn bind_task_force_id(force_by_name: &HashMap<&str, TaskForceId>, name: &TaskForceName, owner: &str) -> RaResult<TaskForceId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference { kind: "task_force", name: String::new(), owner: owner.to_string() });
    }
    force_by_name.get(name.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
        kind: "task_force",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

/// 空 / `NONE` / `<none>`：零售 INI 常见「无引用」哨兵（team2 / script / tag / 条件对象等）。
fn is_absent_name_sentinel(raw: &str) -> bool {
    let t = raw.trim();
    t.is_empty() || t.eq_ignore_ascii_case("NONE") || t.eq_ignore_ascii_case("<NONE>")
}

fn bind_optional_script_id(script_by_name: &HashMap<&str, ScriptTypeId>, name: &ScriptTypeName, owner: &str) -> RaResult<Option<ScriptTypeId>> {
    if is_absent_name_sentinel(name.as_str()) {
        return Ok(None);
    }
    script_by_name.get(name.as_str()).copied().map(Some).ok_or_else(|| RaError::UnknownReference {
        kind: "script_type",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_techno_id(defs: &RuntimeDefinitions, name: &TechnoName, owner: &str) -> RaResult<TypeId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference { kind: "techno", name: String::new(), owner: owner.to_string() });
    }
    defs.techno.get_name(name).map(|t| t.id).ok_or_else(|| RaError::UnknownReference {
        kind: "techno",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_super_weapon_id(defs: &RuntimeDefinitions, name: &TechnoName, owner: &str) -> RaResult<TypeId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference { kind: "super_weapon", name: String::new(), owner: owner.to_string() });
    }
    let sw = SuperWeaponName::parse(name.as_str());
    defs.super_weapons.get_name(&sw).map(|d| d.id).ok_or_else(|| RaError::UnknownReference {
        kind: "super_weapon",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

/// 按条件种类绑定 `condition_object`；空名保持 `None`；无关条件忽略对象列。
fn bind_ai_trigger_condition_object(defs: &RuntimeDefinitions, trigger: &MapAiTrigger) -> RaResult<Option<TypeId>> {
    let owner = format!("MapAiTrigger:{}:condition_object", trigger.id.as_str());
    match trigger.condition {
        AiTriggerConditionKind::EnemyOwns | AiTriggerConditionKind::OwnOwns | AiTriggerConditionKind::NeutralOwns => {
            if is_absent_name_sentinel(trigger.condition_object.as_str()) {
                Ok(None)
            }
            else {
                bind_techno_id(defs, &trigger.condition_object, &owner).map(Some)
            }
        }
        AiTriggerConditionKind::OwnSuperWeaponCharge => {
            if is_absent_name_sentinel(trigger.condition_object.as_str()) {
                Ok(None)
            }
            else {
                bind_super_weapon_id(defs, &trigger.condition_object, &owner).map(Some)
            }
        }
        AiTriggerConditionKind::Always
        | AiTriggerConditionKind::EnemyYellowPower
        | AiTriggerConditionKind::EnemyRedPower
        | AiTriggerConditionKind::EnemyCredits
        | AiTriggerConditionKind::OwnCredits
        | AiTriggerConditionKind::Unsupported(_) => Ok(None),
    }
}

fn bind_house_id(defs: &RuntimeDefinitions, name: &HouseName, owner: &str) -> RaResult<HouseId> {
    if name.is_none_sentinel() {
        return Err(RaError::UnknownReference { kind: "house", name: String::new(), owner: owner.to_string() });
    }
    if name.is_all_sentinel() {
        return Err(RaError::UnknownReference { kind: "house", name: name.as_str().to_string(), owner: owner.to_string() });
    }
    defs.houses.get_name(name).map(|h| h.id).ok_or_else(|| RaError::UnknownReference {
        kind: "house",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_tag_id(tag_by_name: &HashMap<&str, TagId>, name: &TagName, owner: &str) -> RaResult<Option<TagId>> {
    // 空列与零售哨兵 `None` / `<none>`（装载期大写）均表示无 Tag。
    if is_absent_name_sentinel(name.as_str()) {
        return Ok(None);
    }
    tag_by_name.get(name.as_str()).copied().map(Some).ok_or_else(|| RaError::UnknownReference {
        kind: "tag",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_trigger_id(trigger_by_name: &HashMap<&str, TriggerId>, name: &TriggerName, owner: &str) -> RaResult<TriggerId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference { kind: "trigger", name: String::new(), owner: owner.to_string() });
    }
    trigger_by_name.get(name.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
        kind: "trigger",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_linked_trigger_id(trigger_by_name: &HashMap<&str, TriggerId>, name: &TriggerName, owner: &str) -> RaResult<Option<TriggerId>> {
    if is_absent_name_sentinel(name.as_str()) {
        return Ok(None);
    }
    trigger_by_name.get(name.as_str()).copied().map(Some).ok_or_else(|| RaError::UnknownReference {
        kind: "trigger",
        name: name.as_str().to_string(),
        owner: format!("linked:{owner}"),
    })
}

fn bind_mission_kind(name: &MissionName, owner: &str) -> RaResult<Option<MissionKind>> {
    if name.is_empty() {
        return Ok(None);
    }
    MissionKind::from_name(name).map(Some).ok_or_else(|| RaError::UnknownReference {
        kind: "mission",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}
