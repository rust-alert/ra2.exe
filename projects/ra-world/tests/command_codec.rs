//! `GameCommand` 编解码集成测试。

use ra_world::{GameCommand, decode_commands, encode_commands};

#[test]
fn command_codec_roundtrip() {
    let cmds =
        vec![GameCommand::MoveTo { entity_index: 3, x: 10, y: 20 }, GameCommand::Attack { attacker_index: 3, target_index: 7 }];
    let bytes = encode_commands(&cmds);
    assert_eq!(decode_commands(&bytes), Some(cmds));
}
