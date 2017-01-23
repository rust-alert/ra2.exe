//! 地图预放实体 → [`PreparedPlacement`] 规则绑定。

use std::collections::HashMap;

use crate::{
    HouseId, HouseName, MapAiTrigger, MapCellTag, MapPlacedEntity, MapScriptType, MapTag, MapTaskForce, MapTeamType, MissionKind, MissionName,
    PreparedAiTrigger, PreparedCellTag, PreparedMap, PreparedPlacement, PreparedScriptType, PreparedTag, PreparedTaskForce, PreparedTaskForceEntry,
    PreparedTeamType, RaError, RaResult, RuntimeDefinitions, ScriptTypeId, ScriptTypeName, TagId, TagName, TaskForceId, TaskForceName, TeamTypeName, TechnoName, TypeId,
};

/// 将 `[Tags]` 投影为稳定 [`PreparedTag`] 表。
pub fn bind_map_tags(tags: &[MapTag]) -> Vec<PreparedTag> {
    let mut out = Vec::with_capacity(tags.len());
    let mut next = 1u32;
    for tag in tags {
        if tag.id.is_empty() {
            continue;
        }
        let id = TagId(next);
        next = next.saturating_add(1);
        out.push(PreparedTag {
            id,
            name: tag.id.clone(),
            persistence: tag.persistence,
            editor_name: tag.name.clone(),
            trigger_id: tag.trigger_id.clone(),
        });
    }
    out
}

/// 将 [`MapPlacedEntity`] 列表绑定为稳定 id 的 [`PreparedPlacement`]。
///
/// - 未知 `type_id` → [`RaError::UnknownReference`]（techno）
/// - 未知 / 空 `owner` → [`RaError::UnknownReference`]（house）
/// - 非空未知 `tag` → [`RaError::UnknownReference`]（tag）
/// - 非空未知 `mission` → [`RaError::UnknownReference`]（mission）
pub fn bind_map_placements(
    entities: &[MapPlacedEntity],
    defs: &RuntimeDefinitions,
    tags: &[PreparedTag],
) -> RaResult<Vec<PreparedPlacement>> {
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
        let Some(tag) = bind_tag_id(&tag_by_name, &cell.tag_id, "MapCellTag")? else {
            return Err(RaError::UnknownReference {
                kind: "tag",
                name: String::new(),
                owner: "MapCellTag".to_string(),
            });
        };
        out.push(PreparedCellTag {
            x: cell.x,
            y: cell.y,
            tag,
        });
    }
    Ok(out)
}

/// 将 `[TaskForces]` 绑定为稳定 techno id 的 [`PreparedTaskForce`]。
///
/// - 未知成员 `type_id` → [`RaError::UnknownReference`]（techno）
pub fn bind_map_task_forces(task_forces: &[MapTaskForce], defs: &RuntimeDefinitions) -> RaResult<Vec<PreparedTaskForce>> {
    let mut out = Vec::with_capacity(task_forces.len());
    let mut next = 1u32;
    for force in task_forces {
        if force.id.is_empty() {
            continue;
        }
        let mut entries = Vec::with_capacity(force.entries.len());
        for entry in &force.entries {
            entries.push(PreparedTaskForceEntry {
                count: entry.count,
                definition_id: bind_techno_id(defs, &entry.type_id, "MapTaskForce")?,
            });
        }
        let id = TaskForceId(next);
        next = next.saturating_add(1);
        out.push(PreparedTaskForce {
            id,
            name: force.id.clone(),
            editor_name: force.name.clone(),
            entries,
            group: force.group,
        });
    }
    Ok(out)
}



/// 将 `[ScriptTypes]` 投影为稳定 [`PreparedScriptType`] 表。
pub fn bind_map_script_types(script_types: &[MapScriptType]) -> Vec<PreparedScriptType> {
    let mut out = Vec::with_capacity(script_types.len());
    let mut next = 1u32;
    for script in script_types {
        if script.id.is_empty() {
            continue;
        }
        let id = ScriptTypeId(next);
        next = next.saturating_add(1);
        out.push(PreparedScriptType {
            id,
            name: script.id.clone(),
            editor_name: script.name.clone(),
            steps: script.steps.clone(),
        });
    }
    out
}

/// 将 `[TeamTypes]` 绑定为 [`PreparedTeamType`]。
///
/// - 未知 / 空 `house` → [`RaError::UnknownReference`]（house）
/// - 未知非空 `script` → [`RaError::UnknownReference`]（script）
/// - 未知 / 空 `task_force` → [`RaError::UnknownReference`]（task_force）
/// - 未知非空 `tag` → [`RaError::UnknownReference`]（tag）
pub fn bind_map_team_types(
    team_types: &[MapTeamType],
    defs: &RuntimeDefinitions,
    tags: &[PreparedTag],
    task_forces: &[PreparedTaskForce],
    script_types: &[PreparedScriptType],
) -> RaResult<Vec<PreparedTeamType>> {
    let tag_by_name: HashMap<&str, TagId> = tags.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let task_force_by_name: HashMap<&str, TaskForceId> = task_forces.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let script_by_name: HashMap<&str, ScriptTypeId> = script_types.iter().map(|s| (s.name.as_str(), s.id)).collect();
    let mut out = Vec::with_capacity(team_types.len());
    for team in team_types {
        let house = bind_house_id(defs, &team.house, "MapTeamType")?;
        let script = bind_script_id(&script_by_name, &team.script, "MapTeamType")?;
        let task_force = bind_task_force_id(&task_force_by_name, &team.task_force, "MapTeamType")?;
        let tag = bind_tag_id(&tag_by_name, &team.tag, "MapTeamType")?;
        out.push(PreparedTeamType {
            id: team.id.clone(),
            name: team.name.clone(),
            house,
            script,
            task_force,
            tag,
            waypoint: team.waypoint,
            max: team.max,
            priority: team.priority,
            veteran_level: team.veteran_level,
        });
    }
    Ok(out)
}


/// 将 `[AITriggerTypes]` 绑定为 [`PreparedAiTrigger`]。
///
/// - 未知 / 空 `owner_house` → [`RaError::UnknownReference`]（house）
/// - 未知 / 空 `team` → [`RaError::UnknownReference`]（team）
pub fn bind_map_ai_triggers(
    ai_triggers: &[MapAiTrigger],
    defs: &RuntimeDefinitions,
    team_types: &[PreparedTeamType],
) -> RaResult<Vec<PreparedAiTrigger>> {
    let team_names: HashMap<&str, ()> = team_types.iter().map(|t| (t.id.as_str(), ())).collect();
    let mut out = Vec::with_capacity(ai_triggers.len());
    for trigger in ai_triggers {
        let owner_house = bind_house_id(defs, &trigger.owner_house, "MapAiTrigger")?;
        let team = bind_team_type_name(&team_names, &trigger.team, "MapAiTrigger")?;
        out.push(PreparedAiTrigger {
            id: trigger.id.clone(),
            name: trigger.name.clone(),
            team,
            owner_house,
            tech_level: trigger.tech_level,
        });
    }
    Ok(out)
}

/// 就地填充 [`PreparedMap`] 绑定字段；失败时不改动已有字段。
pub fn bind_prepared_map_placements(prepared: &mut PreparedMap, defs: &RuntimeDefinitions) -> RaResult<()> {
    let tags = bind_map_tags(&prepared.definition.tags);
    let cell_tags = bind_map_cell_tags(&prepared.definition.cell_tags, &tags)?;
    let task_forces = bind_map_task_forces(&prepared.definition.task_forces, defs)?;
    let script_types = bind_map_script_types(&prepared.definition.script_types);
    let team_types = bind_map_team_types(&prepared.definition.team_types, defs, &tags, &task_forces, &script_types)?;
    let ai_triggers = bind_map_ai_triggers(&prepared.definition.ai_triggers, defs, &team_types)?;
    let placements = bind_map_placements(&prepared.definition.entities, defs, &tags)?;
    prepared.tags = tags;
    prepared.cell_tags = cell_tags;
    prepared.task_forces = task_forces;
    prepared.script_types = script_types;
    prepared.team_types = team_types;
    prepared.ai_triggers = ai_triggers;
    prepared.placements = placements;
    Ok(())
}

fn bind_techno_id(defs: &RuntimeDefinitions, name: &TechnoName, owner: &str) -> RaResult<TypeId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference {
            kind: "techno",
            name: String::new(),
            owner: owner.to_string(),
        });
    }
    defs.techno.get_name(name).map(|t| t.id).ok_or_else(|| RaError::UnknownReference {
        kind: "techno",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_house_id(defs: &RuntimeDefinitions, name: &HouseName, owner: &str) -> RaResult<HouseId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference {
            kind: "house",
            name: String::new(),
            owner: owner.to_string(),
        });
    }
    defs.houses.get_name(name).map(|h| h.id).ok_or_else(|| RaError::UnknownReference {
        kind: "house",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_tag_id(tag_by_name: &HashMap<&str, TagId>, name: &TagName, owner: &str) -> RaResult<Option<TagId>> {
    if name.is_empty() {
        return Ok(None);
    }
    tag_by_name.get(name.as_str()).copied().map(Some).ok_or_else(|| RaError::UnknownReference {
        kind: "tag",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
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

fn bind_script_id(
    script_by_name: &HashMap<&str, ScriptTypeId>,
    name: &ScriptTypeName,
    owner: &str,
) -> RaResult<Option<ScriptTypeId>> {
    if name.is_empty() {
        return Ok(None);
    }
    script_by_name.get(name.as_str()).copied().map(Some).ok_or_else(|| RaError::UnknownReference {
        kind: "script",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_task_force_id(
    task_force_by_name: &HashMap<&str, TaskForceId>,
    name: &TaskForceName,
    owner: &str,
) -> RaResult<TaskForceId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference {
            kind: "task_force",
            name: String::new(),
            owner: owner.to_string(),
        });
    }
    task_force_by_name.get(name.as_str()).copied().ok_or_else(|| RaError::UnknownReference {
        kind: "task_force",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_team_type_name(team_names: &HashMap<&str, ()>, name: &TeamTypeName, owner: &str) -> RaResult<TeamTypeName> {
    if name.is_empty() {
        return Err(RaError::UnknownReference {
            kind: "team",
            name: String::new(),
            owner: owner.to_string(),
        });
    }
    if team_names.contains_key(name.as_str()) {
        Ok(name.clone())
    } else {
        Err(RaError::UnknownReference {
            kind: "team",
            name: name.as_str().to_string(),
            owner: owner.to_string(),
        })
    }
}

