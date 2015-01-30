//! 无窗口遭遇战夹具。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use std::sync::Arc;

use ra_engine::{Engine, EngineConfig, GameCommand, BattleOutcome, BattleState, RenderSnapshot, Session};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{GameEdition, RuntimeDefinitions};

use crate::alpha_skirmish_v1;

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
        self.session.expect_game_mut().push_command(command);
    }

    /// 精确推进指定次数，不依赖墙钟、窗口事件或 GPU。
    pub fn advance(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.session.tick(&self.engine.runtime());
            if self.session.expect_game().outcome.is_some() {
                break;
            }
        }
    }

    /// 采集当前观测。
    pub fn observe(&self) -> HeadlessObservation {
        let game = self.session.expect_game();
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
    let rules_text = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
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
    let world = BattleState::new(GameEdition::Ra2, &rules_db, map);
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
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
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
    }];
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    HeadlessCase::new(Session::from_state(world, "ra-testing mcv deploy open"))
}

/// 已展开建造场的开局夹具，供放置建筑 / 经济 headless 使用。
pub fn yard_open() -> HeadlessCase {
    let slice = alpha_skirmish_v1();
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAPOWR\n2=GAREFN\n3=GAPILE\n\
[InfantryTypes]\n0=E1\n\
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength=600\nSight=4\nCost=600\n\
[GAREFN]\nRefinery=yes\nStrength=900\nSight=4\nCost=2000\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\n\
[E1]\nOwner=Americans\nStrength=125\nSpeed=4\nSight=5\nCost=200\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
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
    }];
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
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
[GACNST]\nConstructionYard=yes\nOwner=Americans\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NACNST]\nConstructionYard=yes\nOwner=Russians\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[NAPOWR]\nPower=200\nOwner=Russians\nStrength=600\nSight=4\nCost=600\nArmor=wood\n\
[NAHAND]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Russians\nStrength=500\nSight=5\nCost=500\nArmor=wood\n\
[NAWEAP]\nPower=-30\nPowered=yes\nFactory=UnitType\nOwner=Russians\nStrength=1000\nSight=5\nCost=2000\nArmor=wood\n\
[NAREFN]\nPower=-50\nPowered=yes\nRefinery=yes\nOwner=Russians\nStrength=900\nSight=4\nCost=2000\nArmor=wood\n\
[E2]\nOwner=Russians\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\n\
[90mm]\nDamage=75\nROF=8\nRange=5\nWarhead=AP\n\
[AP]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let techno_types = TechnoTypeRegistry::from_rules(&rules);
    let warheads = WarheadRegistry::from_names(&rules, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types,
        warheads,
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
        },
    ];
    let mut world = BattleState::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    assert!(world.set_house_funds(slice.ai_house, slice.starting_funds));
    let mut session = Session::from_state(world, "ra-testing ai skirmish open");
    session.expect_game_mut().ai_enabled = true;
    HeadlessCase::new(session)
}
