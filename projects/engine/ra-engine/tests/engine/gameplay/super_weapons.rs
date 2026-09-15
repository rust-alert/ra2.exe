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
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nSuperWeapon=LightningStorm\n", )
}

fn sw_world() -> ra_engine::BattleState {
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-cap");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 5,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, sw_defs(), map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    // 预放 GAPILE 耗电；补足供电，否则超武充能在低电下停摆。
    world.players[0].power_output = 200;
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

#[test]
fn order_fire_nuke_applies_weapon_damage_in_spread() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NAMISL\n\
[VehicleTypes]\n0=TGT\n\
[SuperWeaponTypes]\n0=NukeSpecial\n\
[NukeSpecial]\nType=MultiMissile\nRechargeTime=1\nSidebarImage=NUKEICON\nWeapon=NukePayload\n\
[NukePayload]\nDamage=80\nROF=1\nRange=10\nWarhead=NukeWH\n\
[NukeWH]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\nSpread=1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[NAMISL]\nPower=-50\nPowered=yes\nOwner=Americans\nStrength=800\nSight=5\nCost=1000\nTechLevel=1\nSuperWeapon=NukeSpecial\n\
[TGT]\nStrength=200\nSpeed=0\nSight=1\nCost=100\nArmor=none\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-nuke");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "NAMISL".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "SOVIETS".into(),
            type_id: "TGT".into(),
            health: 256,
            x: 8,
            y: 8,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "SOVIETS".into(),
            type_id: "TGT".into(),
            health: 256,
            x: 9,
            y: 8,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "SOVIETS".into(),
            type_id: "TGT".into(),
            health: 256,
            x: 12,
            y: 12,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    let mut session = Session::from_state(world, "sw-nuke");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    assert!(
        session.expect_battle().snapshot_capabilities(&[]).super_weapon_items.iter().any(|i| i.ready && i.enabled),
        "expected ready Nuke"
    );
    let far_before = session.expect_battle().world.ecs_health(session.expect_battle().world.entity_id_at(4).expect("far")).expect("hp").0;
    session.expect_battle_mut().order_fire_super_weapon("NukeSpecial", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    let near = session.expect_battle().world.ecs_health(session.expect_battle().world.entity_id_at(2).expect("near")).expect("hp").0;
    let adjacent = session.expect_battle().world.ecs_health(session.expect_battle().world.entity_id_at(3).expect("adj")).expect("hp").0;
    let far = session.expect_battle().world.ecs_health(session.expect_battle().world.entity_id_at(4).expect("far")).expect("hp").0;
    assert_eq!(near, 120, "center victim should take 80 damage");
    assert_eq!(adjacent, 120, "spread=1 victim should take 80 damage");
    assert_eq!(far, far_before, "out-of-spread victim untouched");
}

#[test]
fn order_fire_iron_curtain_grants_invulnerability() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=NAIRON\n\
[VehicleTypes]\n0=MTNK\n\
[SuperWeaponTypes]\n0=IronCurtain\n\
[IronCurtain]\nType=IronCurtain\nRechargeTime=1\nSidebarImage=IRONICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[NAIRON]\nPower=-50\nPowered=yes\nOwner=Americans\nStrength=800\nSight=5\nCost=1000\nTechLevel=1\nSuperWeapon=IronCurtain\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\n\
[Gun]\nDamage=40\nROF=2\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-iron");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "NAIRON".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "AMERICANS".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 8,
            y: 8,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "SOVIETS".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 9,
            y: 8,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    let mut session = Session::from_state(world, "sw-iron");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    session.expect_battle_mut().order_fire_super_weapon("IronCurtain", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());

    let ally = session.expect_battle().world.entity_id_at(2).expect("ally");
    let foe = session.expect_battle().world.entity_id_at(3).expect("foe");
    let before_ally = session.expect_battle().world.ecs_health(ally).expect("hp").0;
    session.expect_battle_mut().world.push_command(ra_engine::GameCommand::Attack { attacker: foe, target: ally });
    for _ in 0..20 {
        session.expect_battle_mut().world.advance_tick();
    }
    assert_eq!(
        session.expect_battle().world.ecs_health(ally).expect("hp").0,
        before_ally,
        "allied tank under IronCurtain must ignore incoming damage"
    );
}

#[test]
fn order_fire_paradrop_spawns_payload_infantry() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=GAPILL\n\
[InfantryTypes]\n0=E1\n1=E2\n\
[SuperWeaponTypes]\n0=ParaDrop\n\
[ParaDrop]\nType=ParaDrop\nRechargeTime=1\nSidebarImage=PARAICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPILL]\nPower=-20\nPowered=yes\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nSuperWeapon=ParaDrop\n\
[E1]\nStrength=125\nSpeed=64\nSight=5\nCost=100\nArmor=none\n\
[E2]\nStrength=125\nSpeed=64\nSight=5\nCost=100\nArmor=none\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-para");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPILL".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    assert!(!world.definitions.paradrop.payload.is_empty());
    let before = (0..64).filter_map(|i| world.entity_id_at(i)).count();
    let mut session = Session::from_state(world, "sw-para");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    session.expect_battle_mut().order_fire_super_weapon("ParaDrop", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    let after = (0..64).filter_map(|i| session.expect_battle().world.entity_id_at(i)).count();
    assert!(after > before, "paradrop should spawn infantry: before={before} after={after}");
}

#[test]
fn order_fire_chronosphere_arms_then_teleports_units() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=GACHRON\n\
[VehicleTypes]\n0=MTNK\n\
[SuperWeaponTypes]\n0=ChronoSphere\n\
[ChronoSphere]\nType=ChronoSphere\nRechargeTime=1\nSidebarImage=CHRONICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GACHRON]\nPower=-50\nPowered=yes\nOwner=Americans\nStrength=800\nSight=5\nCost=1000\nTechLevel=1\nSuperWeapon=ChronoSphere\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=none\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-chrono");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GACHRON".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "AMERICANS".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    let mut session = Session::from_state(world, "sw-chrono");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    session.expect_battle_mut().order_fire_super_weapon("ChronoSphere", 4, 4);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    assert!(!session.expect_battle().world.chronosphere_arms.is_empty(), "first click should arm source");
    let caps = session.expect_battle().snapshot_capabilities(&[]);
    let item = caps.super_weapon_items.iter().find(|i| i.type_id.as_ref() == "CHRONOSPHERE").expect("sw");
    assert!(item.ready, "arming must not consume charge");

    session.expect_battle_mut().order_fire_super_weapon("ChronoSphere", 12, 12);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    assert!(session.expect_battle().world.chronosphere_arms.is_empty());
    let tank = session.expect_battle().world.entity_id_at(2).expect("tank");
    let xf = session.expect_battle().world.ecs_transform(tank).expect("xf");
    assert!((xf.0 as i32 - 12).abs() <= 2 && (xf.1 as i32 - 12).abs() <= 2, "tank should teleport near dest: {:?}", xf);
    let after = session.expect_battle().snapshot_capabilities(&[]);
    let item = after.super_weapon_items.iter().find(|i| i.type_id.as_ref() == "CHRONOSPHERE").expect("sw");
    assert!(!item.ready, "warp should consume charge");
}

#[test]
fn order_fire_reveal_marks_cells_and_radar() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=GASPY\n\
[SuperWeaponTypes]\n0=Reveal\n\
[Reveal]\nType=Reveal\nRechargeTime=1\nSidebarImage=REVEALICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GASPY]\nPower=-20\nPowered=yes\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nSuperWeapon=Reveal\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-reveal");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GASPY".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    let mut session = Session::from_state(world, "sw-reveal");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    session.expect_battle_mut().order_fire_super_weapon("Reveal", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    let revealed = session.expect_battle().world.house_reveal.revealed_count("AMERICANS");
    assert!(revealed > 1, "reveal should mark a disk of cells: {revealed}");
    assert!(session.expect_battle().world.house_reveal.is_revealed("AMERICANS", 8, 8));
    assert_eq!(session.expect_battle().world.last_radar_event_cell("AMERICANS"), Some((8, 8)));
}

#[test]
fn order_fire_spy_plane_reveals_with_aircraft_radius() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=GASPPL\n\
[SuperWeaponTypes]\n0=SpyPlane\n\
[SpyPlane]\nType=SpyPlane\nRechargeTime=1\nSidebarImage=SPYPICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GASPPL]\nPower=-20\nPowered=yes\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\nSuperWeapon=SpyPlane\n\
[General]\nAircraftFogReveal=2\nRevealTriggerRadius=9\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-spy");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GASPPL".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    assert_eq!(world.definitions.reveal.aircraft_radius_cells, 2);
    let mut session = Session::from_state(world, "sw-spy");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    session.expect_battle_mut().order_fire_super_weapon("SpyPlane", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    // 半径 2 → 5x5 = 25 格。
    assert_eq!(session.expect_battle().world.house_reveal.revealed_count("AMERICANS"), 25);
    assert_eq!(session.expect_battle().world.last_radar_event_cell("AMERICANS"), Some((8, 8)));
}

#[test]
fn order_fire_amer_paradrop_uses_americans_payload_count() {
    let defs = defs_from_rules_ini(
        b"[BuildingTypes]\n0=GACNST\n1=GAWETH\n\
[InfantryTypes]\n0=E1\n\
[SuperWeaponTypes]\n0=AmerParaDrop\n\
[AmerParaDrop]\nType=AmerParaDrop\nRechargeTime=1\nSidebarImage=PARAICON\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAWETH]\nPower=-50\nPowered=yes\nOwner=Americans\nStrength=800\nSight=5\nCost=1000\nTechLevel=1\nSuperWeapon=AmerParaDrop\n\
[E1]\nStrength=125\nSight=5\nCost=100\nArmor=none\n\
[General]\nAmerParaDropInf=E1\nAmerParaDropNum=4\n",
    );
    let mut map = MapInfo::empty(GameEdition::Ra2, "sw-amerpara");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
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
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAWETH".into(),
            health: 256,
            x: 2,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    world.players[0].power_output = 200;
    assert_eq!(world.definitions.paradrop.americans.len(), 4);
    let before = (0..64).filter_map(|i| world.entity_id_at(i)).count();
    let mut session = Session::from_state(world, "sw-amerpara");
    for _ in 0..SUPER_WEAPON_TICKS_PER_RECHARGE_UNIT {
        session.expect_battle_mut().world.advance_tick();
    }
    session.expect_battle_mut().order_fire_super_weapon("AmerParaDrop", 8, 8);
    session.expect_battle_mut().world.advance_tick();
    assert!(session.expect_battle().world.last_rejects().is_empty(), "{:?}", session.expect_battle().world.last_rejects());
    let after = (0..64).filter_map(|i| session.expect_battle().world.entity_id_at(i)).count();
    assert_eq!(after, before + 4, "amer paradrop should spawn AmerParaDropNum infantry");
}
