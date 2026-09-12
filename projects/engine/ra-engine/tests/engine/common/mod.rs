//! 引擎集成测试共用夹具。

#![allow(dead_code)]

use std::sync::Arc;

use ra_adaptor::runtime_definitions_from_ini_bytes;
use ra_engine::{BattleState, Engine, EngineConfig};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, RuntimeDefinitions};

/// 测试用默认引擎（空定义骨架）。
pub fn test_engine() -> Engine {
    Engine::new(Arc::new(RuntimeDefinitions::default()), EngineConfig::default()).expect("默认引擎应可构造")
}

/// 冻结定义播种世界（引擎侧只消费 `RuntimeDefinitions`）。
pub fn battle_from_defs(edition: GameEdition, defs: Arc<RuntimeDefinitions>, map: MapInfo) -> BattleState {
    BattleState::new(edition, defs, map)
}

/// 内联 rules INI → 冻结定义（装载在 adaptor）。
pub fn defs_from_rules_ini(rules_ini: &[u8]) -> Arc<RuntimeDefinitions> {
    Arc::new(
        runtime_definitions_from_ini_bytes(GameEdition::Ra2, rules_ini, None).expect("测试 rules INI 必须可投影"),
    )
}

/// 含 MTNK 坦克类型的最小冻结定义（Strength=400，带主武器）。
pub fn defs_with_mtnk() -> Arc<RuntimeDefinitions> {
    defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=100\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    )
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
    let defs = defs_from_rules_ini(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    );
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
            tag: Default::default(),
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
            tag: Default::default(),
        },
    ];
    battle_from_defs(GameEdition::Ra2, defs, map)
}
