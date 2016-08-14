//! 矿柱产矿状态机：中点触发后写入邻格可采 overlay。

use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry, overlay_types_from_rules};
use ra_engine::BattleState;
use ra_map::{MapInfo, TerrainObject};
use ra_types::GameEdition;

fn spawn_world() -> BattleState {
    let rules_text = br#"
[OverlayTypes]
0=TIB01

[TIB01]
Tiberium=yes

[TIBTRE01]
SpawnsTiberium=yes
IsAnimated=yes
AnimationRate=1
AnimationProbability=1
"#;
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: overlay_types_from_rules(&rules),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::default(),
        warheads: WarheadRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "terrain-spawn");
    map.width = 8;
    map.height = 8;
    map.terrain_objects = vec![TerrainObject { x: 4, y: 4, name: "TIBTRE01".into() }];
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
    assert_eq!(world.terrain_spawners.len(), 1);
    // 两帧：中点 = 1，Started 后下一帧即 SpawnDue。
    world.apply_ore_tree_frame_counts(&[(4, 4, 2)]);
    world
}

#[test]
fn midpoint_tick_places_ore_on_neighbor() {
    let mut world = spawn_world();
    assert!(world.map.overlays.is_empty());

    world.advance_tick();
    assert!(world.map.overlays.is_empty());
    assert_eq!(world.terrain_spawners[0].render_frame(), 0);

    world.advance_tick();
    assert_eq!(world.map.overlays.len(), 1);
    let cell = &world.map.overlays[0];
    assert_eq!(cell.x, 4);
    assert_eq!(cell.y, 3);
    assert_eq!(cell.overlay_id, 0);
    assert_eq!(cell.data, 3);
    assert_eq!(world.take_overlay_paint_dirty(), vec![(4, 3)]);
    assert!(matches!(world.terrain_spawners[0].phase, ra_engine::TerrainSpawnerPhase::Idle));
}

// 自顶层 `gameplay__terrain_spawn_unit.rs` 并入。

// 自 engine/ra-engine/src/gameplay/terrain_spawn.rs :: tests
use ra_engine::gameplay::terrain_spawn::*;
use ra_map::OverlayCell;
use ra_types::{TerrainSpawnerDefinition, TerrainSpawnerDefinitions};

#[test]
fn idle_stays_without_roll() {
    let mut s = TerrainSpawnerState::new(1, 2, "TIBTRE01", 3000, 3, 22);
    assert_eq!(s.tick(3000), TerrainSpawnerTick::Idle);
    assert_eq!(s.render_frame(), 0);
}

#[test]
fn roll_starts_and_midpoint_spawns() {
    let mut s = TerrainSpawnerState::new(1, 2, "TIBTRE01", 3000, 3, 22);
    assert_eq!(s.midpoint_frame, 11);
    assert_eq!(s.tick(0), TerrainSpawnerTick::Started);
    assert_eq!(s.render_frame(), 0);
    assert_eq!(s.tick(0), TerrainSpawnerTick::Active);
    assert_eq!(s.tick(0), TerrainSpawnerTick::Active);
    assert_eq!(s.tick(0), TerrainSpawnerTick::Active);
    assert_eq!(s.render_frame(), 1);
    for _ in 0..(9 * 3) {
        let _ = s.tick(0);
    }
    assert_eq!(s.render_frame(), 10);
    assert_eq!(s.tick(0), TerrainSpawnerTick::Active);
    assert_eq!(s.tick(0), TerrainSpawnerTick::Active);
    assert_eq!(s.tick(0), TerrainSpawnerTick::SpawnDue);
    assert!(matches!(s.phase, TerrainSpawnerPhase::Idle));
    assert_eq!(s.render_frame(), 0);
}

#[test]
fn seeds_from_map_terrain() {
    let mut defs = TerrainSpawnerDefinitions::default();
    defs.insert(TerrainSpawnerDefinition {
        type_key: "TIBTRE01".into(),
        animation_probability_micros: 3000,
        animation_rate_ticks: 3,
    });
    let mut map = ra_map::MapInfo::empty(ra_types::GameEdition::Ra2, "t");
    map.terrain_objects = vec![TerrainObject { x: 5, y: 6, name: "TIBTRE01".into() }, TerrainObject { x: 1, y: 1, name: "TREE01".into() }];
    let seeded = seed_terrain_spawners(&map, &defs);
    assert_eq!(seeded.len(), 1);
    assert_eq!(seeded[0].x, 5);
    assert_eq!(seeded[0].animation_probability_micros, 3000);
    assert_eq!(seeded[0].animation_rate_ticks, 3);
}

fn ore_overlay_types() -> OverlayTypeRegistry {
    let doc = IniDocument::parse(
        br#"
[OverlayTypes]
0=BRIDGE1
1=TIB01

[BRIDGE1]
Land=Road

[TIB01]
Tiberium=yes
"#,
    )
    .expect("ini");
    overlay_types_from_rules(&doc)
}

#[test]
fn place_spawned_ore_writes_first_empty_neighbor() {
    let reg = ore_overlay_types();
    let mut overlays = Vec::new();
    assert_eq!(place_spawned_ore(&mut overlays, &reg, 5, 5, |x, y| x < 10 && y < 10), Some((5, 4)));
    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].x, 5);
    assert_eq!(overlays[0].y, 4);
    assert_eq!(overlays[0].overlay_id, 1);
    assert_eq!(overlays[0].data, SPAWN_ORE_DENSITY);
}

#[test]
fn place_spawned_ore_raises_existing_harvestable_density() {
    let reg = ore_overlay_types();
    let mut overlays = vec![OverlayCell { x: 5, y: 4, overlay_id: 1, data: 2 }];
    assert_eq!(place_spawned_ore(&mut overlays, &reg, 5, 5, |x, y| x < 10 && y < 10), Some((5, 4)));
    assert_eq!(overlays.len(), 1);
    assert_eq!(overlays[0].data, 3);
}

#[test]
fn place_spawned_ore_skips_non_harvestable_occupied_cell() {
    let reg = ore_overlay_types();
    let mut overlays = vec![OverlayCell { x: 5, y: 4, overlay_id: 0, data: 0 }];
    assert_eq!(place_spawned_ore(&mut overlays, &reg, 5, 5, |x, y| x < 10 && y < 10), Some((6, 4)));
    assert_eq!(overlays.len(), 2);
    assert_eq!(overlays[1].x, 6);
    assert_eq!(overlays[1].y, 4);
    assert_eq!(overlays[1].overlay_id, 1);
}
