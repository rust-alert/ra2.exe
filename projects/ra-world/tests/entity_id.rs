//! `EntityId`、玩家状态与命令拒绝。

use ra_adaptor::RulesDb;
use ra_assets::{ColorSchemes, IniDocument, OverlayTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};
use ra_world::{CommandRejectReason, GameCommand, World};

fn duel_world() -> World {
    let rules_text = b"[VehicleTypes]\n0=MTNK\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nArmor=heavy\n";
    let rules = IniDocument::parse(rules_text).expect("测试 INI 必须有效");
    let rules_db = RulesDb {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
    };
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
    World::new(GameEdition::Ra2, &rules_db, map)
}

#[test]
fn seeds_stable_entity_ids_and_players() {
    let world = duel_world();
    assert_eq!(world.entities[0].id, EntityId(1));
    assert_eq!(world.entities[1].id, EntityId(2));
    assert_eq!(world.entity_index(EntityId(2)), Some(1));
    assert_eq!(world.entity_index(EntityId(99)), None);
    assert_eq!(world.players.len(), 2);
    assert_eq!(world.players[0].house, "Americans");
    assert_eq!(world.players[1].house, "Russians");
    assert_eq!(world.players[0].funds, 0);
}

#[test]
fn records_reject_for_missing_entity_command() {
    let mut world = duel_world();
    world.push_command(GameCommand::MoveTo { entity_index: 99, x: 1, y: 1 });
    world.advance_tick();
    assert_eq!(world.last_rejects().len(), 1);
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::EntityNotFound);
    assert_eq!(world.entities[0].x, 4);
}

#[test]
fn records_reject_for_self_attack() {
    let mut world = duel_world();
    world.push_command(GameCommand::Attack { attacker_index: 0, target_index: 0 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
    assert!(world.entities[0].attack_target.is_none());
}
