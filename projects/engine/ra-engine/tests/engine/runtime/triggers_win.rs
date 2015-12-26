//! 触发器：计时条件可驱动 Win。

use crate::common::{test_engine};
use ra_adaptor::RulesSystem;
use ra_assets::{CountryRegistry, ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{BattleOutcome, BattleState, Session, SessionBootKind};
use ra_map::MapInfo;
use ra_types::GameEdition;

fn empty_rules() -> RulesSystem {
    let rules = IniDocument::parse(b"[General]\n").unwrap();
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
fn timer_trigger_fires_win_on_campaign() {
    let text = b"\
[Map]\nSize=0,0,8,8\nTheater=TEMPERATE\n\
[Houses]\n0=Americans\n\
[Americans]\nCountry=Americans\nPlayerControl=yes\n\
[Tags]\nT1=0,Start,TR1\n\
[Triggers]\nTR1=Americans,<none>,Mission Start,0,1,1,1,0\n\
[Events]\nTR1=1,13,0,0\n\
[Actions]\nTR1=1,1,0,0,0,0,0,0,Americans\n\
";
    let map = MapInfo::parse_ini(GameEdition::Ra2, "trig.map", text).unwrap();
    assert!(!map.scripting.triggers.is_empty());
    let engine = test_engine();
    let mut session = Session::from_state(BattleState::new(GameEdition::Ra2, &empty_rules(), map), "trig");
    session.expect_battle_mut().boot_kind = SessionBootKind::Campaign;
    session.expect_battle_mut().world.ensure_house("Americans");
    let _ = session.expect_battle_mut().world.prefer_local_house("Americans");

    for _ in 0..5 {
        session.tick(&engine.runtime());
        if session.expect_battle().outcome.is_some() {
            break;
        }
    }
    assert_eq!(
        session.expect_battle().outcome,
        Some(BattleOutcome::Victory {
            owner: "Americans".into()
        })
    );
}
