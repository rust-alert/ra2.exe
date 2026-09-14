//! 遭遇战胜负：短局 / 长局保活与 sole_victor。

use crate::common::{battle_from_defs, defs_from_rules_ini, defs_with_mtnk};
use ra_engine::Session;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;
use std::sync::Arc;

fn push_unit(map: &mut MapInfo, owner: &str, type_id: &str, x: u16, y: u16) {
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: owner.into(),
        type_id: type_id.into(),
        health: 256,
        x,
        y,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
}

fn push_structure(map: &mut MapInfo, owner: &str, type_id: &str, x: u16, y: u16) {
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: owner.into(),
        type_id: type_id.into(),
        health: 256,
        x,
        y,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
}

fn defs_with_base_units() -> Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[General]\nBaseUnit=AMCV,SMCV\n\
[VehicleTypes]\n0=MTNK\n1=AMCV\n2=SMCV\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=GAWALL\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n\
[AMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nDeploysInto=GACNST\nOwner=Americans\n\
[SMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nDeploysInto=NACNST\nOwner=Russians\n\
[GACNST]\nStrength=1000\nArmor=wood\nFoundation=2x2\n\
[NACNST]\nStrength=1000\nArmor=wood\nFoundation=2x2\n\
[GAWALL]\nStrength=100\nArmor=concrete\nWall=yes\nBuildCat=Combat\nFoundation=1x1\n",
    )
}

#[test]
fn long_game_living_structure_prevents_sole_victor() {
    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "victory");
    map.width = 16;
    map.height = 16;
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_structure(&mut map, "SOVIETS", "MTNK", 8, 8);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "victory");
    session.expect_battle_mut().set_short_game(false);
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    let max = session.expect_battle().world.ecs_health(enemy).expect("health").1;
    assert!(session.expect_battle_mut().world.set_ecs_type_id(enemy, "NACNST", MapEntityKind::Structure));
    assert_eq!(session.expect_battle_mut().world.players.len(), 2);
    assert!(session.expect_battle().sole_victor().is_none());
    assert!(session.expect_battle_mut().world.set_ecs_health(enemy, 0, max, true));
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn ambient_units_do_not_block_sole_victor() {
    let defs = defs_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "ambient-victory");
    map.width = 16;
    map.height = 16;
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_unit(&mut map, "SOVIETS", "MTNK", 8, 4);
    push_unit(&mut map, "NEUTRAL", "MTNK", 12, 4);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "ambient-victory");
    session.expect_battle_mut().set_short_game(false);
    assert_eq!(session.expect_battle().world.players.len(), 3);
    assert!(session.expect_battle().sole_victor().is_none());
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    let max = session.expect_battle().world.ecs_health(enemy).expect("health").1;
    assert!(session.expect_battle_mut().world.set_ecs_health(enemy, 0, max, true));
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn special_house_does_not_block_sole_victor() {
    let defs = defs_with_mtnk();
    let mut map = MapInfo::empty(GameEdition::Ra2, "special-victory");
    map.width = 16;
    map.height = 16;
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_unit(&mut map, "SOVIETS", "MTNK", 8, 4);
    push_structure(&mut map, "SPECIAL", "MTNK", 12, 4);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs.clone(), map), "special-victory");
    session.expect_battle_mut().set_short_game(false);
    let enemy = session.expect_battle().world.entity_id_at(1).expect("entity");
    let max = session.expect_battle().world.ecs_health(enemy).expect("health").1;
    assert!(session.expect_battle_mut().world.set_ecs_health(enemy, 0, max, true));
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn short_game_ordinary_units_do_not_keep_house_alive() {
    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "short-units");
    map.width = 16;
    map.height = 16;
    push_structure(&mut map, "AMERICANS", "GACNST", 2, 2);
    push_unit(&mut map, "SOVIETS", "MTNK", 8, 8);
    let session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "short-units");
    assert!(session.expect_battle().short_game);
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn short_game_base_unit_keeps_house_alive() {
    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "short-base");
    map.width = 16;
    map.height = 16;
    push_structure(&mut map, "AMERICANS", "GACNST", 2, 2);
    push_unit(&mut map, "SOVIETS", "SMCV", 8, 8);
    let session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "short-base");
    assert!(session.expect_battle().sole_victor().is_none());
}

#[test]
fn short_game_no_buildings_defeats_even_with_tanks() {
    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "short-tanks");
    map.width = 16;
    map.height = 16;
    // 美军有建造场+坦克；苏军只有坦克 → 短局下苏军已出局，美军独活。
    push_structure(&mut map, "AMERICANS", "GACNST", 2, 2);
    push_unit(&mut map, "AMERICANS", "MTNK", 3, 3);
    push_unit(&mut map, "SOVIETS", "MTNK", 8, 8);
    push_unit(&mut map, "SOVIETS", "MTNK", 9, 8);
    let session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "short-tanks");
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn short_game_local_loss_writes_defeat() {
    use crate::common::test_engine;
    use ra_engine::BattleOutcome;

    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "short-defeat");
    map.width = 16;
    map.height = 16;
    // 本地 AMERICANS 只有坦克；苏军有建造场 → 短局下本地已出局。
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_structure(&mut map, "SOVIETS", "NACNST", 8, 8);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "short-defeat");
    assert_eq!(session.expect_battle().sole_victor(), Some("SOVIETS"));
    session.tick(&test_engine().runtime());
    assert_eq!(session.expect_battle().outcome, Some(BattleOutcome::Defeat { reason: String::new() }));
    assert!(session.expect_battle().paused);
}

#[test]
fn savour_delay_defers_outcome_lock() {
    use crate::common::test_engine;
    use ra_engine::BattleOutcome;
    use std::sync::Arc;

    let mut defs = (*defs_with_base_units()).clone();
    defs.savour_delay_ticks = 2;
    let mut map = MapInfo::empty(GameEdition::Ra2, "savour");
    map.width = 16;
    map.height = 16;
    push_structure(&mut map, "AMERICANS", "GACNST", 2, 2);
    push_unit(&mut map, "SOVIETS", "MTNK", 8, 8);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, Arc::new(defs), map), "savour");
    let engine = test_engine();
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());
    assert!(session.expect_battle().pending_savour_outcome.is_some());
    assert!(!session.expect_battle().paused);
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());
    session.tick(&engine.runtime());
    assert_eq!(session.expect_battle().outcome, Some(BattleOutcome::Victory { owner: "AMERICANS".into() }));
    assert!(session.expect_battle().paused);
}

#[test]
fn wall_alone_does_not_keep_house_alive_in_short_game() {
    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "short-wall");
    map.width = 16;
    map.height = 16;
    push_structure(&mut map, "AMERICANS", "GACNST", 2, 2);
    // 苏军只剩围墙：短局下应出局。
    push_structure(&mut map, "SOVIETS", "GAWALL", 8, 8);
    let session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "short-wall");
    assert!(session.expect_battle().short_game);
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn wall_alone_does_not_keep_house_alive_in_long_game() {
    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "long-wall");
    map.width = 16;
    map.height = 16;
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_structure(&mut map, "SOVIETS", "GAWALL", 8, 8);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "long-wall");
    session.expect_battle_mut().set_short_game(false);
    assert_eq!(session.expect_battle().sole_victor(), Some("AMERICANS"));
}

#[test]
fn long_game_wipe_all_mobiles_and_buildings_defeats_local() {
    use crate::common::test_engine;
    use ra_engine::BattleOutcome;

    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "long-wipe");
    map.width = 16;
    map.height = 16;
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_structure(&mut map, "SOVIETS", "NACNST", 8, 8);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "long-wipe");
    session.expect_battle_mut().set_short_game(false);
    let local = session.expect_battle().world.entity_id_at(0).expect("local tank");
    let max = session.expect_battle().world.ecs_health(local).expect("health").1;
    assert!(session.expect_battle_mut().world.set_ecs_health(local, 0, max, true));
    assert_eq!(session.expect_battle().sole_victor(), Some("SOVIETS"));
    session.tick(&test_engine().runtime());
    assert_eq!(session.expect_battle().outcome, Some(BattleOutcome::Defeat { reason: String::new() }));
}

#[test]
fn mutual_wipe_writes_stalemate_defeat() {
    use crate::common::test_engine;
    use ra_engine::BattleOutcome;

    let defs = defs_with_base_units();
    let mut map = MapInfo::empty(GameEdition::Ra2, "stalemate");
    map.width = 16;
    map.height = 16;
    push_unit(&mut map, "AMERICANS", "MTNK", 4, 4);
    push_unit(&mut map, "SOVIETS", "MTNK", 8, 8);
    let mut session = Session::from_state(battle_from_defs(GameEdition::Ra2, defs, map), "stalemate");
    session.expect_battle_mut().set_short_game(false);
    for idx in [0usize, 1] {
        let id = session.expect_battle().world.entity_id_at(idx).expect("unit");
        let max = session.expect_battle().world.ecs_health(id).expect("health").1;
        assert!(session.expect_battle_mut().world.set_ecs_health(id, 0, max, true));
    }
    assert!(session.expect_battle().sole_victor().is_none());
    session.tick(&test_engine().runtime());
    assert_eq!(session.expect_battle().outcome, Some(BattleOutcome::Defeat { reason: "stalemate".into() }));
    assert!(session.expect_battle().paused);
}
