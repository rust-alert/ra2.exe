//! 引擎集成测试共用夹具。

#![allow(dead_code)]

use std::sync::Arc;

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{Engine, EngineConfig, MatchState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, RuntimeDefinitions};

/// 测试用默认引擎（空定义骨架）。
pub fn test_engine() -> Engine {
    Engine::new(Arc::new(RuntimeDefinitions::default()), EngineConfig::default()).expect("默认引擎应可构造")
}

/// 含 MTNK 坦克类型的最小规则库（Strength=400）。
pub fn rules_with_mtnk() -> RulesDb {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n",
    )
    .unwrap();
    RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
    }
}

/// 20×30 空图。
pub fn map_with_size() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.width = 20;
    map.height = 30;
    map
}

/// 美俄各一辆 MTNK 的对决世界（Strength=200，供身份 / 拒绝测例）。
pub fn duel_mtnk_world() -> MatchState {
    let rules_text = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
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
        },
    ];
    MatchState::new(GameEdition::Ra2, &rules_db, map)
}
