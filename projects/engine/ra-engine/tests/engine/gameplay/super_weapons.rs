//! 超武充能进能力快照。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{CommandRejectReason, SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn sw_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(b"[BuildingTypes]\n0=GACNST\n1=GAPILE\n\
[SuperWeaponTypes]\n0=LightningStorm\n\
[LightningStorm]\nUIName=NAME:LS\nType=LightningStorm\nRechargeTime=1\nSidebarImage=SSWLSICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nSuperWeapon=LightningStorm\n",)
}

fn sw_world() -> ra_engine::BattleState {
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-cap");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Americans".into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 4,
            y: 4,
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
            x: 5,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, sw_defs(), map);
    assert!(world.set_house_funds("Americans", 10_000));
    world
}

#[test]
fn capabilities_project_super_weapon_charge_progress() {
    let mut session = Session::from_state(sw_world(), "sw");
    let caps0 = session.expect_battle().snapshot_capabilities(&[]);
    assert_eq!(caps0.super_weapon_items.len(), 1, "{:?}", caps0.super_weapon_items);
    let item0 = &caps0.super_weapon_items[0];
    assert_eq!(item0.type_id.as_ref(), "LIGHTNINGSTORM");
    assert_eq!(item0.sidebar_image.as_ref(), "SSWLSICON");
    assert_eq!(item0.required_ticks, SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT);
    assert_eq!(item0.charge_ticks, 0);
    assert!(!item0.ready);
    assert!(!item0.enabled);
    assert_eq!(item0.disabled_reason, Some(CommandRejectReason::SuperWeaponNotReady));

    // 推进到即将就绪的前一 tick。
    for _ in 0..(SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT - 1) {
        session.expect_battle_mut().world.advance_tick();
    }
    let caps_mid = session.expect_battle().snapshot_capabilities(&[]);
    let mid = &caps_mid.super_weapon_items[0];
    assert_eq!(mid.charge_ticks, SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT - 1);
    assert!(!mid.ready);

    session.expect_battle_mut().world.advance_tick();
    let caps_ready = session.expect_battle().snapshot_capabilities(&[]);
    let ready = &caps_ready.super_weapon_items[0];
    assert!(ready.ready);
    assert!(ready.enabled);
    assert_eq!(ready.disabled_reason, None);
}

#[test]
fn order_fire_super_weapon_starts_lightning_storm() {
    let mut session = Session::from_state(sw_world(), "sw-fire");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    assert!(
        session.expect_battle().snapshot_capabilities(&[]).super_weapon_items.iter().any(|i| i.ready && i.enabled),
        "expected ready LightningStorm"
    );

    session.expect_battle_mut().order_fire_super_weapon("LightningStorm", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    assert!(session.expect_battle().world.lightning_storm.is_some());

    let after = session.expect_battle().snapshot_capabilities(&[]);
    let item = after.super_weapon_items.iter().find(|i| i.type_id.as_ref() == "LIGHTNINGSTORM").expect("sw item");
    assert!(!item.ready);
    assert!(item.charge_ticks < item.required_ticks);
    assert_eq!(item.disabled_reason, Some(CommandRejectReason::SuperWeaponNotReady));
}
