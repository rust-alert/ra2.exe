//! AI 经 GameCommand 自动攻击。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use crate::common::test_engine;
use ra_engine::{Session, MatchState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

fn duel_session() -> Session {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\n\
[Gun]\nDamage=40\nROF=2\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n",
    )
    .unwrap();
    let techno_types = TechnoTypeRegistry::from_rules(&doc);
    let warheads = WarheadRegistry::from_names(&doc, techno_types.iter().map(|t| t.warhead.as_str()));
    let rules = RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types,
        warheads,
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-duel");
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Americans".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 6,
        y: 4,
        facing: 0,
        sub_cell: 0,
    });
    let mut session = Session::from_state(MatchState::new(GameEdition::Ra2, &rules, map), "ai");
    session.expect_game_mut().ai_enabled = true;
    session
}

#[test]
fn ai_issues_attack_via_commands() {
    let engine = test_engine();
    let mut session = duel_session();
    let before = session.expect_game_mut().world.entities[0].health;
    for _ in 0..30 {
        session.tick(&engine.runtime());
    }
    assert!(session.expect_game_mut().world.entities[1].attack_target == Some(EntityId(1)) || session.expect_game_mut().world.entities[0].health < before);
    assert!(session.expect_game_mut().world.entities[0].health < before);
}
