//! AI 矿场落点优先靠近可采矿。

use crate::common::{battle_from_defs, defs_from_rules_ini, test_engine};
use ra_engine::{PRODUCE_TICKS, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo, OverlayCell};
use ra_types::GameEdition;

#[test]
fn ai_places_refinery_closer_to_ore_than_opposite_side() {
    let engine = test_engine();
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAREFN\n\
[OverlayTypes]\n0=TIB01\n\
[TIB01]\nTiberium=yes\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Soviets\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\nFoundation=2x2\n\
[NAPOWR]\nPower=200\nOwner=Soviets\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\nFoundation=2x2\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Soviets\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\nFoundation=3x3\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-refn-ore");
    map.width = 24;
    map.height = 24;
    // 矿在建造场东侧远处。
    map.overlays = vec![OverlayCell { x: 20, y: 9, overlay_id: 0, data: 4 }];
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
        x: 8,
        y: 11,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("SOVIETS", 10_000));
    let mut session = Session::from_state(world, "ai-refn-ore");
    session.expect_battle_mut().ai_enabled = true;
    for _ in 0..(PRODUCE_TICKS + 8) {
        session.tick(&engine.runtime());
        if session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAREFN").is_some() {
            break;
        }
    }
    let refn = session.expect_battle().world.find_entity_id_by_owner_type("SOVIETS", "NAREFN").expect("refinery");
    let (rx, ry, _) = session.expect_battle().world.ecs_transform(refn).expect("xf");
    // 3x3 中心应更靠近东侧矿（x=20），而不是西侧空地。
    let cx = rx + 1;
    assert!(cx >= 10, "refinery should lean toward eastern ore, got anchor=({rx},{ry}) center_x={cx}");
    // 贴矿不得压到矿格本身。
    assert!(
        !(rx <= 20 && 20 < rx + 3 && ry <= 9 && 9 < ry + 3),
        "refinery footprint must not cover ore at (20,9), got anchor=({rx},{ry})"
    );
}
