//! AI 经 Deploy 展开 MCV。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::{Session, MatchState};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::GameEdition;

#[test]
fn ai_deploys_mcv_via_command() {
    let doc = IniDocument::parse(
        b"[VehicleTypes]\n0=SMCV\n\
[BuildingTypes]\n0=NACNST\n1=GACNST\n\
[SMCV]\nStrength=1000\nSpeed=32\nSight=4\nCost=2500\n\
[NACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n\
[GACNST]\nStrength=1000\nSight=8\nCost=2500\nArmor=concrete\n",
    )
    .unwrap();
    let rules = RulesDb {
        edition: GameEdition::Ra2,
        rules: doc.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&doc),
        warheads: WarheadRegistry::default(),
    };
    let mut map = MapInfo::empty(GameEdition::Ra2, "ai-deploy");
    map.width = 16;
    map.height = 16;
    // 本地玩家先占 PlayerId(0)；AI 控制 Soviets。
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Americans".into(),
        type_id: "GACNST".into(),
        health: 256,
        x: 2,
        y: 2,
        facing: 0,
        sub_cell: 0,
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "Soviets".into(),
        type_id: "SMCV".into(),
        health: 256,
        x: 8,
        y: 8,
        facing: 0,
        sub_cell: 0,
    });
    let mut session = Session::from_state(MatchState::new(GameEdition::Ra2, &rules, map), "ai-deploy");
    session.ai_enabled = true;
    session.tick();
    assert_eq!(session.world.entities[1].kind, MapEntityKind::Structure);
    assert_eq!(session.world.entities[1].type_id, "NACNST");
}
