//! 攻击移动：途中接敌，战后续行最终目的地。

use crate::common::{battle_from_defs, defs_with_mtnk, map_with_size};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind};
use ra_types::{EntityId, GameEdition};

#[test]
fn attack_move_engages_then_resumes_goal() {
    let defs = defs_with_mtnk();
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
    let attacker = world.entity_id_at(0).expect("attacker");
    let victim = world.entity_id_at(1).expect("victim");
    assert!(world.set_ecs_speed(victim, 0));

    world.push_command(GameCommand::AttackMove { entity: EntityId(1), x: 16, y: 10 });
    world.advance_tick();

    assert!(
        world.ecs_mission(attacker).expect("mission").as_ref().eq_ignore_ascii_case("AttackMove"),
        "AttackMove command must set mission"
    );
    assert_eq!(world.ecs_attack_state(attacker).expect("atk").0, Some(victim));
    assert_eq!(world.ecs_waypoints(attacker).expect("wp"), vec![(16, 10)]);
    assert_eq!(world.ecs_move_destination(attacker).expect("dest"), (Some(12), Some(10)));

    for _ in 0..64 {
        world.advance_tick();
        if world.ecs_health(victim).expect("health").2 {
            break;
        }
    }
    assert!(world.ecs_health(victim).expect("health").2);
    // 击杀当 tick 已在 apply_damage 里清目标并弹出航点续行。
    assert_eq!(world.ecs_attack_state(attacker).expect("atk").0, None);
    assert_eq!(world.ecs_move_destination(attacker).expect("dest"), (Some(16), Some(10)));
    assert!(world.ecs_waypoints(attacker).expect("wp").is_empty());
}
