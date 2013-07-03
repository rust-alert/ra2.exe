use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_session::{MatchOutcome, RenderSnapshot, Session};
use ra_types::GameEdition;
use ra_world::{GameCommand, World};

/// 无窗口测试用例。所有推进都经过 `Session::tick`，避免测试与产品运行路径分叉。
#[derive(Debug)]
pub struct HeadlessCase {
    pub session: Session,
}

/// 用例运行后的稳定观测值，供集成测试记录和断言。
#[derive(Debug, Clone)]
pub struct HeadlessObservation {
    pub tick: u64,
    pub state_hash: u64,
    pub outcome: Option<MatchOutcome>,
    pub snapshot: RenderSnapshot,
}

impl HeadlessCase {
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

    pub fn observe(&self) -> HeadlessObservation {
        HeadlessObservation {
            tick: self.session.world.tick,
            state_hash: self.session.world.state_hash(),
            outcome: self.session.outcome.clone(),
            snapshot: self.session.snapshot(),
        }
    }
}

/// 标准二人坦克遭遇战。它是测试数据，不代表 Alpha 最终平衡或内容范围。
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_duel_reaches_a_repeatable_victory() {
        let mut case = standard_duel();
        case.command(GameCommand::Attack { attacker_index: 0, target_index: 1 });
        case.advance(64);
        let result = case.observe();
        assert_eq!(result.outcome, Some(MatchOutcome::Victory { owner: "Americans".into() }));
        assert!(result.tick > 0);
        assert!(result.snapshot.units.iter().any(|unit| unit.dead));
    }

    #[test]
    fn equal_scripts_produce_equal_observations() {
        let mut first = standard_duel();
        let mut second = standard_duel();
        for case in [&mut first, &mut second] {
            case.command(GameCommand::MoveTo { entity_index: 0, x: 5, y: 8 });
            case.advance(3);
        }
        let a = first.observe();
        let b = second.observe();
        assert_eq!(a.tick, b.tick);
        assert_eq!(a.state_hash, b.state_hash);
        assert_eq!(a.snapshot.units[0].x, b.snapshot.units[0].x);
    }
}
