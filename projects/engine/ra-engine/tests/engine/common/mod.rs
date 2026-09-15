//! 引擎集成测试共用夹具。

#![allow(dead_code)]

use std::sync::Arc;

use ra_adaptor::runtime_definitions_from_ini_bytes;
use ra_engine::{BattleState, Engine, EngineConfig};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, RuntimeDefinitions};

/// 测试夹具缺 `[Countries]` 时注入常用国家，便于地图 Owner 绑定到稳定 `HouseId`。
const TEST_COUNTRIES_PREFIX: &[u8] = b"[Countries]\n0=Americans\n1=Russians\n2=Soviets\n3=Alliance\n4=France\n\
[Americans]\nSide=GDI\n\
[Russians]\nSide=Nod\n\
[Soviets]\nSide=Nod\n\
[Alliance]\nSide=GDI\n\
[France]\nSide=GDI\n";

/// 缺 `[AI]` 时注入最小建造表，与常见夹具类型名对齐；未入库名由 adaptor 软跳过。
///
/// `Production=0`：无地图 `IQ=` 或 `IQ=0` 时仍可推进兵营 / 车厂类（产品 rules 自带 `[IQ]` 时不注入）。
const TEST_AI_CONTROLS_SUFFIX: &[u8] = b"\n[AI]\nAIBaseSpacing=1\nPowerSurplus=50\n\
BuildPower=NAPOWR,GAPOWR\n\
BuildRefinery=NAREFN,GAREFN\nRefineryRatio=.16\nRefineryLimit=4\n\
BuildBarracks=NAHAND,GAPILE\nBarracksRatio=.1\nBarracksLimit=2\n\
BuildWeapons=NAWEAP,GAWEAP\nWarRatio=.1\nWarLimit=2\n\
[IQ]\nMaxIQLevels=5\nProduction=0\n";

fn rules_has_section(upper: &[u8], needle: &[u8]) -> bool {
    upper.windows(needle.len()).any(|w| w == needle)
}

/// 内联 rules → 冻结定义；无 `[Countries]` 时自动补美俄；无 `[AI]` 时补最小 `AiControls` 表。
///
/// 装载在 adaptor；单测默认 `savour_delay_ticks = 0`，避免隐式 SavourDelay 拉长用例。
pub fn defs_from_rules_ini(rules_ini: &[u8]) -> Arc<RuntimeDefinitions> {
    let upper = rules_ini.to_ascii_uppercase();
    let mut bytes = if rules_has_section(&upper, b"[COUNTRIES]") {
        rules_ini.to_vec()
    }
    else {
        let mut out = TEST_COUNTRIES_PREFIX.to_vec();
        out.extend_from_slice(rules_ini);
        out
    };
    // `[AI]` 四字节恰为节名（不会误伤 `[AITriggerTypes]` 等）。
    if !rules_has_section(&upper, b"[AI]") {
        bytes.extend_from_slice(TEST_AI_CONTROLS_SUFFIX);
    }
    let mut defs = runtime_definitions_from_ini_bytes(GameEdition::Ra2, &bytes, None).expect("测试 rules INI 必须可投影");
    defs.savour_delay_ticks = 0;
    Arc::new(defs)
}

/// 测试用默认引擎（空定义骨架）。
pub fn test_engine() -> Engine {
    Engine::new(Arc::new(RuntimeDefinitions::default()), EngineConfig::default()).expect("默认引擎应可构造")
}

/// 冻结定义播种世界：先 [`ra_engine::validate_map_for_battle`]，再 [`BattleState::from_prepared`]。
///
/// 与产品 boot / 会话入口同一 `PreparedMap` 契约（含几何拒绝）。
pub fn battle_from_defs(edition: GameEdition, defs: Arc<RuntimeDefinitions>, map: MapInfo) -> BattleState {
    let prepared = ra_engine::validate_map_for_battle(&map, &defs).expect("map prepare");
    BattleState::from_prepared(edition, defs, map, prepared).expect("battle seed")
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
            owner: "AMERICANS".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 4,
            y: 8,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: "RUSSIANS".into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 8,
            y: 8,
            facing: 128,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    battle_from_defs(GameEdition::Ra2, defs, map)
}
