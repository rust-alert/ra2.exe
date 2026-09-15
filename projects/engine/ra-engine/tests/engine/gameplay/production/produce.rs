//! 兵营 / 战车工厂生产。

use crate::common::{battle_from_defs, defs_from_rules_ini};
use ra_engine::{BattleState, CommandRejectReason, GameCommand, PRODUCE_TICKS};
use ra_map::{MapEntity, MapEntityKind, MapInfo};
use ra_types::{EntityId, GameEdition, PlayerId};

fn factory_world() -> BattleState {
    let rules_text = b"[InfantryTypes]\n0=E1\n\
[VehicleTypes]\n0=MTNK\n\
[BuildingTypes]\n0=GAPILE\n1=GAWEAP\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n\
[MTNK]\nStrength=200\nSpeed=64\nSight=6\nCost=800\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n\
[GAWEAP]\nPower=-30\nPowered=yes\nFactory=UnitType\nOwner=Americans\nStrength=1000\nSight=5\nCost=2000\nTechLevel=1\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "produce");
    map.width = 16;
    map.height = 16;
    map.entities = vec![
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAPILE".into(),
            health: 256,
            x: 4,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
        MapEntity {
            kind: MapEntityKind::Structure,
            owner: "AMERICANS".into(),
            type_id: "GAWEAP".into(),
            health: 256,
            x: 8,
            y: 4,
            facing: 0,
            sub_cell: 0,
            mission: Default::default(),
            tag: Default::default(),
        },
    ];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 10_000));
    // 预放工厂耗电；补足供电，避免测例落入低电半速。
    world.players[0].power_output = 200;
    world
}

#[test]
fn produce_infantry_spawns_after_queue_ticks() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    // 边造边扣：首 tick 只扣一步，不是全额。
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 200 / PRODUCE_TICKS as i32));
    assert_eq!(world.entity_count(), 2);
    for _ in 0..(PRODUCE_TICKS - 1) {
        assert_eq!(world.entity_count(), 2);
        world.advance_tick();
    }
    assert_eq!(world.entity_count(), 3);
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 200));
    let unit = world.entity_id_at(2).expect("entity");
    assert_eq!(unit, EntityId(3));
    let identity = world.ecs_identity(unit).expect("id");
    assert_eq!(identity.1, MapEntityKind::Infantry);
    assert_eq!(identity.0.as_ref(), "E1");
    assert_eq!(world.ecs_owner(unit).expect("owner").as_ref(), "AMERICANS");
}

#[test]
fn produce_starts_with_insufficient_funds_and_pauses() {
    let mut world = factory_world();
    assert!(world.set_house_funds("AMERICANS", 50));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    // 原版：钱不够也能开单；首步应付 10，有钱则推进。
    assert!(world.last_rejects().is_empty(), "{:?}", world.last_rejects());
    let factory = world.entity_id_at(0).expect("barracks");
    assert!(world.ecs_produce_item(factory).expect("queue").is_some());
    assert_eq!(world.house_funds("AMERICANS"), Some(50 - 200 / PRODUCE_TICKS as i32));

    // 花光后推进应暂停（剩余 tick 不变）。
    assert!(world.set_house_funds("AMERICANS", 0));
    let rem_before = world.ecs_produce_remaining(factory).expect("queue").expect("item");
    world.advance_tick();
    let rem_after = world.ecs_produce_remaining(factory).expect("queue").expect("item");
    assert_eq!(rem_after, rem_before, "broke production must pause");
    assert_eq!(world.entity_count(), 2);

    // 补钱后自动继续，满 tick 出兵。
    assert!(world.set_house_funds("AMERICANS", 10_000));
    for _ in 0..PRODUCE_TICKS {
        if world.entity_count() >= 3 {
            break;
        }
        world.advance_tick();
    }
    assert_eq!(world.entity_count(), 3);
}

#[test]
fn produce_enqueues_second_unit_while_busy() {
    let mut world = factory_world();
    let e1 = world.definitions.techno.get("E1").expect("E1").id;
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: e1 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: e1 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "unit FIFO should accept a second Produce: {:?}", world.last_rejects());
    // 候补未开工不扣款；两 tick 只扣队首两步。
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - 2 * (200 / PRODUCE_TICKS as i32)));
    let factory = world.entity_id_at(0).expect("barracks");
    let queue = world.ecs_produce_unit_queue_len(factory).expect("queue");
    assert!(queue.0, "head item");
    assert_eq!(queue.1, 1, "one pending behind head");
}

#[test]
fn produce_fifo_promotes_pending_after_spawn() {
    let mut world = factory_world();
    let e1 = world.definitions.techno.get("E1").expect("E1").id;
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: e1 });
    world.advance_tick();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: e1 });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    for _ in 0..(PRODUCE_TICKS - 1) {
        world.advance_tick();
    }
    // 队首完工出兵后，候补应立即提拔为新队首。
    assert_eq!(world.entity_count(), 3);
    let factory = world.entity_id_at(0).expect("barracks");
    assert!(world.ecs_produce_item(factory).expect("queue").is_some(), "pending promoted to head");
    assert_eq!(world.ecs_produce_unit_queue_len(factory).expect("q").1, 0);
}

#[test]
fn produce_rejects_when_unit_queue_full() {
    let mut world = factory_world();
    let e1 = world.definitions.techno.get("E1").expect("E1").id;
    // 同一 tick 内连入队首 + 29 候补 = 30，避免推进期间队首完工缩队。
    for _ in 0..30 {
        world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: e1 });
    }
    world.advance_tick();
    assert!(world.last_rejects().is_empty(), "{:?}", world.last_rejects());
    let factory = world.entity_id_at(0).expect("barracks");
    assert_eq!(world.ecs_produce_unit_queue_len(factory).expect("q"), (true, 29));

    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: e1 });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::QueueFull);
}

#[test]
fn produce_rejects_without_matching_factory() {
    let mut world = factory_world();
    let id = world.entity_id_at(0).expect("entity");
    let max = world.ecs_health(world.entity_id_at(0).expect("entity")).expect("health").1;
    assert!(world.set_ecs_health(id, 0, max, true));
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::MissingPrerequisite);
}

#[test]
fn cancel_produce_refunds_and_clears_queue() {
    let mut world = factory_world();
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    let paid = 200 / PRODUCE_TICKS as i32;
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000 - paid));
    assert!(world.take_eva_cues().is_empty());

    world.push_command(GameCommand::CancelProduce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    assert_eq!(world.house_funds("AMERICANS"), Some(10_000));
    let cues = world.take_eva_cues();
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].house.as_ref(), "AMERICANS");
    assert_eq!(cues[0].event, "EVA_Canceled");

    // 取消后可再次排队，并在满 tick 后出兵。
    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    assert!(world.last_rejects().is_empty());
    for _ in 0..(PRODUCE_TICKS - 1) {
        world.advance_tick();
    }
    assert_eq!(world.entity_count(), 3);
}

#[test]
fn cancel_produce_rejects_when_not_queued() {
    let mut world = factory_world();
    world.push_command(GameCommand::CancelProduce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    assert_eq!(world.last_rejects()[0].reason, CommandRejectReason::InvalidTarget);
}

#[test]
fn funds_nag_repeats_on_speak_delay_while_broke_with_factory() {
    let rules_text = b"[AudioVisual]\nSpeakDelay=0.003\n\
[InfantryTypes]\n0=E1\n\
[BuildingTypes]\n0=GAPILE\n\
[E1]\nStrength=125\nSpeed=4\nSight=5\nCost=200\nTechLevel=1\n\
[GAPILE]\nPower=-20\nPowered=yes\nFactory=InfantryType\nOwner=Americans\nStrength=500\nSight=5\nCost=500\nTechLevel=1\n";
    let defs = defs_from_rules_ini(rules_text);
    let mut map = MapInfo::empty(GameEdition::Ra2, "funds_nag");
    map.width = 16;
    map.height = 16;
    map.entities = vec![MapEntity {
        kind: MapEntityKind::Structure,
        owner: "AMERICANS".into(),
        type_id: "GAPILE".into(),
        health: 256,
        x: 4,
        y: 4,
        facing: 0,
        sub_cell: 0,
        mission: Default::default(),
        tag: Default::default(),
    }];
    let mut world = battle_from_defs(GameEdition::Ra2, defs, map);
    assert!(world.set_house_funds("AMERICANS", 50));
    // SpeakDelay=0.003 → ftol(2.7)=2 tick 周期。

    world.advance_tick();
    let first = world.take_eva_cues();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].event, "EVA_InsufficientFunds");

    world.advance_tick();
    assert!(world.take_eva_cues().is_empty(), "冷却中不应再播");

    world.advance_tick();
    let again = world.take_eva_cues();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0].event, "EVA_InsufficientFunds");
}

#[test]
fn low_power_halves_production_tick_rate() {
    let mut world = factory_world();
    // 工厂种子未记账电力；显式制造低电。
    world.players[0].power_output = 0;
    world.players[0].power_drain = 50;
    assert!(world.players[0].low_power());

    world.push_command(GameCommand::Produce { player: PlayerId(0), type_id: world.definitions.techno.get("E1").expect("E1").id });
    world.advance_tick();
    let factory = world.entity_id_at(0).expect("barracks");
    let start = world.ecs_produce_remaining(factory).expect("queue").expect("item");

    // 满电时这两次都会扣 tick；低电时奇 tick 跳过，只扣一次。
    world.advance_tick();
    world.advance_tick();
    let after = world.ecs_produce_remaining(factory).expect("queue").expect("item");
    assert_eq!(after, start.saturating_sub(1), "low power should advance production only on even ticks");
}
