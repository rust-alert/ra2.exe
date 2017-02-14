//! 地图预放实体 → [`PreparedPlacement`] 规则绑定。

use std::collections::HashMap;

use crate::{
    HouseId, HouseName, MapAction, MapCellTag, MapEvent, MapHouse, MapPlacedEntity, MapTag, MapTrigger, MissionKind, MissionName,
    PreparedAction, PreparedCellTag, PreparedEvent, PreparedHouse, PreparedMap, PreparedPlacement, PreparedTag, PreparedTrigger, RaError,
    RaResult, RuntimeDefinitions, TagId, TagName, TechnoName, TriggerId, TriggerName, TypeId,
};

/// 将 `[Houses]` 投影为稳定 [`PreparedHouse`] 表。
///
/// - 空 / 未知 `Country=` → [`RaError::UnknownReference`]
/// - `Allies=` 中空 / `NONE` 项跳过；非空未知 → [`RaError::UnknownReference`]
pub fn bind_map_houses(houses: &[MapHouse], defs: &RuntimeDefinitions) -> RaResult<Vec<PreparedHouse>> {
    let mut out = Vec::with_capacity(houses.len());
    for house in houses {
        let country = bind_house_id(defs, &house.country, &format!("MapHouse:{}", house.name))?;
        let mut allies = Vec::with_capacity(house.allies.len());
        for ally in &house.allies {
            if ally.is_empty() || ally.as_str().eq_ignore_ascii_case("NONE") {
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
/// - 空 / 未知 `house` → [`RaError::UnknownReference`]
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
        let house = bind_house_id(defs, &trigger.house, &format!("MapTrigger:{}", trigger.id.as_str()))?;
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
        out.push(PreparedTag {
            id,
            name: tag.id.clone(),
            persistence: tag.persistence,
            editor_name: tag.name.clone(),
            trigger_id,
        });
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

/// 将 `[Actions]` 投影为稳定 [`PreparedAction`] 表；未知 trigger id 拒绝。
pub fn bind_map_actions(actions: &[MapAction], triggers: &[PreparedTrigger]) -> RaResult<Vec<PreparedAction>> {
    let trigger_by_name: HashMap<&str, TriggerId> = triggers.iter().map(|t| (t.name.as_str(), t.id)).collect();
    let mut out = Vec::with_capacity(actions.len());
    for action in actions {
        if action.id.is_empty() {
            continue;
        }
        let trigger_id = bind_trigger_id(&trigger_by_name, &action.id, "MapAction")?;
        out.push(PreparedAction { trigger_id, commands: action.commands.clone() });
    }
    Ok(out)
}

/// 就地填充 [`PreparedMap`] 的 houses / triggers / events / actions / tags / cell_tags / placements；
/// 失败时不改动已有字段。
pub fn bind_prepared_map_placements(prepared: &mut PreparedMap, defs: &RuntimeDefinitions) -> RaResult<()> {
    let houses = bind_map_houses(&prepared.definition.houses, defs)?;
    let triggers = bind_map_triggers(&prepared.definition.triggers, defs)?;
    let events = bind_map_events(&prepared.definition.events, &triggers)?;
    let actions = bind_map_actions(&prepared.definition.actions, &triggers)?;
    let tags = bind_map_tags(&prepared.definition.tags, &triggers)?;
    let cell_tags = bind_map_cell_tags(&prepared.definition.cell_tags, &tags)?;
    let placements = bind_map_placements(&prepared.definition.entities, defs, &tags)?;
    prepared.houses = houses;
    prepared.triggers = triggers;
    prepared.events = events;
    prepared.actions = actions;
    prepared.tags = tags;
    prepared.cell_tags = cell_tags;
    prepared.placements = placements;
    Ok(())
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

fn bind_house_id(defs: &RuntimeDefinitions, name: &HouseName, owner: &str) -> RaResult<HouseId> {
    if name.is_empty() {
        return Err(RaError::UnknownReference { kind: "house", name: String::new(), owner: owner.to_string() });
    }
    defs.houses.get_name(name).map(|h| h.id).ok_or_else(|| RaError::UnknownReference {
        kind: "house",
        name: name.as_str().to_string(),
        owner: owner.to_string(),
    })
}

fn bind_tag_id(tag_by_name: &HashMap<&str, TagId>, name: &TagName, owner: &str) -> RaResult<Option<TagId>> {
    // 空列与零售哨兵 `None`（装载期大写为 `NONE`）均表示无 Tag。
    if name.is_empty() || name.as_str().eq_ignore_ascii_case("NONE") {
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
    if name.is_empty() || name.as_str().eq_ignore_ascii_case("<NONE>") || name.as_str().eq_ignore_ascii_case("NONE") {
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
