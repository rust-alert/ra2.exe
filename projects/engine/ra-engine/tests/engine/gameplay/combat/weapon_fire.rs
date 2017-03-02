//! 开火时按武器 `Report=` 排队对局短音效。

use crate::common::{battle_from_defs, defs_from_rules_ini, map_with_size};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn resolve_combat_queues_weapon_report_sfx() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=100\nROF=8\nRange=6\nWarhead=SA\nReport=TankCannon\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "RUSSIANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    let a = world.entity_id_at(0).expect("entity");
    let b = world.entity_id_at(1).expect("entity");
    assert!(world.clear_ecs_movement(a));
    assert!(world.clear_ecs_movement(b));
    assert!(world.set_ecs_speed(b, 0));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    let cues = world.take_battle_sfx_cues();
    assert!(
        cues.iter().any(|c| c.event.eq_ignore_ascii_case("TankCannon")),
        "firing must queue the weapon Report as a battle sfx cue: {cues:?}"
    );
}
