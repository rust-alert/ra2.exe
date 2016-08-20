//! 遭遇战开局：席位航点放置 MCV。
use std::sync::Arc;

use ra_adaptor::{ResourceChain, RulesSystem, build_runtime_definitions};
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::open_skirmish_session;
use ra_map::{MapEntity, MapEntityKind, MapInfo, Waypoint};
use ra_types::{AssetSource, GameEdition, RaError, RaResult, TerrainSpawnerDefinitions};

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
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    }
}

fn map_with_starts() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "starts");
    map.width = 32;
    map.height = 32;
    map.waypoints = vec![Waypoint { index: 0, x: 4, y: 4 }, Waypoint { index: 1, x: 20, y: 20 }];
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
fn open_skirmish_places_mcv_at_seat_waypoints() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let opened = open_skirmish_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())),
        map_with_starts(),
        "t".into(),
        (0, 0),
        Some("Americans"),
        &["Americans", "Russians"],
        0,
    )
    .expect("应成功开局");
    let snap = opened.session.expect_battle().snapshot(&[]);
    let units: Vec<_> = snap
        .units
        .iter()
        .filter(|u| matches!(u.type_id.as_ref(), "AMCV" | "SMCV"))
        .map(|u| (u.owner.as_ref().to_string(), u.type_id.as_ref().to_string(), u.x, u.y))
        .collect();
    assert!(units.contains(&("Americans".into(), "AMCV".into(), 4, 4)), "{units:?}");
    assert!(units.contains(&("Russians".into(), "SMCV".into(), 20, 20)), "{units:?}");
}

#[test]
fn open_skirmish_strips_map_preplaced_mobiles() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let mut map = map_with_starts();
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
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
        owner: "Russians".into(),
        type_id: "ADOG".into(),
        health: 256,
        x: 10,
        y: 10,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Russians".into(),
        type_id: "DEST".into(),
        health: 256,
        x: 12,
        y: 12,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    let opened = open_skirmish_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())),
        map,
        "t".into(),
        (0, 0),
        Some("Americans"),
        &["Americans", "Russians"],
        0,
    )
    .expect("应成功开局");
    assert!(opened.note.contains("strip_mobiles#2"), "{}", opened.note);
    let snap = opened.session.expect_battle().snapshot(&[]);
    let type_ids: Vec<_> = snap.units.iter().map(|u| u.type_id.as_ref().to_string()).collect();
    assert!(type_ids.iter().any(|t| t == "AMCV"), "{type_ids:?}");
    assert!(type_ids.iter().any(|t| t == "SMCV"), "{type_ids:?}");
    assert!(!type_ids.iter().any(|t| t == "ADOG" || t == "DEST"), "{type_ids:?}");
}

#[test]
fn open_skirmish_fails_when_start_waypoint_missing() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let mut map = map_with_starts();
    map.waypoints.retain(|w| w.index == 0);
    let err = open_skirmish_session(&RulesBytesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&mcv_rules())),
        map,
        "t".into(),
        (0, 0),
        Some("Americans"),
        &["Americans", "Russians"],
        0,
    )
    .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("航点") || msg.contains("席位"), "{msg}");
}
