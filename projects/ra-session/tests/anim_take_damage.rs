//! 受击闪白派生 TakeDamage 动画状态。

mod common;

use common::rules_with_mtnk;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_session::{AnimState, Session};
use ra_types::GameEdition;
use ra_world::{GameCommand, HIT_FLASH_TICKS, World};

#[test]
fn snapshot_anim_state_take_damage_then_die() {
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
    let mut session = Session::new(World::new(GameEdition::Ra2, &rules, map), "hit");
    // 压低生命，下一击即可致死或可见闪白。
    session.world.entities[1].health = 30;
    session.world.entities[0].attack_damage = 20;
    session.world.entities[0].attack_range = 4;
    session.world.entities[0].attack_cooldown = 0;
    session.world.entities[0].attack_cooldown_max = 8;
    session.push_command(GameCommand::Attack {
        attacker_index: 0,
        target_index: 1,
    });
    session.tick();
    let snap = session.snapshot();
    let tgt = snap.units.iter().find(|u| u.index == 1).unwrap();
    if tgt.dead {
        assert_eq!(tgt.anim_state, AnimState::Die);
    } else {
        assert_eq!(tgt.anim_state, AnimState::TakeDamage);
        assert!(session.world.entities[1].hit_flash <= HIT_FLASH_TICKS);
        assert!(session.world.entities[1].hit_flash > 0);
    }
}
