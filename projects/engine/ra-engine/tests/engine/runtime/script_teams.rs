//! Create Team 动作可生成 TaskForce 单位。

use crate::common::test_engine;
use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleState, Session, SessionBootKind};
use ra_map::MapInfo;
use ra_types::GameEdition;

fn rules_with_e1() -> RulesSystem {
    let rules = IniDocument::parse(
        b"[InfantryTypes]\n0=E1\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nArmor=none\nOwner=Russians\n",
    )
    .unwrap();
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    }
}

#[test]
fn create_team_action_spawns_task_force() {
    let text = b"\
[Map]\nSize=0,0,16,16\nTheater=TEMPERATE\n\
[Waypoints]\n0=5,5\n\
[Triggers]\nTR1=Russians,<none>,Reinforce,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,4,0,TM1,0,0,0,0,A\n\
[TaskForces]\n0=TF1\n\
[TF1]\nName=Squad\n0=2,E1\nGroup=-1\n\
[TeamTypes]\n0=TM1\n\
[TM1]\nName=Team\nHouse=Russians\nScript=SC1\nTaskForce=TF1\nMax=1\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "team.map", text).unwrap();
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &rules_with_e1(), map), "team");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.tick(&engine.runtime());
    let snap = session.expect_battle().snapshot(&[]);
    let e1 = snap.units.iter().filter(|u| u.type_id.as_ref() == "E1").count();
    assert!(e1 >= 2, "expected reinforced E1 units, got {e1} in {:?}", snap.units.iter().map(|u| u.type_id.as_ref()).collect::<Vec<_>>());
}
