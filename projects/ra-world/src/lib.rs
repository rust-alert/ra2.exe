//! 确定性世界推进。不依赖渲染器与文件系统。

use ra_map::{MapEntity, MapInfo};
use ra_rules::RulesDb;
use ra_types::{GameEdition, PlayerId};

#[derive(Debug, Clone)]
pub struct World {
    pub edition: GameEdition,
    pub tick: u64,
    pub map: MapInfo,
    /// 由地图放置段播种的实体快照（后续仿真就地推进）。
    pub entities: Vec<MapEntity>,
    pub local_player: PlayerId,
    state_hash: u64,
}

impl World {
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let _ = rules;
        let entities = map.entities.clone();
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
                .wrapping_add((e.facing as u64) << 32);
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
    use ra_map::MapEntityKind;
    use ra_rules::{ColorSchemes, OverlayTypeRegistry, RulesDb};
    use ra_types::GameEdition;

    fn empty_rules() -> RulesDb {
        RulesDb {
            edition: GameEdition::Ra2,
            rules: IniDocument::default(),
            art: IniDocument::default(),
            overlay_types: OverlayTypeRegistry::default(),
            color_schemes: ColorSchemes::default(),
        }
    }

    #[test]
    fn seeds_entities_and_hashes() {
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.entities.push(ra_map::MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 20,
            facing: 64,
            sub_cell: 0,
        });
        let world = World::new(GameEdition::Ra2, &empty_rules(), map);
        assert_eq!(world.entities.len(), 1);
        assert_ne!(world.state_hash(), 0);
    }
}
