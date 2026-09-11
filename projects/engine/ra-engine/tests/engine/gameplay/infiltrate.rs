//! 间谍渗透：Agent 邻接敌建筑后结算并阵亡。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

fn spy_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(b"[Countries]\n0=Americans\n1=Russians\n\
[Americans]\nSide=GDI\nMultiplay=yes\n\
[Russians]\nSide=Nod\nMultiplay=yes\n\
[General]\nPrerequisiteTech=GATECH,NATECH\n\
[InfantryTypes]\n0=SPY\n1=E1\n2=SEAL\n\
[BuildingTypes]\n0=GAPOWR\n1=GAREFN\n2=GAPILE\n3=GACNST\n4=GATECH\n5=NATECH\n\
[SPY]\nAgent=yes\nOwner=Americans\nStrength=100\nSpeed=32\nSight=4\nCost=1000\nTechLevel=1\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=24\nSight=4\nCost=200\nTechLevel=1\n\
[SEAL]\nOwner=Americans\nStrength=200\nSpeed=24\nSight=6\nCost=1000\nTechLevel=1\nRequiresStolenSovietTech=yes\n\
[GAPOWR]\nPower=200\nOwner=Americans,Russians\nStrength=750\nSight=4\nCost=800\nTechLevel=1\n\
[GAREFN]\nRefinery=yes\nOwner=Americans,Russians\nStrength=1000\nSight=4\nCost=2000\nTechLevel=1\n\
[GAPILE]\nFactory=InfantryType\nOwner=Americans,Russians\nStrength=600\nSight=5\nCost=500\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans,Russians\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GATECH]\nOwner=Americans\nStrength=500\nSight=6\nCost=2000\nTechLevel=1\n\
[NATECH]\nOwner=Russians\nStrength=500\nSight=6\nCost=2000\nTechLevel=1\n")
}

fn spy_world(spy_x: u16, spy_y: u16, building_type: &str, bx: u16, by: u16) -> BattleState {
    let defs = spy_defs();
    let mut map = MapInfo::empty(GameEdition::Ra2, "spy-infiltrate");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Infantry,
            owner: "Americans".into(),
            type_id: "SPY".into(),
            health: 256,
            x: spy_x,
            y: spy_y,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Russians".into(),
            type_id: building_type.into(),
            health: 256,
            x: bx,
            y: by,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 1,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    world.set_all_players_funds(10_000);
    // 受害方有耗电，便于验证断电后低电。
    if let Some(p) = world.players.iter_mut().find(|p| p.house.as_ref() == "Russians") {
        p.power_output = 200;
        p.power_drain = 50;
    }
    world
}

#[test]
fn non_agent_cannot_infiltrate() {
    let mut world = spy_world(4, 4, "GAPOWR", 5, 4);
    let spy = world.entity_id_at(0).expect("spy");
    assert!(world.set_ecs_type_id(spy, "E1", MapEntityKind::Infantry));
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::Infiltrate { agent: spy, building });
    world.advance_tick();
    assert!(world.last_rejects().iter().any(|r| r.reason == CommandRejectReason::InvalidTarget));
}

#[test]
fn spy_infiltrates_power_plant_and_dies() {
    let mut world = spy_world(4, 4, "GAPOWR", 5, 4);
    let spy = world.entity_id_at(0).expect("spy");
    let building = world.entity_id_at(1).expect("building");
    world.push_command(GameCommand::Infiltrate { agent: spy, building });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // 已邻接：本 tick combat 阶段应结算。
    assert!(world.ecs_get_health_dead(spy));
    let victim = world.players.iter().find(|p| p.house.as_ref() == "Russians").expect("victim");
    assert!(victim.power_blackout_ticks > 0);
    assert!(victim.low_power());
}

#[test]
fn spy_steals_funds_from_refinery() {
    let mut world = spy_world(4, 4, "GAREFN", 5, 4);
    let spy = world.entity_id_at(0).expect("spy");
    let building = world.entity_id_at(1).expect("building");
    let ally_before = world.house_funds("Americans").unwrap_or(0);
    let victim_before = world.house_funds("Russians").unwrap_or(0);
    world.push_command(GameCommand::Infiltrate { agent: spy, building });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert!(world.ecs_get_health_dead(spy));
    let ally_after = world.house_funds("Americans").unwrap_or(0);
    let victim_after = world.house_funds("Russians").unwrap_or(0);
    assert!(ally_after > ally_before);
    assert!(victim_after < victim_before);
    assert_eq!(ally_after - ally_before, victim_before - victim_after);
}

#[test]
fn spy_infiltrates_barracks_promotes_infantry() {
    let mut world = spy_world(4, 4, "GAPILE", 5, 4);
    // 敌方兵营在 (5,4)；己方也有兵营便于生产。
    let spy = world.entity_id_at(0).expect("spy");
    let enemy_pile = world.entity_id_at(1).expect("enemy pile");
    world.push_command(GameCommand::Infiltrate { agent: spy, building: enemy_pile });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    let americans = world.players.iter().find(|p| p.house.as_ref() == "Americans").expect("ally");
    assert!(americans.promoted_infantry);

    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "E1".into() });
    // 推进生产完成。
    let mut produced = None;
    for _ in 0..600 {
        world.advance_tick();
        produced = world.entity_ids().into_iter().find(|&id| {
            world.ecs_health(id).is_some_and(|(_, _, dead)| !dead)
                && world.ecs_identity(id).is_some_and(|(t, k)| k == MapEntityKind::Infantry && t.as_ref() == "E1")
        });
        if produced.is_some() {
            break;
        }
    }
    let e1 = produced.expect("produced E1");
    let (_, max, _) = world.ecs_health(e1).expect("health");
    assert!(max > 125, "promoted E1 should exceed base strength 125, got {max}");
}

#[test]
fn spy_infiltrates_soviet_lab_grants_stolen_tech_for_seal() {
    let mut world = spy_world(4, 4, "NATECH", 5, 4);
    let spy = world.entity_id_at(0).expect("spy");
    let lab = world.entity_id_at(1).expect("lab");

    // 未偷科技前 SEAL 不可生产。
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "SEAL".into() });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);

    world.push_command(GameCommand::Infiltrate { agent: spy, building: lab });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    let americans = world.players.iter().find(|p| p.house.as_ref() == "Americans").expect("ally");
    assert!(americans.stolen_soviet_tech);

    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: "SEAL".into() });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "SEAL should queue after stolen soviet tech: {:?}", world.last_rejects());
}

/// 测试辅助：实体是否已死亡。
trait HealthDeadExt {
    fn ecs_get_health_dead(&self, id: EntityId) -> bool;
}

impl HealthDeadExt for BattleState {
    fn ecs_get_health_dead(&self, id: EntityId) -> bool {
        self.ecs_health(id).map(|(_, _, dead)| dead).unwrap_or(true)
    }
}
