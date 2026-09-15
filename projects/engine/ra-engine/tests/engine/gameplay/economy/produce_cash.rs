//! 油田等 `ProduceCash*` 周期产钱。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn oil_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[Countries]\n0=Americans\n1=Neutral\n\
[Americans]\nSide=GDI\nMultiplay=yes\n\
[Neutral]\nSide=Civilian\nMultiplay=no\n\
[BuildingTypes]\n0=CAOILD\n\
[CAOILD]\nCapturable=yes\nFoundation=2x2\nProduceCashStartup=1000\nProduceCashAmount=20\nProduceCashDelay=50\n\
Owner=Americans,Neutral\nStrength=1000\nSight=4\nCost=1500\nTechLevel=-1\n",
    )
}

fn oil_world(owner: &str) -> ra_engine::BattleState {
    let defs = oil_defs();
    let mut map = MapInfo::empty(GameEdition::Ra2, "produce-cash");
    map.width = 12;
    map.height = 12;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: owner.into(),
        type_id: "CAOILD".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    // 地图仅 Neutral 实体时不会播种美国人席位；显式登记以便断言资金不变。
    world.ensure_house("AMERICANS");
    assert!(world.set_house_funds("AMERICANS", 100));
    world
}

#[test]
fn player_owned_oil_pays_periodic_cash_without_startup() {
    let mut world = oil_world("AMERICANS");
    for _ in 0..49 {
        world.advance_tick();
        assert_eq!(world.house_funds("AMERICANS"), Some(100));
    }
    world.advance_tick();
    assert_eq!(world.house_funds("AMERICANS"), Some(120));
}

#[test]
fn neutral_owned_oil_does_not_pay_periodic_cash() {
    let mut world = oil_world("NEUTRAL");
    for _ in 0..120 {
        world.advance_tick();
    }
    assert_eq!(world.house_funds("AMERICANS"), Some(100));
}

#[test]
fn powered_cash_building_stops_under_low_power() {
    let defs = defs_from_rules_ini(
        b"[Countries]\n0=Americans\n\
[Americans]\nSide=GDI\nMultiplay=yes\n\
[BuildingTypes]\n0=GACASH\n\
[GACASH]\nPower=-40\nPowered=yes\nFoundation=2x2\nProduceCashAmount=20\nProduceCashDelay=50\n\
Owner=Americans\nStrength=800\nSight=4\nCost=1500\nTechLevel=1\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "cash-low-power");
    map.width = 12;
    map.height = 12;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACASH".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 100));
    // 播种后耗电 40、供电 0 → 低电，周期产钱应停。
    assert!(world.players[0].low_power());
    for _ in 0..120 {
        world.advance_tick();
    }
    assert_eq!(world.house_funds("AMERICANS"), Some(100));

    world.players[0].power_output = 200;
    assert!(!world.players[0].low_power());
    for _ in 0..50 {
        world.advance_tick();
    }
    assert_eq!(world.house_funds("AMERICANS"), Some(120));
}
