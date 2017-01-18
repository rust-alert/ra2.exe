//! 地图预放实体 → [`PreparedPlacement`] 规则绑定。

use crate::{
    HouseId, HouseName, MapPlacedEntity, PreparedMap, PreparedPlacement, RaError, RaResult, RuntimeDefinitions, TechnoName,
    TypeId,
};

/// 将 [`MapPlacedEntity`] 列表绑定为稳定 id 的 [`PreparedPlacement`]。
///
/// - 未知 `type_id` → [`RaError::UnknownReference`]（techno）
/// - 未知 / 空 `owner` → [`RaError::UnknownReference`]（house）
pub fn bind_map_placements(entities: &[MapPlacedEntity], defs: &RuntimeDefinitions) -> RaResult<Vec<PreparedPlacement>> {
    let mut out = Vec::with_capacity(entities.len());
    for entity in entities {
        let definition_id = bind_techno_id(defs, &entity.type_id, "MapPlacement")?;
        let owner = bind_house_id(defs, &entity.owner, "MapPlacement")?;
        out.push(PreparedPlacement {
            kind: entity.kind,
            owner,
            definition_id,
            health: entity.health,
            x: entity.x,
            y: entity.y,
            facing: entity.facing,
            sub_cell: entity.sub_cell,
            mission: entity.mission.clone(),
            tag: entity.tag.clone(),
        });
    }
    Ok(out)
}

/// 就地填充 [`PreparedMap::placements`]；失败时不改动已有 placements。
pub fn bind_prepared_map_placements(prepared: &mut PreparedMap, defs: &RuntimeDefinitions) -> RaResult<()> {
    let placements = bind_map_placements(&prepared.definition.entities, defs)?;
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
