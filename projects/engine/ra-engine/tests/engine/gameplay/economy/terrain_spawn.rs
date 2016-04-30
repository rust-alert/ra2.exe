//! 矿柱产矿状态机：中点触发后写入邻格可采 overlay。

use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
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
        overlay_types: OverlayTypeRegistry::from_rules(&rules),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::default(),
        warheads: WarheadRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "terrain-spawn");
    map.width = 8;
    map.height = 8;
    map.terrain_objects = vec![TerrainObject {
        x: 4,
        y: 4,
        name: "TIBTRE01".into(),
    }];
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
    assert!(matches!(
        world.terrain_spawners[0].phase,
        ra_engine::TerrainSpawnerPhase::Idle
    ));
}
