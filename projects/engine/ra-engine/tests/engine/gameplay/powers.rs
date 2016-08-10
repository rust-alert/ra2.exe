//! 闪电风暴 → Ion 光照档。

use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, start_lightning_storm};
use ra_map::{LightingConfig, LightingProfile};
use ra_types::GameEdition;

fn empty_rules() -> RulesSystem {
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: IniDocument::default(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::default(),
        warheads: WarheadRegistry::default(),
    }
}

fn world_with_ion_map() -> BattleState {
    let bytes = b"[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n[Lighting]\nAmbient=1.0\nGround=0.0\nLevel=0.0\n\
IonAmbient=0.5\nIonRed=0.25\nIonGreen=0.25\nIonBlue=1.0\nIonGround=0.0\nIonLevel=0.0\n";
    let mut map = ra_map::MapInfo::parse_ini(GameEdition::Ra2, "storm.map", bytes).expect("map");
    map.lighting = LightingConfig { ambient: 1.0, ground: 0.0, level: 0.0, ..LightingConfig::identity() };
    BattleState::new(GameEdition::Ra2, &empty_rules(), map)
}

#[test]
fn scheduled_powers_phase_flips_ion_profile() {
    let mut world = world_with_ion_map();
    assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
    start_lightning_storm(&mut world, 4, 4, 1, 1);
    assert_eq!(world.map.lighting_profile, LightingProfile::Normal);

    world.advance_tick(); // deferment 1→0 → Ion
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    let ion = world.map.tint_at(0, 0, 0);

    world.advance_tick(); // duration 1→0
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    world.advance_tick(); // ending sentinel
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    world.advance_tick(); // cleanup → Normal
    assert!(world.lightning_storm.is_none());
    assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
    let normal = world.map.tint_at(0, 0, 0);
    assert!(ion[0] < normal[0], "ion={ion:?} normal={normal:?}");
}

// 自顶层 `gameplay__powers_unit.rs` 并入。

// 自 engine/ra-engine/src/gameplay/powers.rs :: tests
use ra_engine::gameplay::powers::*;

fn gameplay_powers_empty_rules() -> RulesSystem {
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: IniDocument::default(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::default(),
        warheads: WarheadRegistry::default(),
    }
}

fn world_with_ion_keys() -> BattleState {
    let bytes = b"[Map]\nSize=0,0,10,10\nTheater=TEMPERATE\n[Lighting]\nAmbient=1.0\nGround=0.0\nLevel=0.0\n\
IonAmbient=0.5\nIonRed=0.25\nIonGreen=0.25\nIonBlue=1.0\nIonGround=0.0\nIonLevel=0.0\n";
    let mut map = ra_map::MapInfo::parse_ini(GameEdition::Ra2, "storm.map", bytes).expect("map");
    // 无压暗便于断言。
    map.lighting = LightingConfig { ambient: 1.0, ground: 0.0, level: 0.0, ..LightingConfig::identity() };
    BattleState::new(GameEdition::Ra2, &gameplay_powers_empty_rules(), map)
}

#[test]
fn deferment_then_duration_switches_ion_and_back() {
    let mut world = world_with_ion_keys();
    assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
    start_lightning_storm(&mut world, 5, 5, 1, 2);
    assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
    tick_lightning_storm(&mut world);
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    let ion_tint = world.map.tint_at(0, 0, 0);
    tick_lightning_storm(&mut world); // duration 2→1
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    tick_lightning_storm(&mut world); // duration 1→0
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    tick_lightning_storm(&mut world); // → ending sentinel
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
    assert!(world.lightning_storm.is_some());
    tick_lightning_storm(&mut world); // cleanup
    assert!(world.lightning_storm.is_none());
    assert_eq!(world.map.lighting_profile, LightingProfile::Normal);
    let normal_tint = world.map.tint_at(0, 0, 0);
    assert!(ion_tint[0] < normal_tint[0], "ion={ion_tint:?} normal={normal_tint:?}");
    assert!(ion_tint[2] > ion_tint[0]);
}

#[test]
fn zero_deferment_starts_on_ion() {
    let mut world = world_with_ion_keys();
    start_lightning_storm(&mut world, 1, 1, 0, 1);
    assert_eq!(world.map.lighting_profile, LightingProfile::Ion);
}
