//! 自顶层 `lighting.rs`。

use ra_assets::IniDocument;
use ra_map::{
    LEPTONS_PER_CELL, LightingConfig, LightingProfile, MapEntity, MapEntityKind, MapInfo, RadiationLightRules, RadiationLightSite,
    StructureLightTable, apply_rgba_tint, cell_light_scalar, cell_tint, cell_tint_with_lights, collect_structure_point_lights, parse_lighting,
    parse_map_lighting, point_light_at, terrain_tint,
};
use ra_types::GameEdition;

#[test]
fn parse_lighting_reads_section_keys() {
    let doc = IniDocument::parse(b"[Lighting]\nAmbient=0.8\nRed=1.0\nGreen=0.9\nBlue=0.7\nGround=0.1\nLevel=0.05\n").expect("ini");
    let cfg = parse_lighting(&doc);
    assert!((cfg.ambient - 0.8).abs() < 1e-4);
    assert!((cfg.green - 0.9).abs() < 1e-4);
    assert!((cfg.blue - 0.7).abs() < 1e-4);
    assert!((cfg.ground - 0.1).abs() < 1e-4);
    assert!((cfg.level - 0.05).abs() < 1e-4);
}

#[test]
fn parse_lighting_missing_section_uses_retail_defaults() {
    let doc = IniDocument::parse(b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n").expect("ini");
    let cfg = parse_lighting(&doc);
    assert_eq!(cfg, LightingConfig::default());
}

#[test]
fn map_info_parse_ini_stores_lighting() {
    let bytes = b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n[Lighting]\nAmbient=0.5\nGround=0.0\nLevel=0.0\n";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "lit.map", bytes).expect("map");
    assert!((map.lighting.ambient - 0.5).abs() < 1e-4);
    let tint = terrain_tint(&map.lighting);
    assert!((tint[0] - 0.5).abs() < 1e-3);
}

#[test]
fn default_ground_level_tint_is_0_95() {
    let tint = terrain_tint(&LightingConfig::default());
    assert!((tint[0] - 0.95).abs() < 1e-3);
}

#[test]
fn apply_rgba_tint_scales_rgb_keeps_alpha() {
    let mut rgba = vec![200u8, 100, 50, 255];
    apply_rgba_tint(&mut rgba, [0.5, 0.5, 0.5]);
    assert_eq!(rgba, vec![100, 50, 25, 255]);
}

#[test]
fn cell_tint_rises_with_elevation() {
    let cfg = LightingConfig::default();
    assert!(cell_light_scalar(&cfg, 8) > cell_light_scalar(&cfg, 0));
    let low = cell_tint(&cfg, 0);
    let high = cell_tint(&cfg, 8);
    assert!(high[0] > low[0]);
}

#[test]
fn parse_map_lighting_reads_ion_keys() {
    let doc =
        IniDocument::parse(b"[Lighting]\nAmbient=1.0\nIonAmbient=0.5\nIonRed=0.2\nIonGreen=0.3\nIonBlue=0.9\nIonGround=0.1\nIonLevel=0.02\n")
            .expect("ini");
    let profiles = parse_map_lighting(&doc);
    assert!((profiles.normal.ambient - 1.0).abs() < 1e-4);
    assert!((profiles.ion.ambient - 0.5).abs() < 1e-4);
    assert!((profiles.ion.red - 0.2).abs() < 1e-4);
    assert!((profiles.ion.green - 0.3).abs() < 1e-4);
    assert!((profiles.ion.blue - 0.9).abs() < 1e-4);
    assert!((profiles.ion.ground - 0.1).abs() < 1e-4);
    assert!((profiles.ion.level - 0.02).abs() < 1e-4);
}

#[test]
fn parse_map_lighting_missing_ion_uses_retail_ion_defaults() {
    let doc = IniDocument::parse(b"[Lighting]\nAmbient=0.8\n").expect("ini");
    let profiles = parse_map_lighting(&doc);
    assert_eq!(profiles.ion, LightingConfig::ion_default());
    assert!((profiles.normal.ambient - 0.8).abs() < 1e-4);
}

#[test]
fn map_info_ion_profile_switches_tint() {
    let bytes = b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n[Lighting]\nAmbient=1.0\nGround=0.0\nLevel=0.0\n\
IonAmbient=0.5\nIonRed=0.25\nIonGreen=0.25\nIonBlue=1.0\nIonGround=0.0\nIonLevel=0.0\n";
    let mut map = MapInfo::parse_ini(GameEdition::Ra2, "ion.map", bytes).expect("map");
    assert_eq!(map.lighting_profile, LightingProfile::Normal);
    let normal = map.tint_at(0, 0, 0);
    assert!((normal[0] - 1.0).abs() < 1e-2, "normal={normal:?}");
    map.set_lighting_profile(LightingProfile::Ion);
    let ion = map.tint_at(0, 0, 0);
    // Ion 档偏蓝：蓝通道相对更强，且 ambient 更暗。
    assert!(ion[0] < normal[0], "ion={ion:?} normal={normal:?}");
    assert!(ion[2] > ion[0], "expected blue-heavy ion tint {ion:?}");
}

#[test]
fn point_light_brightens_near_cell() {
    let cfg = LightingConfig::identity();
    let light = point_light_at(10, 10, LEPTONS_PER_CELL * 4, 0.5, [1.0, 1.0, 1.0]);
    let near = cell_tint_with_lights(&cfg, 0, 10, 10, &[light.clone()]);
    let far = cell_tint_with_lights(&cfg, 0, 40, 40, &[light]);
    assert!(near[0] > far[0], "near={near:?} far={far:?}");
    assert!((far[0] - 1.0).abs() < 1e-3);
}

#[test]
fn refresh_point_lights_from_rules_structures() {
    let rules =
        IniDocument::parse(b"[GAYARD]\nLightIntensity=0.4\nLightVisibility=2000\nLightRedTint=1.0\nLightGreenTint=0.8\nLightBlueTint=0.5\n")
            .expect("rules");
    let mut map = MapInfo::empty(GameEdition::Ra2, "lit.map");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GAYARD".into(),
        health: 256,
        x: 5,
        y: 7,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.refresh_point_lights(&StructureLightTable::from_rules_ini(&rules));
    assert_eq!(map.point_lights.len(), 1);
    assert_eq!(map.structure_point_lights.len(), 1);
    assert_eq!(map.point_lights[0].x, 5);
    assert_eq!(map.point_lights[0].y, 7);
    let collected = collect_structure_point_lights(&map.entities, &StructureLightTable::from_rules_ini(&rules));
    assert_eq!(collected, map.structure_point_lights);
    let tint = map.tint_at(5, 7, 0);
    assert!(tint[0] > 1.0 || tint[1] > 0.9, "expected light boost, tint={tint:?}");
}

#[test]
fn refresh_radiation_lights_merges_green_glow() {
    let mut map = MapInfo::empty(GameEdition::Ra2, "rad.map");
    map.lighting = LightingConfig::identity();
    let sites = [RadiationLightSite::with_spread(8, 8, 4, 500, 500, 500)];
    let rules = RadiationLightRules::default();
    map.refresh_radiation_lights(&sites, &rules);
    assert_eq!(map.radiation_point_lights.len(), 1);
    assert_eq!(map.point_lights.len(), 1);
    assert_eq!(map.radiation_point_lights[0].tint, [0, 1000, 0]);
    let tint = map.tint_at(8, 8, 0);
    assert!(tint[1] > tint[0], "expected green-heavy radiation tint {tint:?}");
    // 建筑光刷新不得冲掉辐射光。
    let struct_rules = IniDocument::parse(b"[GAYARD]\nLightIntensity=0.1\nLightVisibility=512\n").expect("rules");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "GAYARD".into(),
        health: 256,
        x: 1,
        y: 1,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    });
    map.refresh_point_lights(&StructureLightTable::from_rules_ini(&struct_rules));
    assert_eq!(map.structure_point_lights.len(), 1);
    assert_eq!(map.radiation_point_lights.len(), 1);
    assert_eq!(map.point_lights.len(), 2);
}

// 自顶层 `lighting_unit.rs` 并入。

// 自 engine/ra-map/src/lighting.rs :: tests

#[test]
fn ion_default_is_blueish_storm() {
    let tint = terrain_tint(&LightingConfig::ion_default());
    assert!(tint[2] > tint[0], "ion tint={tint:?}");
    assert!(tint[0] < 0.5, "ion should be darker than full white");
}

#[test]
fn lighting_default_ground_level_tint_is_0_95() {
    let tint = terrain_tint(&LightingConfig::default());
    assert!((tint[0] - 0.95).abs() < 1e-3);
    assert!((tint[1] - 0.95).abs() < 1e-3);
    assert!((tint[2] - 0.95).abs() < 1e-3);
}

#[test]
fn identity_config_is_full_bright() {
    let tint = terrain_tint(&LightingConfig::identity());
    assert!((tint[0] - 1.0).abs() < 1e-3);
}

#[test]
fn level_raises_high_cells() {
    let cfg = LightingConfig::default();
    let low = cell_light_scalar(&cfg, 0);
    let high = cell_light_scalar(&cfg, 10);
    assert!(high > low);
}

#[test]
fn point_light_linear_falloff_on_white() {
    let cfg = LightingConfig { ambient: 0.5, ground: 0.0, level: 0.0, ..LightingConfig::identity() };
    let light = point_light_at(10, 10, 5 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
    let lights = [light];
    let center = cell_tint_with_lights(&cfg, 0, 10, 10, &lights);
    assert!((center[0] - 1.5).abs() < 0.02, "center={}", center[0]);
    let d2 = cell_tint_with_lights(&cfg, 0, 12, 10, &lights);
    let expected = 0.5 + (5.0 - 2.0) / 5.0;
    assert!((d2[0] - expected).abs() < 0.02, "d2={} expected={}", d2[0], expected);
    let far = cell_tint_with_lights(&cfg, 0, 16, 10, &lights);
    assert!((far[0] - 0.5).abs() < 0.02, "far={}", far[0]);
}

#[test]
fn point_light_edge_zero_contribution() {
    let cfg = LightingConfig { ambient: 0.5, ground: 0.0, level: 0.0, ..LightingConfig::identity() };
    let light = point_light_at(2, 2, 2 * LEPTONS_PER_CELL, 1.0, [1.0, 1.0, 1.0]);
    let edge = cell_tint_with_lights(&cfg, 0, 4, 2, &[light.clone()]);
    assert!((edge[0] - 0.5).abs() < 0.02);
    let inside = cell_tint_with_lights(&cfg, 0, 3, 2, &[light]);
    assert!((inside[0] - 1.0).abs() < 0.02);
}

#[test]
fn negative_point_light_darkens() {
    let cfg = LightingConfig { ambient: 0.8, ground: 0.0, level: 0.0, ..LightingConfig::identity() };
    let light = point_light_at(2, 2, 2 * LEPTONS_PER_CELL, -0.2, [1.0, 1.0, 1.0]);
    let center = cell_tint_with_lights(&cfg, 0, 2, 2, &[light]);
    assert!((center[0] - 0.6).abs() < 0.02);
}

#[test]
fn collect_structure_lights_from_rules() {
    let rules = IniDocument::parse(
        b"[LAMP]\nLightVisibility=512\nLightIntensity=0.5\nLightRedTint=1\nLightGreenTint=1\nLightBlueTint=1\n\
[DARK]\nLightVisibility=256\nLightIntensity=-0.25\n\
[ZERO]\nLightVisibility=4096\nLightIntensity=0\n",
    )
    .expect("rules");
    let entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Neutral".into(),
            type_id: "LAMP".into(),
            health: 256,
            x: 3,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "Neutral".into(),
            type_id: "ZERO".into(),
            health: 256,
            x: 1,
            y: 1,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "LAMP".into(),
            health: 256,
            x: 9,
            y: 9,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let lights = collect_structure_point_lights(&entities, &StructureLightTable::from_rules_ini(&rules));
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].x, 3);
    assert_eq!(lights[0].y, 4);
    assert_eq!(lights[0].radius_leptons, 512);
    assert!(lights[0].intensity > 0);
}
