//! 引擎集成测试共用夹具。

#![allow(dead_code)]

use std::sync::Arc;

use ra_adaptor::RulesSystem;
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, Engine, EngineConfig};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, RuntimeDefinitions};

/// 测试用默认引擎（空定义骨架）。
pub fn test_engine() -> Engine {
    Engine::new(Arc::new(RuntimeDefinitions::default()), EngineConfig::default()).expect("默认引擎应可构造")
}

/// 含 MTNK 坦克类型的最小规则库（Strength=400，带主武器）。
pub fn rules_with_mtnk() -> RulesSystem {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=100\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    )
    .unwrap();
    let techno_types = TechnoTypeRegistry::from_rules(&doc);
    let warheads = WarheadRegistry::from_names(&doc, techno_types.iter().map(|t| t.warhead.as_str()));
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
    }
}

/// 20×30 空图。
pub fn map_with_size() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    map
}

/// 美俄各一辆 MTNK 的对决世界（Strength=200，带主武器，供身份 / 拒绝 / 互殴测例）。
pub fn duel_mtnk_world() -> BattleState {
    let rules_text = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(&rules, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "entity-id-duel");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Americans".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 4,
            y: 8,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "Russians".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 8,
            y: 8,
            facing: 128,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    BattleState::new(GameEdition::Ra2, &rules_db, map)
}
