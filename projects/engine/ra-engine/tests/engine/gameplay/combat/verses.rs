//! 弹头 Verses 相对护甲结算伤害。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

#[test]
fn verses_scales_damage_against_armor() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=ATK\n1=TGT\n\
[ATK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=Gun\nArmor=none\n\
[TGT]\nStrength=400\nSpeed=0\nSight=1\nCost=100\nArmor=heavy\n\
[Gun]\nDamage=100\nROF=1\nRange=8\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,50%,100%,100%,100%,100%,100%\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "verses");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "ATK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "SOVIETS".into(),
        type_id: "TGT".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat").attack_damage, 100);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(0).expect("entity")).expect("combat").attack_verses[5], 50);
    assert_eq!(world.ecs_combat_view(world.entity_id_at(1).expect("entity")).expect("combat").armor, ra_types::ArmorKind::Heavy);
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    // 100 * 50% = 50
    assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, 350);
}

#[test]
fn force_fire_rejected_when_verses_disallow_f() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=ATK\n1=TGT\n\
[ATK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=Gun\nArmor=none\n\
[TGT]\nStrength=400\nSpeed=0\nSight=1\nCost=100\nArmor=heavy\n\
[Gun]\nDamage=100\nROF=1\nRange=8\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,0%,100%,100%,100%,100%,100%\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "verses-f");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "ATK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "SOVIETS".into(),
        type_id: "TGT".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    let before = world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0;
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    assert!(
        world.last_rejects().iter().any(|r| r.reason == ra_engine::CommandRejectReason::InvalidTarget),
        "{:?}",
        world.last_rejects()
    );
    assert_eq!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").0, before);
    assert!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0.is_none());
}

#[test]
fn zero_verses_with_f_allows_force_fire_command() {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=ATK\n1=TGT\n\
[ATK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=Gun\nArmor=none\n\
[TGT]\nStrength=400\nSpeed=0\nSight=1\nCost=100\nArmor=heavy\n\
[Gun]\nDamage=100\nROF=1\nRange=8\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,0%F,100%,100%,100%,100%,100%\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "verses-f-ok");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "ATK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "SOVIETS".into(),
        type_id: "TGT".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "{:?}", world.last_rejects());
    assert_eq!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0, Some(EntityId(2)));
}

fn verses_duel_world(verses: &str) -> ra_engine::BattleState {
    let ini = format!(
        "[VehicleTypes]\n0=ATK\n1=TGT\n\
[ATK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=GunA\nArmor=none\n\
[TGT]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nPrimary=GunB\nArmor=heavy\n\
[GunA]\nDamage=10\nROF=8\nRange=8\nWarhead=HA\n\
[GunB]\nDamage=10\nROF=8\nRange=8\nWarhead=HB\n\
[HA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n\
[HB]\nVerses={verses}\n"
    );
    let defs = defs_from_rules_ini(ini.as_bytes());
    let mut map = MapInfo::empty(GameEdition::Ra2, "verses-r");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "ATK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "SOVIETS".into(),
        type_id: "TGT".into(),
        health: 256,
        x: 5,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    battle_from_defs(GameEdition::Ra2, defs, map)
}

#[test]
fn victim_retaliates_when_verses_allow_r() {
    let mut world = verses_duel_world("100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%");
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    assert_eq!(world.ecs_attack_state(world.entity_id_at(1).expect("victim")).expect("atk").0, Some(EntityId(1)));
}

#[test]
fn victim_skips_retaliate_when_verses_zero_without_r() {
    let mut world = verses_duel_world("0%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%");
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    assert!(world.ecs_attack_state(world.entity_id_at(1).expect("victim")).expect("atk").0.is_none());
}

#[test]
fn victim_retaliates_when_zero_multiplier_has_r_flag() {
    let mut world = verses_duel_world("0%R,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%");
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    world.advance_tick();
    assert_eq!(world.ecs_attack_state(world.entity_id_at(1).expect("victim")).expect("atk").0, Some(EntityId(1)));
}
