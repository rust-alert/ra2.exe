use ra_types::{CommandId, PlayerId, ScheduledCommand, Tick};

use super::codec::{decode_command, encode_command};

/// 编码已调度命令：信封 + 载荷。
pub fn encode_scheduled(cmd: &ScheduledCommand) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&cmd.id.0.to_be_bytes());
    b.push(cmd.player.0);
    b.extend_from_slice(&cmd.tick.0.to_be_bytes());
    let body = encode_command(&cmd.body);
    b.extend_from_slice(&(body.len() as u16).to_be_bytes());
    b.extend_from_slice(&body);
    b
}

/// 解码已调度命令。
pub fn decode_scheduled(bytes: &[u8]) -> Option<ScheduledCommand> {
    if bytes.len() < 8 + 1 + 8 + 2 {
        return None;
    }
    let id = CommandId(u64::from_be_bytes(bytes[0..8].try_into().ok()?));
    let player = PlayerId(bytes[8]);
    let tick = Tick(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
    let n = u16::from_be_bytes(bytes[17..19].try_into().ok()?) as usize;
    if bytes.len() < 19 + n {
        return None;
    }
    let body = decode_command(&bytes[19..19 + n])?;
    Some(ScheduledCommand::new(id, player, tick, body))
}
