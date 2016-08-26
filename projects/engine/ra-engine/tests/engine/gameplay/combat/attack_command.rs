//! 单位互殴伤害与击杀。

use crate::common::{map_with_size, defs_with_mtnk, battle_from_defs};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{GameEdition, EntityId};

#[test]
fn attack_command_damages_and_kills() {
    let defs = defs_with_mtnk();
    let mut map = map_with_size();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Russians".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 12,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs.clone(), map);
    // 取消航点游荡，专注开火。
    let a = world.entity_id_at(0).expect("entity");
    let b = world.entity_id_at(1).expect("entity");
    assert!(world.clear_ecs_movement(a));
    assert!(world.clear_ecs_movement(b));
    assert!(world.set_ecs_speed(b, 0));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    let start_hp = world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0;
    world.advance_tick();
    assert_eq!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0, Some(EntityId(2)));
    assert!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0 < start_hp);
    for _ in 0..64 {
        world.advance_tick();
        if world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2 {
            break;
        }
    }
    assert!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2);
    assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, 0);
    assert_eq!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0, None);
    let cues = world.take_eva_cues();
    assert!(
        cues.iter().any(|c| c.event == "EVA_UnitLost" && c.house.as_ref() == "Russians"),
        "killing a mobile unit must cue EVA_UnitLost for the victim house: {cues:?}"
    );
}
