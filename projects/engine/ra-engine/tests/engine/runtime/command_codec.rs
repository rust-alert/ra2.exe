//! `GameCommand` / `ScheduledCommand` 编解码集成测试。

use ra_engine::{GameCommand, decode_commands, decode_scheduled, encode_commands, encode_scheduled};
use ra_types::{CommandId, EntityId, PlayerId, ScheduledCommand, Tick, TypeId};

#[test]
fn command_codec_roundtrip() {
    let cmds = vec![
        GameCommand::MoveTo { entity: EntityId(3), x: 10, y: 20 },
        GameCommand::MovePath { entity: EntityId(3), points: vec![(10, 20), (12, 22), (14, 18)] },
        GameCommand::Attack { attacker: EntityId(3), target: EntityId(7) },
        GameCommand::Deploy { entity: EntityId(1) },
        GameCommand::PlaceBuilding { player: PlayerId(0), type_id: TypeId(1), x: 6, y: 4 },
        GameCommand::Produce { player: PlayerId(1), type_id: TypeId(2) },
        GameCommand::SetRallyPoint { factory: EntityId(2), x: 9, y: 3 },
        GameCommand::Infiltrate { agent: EntityId(8), building: EntityId(9) },
        GameCommand::CancelProduce { player: PlayerId(0), type_id: TypeId(2) },
        GameCommand::CaptureBuilding { engineer: EntityId(10), building: EntityId(11) },
        GameCommand::Guard { entity: EntityId(12) },
        GameCommand::Stop { entity: EntityId(15) },
        GameCommand::AttackMove { entity: EntityId(16), x: 4, y: 5 },
        GameCommand::Scatter { entity: EntityId(17) },
        GameCommand::Delete { entity: EntityId(18) },
        GameCommand::SellBuilding { player: PlayerId(0), building: EntityId(13) },
        GameCommand::RepairBuilding { player: PlayerId(0), building: EntityId(14) },
    ];
    let bytes = encode_commands(&cmds);
    assert_eq!(decode_commands(&bytes), Some(cmds));
}

#[test]
fn scheduled_codec_roundtrip() {
    let cmd = ScheduledCommand::new(CommandId(9), PlayerId(2), Tick(15), GameCommand::MoveTo { entity: EntityId(4), x: 1, y: 2 });
    let bytes = encode_scheduled(&cmd);
    assert_eq!(decode_scheduled(&bytes), Some(cmd));
}
