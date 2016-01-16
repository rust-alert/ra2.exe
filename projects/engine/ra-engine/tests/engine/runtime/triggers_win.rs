//! 触发器：计时条件可驱动 Win。

use crate::common::{test_engine};
use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleOutcome, BattleState, Session, SessionBootKind};
use ra_map::MapInfo;
use ra_types::GameEdition;

fn empty_rules() -> RulesSystem {
    let rules = IniDocument::parse(b"[General]\n").unwrap();
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

#[test]
fn timer_trigger_fires_win_on_campaign() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nT1=0,Start,TR1\n\
[Triggers]\nTR1=Americans,<none>,Mission Start,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "trig.map", text).unwrap();
    assert!(!map.scripting.triggers.is_empty());
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "trig");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    for _ in 0..5 {
        session.tick(&engine.runtime());
        if session.expect_battle().outcome.is_some() {
            break;
        }
    }
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn cell_tag_entered_fires_win_on_campaign() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nZONE=0,Zone,TRZ\n\
[Triggers]\nTRZ=Americans,<none>,Enter Zone,0,1,1,1,0\n\
[Events]\nTRZ=1,1,0,0\n\
[Actions]\nTRZ=1,1,0,0,0,0,0,0,Americans\n\
[CellTags]\n5005=ZONE\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "cell.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Americans".into(),
        type_id: "E1".into(),
        health: 256,
        x: 5,
        y: 5,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    assert_eq!(map.scripting.cell_tags.len(), 1);
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "cell");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn destroyed_tagged_entity_fires_win_on_campaign() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nOBJ=0,Objective,TRD\n\
[Triggers]\nTRD=Americans,<none>,Obj Dead,0,1,1,1,0\n\
[Events]\nTRD=1,11,0,0\n\
[Actions]\nTRD=1,1,0,0,0,0,0,0,Americans\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "dead.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Russians".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: "OBJ".into(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "dead");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    // 存活目标时不应立刻胜利。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());

    let id = session
        .expect_battle()
        .world
        .find_entity_id_by_type("NACNST")
        .expect("tagged structure");
    assert!(session.expect_battle_mut().world.set_ecs_health(id, 0, 1, true));

    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn enable_trigger_action_unlocks_disabled_win() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TR1=Americans,<none>,Enable Next,0,1,1,1,0\n\
TR2=Americans,<none>,Win Later,1,1,1,1,0\n\
[Events]\n\
TR1=1,13,0,0\n\
TR2=1,13,0,0\n\
[Actions]\n\
TR1=1,53,0,TR2,0,0,0,0,A\n\
TR2=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "enable.map", text).unwrap();
    assert!(map.scripting.triggers.iter().any(|t| t.id == "TR2" && t.disabled));
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "enable");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none(), "disabled TR2 must not win on first tick");

    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn force_trigger_action_fires_target_win() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TR1=Americans,<none>,Force Win,0,1,1,1,0\n\
TR2=Americans,<none>,Win Body,1,1,1,1,0\n\
[Events]\n\
TR1=1,13,0,0\n\
TR2=1,13,99,0\n\
[Actions]\n\
TR1=1,40,0,TR2,0,0,0,0,A\n\
TR2=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "force.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "force");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn disable_trigger_action_blocks_win_path() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TR1=Americans,<none>,Disable Win,0,1,1,1,0\n\
TR2=Americans,<none>,Win Body,0,1,1,1,0\n\
[Events]\n\
TR1=1,13,0,0\n\
TR2=1,13,10,0\n\
[Actions]\n\
TR1=1,54,0,TR2,0,0,0,0,A\n\
TR2=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "disable.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "disable");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    for _ in 0..5 {
        session.tick(&engine.runtime());
    }
    assert!(
        session.expect_battle().outcome.is_none(),
        "TR1 should disable TR2 before its timer reaches zero"
    );
}

#[test]
fn destroy_trigger_action_blocks_win_path() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TR1=Americans,<none>,Destroy Win,0,1,1,1,0\n\
TR2=Americans,<none>,Win Body,0,1,1,1,0\n\
[Events]\n\
TR1=1,13,0,0\n\
TR2=1,13,10,0\n\
[Actions]\n\
TR1=1,12,0,TR2,0,0,0,0,A\n\
TR2=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "destroy-trig.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "destroy-trig");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    for _ in 0..5 {
        session.tick(&engine.runtime());
    }
    assert!(
        session.expect_battle().outcome.is_none(),
        "TR1 should destroy TR2 so its Win never fires"
    );
}

#[test]
fn change_house_action_reassigns_tagged_entities() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nPlayerControl=no\n\
[Tags]\nOBJ=0,Defector,TRC\n\
[Triggers]\nTRC=Americans,<none>,Change Side,0,1,1,1,0\n\
[Events]\nTRC=1,13,0,0\n\
[Actions]\nTRC=1,14,0,0,0,0,0,0,Americans\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "change-house.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Russians".into(),
        type_id: "E1".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: "OBJ".into(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "change-house");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    session.expect_battle_mut().world.ensure_house("Russians");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    let id = session
        .expect_battle()
        .world
        .find_entity_id_by_type("E1")
        .expect("tagged infantry");
    assert_eq!(
        session.expect_battle().world.ecs_owner(id).as_deref(),
        Some("Russians")
    );

    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().world.ecs_owner(id).as_deref(),
        Some("Americans"),
        "Change House should reassign tagged objects"
    );
}
