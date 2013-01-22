//! 确定性世界推进。不依赖渲染器与文件系统。

use ra_map::MapInfo;
use ra_rules::RulesDb;
use ra_types::{GameEdition, PlayerId};

#[derive(Debug, Clone)]
pub struct World {
    pub edition: GameEdition,
    pub tick: u64,
    pub map: MapInfo,
    pub local_player: PlayerId,
    state_hash: u64,
}

impl World {
    pub fn new(edition: GameEdition, rules: &RulesDb, map: MapInfo) -> Self {
        let _ = rules;
        Self {
            edition,
            tick: 0,
            map,
            local_player: PlayerId(0),
            state_hash: 0,
        }
    }

    pub fn advance_tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.state_hash = self
            .state_hash
            .wrapping_mul(1099511628211)
            .wrapping_add(self.tick)
            .wrapping_add(self.edition.as_str().len() as u64);
    }

    pub fn state_hash(&self) -> u64 {
        self.state_hash
    }
}
