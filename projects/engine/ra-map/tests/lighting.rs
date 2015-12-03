use ra_assets::IniDocument;
use ra_map::{
    LightingConfig, MapEntity, MapEntityKind, MapInfo, apply_rgba_tint, cell_light_scalar, cell_tint,
    cell_tint_with_lights, collect_structure_point_lights, parse_lighting, point_light_at, terrain_tint,
    LEPTONS_PER_CELL,
};
use ra_types::GameEdition;

#[test]
fn parse_lighting_reads_section_keys() {
    let doc = IniDocument::parse(
        b"[Lighting]\nAmbient=0.8\nRed=1.0\nGreen=0.9\nBlue=0.7\nGround=0.1\nLevel=0.05\n",
    )
    .expect("ini");
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
    let rules = IniDocument::parse(
        b"[GAYARD]\nLightIntensity=0.4\nLightVisibility=2000\nLightRedTint=1.0\nLightGreenTint=0.8\nLightBlueTint=0.5\n",
    )
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
    });
    map.refresh_point_lights(&rules);
    assert_eq!(map.point_lights.len(), 1);
    assert_eq!(map.point_lights[0].x, 5);
    assert_eq!(map.point_lights[0].y, 7);
    let collected = collect_structure_point_lights(&map.entities, &rules);
    assert_eq!(collected, map.point_lights);
    let tint = map.tint_at(5, 7, 0);
    assert!(tint[0] > 1.0 || tint[1] > 0.9, "expected light boost, tint={tint:?}");
}
