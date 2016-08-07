use ra_types::{EntityId, PlayerId};

use super::types::GameCommand;


/// 编码单条命令载荷为网络 `payload` 字节（不含调度信封）。
pub fn encode_command(cmd: &GameCommand) -> Vec<u8> {
    let mut b = Vec::new();
    match *cmd {
        GameCommand::MoveTo { entity, x, y } => {
            b.push(1);
            b.extend_from_slice(&entity.0.to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::MovePath { entity, ref points } => {
            b.push(13);
            b.extend_from_slice(&entity.0.to_be_bytes());
            let n = points.len().min(u16::MAX as usize) as u16;
            b.extend_from_slice(&n.to_be_bytes());
            for &(x, y) in points.iter().take(n as usize) {
                b.extend_from_slice(&x.to_be_bytes());
                b.extend_from_slice(&y.to_be_bytes());
            }
        }
        GameCommand::Attack { attacker, target } => {
            b.push(2);
            b.extend_from_slice(&attacker.0.to_be_bytes());
            b.extend_from_slice(&target.0.to_be_bytes());
        }
        GameCommand::Deploy { entity } => {
            b.push(3);
            b.extend_from_slice(&entity.0.to_be_bytes());
        }
        GameCommand::PlaceBuilding { player, ref type_id, x, y } => {
            b.push(4);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Produce { player, ref type_id } => {
            b.push(5);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
        }
        GameCommand::SetRallyPoint { factory, x, y } => {
            b.push(6);
            b.extend_from_slice(&factory.0.to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Infiltrate { agent, building } => {
            b.push(7);
            b.extend_from_slice(&agent.0.to_be_bytes());
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::CancelProduce { player, ref type_id } => {
            b.push(8);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
        }
        GameCommand::CaptureBuilding { engineer, building } => {
            b.push(9);
            b.extend_from_slice(&engineer.0.to_be_bytes());
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::Guard { entity } => {
            b.push(10);
            b.extend_from_slice(&entity.0.to_be_bytes());
        }
        GameCommand::SellBuilding { player, building } => {
            b.push(11);
            b.push(player.0);
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::RepairBuilding { player, building } => {
            b.push(12);
            b.push(player.0);
            b.extend_from_slice(&building.0.to_be_bytes());
        }
        GameCommand::FireSuperWeapon {
            player,
            ref type_id,
            x,
            y,
        } => {
            b.push(14);
            b.push(player.0);
            let id_bytes = type_id.as_bytes();
            b.extend_from_slice(&(id_bytes.len() as u16).to_be_bytes());
            b.extend_from_slice(id_bytes);
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
    }
    b
}

/// 解码单条命令；失败返回 `None`。
pub fn decode_command(bytes: &[u8]) -> Option<GameCommand> {
    if bytes.is_empty() {
        return None;
    }
    match bytes[0] {
        1 => {
            if bytes.len() < 1 + 8 + 2 + 2 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let x = u16::from_be_bytes(bytes[9..11].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[11..13].try_into().ok()?);
            Some(GameCommand::MoveTo { entity, x, y })
        }
        2 => {
            if bytes.len() < 1 + 8 + 8 {
                return None;
            }
            let attacker = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let target = EntityId(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
            Some(GameCommand::Attack { attacker, target })
        }
        3 => {
            if bytes.len() < 1 + 8 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            Some(GameCommand::Deploy { entity })
        }
        4 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len + 2 + 2 {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            let xy = 4 + id_len;
            let x = u16::from_be_bytes(bytes[xy..xy + 2].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[xy + 2..xy + 4].try_into().ok()?);
            Some(GameCommand::PlaceBuilding { player, type_id, x, y })
        }
        5 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            Some(GameCommand::Produce { player, type_id })
        }
        6 => {
            if bytes.len() < 1 + 8 + 2 + 2 {
                return None;
            }
            let factory = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let x = u16::from_be_bytes(bytes[9..11].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[11..13].try_into().ok()?);
            Some(GameCommand::SetRallyPoint { factory, x, y })
        }
        7 => {
            if bytes.len() < 1 + 8 + 8 {
                return None;
            }
            let agent = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let building = EntityId(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
            Some(GameCommand::Infiltrate { agent, building })
        }
        8 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            Some(GameCommand::CancelProduce { player, type_id })
        }
        9 => {
            if bytes.len() < 1 + 8 + 8 {
                return None;
            }
            let engineer = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let building = EntityId(u64::from_be_bytes(bytes[9..17].try_into().ok()?));
            Some(GameCommand::CaptureBuilding { engineer, building })
        }
        10 => {
            if bytes.len() < 1 + 8 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            Some(GameCommand::Guard { entity })
        }
        11 => {
            if bytes.len() < 1 + 1 + 8 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let building = EntityId(u64::from_be_bytes(bytes[2..10].try_into().ok()?));
            Some(GameCommand::SellBuilding { player, building })
        }
        12 => {
            if bytes.len() < 1 + 1 + 8 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let building = EntityId(u64::from_be_bytes(bytes[2..10].try_into().ok()?));
            Some(GameCommand::RepairBuilding { player, building })
        }
        13 => {
            if bytes.len() < 1 + 8 + 2 {
                return None;
            }
            let entity = EntityId(u64::from_be_bytes(bytes[1..9].try_into().ok()?));
            let n = u16::from_be_bytes(bytes[9..11].try_into().ok()?) as usize;
            let need = 1 + 8 + 2 + n * 4;
            if bytes.len() < need {
                return None;
            }
            let mut points = Vec::with_capacity(n);
            let mut off = 11;
            for _ in 0..n {
                let x = u16::from_be_bytes(bytes[off..off + 2].try_into().ok()?);
                let y = u16::from_be_bytes(bytes[off + 2..off + 4].try_into().ok()?);
                points.push((x, y));
                off += 4;
            }
            Some(GameCommand::MovePath { entity, points })
        }
        14 => {
            if bytes.len() < 1 + 1 + 2 {
                return None;
            }
            let player = PlayerId(bytes[1]);
            let id_len = u16::from_be_bytes(bytes[2..4].try_into().ok()?) as usize;
            if bytes.len() < 1 + 1 + 2 + id_len + 2 + 2 {
                return None;
            }
            let type_id = std::str::from_utf8(&bytes[4..4 + id_len]).ok()?.to_string();
            let xy = 4 + id_len;
            let x = u16::from_be_bytes(bytes[xy..xy + 2].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[xy + 2..xy + 4].try_into().ok()?);
            Some(GameCommand::FireSuperWeapon { player, type_id, x, y })
        }
        _ => None,
    }
}

/// 编码命令列表：`[u16 count][cmd…]`（每条前缀 `u16` 长度）。
pub fn encode_commands(cmds: &[GameCommand]) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&(cmds.len() as u16).to_be_bytes());
    for cmd in cmds {
        let one = encode_command(cmd);
        b.extend_from_slice(&(one.len() as u16).to_be_bytes());
        b.extend_from_slice(&one);
    }
    b
}

/// 解码命令载荷列表。
pub fn decode_commands(bytes: &[u8]) -> Option<Vec<GameCommand>> {
    if bytes.len() < 2 {
        return None;
    }
    let count = u16::from_be_bytes(bytes[0..2].try_into().ok()?) as usize;
    let mut i = 2usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        if i + 2 > bytes.len() {
            return None;
        }
        let n = u16::from_be_bytes(bytes[i..i + 2].try_into().ok()?) as usize;
        i += 2;
        if i + n > bytes.len() {
            return None;
        }
        let cmd = decode_command(&bytes[i..i + n])?;
        out.push(cmd);
        i += n;
    }
    Some(out)
}
