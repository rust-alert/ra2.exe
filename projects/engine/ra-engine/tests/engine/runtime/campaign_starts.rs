//! 战役开局：保留预放机动，不种席位 MCV。

use ra_adaptor::{ResourceChain, RulesSystem};
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
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
    map.waypoints = vec![
        Waypoint { index: 0, x: 4, y: 4 },
        Waypoint { index: 1, x: 20, y: 20 },
    ];
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
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
    });
    map
}

struct RulesBytesSource;
impl AssetSource for RulesBytesSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        let chain = ResourceChain::for_edition(GameEdition::Ra2);
        if relative.eq_ignore_ascii_case(chain.rules_ini) {
            Ok(b"[General]\n".to_vec())
        } else {
            Err(RaError::MissingFile(relative.to_string()))
        }
    }
}

#[test]
fn open_campaign_keeps_preplaced_mobiles_and_skips_mcv_seed() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_campaign_session(
        &RulesBytesSource,
        &chain,
        &mcv_rules(),
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
fn open_skirmish_still_strips_when_campaign_path_exists() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_skirmish_session(
        &RulesBytesSource,
        &chain,
        &mcv_rules(),
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
