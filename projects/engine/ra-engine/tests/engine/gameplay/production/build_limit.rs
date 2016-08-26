//! BuildLimit：同类型存活数达上限后不可再建/再生产。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, PlayerId};

fn limit_world() -> BattleState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\nOwner=Americans\nBuildLimit=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "build-limit");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Infantry,
            owner: "Americans".into(),
            type_id: "E1".into(),
            health: 256,
            x: 6,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn produce_rejects_when_build_limit_reached() {
    let mut world = limit_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::QueueFull);
}
