//! 受击闪白派生 TakeDamage 动画状态。

use crate::common::{rules_with_mtnk, test_engine};
use ra_engine::{AnimState, GameCommand, HIT_FLASH_TICKS, MatchState, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn snapshot_anim_state_take_damage_then_die() {
    let engine = test_engine();
    let rules = rules_with_mtnk();
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
    });
    let mut session = Session::from_state(MatchState::new(GameEdition::Ra2, &rules, map), "hit");
    let attacker = session.expect_game().world.entities[0].id;
    let target = session.expect_game().world.entities[1].id;
    assert!(session.expect_game_mut().world.set_ecs_health(target, 30, 30, false));
    assert!(session.expect_game_mut().world.set_ecs_attack_power(attacker, 20, 4, 8));
    session.expect_game_mut().push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    session.tick(&engine.runtime());
    let snap = session.expect_game().snapshot(&[]);
    let tgt = snap.units.iter().find(|u| u.id == EntityId(2)).unwrap();
    if tgt.dead {
        assert_eq!(tgt.anim_state, AnimState::Die);
    }
    else {
        assert_eq!(tgt.anim_state, AnimState::TakeDamage);
        assert!(session.expect_game_mut().world.entities[1].hit_flash <= HIT_FLASH_TICKS);
        assert!(session.expect_game_mut().world.entities[1].hit_flash > 0);
    }
}
