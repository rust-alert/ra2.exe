//! 动作码 3：Production Begins 打开指定 house 的 AI 生产。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{PRODUCE_TICKS, Session, SessionBootKind};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn production_begins_action_enables_campaign_ai_produce() {
    let defs = defs_from_rules_ini(
        b"[InfantryTypes]\n0=E2\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAHAND\n\
[E2]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nTechLevel=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Soviets\nStrength=500\nSight=5\nCost=500\nArmor=wood\nTechLevel=1\n",
    );
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Soviets\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Soviets]\nCountry=Soviets\nPlayerControl=no\n\
[Triggers]\nTR1=Soviets,<none>,BeginProd,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,3,0,0,0,0,0,0,Soviets\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "prod-begin.map", text).unwrap();
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "NAPOWR".into(),
        health: 256,
        x: 9,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });

    let engine = test_engine();
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    world.ensure_house("AMERICANS");
    world.ensure_house("SOVIETS");
    let _ = world.prefer_local_house("AMERICANS");
    assert!(world.set_house_funds("SOVIETS", 10_000));
    for player in &mut world.players {
        player.production_begun = false;
    }
    assert!(!world.house_production_begun("SOVIETS"));

    let mut session = Session::from_state(world, "prod-begin");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().ai_enabled = false;

    // 首 tick：计时事件触发 Production Begins，尚不一定完成落兵营。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().world.house_production_begun("SOVIETS"), "action 3 must open Soviet production");
    assert!(!session.expect_battle().world.house_production_begun("AMERICANS"));

    for _ in 0..(PRODUCE_TICKS + 8) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAHAND").is_some() {
            break;
        }
    }
    assert!(
        session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAHAND").is_some(),
        "campaign AI should place barracks after Production Begins"
    );
}

#[test]
fn production_begins_is_not_campaign_blocking_gap() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Triggers]\nTR1=Soviets,<none>,X,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,3,0,0,0,0,0,0,Soviets\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "gap.map", text).unwrap();
    let gaps = ra_map::map_scripting_capability_gaps(&map);
    assert!(gaps.iter().all(|g| g.code != "map.action.3 unsupported"), "{gaps:?}");
    assert!(ra_map::campaign_blocking_capability_message(&map).is_none());
}
