//! 地图预放实体 → [`PreparedPlacement`] 规则绑定。

use std::collections::HashMap;

use crate::{
    HouseId, HouseName, MapCellTag, MapPlacedEntity, MapTag, MissionKind, MissionName, PreparedCellTag, PreparedMap, PreparedPlacement, PreparedTag, RaError, RaResult, RuntimeDefinitions,
    TagId, TagName, TechnoName, TypeId,
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

/// 就地填充 [`PreparedMap::tags`] / [`PreparedMap::cell_tags`] / [`PreparedMap::placements`]；失败时不改动已有字段。
pub fn bind_prepared_map_placements(prepared: &mut PreparedMap, defs: &RuntimeDefinitions) -> RaResult<()> {
    let tags = bind_map_tags(&prepared.definition.tags);
    let cell_tags = bind_map_cell_tags(&prepared.definition.cell_tags, &tags)?;
    let placements = bind_map_placements(&prepared.definition.entities, defs, &tags)?;
    prepared.tags = tags;
    prepared.cell_tags = cell_tags;
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
