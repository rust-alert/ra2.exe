//! MCV / `Deployer` 部署与资金播种。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

fn mcv_world() -> BattleState {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=AMCV\n1=MTNK\n\
[BuildingTypes]\n0=GACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "mcv-deploy");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "AMCV".into(),
        health: 256,
        x: 5,
        y: 5,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    battle_from_defs(GameEdition::Ra2, defs, map)
}

fn gi_world() -> BattleState {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nDeployer=yes\nOwner=Americans\nStrength=125\nSpeed=4\nSight=5\nCost=200\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "gi-deploy");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "AMERICANS".into(),
        type_id: "E1".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    battle_from_defs(GameEdition::Ra2, defs, map)
}

fn undeploy_vehicle_world() -> BattleState {
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=SREF\n1=SREFDEP\n\
[SREF]\nOwner=Americans\nStrength=600\nSpeed=4\nSight=6\nCost=1200\n\
[SREFDEP]\nUndeploysInto=SREF\nOwner=Americans\nStrength=600\nSpeed=0\nSight=8\nCost=1200\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "undeploy-vehicle");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "SREFDEP".into(),
        health: 256,
        x: 6,
        y: 6,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    battle_from_defs(GameEdition::Ra2, defs, map)
}

#[test]
fn set_house_funds_updates_player_state() {
    let mut world = mcv_world();
    assert_eq!(world.house_funds("AMERICANS"), Some(0));
    assert!(world.set_house_funds("AMERICANS", 10_000));
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000));
    assert!(!world.set_house_funds("Nobody", 1));
}

#[test]
fn set_all_players_funds_seeds_every_house() {
    let mut world = mcv_world();
    world.set_all_players_funds(8_000);
    assert_eq!(world.house_funds("AMERICANS"), Some(8_000));
    for player in &world.players {
        assert_eq!(player.funds, 8_000);
    }
}

#[test]
fn deploy_mcv_becomes_construction_yard() {
    let mut world = mcv_world();
    let id = world.entity_id_at(0).expect("entity");
    world.push_command(GameCommand::Deploy { entity: EntityId(1) });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.entity_id_at(0).expect("entity"), id);
    assert_eq!(world.entity_id_at(0).expect("entity"), EntityId(1));
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").1, MapEntityKind::Structure);
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").0.as_ref(), "GACNST");
    assert_eq!(world.ecs_speed(world.entity_id_at(0).expect("entity")).expect("speed"), 0);
    assert!(world.ecs_attack_state(world.entity_id_at(0).expect("entity")).expect("atk").0.is_none());
    assert!(world.take_eva_cues().iter().all(|c| c.event != "EVA_UnitLost"), "Deploy must not announce unit lost");
}

#[test]
fn deploy_rejects_non_mcv_unit() {
    let mut world = mcv_world();
    let id = world.entity_id_at(0).expect("entity");
    assert!(world.set_ecs_type_id(id, "MTNK", MapEntityKind::Unit));
    world.push_command(GameCommand::Deploy { entity: EntityId(1) });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::CannotDeploy);
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").1, MapEntityKind::Unit);
    assert_eq!(world.ecs_identity(world.entity_id_at(0).expect("entity")).expect("id").0.as_ref(), "MTNK");
}

#[test]
fn deploy_gi_toggles_deployer_stance() {
    let mut world = gi_world();
    let id = world.entity_id_at(0).expect("entity");
    assert_eq!(world.ecs_deployed(id), Some(false));
    assert_eq!(world.ecs_speed(id).expect("speed"), 4);

    world.push_command(GameCommand::Deploy { entity: id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_deployed(id), Some(true));
    assert_eq!(world.ecs_speed(id).expect("speed"), 0);
    assert_eq!(world.ecs_identity(id).expect("id").0.as_ref(), "E1");
    assert_eq!(world.ecs_identity(id).expect("id").1, MapEntityKind::Infantry);

    world.push_command(GameCommand::Deploy { entity: id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_deployed(id), Some(false));
    assert_eq!(world.ecs_speed(id).expect("speed"), 4);
}

#[test]
fn move_clears_deployer_stance() {
    let mut world = gi_world();
    let id = world.entity_id_at(0).expect("entity");
    world.push_command(GameCommand::Deploy { entity: id });
    world.advance_tick();
    assert_eq!(world.ecs_deployed(id), Some(true));

    world.push_command(GameCommand::MoveTo { entity: id, x: 8, y: 8 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_deployed(id), Some(false));
    assert_eq!(world.ecs_speed(id).expect("speed"), 4);
}

#[test]
fn undeploys_into_converts_back_to_mobile_type() {
    let mut world = undeploy_vehicle_world();
    let id = world.entity_id_at(0).expect("entity");
    assert_eq!(world.ecs_identity(id).expect("id").0.as_ref(), "SREFDEP");

    world.push_command(GameCommand::Deploy { entity: id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.ecs_identity(id).expect("id").0.as_ref(), "SREF");
    assert_eq!(world.ecs_identity(id).expect("id").1, MapEntityKind::Unit);
    assert_eq!(world.ecs_speed(id).expect("speed"), 4);
    assert_eq!(world.ecs_deployed(id), Some(false));
}
