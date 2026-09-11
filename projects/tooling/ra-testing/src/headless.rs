//! 无窗口遭遇战夹具。

use ra_adaptor::{RulesSystem, build_runtime_definitions};
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, RulesGlobals, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use std::sync::Arc;

use ra_engine::{BattleOutcome, BattleState, Engine, EngineConfig, GameCommand, RenderSnapshot, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, RuntimeDefinitions, TerrainSpawnerDefinitions};

use crate::alpha_skirmish_v1;

fn battle_from_rules(rules: &RulesSystem, map: MapInfo) -> BattleState {
    BattleState::new(rules.edition, Arc::new(build_runtime_definitions(rules)), map)
}

/// 无窗口测试用例。所有推进都经过 `Session::tick` + `EngineRuntime`，与产品路径一致。
#[derive(Debug)]
pub struct HeadlessCase {
    /// 长期引擎（定义 / 调度上下文）。
    pub engine: Engine,
    /// 被测会话。
    pub session: Session,
}

/// 用例运行后的稳定观测值，供集成测试记录和断言。
#[derive(Debug, Clone)]
pub struct HeadlessObservation {
    /// 世界 tick。
    pub tick: u64,
    /// 状态摘要。
    pub state_hash: u64,
    /// 对局结果（若已结束）。
    pub outcome: Option<BattleOutcome>,
    /// 呈现快照。
    pub snapshot: RenderSnapshot,
}

fn default_engine() -> Engine {
    Engine::new(Arc::new(RuntimeDefinitions::default()), EngineConfig::default()).expect("默认引擎应可构造")
}

impl HeadlessCase {
    /// 包装已有会话（附带默认引擎）。
    pub fn new(session: Session) -> Self {
        Self { engine: default_engine(), session }
    }

    /// 在指定 tick 之前入队命令；下一次 `tick` 会按产品路径消费。
    pub fn command(&mut self, command: GameCommand) {
        self.session.expect_battle_mut().push_command(command);
    }

    /// 排队建造至完工再点选落位（费用在 `Produce` 时扣除）。
    pub fn produce_and_place(&mut self, player: ra_types::PlayerId, type_id: &str, x: u16, y: u16) {
        use ra_engine::PRODUCE_TICKS;
        self.command(GameCommand::Produce { player, type_id: type_id.into() });
        self.advance(1);
        for _ in 0..=PRODUCE_TICKS {
            let ready = self
                .session
                .expect_battle()
                .world
                .players
                .iter()
                .find(|p| p.id == player)
                .and_then(|p| self.session.expect_battle().world.house_ready_building(p.house.as_ref()));
            if ready.is_some_and(|r| r.as_ref().eq_ignore_ascii_case(type_id)) {
                break;
            }
            self.advance(1);
        }
        self.command(GameCommand::PlaceBuilding { player, type_id: type_id.into(), x, y });
        self.advance(1);
    }

    /// 精确推进指定次数，不依赖墙钟、窗口事件或 GPU。
    pub fn advance(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.session.tick(&self.engine.runtime());
            if self.session.expect_battle().outcome.is_some() {
                break;
            }
        }
    }

    /// 采集当前观测。
    pub fn observe(&self) -> HeadlessObservation {
        let game = self.session.expect_battle();
        HeadlessObservation {
            tick: game.world.tick,
            state_hash: game.world.state_hash(),
            outcome: game.outcome.clone(),
            snapshot: game.snapshot(&[]),
        }
    }
}

/// 标准二人坦克遭遇战。
///
/// 这是 `alpha-skirmish-v1` 冻结竖切的最小战斗前身：固定规则、两名玩家、可重复命令脚本。
/// 完整竖切的建筑、经济与开局 MCV 见 `alpha_skirmish_v1`。
pub fn standard_duel() -> HeadlessCase {
    // 须声明 Primary / Warhead：无武器时 attack_damage=0，决斗永远打不死。
    let rules_text = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[90mm]\nDamage=50\nROF=8\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(&rules, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
        super_weapons: SuperWeaponTypeRegistry::default(),
    };

    let mut map = MapInfo::empty(GameEdition::Ra2, "testing-duel");
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
    let world = battle_from_rules(&rules_db, map);
    HeadlessCase::new(Session::from_state(world, "ra-testing standard duel"))
}

/// 单人 MCV 开局夹具：播种盟军 MCV 与冻结竖切初始资金，供部署/经济 headless 使用。
pub fn mcv_deploy_open() -> HeadlessCase {
    let slice = alpha_skirmish_v1();
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n\
[AMCV]\nDeploysInto=GACNST\nOwner=Americans\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let rules_db = RulesSystem {
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
    };

    let mut map = MapInfo::empty(GameEdition::Ra2, "testing-mcv-deploy");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Unit,
        owner: slice.human_house.into(),
        type_id: slice.allied_mcv.into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    HeadlessCase::new(Session::from_state(world, "ra-testing mcv deploy open"))
}

/// 已展开建造场的开局夹具，供放置建筑 / 经济 headless 使用。
pub fn yard_open() -> HeadlessCase {
    let slice = alpha_skirmish_v1();
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n3=GAPILE\n\
[InfantryTypes]\n0=E1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nTechLevel=1\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\nTechLevel=1\n\
[GAREFN]\nRefinery=yes\nPower=-50\nPowered=yes\nOwner=Americans\nStrength=900\nSight=4\nCost=2000\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let rules_db = RulesSystem {
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
    };

    let mut map = MapInfo::empty(GameEdition::Ra2, "testing-yard-open");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: slice.human_house.into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: String::new(),
        tag: String::new(),
    }];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    HeadlessCase::new(Session::from_state(world, "ra-testing yard open"))
}

/// 双人 AI 遭遇战开局：本地盟军建造场 + 苏军 MCV，AI 经 `GameCommand` 展开基地。
pub fn ai_skirmish_open() -> HeadlessCase {
    let slice = alpha_skirmish_v1();
    let rules_text = b"[VehicleTypes]\n0=SMCV\n1=MTNK\n\
[BuildingTypes]\n0=GACNST\n1=NACNST\n2=NAPOWR\n3=NAHAND\n4=NAWEAP\n5=NAREFN\n\
[InfantryTypes]\n0=E2\n\
[SMCV]\nDeploysInto=NACNST\nOwner=Russians\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\nArmor=heavy\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\nPrimary=90mm\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\nTechLevel=1\n\
[NAPOWR]\nPower=200\nOwner=Russians\nStrength=600\nSight=4\nCost=600\nArmor=wood\nTechLevel=1\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Russians\nStrength=500\nSight=5\nCost=500\nArmor=wood\nTechLevel=1\n\
[NAWEAP]\nPower=-30\nPowered=yes\nFactory=UnitType\nOwner=Russians\nStrength=1000\nSight=5\nCost=2000\nArmor=wood\nTechLevel=1\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Russians\nStrength=900\nSight=4\nCost=2000\nArmor=wood\nTechLevel=1\n\
[E2]\nOwner=Russians\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nTechLevel=1\n\
[90mm]\nDamage=75\nROF=8\nRange=5\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(&rules, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules_db = RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        globals: RulesGlobals::from_rules(&rules),
        overlay_types: OverlayTypeRegistry::default(),
        terrain_spawners: TerrainSpawnerDefinitions::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types,
        warheads,
        super_weapons: SuperWeaponTypeRegistry::default(),
    };

    let mut map = MapInfo::empty(GameEdition::Ra2, "testing-ai-skirmish");
    map.width = 24;
    map.height = 24;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: slice.human_house.into(),
            type_id: "GACNST".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: slice.ai_house.into(),
            type_id: slice.soviet_mcv.into(),
            health: 256,
            x: 16,
            y: 16,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
        MapEntity {
            kind: MapEntityKind::Unit,
            owner: slice.human_house.into(),
            type_id: "MTNK".into(),
            health: 256,
            x: 14,
            y: 16,
            facing: 0,
            sub_cell: 0,
            mission: String::new(),
            tag: String::new(),
        },
    ];
    let mut world = battle_from_rules(&rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    assert!(world.set_house_funds(slice.ai_house, slice.starting_funds));
    let mut session = Session::from_state(world, "ra-testing ai skirmish open");
    session.expect_battle_mut().ai_enabled = true;
    HeadlessCase::new(session)
}
