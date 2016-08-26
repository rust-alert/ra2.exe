//! 受击闪白派生 TakeDamage 动画状态。

use crate::common::{defs_with_mtnk, test_engine, battle_from_defs};
use ra_engine::{AnimState, GameCommand, HIT_FLASH_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn snapshot_anim_state_take_damage_then_die() {
    let engine = test_engine();
    let defs = defs_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "hit");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "hit");
    let attacker = session.expect_battle().world.entity_id_at(0).expect("entity");
    let target = session.expect_battle().world.entity_id_at(1).expect("entity");
    assert!(session.expect_battle_mut().world.set_ecs_health(target, 30, 30, false));
    assert!(session.expect_battle_mut().world.set_ecs_attack_power(attacker, 20, 4, 8));
    session.expect_battle_mut().push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let tgt = snap.units.iter().find(|u| u.id == EntityId(2)).unwrap();
    if tgt.dead {
        assert_eq!(tgt.anim_state, AnimState::Die);
    }
    else {
        assert_eq!(tgt.anim_state, AnimState::TakeDamage);
        let flash = session.expect_battle().world.ecs_animation(target).expect("anim").1;
        assert!(flash <= HIT_FLASH_TICKS);
        assert!(flash > 0);
    }
}
