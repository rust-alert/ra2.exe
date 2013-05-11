//! 壳层共享的会话：持有世界、转发命令、产出呈现快照。
//!
//! 不碰文件系统与 GPU；桌面 / Web 只负责 I/O 与绘制。

use ra_map::MapEntityKind;
use ra_types::GameEdition;
use ra_world::{GameCommand, World};

/// 一帧呈现用的不可变快照（渲染器应逐步只消费此类数据）。
#[derive(Debug, Clone)]
pub struct RenderSnapshot {
    pub edition: GameEdition,
    pub tick: u64,
    pub state_hash: u64,
    pub units: Vec<SnapshotUnit>,
}

/// 快照中的一个可绘实体。
#[derive(Debug, Clone)]
pub struct SnapshotUnit {
    pub index: usize,
    pub kind: MapEntityKind,
    pub type_id: String,
    pub owner: String,
    pub x: u16,
    pub y: u16,
    pub facing: u8,
    pub turret_facing: u8,
    pub hva_frame: u16,
    pub health: u32,
    pub max_health: u32,
    pub dead: bool,
}

/// 运行中会话。
#[derive(Debug)]
pub struct Session {
    pub world: World,
    pub boot_note: String,
}

impl Session {
    pub fn new(world: World, boot_note: impl Into<String>) -> Self {
        Self {
            world,
            boot_note: boot_note.into(),
        }
    }

    pub fn push_command(&mut self, cmd: GameCommand) {
        self.world.push_command(cmd);
    }

    pub fn tick(&mut self) {
        self.world.advance_tick();
    }

    pub fn snapshot(&self) -> RenderSnapshot {
        let units = self
            .world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e.kind,
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft
                )
            })
            .map(|(index, e)| SnapshotUnit {
                index,
                kind: e.kind,
                type_id: e.type_id.clone(),
                owner: e.owner.clone(),
                x: e.x,
                y: e.y,
                facing: e.facing,
                turret_facing: e.turret_facing,
                hva_frame: e.hva_frame,
                health: e.health,
                max_health: e.max_health,
                dead: e.dead,
            })
            .collect();
        RenderSnapshot {
            edition: self.world.edition,
            tick: self.world.tick,
            state_hash: self.world.state_hash(),
            units,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ra_assets::IniDocument;
    use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
    use ra_rules::{
        ColorSchemes, OverlayTypeRegistry, RulesDb, TechnoTypeRegistry,
    };
    use ra_types::GameEdition;
    use ra_world::GameCommand;

    fn rules_with_mtnk() -> RulesDb {
        let doc = IniDocument::parse(
            b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n",
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
    fn session_tick_and_snapshot() {
        let rules = rules_with_mtnk();
        let mut map = MapInfo::empty(GameEdition::Ra2, "t");
        map.width = 20;
        map.height = 30;
        map.waypoints.push(Waypoint {
            index: 0,
            x: 12,
            y: 10,
        });
        map.entities.push(MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 10,
            y: 10,
            facing: 0,
            sub_cell: 0,
        });
        let world = World::new(GameEdition::Ra2, &rules, map);
        let mut session = Session::new(world, "test");
        session.push_command(GameCommand::MoveTo {
            entity_index: 0,
            x: 12,
            y: 10,
        });
        session.tick();
        let snap = session.snapshot();
        assert_eq!(snap.tick, 1);
        assert_eq!(snap.units.len(), 1);
        assert_eq!(snap.units[0].x, 11);
    }
}
