//! 攻击建筑：伤害、死亡清格与电力回收。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::GameCommand;
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition};

fn atk_structure_defs(strength: &str) -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    let ini = format!(
        "[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GAPOWR\n\
[MTNK]\nStrength=400\nSpeed=64\nSight=6\nCost=800\nArmor=none\nPrimary=Gun\n\
[GAPOWR]\nPower=200\nOwner=Americans\nStrength={strength}\nSight=4\nCost=600\nArmor=wood\n\
[Gun]\nDamage=40\nROF=1\nRange=6\nWarhead=SA\n\
[SA]\nVerses=100%,100%,100%,100%,100%,100%,100%,100%,100%,100%,100%\n"
    );
    defs_from_rules_ini(ini.as_bytes())
}

fn atk_structure_world(strength: &str, map_name: &str) -> ra_engine::BattleState {
    let defs = atk_structure_defs(strength);
    let mut map = MapInfo::empty(GameEdition::Ra2, map_name);
    map.width = 16;
    map.height = 16;
    map.entities.push(MapEntity {
        kind: MapEntityKind::Unit,
        owner: "AMERICANS".into(),
        type_id: "MTNK".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "SOVIETS".into(),
        type_id: "GAPOWR".into(),
        health: 256,
        x: 6,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    });
    battle_from_defs(GameEdition::Ra2, defs, map)
}

#[test]
fn attack_structure_kills_and_frees_cell() {
    let mut world = atk_structure_world("80", "atk-bldg");
    world.pass_grid.set_passable(6, 4, false);
    world.players[1].power_output = 200;
    assert!(!world.pass_grid.is_passable(6, 4));
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });
    for _ in 0..10 {
        world.advance_tick();
        if world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2 {
            break;
        }
    }
    assert!(world.ecs_health(world.entity_id_at(1).expect("entity")).expect("health").2);
    assert!(world.pass_grid.is_passable(6, 4));
    assert_eq!(world.players[1].power_output, 0);
}

#[test]
fn structure_damage_cues_base_under_attack_once_per_suppress_window() {
    let mut world = atk_structure_world("400", "atk-base-eva");
    world.pass_grid.set_passable(6, 4, false);
    world.push_command(GameCommand::Attack { attacker: EntityId(1), target: EntityId(2) });

    let mut first_hit = false;
    for _ in 0..8 {
        world.advance_tick();
        let cues = world.take_eva_cues();
        let hit = cues.iter().any(|c| c.event == "EVA_OurBaseIsUnderAttack" && c.house.as_ref() == "SOVIETS");
        if hit {
            first_hit = true;
            break;
        }
    }
    assert!(first_hit, "first structure hit must cue EVA_OurBaseIsUnderAttack");

    let mut repeats = 0u32;
    for _ in 0..30 {
        world.advance_tick();
        let cues = world.take_eva_cues();
        if cues.iter().any(|c| c.event == "EVA_OurBaseIsUnderAttack" && c.house.as_ref() == "SOVIETS") {
            repeats = repeats.saturating_add(1);
        }
    }
    assert_eq!(repeats, 0, "suppress window must block repeat base-under-attack EVA");
}
