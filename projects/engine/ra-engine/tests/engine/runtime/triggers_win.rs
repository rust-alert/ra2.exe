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
fn allow_win_blocks_until_cleared() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TRW=Americans,<none>,Try Win,0,1,1,1,0\n\
TRA=Americans,<none>,Allow,0,1,1,1,0\n\
[Events]\n\
TRW=1,13,0,0\n\
TRA=1,13,2,0\n\
[Actions]\n\
TRW=1,1,0,0,0,0,0,0,Americans\n\
TRA=1,15,0,0,0,0,0,0,A\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "allow-win.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "allow-win");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    // tick0：Win 触发但 Allow Win 阻塞，延后胜利。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());

    // tick1：Allow Win 清阻塞 → 兑现延后胜利。
    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn make_ally_and_make_enemy_update_house_allies() {
    use ra_engine::houses_are_allied;

    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nPlayerControl=no\n\
[Triggers]\n\
TRA=Americans,<none>,Ally,0,1,1,1,0\n\
TRE=Americans,<none>,Enemy,0,1,1,1,0\n\
[Events]\n\
TRA=1,13,0,0\n\
TRE=1,13,2,0\n\
[Actions]\n\
TRA=1,37,0,0,0,0,0,0,Russians\n\
TRE=1,38,0,0,0,0,0,0,Russians\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "ally.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "ally");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    session.expect_battle_mut().world.ensure_house("Russians");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    assert!(!houses_are_allied(&session.expect_battle().world, "Americans", "Russians"));

    session.tick(&engine.runtime());
    assert!(
        houses_are_allied(&session.expect_battle().world, "Americans", "Russians"),
        "action 37 should ally trigger house with param house"
    );

    session.tick(&engine.runtime());
    assert!(
        !houses_are_allied(&session.expect_battle().world, "Americans", "Russians"),
        "action 38 should break the alliance"
    );
}

#[test]
fn all_change_house_reassigns_entire_house() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nPlayerControl=no\n\
[Triggers]\nTRC=Russians,<none>,Defect,0,1,1,1,0\n\
[Events]\nTRC=1,13,0,0\n\
[Actions]\nTRC=1,36,0,0,0,0,0,0,Americans\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "all-house.map", text).unwrap();
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
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Russians".into(),
        type_id: "E1".into(),
        health: 256,
        x: 5,
        y: 5,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "all-house");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    session.expect_battle_mut().world.ensure_house("Russians");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    let owners: Vec<_> = session
        .expect_battle()
        .snapshot(&[])
        .units
        .iter()
        .filter(|u| u.type_id.as_ref() == "E1" && !u.dead)
        .map(|u| u.owner.to_string())
        .collect();
    assert!(!owners.is_empty());
    assert!(
        owners.iter().all(|o| o.eq_ignore_ascii_case("Americans")),
        "action 36 should reassign all entities of the trigger house: {owners:?}"
    );
}

#[test]
fn destroy_all_of_house_kills_living_entities() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nPlayerControl=no\n\
[Triggers]\nTRD=Americans,<none>,Wipe,0,1,1,1,0\n\
[Events]\nTRD=1,13,0,0\n\
[Actions]\nTRD=1,119,0,0,0,0,0,0,Russians\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "wipe.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Russians".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 3,
        y: 3,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
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
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Americans".into(),
        type_id: "E1".into(),
        health: 256,
        x: 6,
        y: 6,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "wipe");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    session.expect_battle_mut().world.ensure_house("Russians");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let russians_alive = snap
        .units
        .iter()
        .filter(|u| u.owner.eq_ignore_ascii_case("Russians") && !u.dead)
        .count();
    let americans_alive = snap
        .units
        .iter()
        .filter(|u| u.owner.eq_ignore_ascii_case("Americans") && !u.dead)
        .count();
    assert_eq!(russians_alive, 0, "action 119 should wipe the target house");
    assert!(americans_alive >= 1, "other houses must remain");
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
TR1=1,22,0,TR2,0,0,0,0,A\n\
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
fn timer_set_action_rearms_target_timer_win() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TR1=Americans,<none>,Arm Timer,0,1,1,1,0\n\
TR2=Americans,<none>,Win Later,0,1,1,1,0\n\
[Events]\n\
TR1=1,13,0,0\n\
TR2=1,13,99,0\n\
[Actions]\n\
TR1=1,27,0,TR2,2,0,0,0,A\n\
TR2=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "timer-set.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "timer-set");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    // tick0：TR1 将 TR2 计时器设为 2（不立刻胜利）。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());

    // tick1：TR2 剩余 1。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());

    // tick2：TR2 归零 → Win。
    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn timer_stop_and_start_pause_countdown() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TRS=Americans,<none>,Stop,0,1,1,1,0\n\
TRG=Americans,<none>,Go,0,1,1,1,0\n\
TRW=Americans,<none>,Win Body,0,1,1,1,0\n\
[Events]\n\
TRS=1,13,0,0\n\
TRG=1,13,3,0\n\
TRW=1,13,2,0\n\
[Actions]\n\
TRS=1,24,0,TRW,0,0,0,0,A\n\
TRG=1,23,0,TRW,0,0,0,0,A\n\
TRW=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "timer-pause.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "timer-pause");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    // tick0：扣减后 TRW rem=1，TRS 暂停；tick1 仍暂停。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());

    // tick2：TRG Start 恢复；tick3：rem 1→0 → Win。
    session.tick(&engine.runtime());
    assert!(session.expect_battle().outcome.is_none());
    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn timer_extend_delays_win() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TRE=Americans,<none>,Extend,0,1,1,1,0\n\
TRW=Americans,<none>,Win Body,0,1,1,1,0\n\
[Events]\n\
TRE=1,13,0,0\n\
TRW=1,13,3,0\n\
[Actions]\n\
TRE=1,25,0,TRW,2,0,0,0,A\n\
TRW=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "timer-ext.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "timer-ext");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    // tick0：TRW 3→2，TRE +2 → rem=4。无 Extend 时约 tick2 胜利；有 Extend 更晚。
    for i in 0..4 {
        session.tick(&engine.runtime());
        assert!(
            session.expect_battle().outcome.is_none(),
            "extend should delay win at tick {i}"
        );
    }
    // rem 4 再经 tick1..4 变为 0：第 5 次 tick（下标 4 之后）触发 Win。
    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}

#[test]
fn timer_shorten_hastens_win() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\n\
TRS=Americans,<none>,Shorten,0,1,1,1,0\n\
TRW=Americans,<none>,Win Body,0,1,1,1,0\n\
[Events]\n\
TRS=1,13,0,0\n\
TRW=1,13,5,0\n\
[Actions]\n\
TRS=1,26,0,TRW,4,0,0,0,A\n\
TRW=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "timer-short.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "timer-short");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    // tick0：TRW 5→4，Shorten -4 → rem=0 → 同 tick 若 TRW 已在 to_fire 则可能未改到；
    // Shorten 在 apply 阶段把 rem 设为 0 并 fired=false，下一 tick 才会 Win。
    session.tick(&engine.runtime());
    if session.expect_battle().outcome.is_none() {
        session.tick(&engine.runtime());
    }
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

#[test]
fn destroy_attached_objects_action_kills_tagged_entities() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nOBJ=0,Target,TRK\n\
[Triggers]\nTRK=Americans,<none>,Kill Tagged,0,1,1,1,0\n\
[Events]\nTRK=1,13,0,0\n\
[Actions]\nTRK=1,32,0,0,0,0,0,0,A\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "destroy-attached.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Russians".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 6,
        y: 6,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: "OBJ".into(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "destroy-attached");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    let id = session
        .expect_battle()
        .world
        .find_entity_id_by_type("NACNST")
        .expect("tagged structure");
    assert_eq!(session.expect_battle().world.ecs_health(id).map(|(_, _, d)| d), Some(false));

    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().world.ecs_health(id).map(|(_, _, d)| d),
        Some(true),
        "action 32 should destroy Tag-bound objects"
    );
}

#[test]
fn destroy_tag_action_kills_named_tag_entities() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Triggers]\nTRK=Americans,<none>,Kill Tag,0,1,1,1,0\n\
[Events]\nTRK=1,13,0,0\n\
[Actions]\nTRK=1,70,0,OBJ,0,0,0,0,A\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "destroy-tag.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Russians".into(),
        type_id: "NACNST".into(),
        health: 256,
        x: 6,
        y: 6,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: "OBJ".into(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Russians".into(),
        type_id: "E1".into(),
        health: 256,
        x: 7,
        y: 7,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "destroy-tag");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let obj_dead = snap
        .units
        .iter()
        .find(|u| u.type_id.as_ref() == "NACNST")
        .map(|u| u.dead)
        .unwrap_or(true);
    let e1_alive = snap
        .units
        .iter()
        .any(|u| u.type_id.as_ref() == "E1" && !u.dead);
    assert!(obj_dead, "action 70 should destroy entities with Tag OBJ");
    assert!(e1_alive, "untagged entities must remain");
}

#[test]
fn destroy_team_action_kills_spawned_team_members() {
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5005\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Americans\nScript=\nTaskForce=TF1\nMax=1\n\
[Triggers]\n\
TR1=Americans,<none>,Spawn,0,1,1,1,0\n\
TR2=Americans,<none>,Wipe,0,1,1,1,0\n\
[Events]\n\
TR1=1,13,0,0\n\
TR2=1,13,2,0\n\
[Actions]\n\
TR1=1,4,0,TM1,0,0,0,0,A\n\
TR2=1,5,0,TM1,0,0,0,0,A\n\
";
    let rules = {
        let doc = IniDocument::parse(
            b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Americans\n",
        )
        .unwrap();
        RulesSystem {
            edition: GameEdition::Ra2,
            rules: doc.clone(),
            art: IniDocument::default(),
            overlay_types: OverlayTypeRegistry::default(),
            color_schemes: ColorSchemes::default(),
            countries: CountryRegistry::default(),
            techno_types: TechnoTypeRegistry::from_rules(&doc),
            warheads: WarheadRegistry::default(),
        }
    };
    let map = MapInfo::parse_ini(GameEdition::Ra2, "destroy-team.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules, map), "destroy-team");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    session.tick(&engine.runtime());
    let living = session
        .expect_battle()
        .snapshot(&[])
        .units
        .iter()
        .filter(|u| u.type_id.as_ref() == "E1" && !u.dead)
        .count();
    assert!(living >= 1, "Create Team should spawn members before Destroy Team");

    session.tick(&engine.runtime());
    let living_after = session
        .expect_battle()
        .snapshot(&[])
        .units
        .iter()
        .filter(|u| u.type_id.as_ref() == "E1" && !u.dead)
        .count();
    assert_eq!(living_after, 0, "action 5 should destroy spawned TeamType members");
}

#[test]
fn all_to_hunt_action_orders_house_attack() {
    use ra_map::{MapEntity, MapEntityKind};

    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nPlayerControl=no\n\
[Triggers]\nTRH=Russians,<none>,Hunt,0,1,1,1,0\n\
[Events]\nTRH=1,13,0,0\n\
[Actions]\nTRH=1,6,0,0,0,0,0,0,Russians\n\
";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "hunt.map", text).unwrap();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Russians".into(),
        type_id: "E1".into(),
        health: 256,
        x: 3,
        y: 3,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Infantry,
        owner: "Americans".into(),
        type_id: "E1".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "hunt");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    session.expect_battle_mut().world.ensure_house("Russians");

    let snap = session.expect_battle().snapshot(&[]);
    let russian = snap.units.iter().find(|u| u.owner.as_ref() == "Russians").map(|u| u.id).expect("RU");
    let american = snap.units.iter().find(|u| u.owner.as_ref() == "Americans").map(|u| u.id).expect("US");

    session.tick(&engine.runtime());
    // All to Hunt queues Attack for next tick apply.
    session.tick(&engine.runtime());
    assert_eq!(
        session.expect_battle().world.ecs_attack_state(russian).map(|(t, _)| t),
        Some(Some(american)),
        "action 6 All to Hunt should Attack nearest hostile"
    );
}
