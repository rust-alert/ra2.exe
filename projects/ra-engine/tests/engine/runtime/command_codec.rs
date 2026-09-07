//! `GameCommand` 编解码集成测试。

use ra_engine::{GameCommand, decode_commands, encode_commands};
use ra_types::PlayerId;

#[test]
fn command_codec_roundtrip() {
    let cmds = vec![
        GameCommand::MoveTo { entity_index: 3, x: 10, y: 20 },
        GameCommand::Attack { attacker_index: 3, target_index: 7 },
        GameCommand::Deploy { entity_index: 1 },
        GameCommand::PlaceBuilding { player: PlayerId(0), type_id: "GAPOWR".into(), x: 6, y: 4 },
        GameCommand::Produce { player: PlayerId(1), type_id: "E1".into() },
        GameCommand::SetRallyPoint { factory_index: 2, x: 9, y: 3 },
    ];
    let bytes = encode_commands(&cmds);
    assert_eq!(decode_commands(&bytes), Some(cmds));
}
