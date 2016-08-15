//! 战役开局：保留预放机动，不种席位 MCV。
use std::sync::Arc;

use ra_adaptor::{ResourceChain, RulesSystem, build_runtime_definitions};
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{SessionBootKind, open_campaign_session, open_skirmish_session};
use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

fn mcv_rules() -> RulesSystem {
    let rules = IniDocument::parse(
        b"[VehicleTypes]\n0=AMCV\n1=SMCV\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Russians\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\n",
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

fn campaign_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "camp");
    map.width = 32;
    map.height = 32;
    map.waypoints = vec![Waypoint { index: 0, x: 4, y: 4 }, Waypoint { index: 1, x: 20, y: 20 }];
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 8,
        y: 8,
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
        x: 9,
        y: 9,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Russians".into(),
        type_id: "HTNK".into(),
        health: 256,
        x: 12,
        y: 12,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map
}

struct RulesBytesSource;
impl AssetSource for RulesBytesSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        let chain = ResourceChain::for_edition(GameEdition::Ra2);
        if relative.eq_ignore_ascii_case(chain.rules_ini) {
            Ok(b"[General]\n".to_vec())
        }
        else {
            Err(RaError::MissingFile(relative.to_string()))
        }
    }
}

#[test]
fn open_campaign_keeps_preplaced_mobiles_and_skips_mcv_seed() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())),
        campaign_map(),
        "t".into(),
        (0, 0),
        Some("Americans"),
        &["Americans"],
        0,
    )
    .expect("战役应成功开局");
    assert!(opened.note.contains("campaign"), "{}", opened.note);
    assert!(!opened.note.contains("strip_mobiles"), "{}", opened.note);
    assert!(!opened.note.contains("starts=["), "{}", opened.note);
    let battle = opened.session.expect_battle();
    assert_eq!(battle.boot_kind, SessionBootKind::Campaign);
    let snap = battle.snapshot(&[]);
    let type_ids: Vec<_> = snap.units.iter().map(|u| u.type_id.as_ref().to_string()).collect();
    assert!(type_ids.iter().any(|t| t == "E1"), "{type_ids:?}");
    assert!(type_ids.iter().any(|t| t == "HTNK"), "{type_ids:?}");
    assert!(type_ids.iter().any(|t| t == "GACNST"), "{type_ids:?}");
    assert!(!type_ids.iter().any(|t| t == "AMCV" || t == "SMCV"), "{type_ids:?}");
}

#[test]
fn open_campaign_seeds_placement_mission_on_identity() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Infantry]\n1=Americans,E1,256,3,3,0,Guard,32\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "mission.map", text).unwrap();
    assert_eq!(map.entities[0].mission, "Guard");
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())), map, "t".into(), (0, 0), Some("Americans"), &["Americans"], 0)
        .expect("战役应成功开局");
    let world = &opened.session.expect_battle().world;
    let id = world.find_entity_id_by_type("E1").expect("E1");
    assert_eq!(world.ecs_mission(id).as_deref(), Some("Guard"));
}

#[test]
fn open_skirmish_still_strips_when_campaign_path_exists() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_skirmish_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())),
        campaign_map(),
        "t".into(),
        (0, 0),
        Some("Americans"),
        &["Americans", "Russians"],
        0,
    )
    .expect("遭遇战应成功开局");
    assert!(opened.note.contains("strip_mobiles#2"), "{}", opened.note);
    assert_eq!(opened.session.expect_battle().boot_kind, SessionBootKind::Skirmish);
}

#[test]
fn open_campaign_applies_map_house_credits() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nCredits=40\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nCredits=25\nPlayerControl=no\n\
[Structures]\n1=Americans,GACNST,256,2,2,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "houses.map", text).unwrap();
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())), map, "t".into(), (0, 0), Some("Americans"), &["Americans"], 0)
        .expect("战役应成功开局");
    assert!(opened.note.contains("map_houses#2"), "{}", opened.note);
    let world = &opened.session.expect_battle().world;
    assert_eq!(world.house_funds("Americans"), Some(4_000));
    assert_eq!(world.house_funds("Russians"), Some(2_500));
}

#[test]
fn open_campaign_applies_basic_starting_credits_when_house_credits_absent() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Basic]\nStartingCredits=10000\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nCredits=25\nPlayerControl=no\n\
[Structures]\n1=Americans,GACNST,256,2,2,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "starting.map", text).unwrap();
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())), map, "t".into(), (0, 0), Some("Americans"), &["Americans"], 0)
        .expect("战役应成功开局");
    assert!(opened.note.contains("starting_credits=10000"), "{}", opened.note);
    let world = &opened.session.expect_battle().world;
    assert_eq!(world.house_funds("Americans"), Some(10_000));
    assert_eq!(world.house_funds("Russians"), Some(2_500));
}

#[test]
fn open_campaign_applies_map_house_tech_level() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=Russians\n\
[Americans]\nCountry=Americans\nCredits=10\nTechLevel=5\nPlayerControl=yes\n\
[Russians]\nCountry=Russians\nCredits=10\nTechLevel=3\nPlayerControl=no\n\
[Structures]\n1=Americans,GACNST,256,2,2,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "houses-tech.map", text).unwrap();
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())), map, "t".into(), (0, 0), Some("Americans"), &["Americans"], 0)
        .expect("战役应成功开局");
    let world = &opened.session.expect_battle().world;
    let americans = world.players.iter().find(|p| p.house.as_ref() == "Americans").expect("Americans");
    let russians = world.players.iter().find(|p| p.house.as_ref() == "Russians").expect("Russians");
    assert_eq!(americans.tech_level, 5);
    assert_eq!(russians.tech_level, 3);
}

#[test]
fn open_campaign_applies_map_house_allies() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n1=France\n2=Russians\n\
[Americans]\nCountry=Americans\nCredits=10\nPlayerControl=yes\nAllies=Americans,France\n\
[France]\nCountry=France\nCredits=10\nPlayerControl=no\nAllies=France,Americans\n\
[Russians]\nCountry=Russians\nCredits=10\nPlayerControl=no\n\
[Structures]\n1=Americans,GACNST,256,2,2,0\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "houses-allies.map", text).unwrap();
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())), map, "t".into(), (0, 0), Some("Americans"), &["Americans"], 0)
        .expect("战役应成功开局");
    let world = &opened.session.expect_battle().world;
    let americans = world.players.iter().find(|p| p.house.as_ref() == "Americans").expect("Americans");
    let france = world.players.iter().find(|p| p.house.as_ref() == "France").expect("France");
    assert!(americans.allies.iter().any(|a| a == "France"), "{:?}", americans.allies);
    assert!(france.allies.iter().any(|a| a == "Americans"), "{:?}", france.allies);
    assert!(ra_engine::houses_are_allied(world, "Americans", "France"));
    assert!(!ra_engine::houses_are_allied(world, "Americans", "Russians"));
}
