//! 超武充能进能力快照。

use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{CommandRejectReason, Session, SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

fn sw_rules() -> RulesSystem {
    let rules = IniDocument::parse(
        b"[BuildingTypes]\n0=GACNST\n1=GAPILE\n\
[SuperWeaponTypes]\n0=LightningStorm\n\
[LightningStorm]\nUIName=NAME:LS\nType=LightningStorm\nRechargeTime=1\nSidebarImage=SSWLSICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nSuperWeapon=LightningStorm\n",
    )
    .expect("测试 INI 必须有效");
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    }
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
    let mut world = ra_engine::BattleState::new(GameEdition::Ra2, &sw_rules(), map);
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
