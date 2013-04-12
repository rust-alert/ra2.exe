//! 确定性世界推进。不依赖渲染器与文件系统。

use ra_map::{MapEntityKind, MapInfo};
use ra_rules::{RulesDb, TechnoKind};
use ra_types::{GameEdition, PlayerId};

/// 世界中的一个已放置实体（由地图播种，后续仿真就地改）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldEntity {
    pub kind: MapEntityKind,
    pub owner: String,
    pub type_id: String,
    pub x: u16,
    pub y: u16,
    pub facing: u8,
    pub sub_cell: u8,
    /// 当前生命（由放置段比例 × Strength）。
    pub health: u32,
    pub max_health: u32,
    pub speed: u32,
    pub techno_kind: Option<TechnoKind>,
}

#[derive(Debug, Clone)]
pub struct World {
    pub edition: GameEdition,
    pub tick: u64,
    pub map: MapInfo,
    pub entities: Vec<WorldEntity>,
    pub local_player: PlayerId,
    state_hash: u64,
}

impl World {
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let entities = map
            .entities
            .iter()
            .map(|e| {
                let tt = rules.techno_types.get(&e.type_id);
                let max_health = tt.map(|t| t.strength).unwrap_or(1).max(1);
                let health = (u64::from(max_health) * u64::from(e.health) / 256) as u32;
                WorldEntity {
                    kind: e.kind,
                    owner: e.owner.clone(),
                    type_id: e.type_id.clone(),
                    x: e.x,
                    y: e.y,
                    facing: e.facing,
                    sub_cell: e.sub_cell,
                    health,
                    max_health,
                    speed: tt.map(|t| t.speed).unwrap_or(0),
                    techno_kind: tt.map(|t| t.kind),
                }
            })
            .collect();
        let mut world = Self {
            edition,
            tick: 0,
            map,
            entities,
            local_player: PlayerId(0),
            state_hash: 0,
        };
        world.rehash();
        world
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.rehash();
    }

    pub fn state_hash(&self) -> u64 {
        self.state_hash
    }

    pub fn bound_techno_count(&self) -> usize {
        self.entities
            .iter()
            .filter(|e| e.techno_kind.is_some())
            .count()
    }

    fn rehash(&mut self) {
        let mut h = self.tick;
        h = h
            .wrapping_mul(1099511628211)
            .wrapping_add(self.edition.as_str().len() as u64);
        h = h
            .wrapping_mul(1099511628211)
            .wrapping_add(self.entities.len() as u64);
        for e in &self.entities {
            h = h
                .wrapping_mul(1099511628211)
                .wrapping_add(e.x as u64)
                .wrapping_add((e.y as u64) << 16)
                .wrapping_add((e.facing as u64) << 32)
                .wrapping_add(u64::from(e.health));
            for b in e.type_id.as_bytes() {
                h = h.wrapping_mul(1099511628211).wrapping_add(u64::from(*b));
            }
        }
        self.state_hash = h;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::IniDocument;
    use ra_map::MapEntity;
    use ra_rules::{
        ColorSchemes, OverlayTypeRegistry, RulesDb, TechnoKind, TechnoTypeRegistry,
    };
    use ra_types::GameEdition;

    fn rules_with_mtnk() -> RulesDb {
        let doc = IniDocument::parse(
            b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=6\nSight=6\nCost=800\nArmor=heavy\n",
        )
        .unwrap();
        RulesDb {
            edition: GameEdition::Ra2,
            rules: doc.clone(),
            art: IniDocument::default(),
            overlay_types: OverlayTypeRegistry::default(),
            color_schemes: ColorSchemes::default(),
            techno_types: TechnoTypeRegistry::from_rules(&doc),
        }
    }

    #[test]
    fn binds_strength_and_speed() {
        let rules = rules_with_mtnk();
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 20,
            facing: 64,
            sub_cell: 0,
        });
        let world = World::new(GameEdition::Ra2, &rules, map);
        assert_eq!(world.entities.len(), 1);
        let e = &world.entities[0];
        assert_eq!(e.max_health, 400);
        assert_eq!(e.health, 400);
        assert_eq!(e.speed, 6);
        assert_eq!(e.techno_kind, Some(TechnoKind::Vehicle));
        assert_eq!(world.bound_techno_count(), 1);
    }
}
