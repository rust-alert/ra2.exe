//! 无窗口遭遇战夹具。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_session::{MatchOutcome, RenderSnapshot, Session};
use ra_types::GameEdition;
use ra_world::{GameCommand, World};

use crate::alpha_skirmish_v1;

/// 无窗口测试用例。所有推进都经过 `Session::tick`，避免测试与产品运行路径分叉。
#[derive(Debug)]
pub struct HeadlessCase {
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
    pub outcome: Option<MatchOutcome>,
    /// 呈现快照。
    pub snapshot: RenderSnapshot,
}

impl HeadlessCase {
    /// 包装已有会话。
    pub fn new(session: Session) -> Self {
        Self { session }
    }

    /// 在指定 tick 之前入队命令；下一次 `tick` 会按产品路径消费。
    pub fn command(&mut self, command: GameCommand) {
        self.session.push_command(command);
    }

    /// 精确推进指定次数，不依赖墙钟、窗口事件或 GPU。
    pub fn advance(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.session.tick();
            if self.session.outcome.is_some() {
                break;
            }
        }
    }

    /// 采集当前观测。
    pub fn observe(&self) -> HeadlessObservation {
        HeadlessObservation {
            tick: self.session.world.tick,
            state_hash: self.session.world.state_hash(),
            outcome: self.session.outcome.clone(),
            snapshot: self.session.snapshot(),
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
    let world = World::new(GameEdition::Ra2, &rules_db, map);
    HeadlessCase::new(Session::new(world, "ra-testing standard duel"))
}

/// 单人 MCV 开局夹具：播种盟军 MCV 与冻结竖切初始资金，供部署/经济 headless 使用。
pub fn mcv_deploy_open() -> HeadlessCase {
    let slice = alpha_skirmish_v1();
    let rules_text = b"[VehicleTypes]\n0=AMCV\n\
[BuildingTypes]\n0=GACNST\n\
[AMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
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
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    HeadlessCase::new(Session::new(world, "ra-testing mcv deploy open"))
}

/// 已展开建造场的开局夹具，供放置建筑 / 经济 headless 使用。
pub fn yard_open() -> HeadlessCase {
    let slice = alpha_skirmish_v1();
    let rules_text = b"[BuildingTypes]\n0=GACNST\n1=GAPOWR\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\n\
[GAPOWR]\nStrength=600\nSight=4\nCost=600\n";
    let rules = IniDocument::parse(rules_text).expect("内置测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
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
    let mut world = World::new(GameEdition::Ra2, &rules_db, map);
    assert!(world.set_house_funds(slice.human_house, slice.starting_funds));
    HeadlessCase::new(Session::new(world, "ra-testing yard open"))
}
