//! 玩家/AI 注入的确定性命令（按 tick 排序消费）。

use ra_types::PlayerId;

/// 单条命令。后续扩展生产等。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    /// 将实体移动到目标格（会重算路径）。
    MoveTo {
        /// 实体在世界实体列表中的下标。
        entity_index: usize,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 指定攻击目标（进入射程后造成伤害）。
    Attack {
        /// 攻击方实体下标。
        attacker_index: usize,
        /// 被攻击方实体下标。
        target_index: usize,
    },
    /// 部署可展开实体（如 MCV → 建造场）。
    Deploy {
        /// 实体在世界实体列表中的下标。
        entity_index: usize,
    },
    /// 在目标格放置建筑（扣费、校验前置与占地）。
    PlaceBuilding {
        /// 出资并拥有该建筑的玩家。
        player: PlayerId,
        /// 规则类型 ID（如 `GAPOWR`）。
        type_id: String,
        /// 目标格 X。
        x: u16,
        /// 目标格 Y。
        y: u16,
    },
    /// 在空闲工厂排队生产单位（立即扣费）。
    Produce {
        /// 出资并拥有产出单位的玩家。
        player: PlayerId,
        /// 规则类型 ID（如 `E1` / `MTNK`）。
        type_id: String,
    },
    /// 为工厂设置生产集结点。
    SetRallyPoint {
        /// 工厂实体下标。
        factory_index: usize,
        /// 集结格 X。
        x: u16,
        /// 集结格 Y。
        y: u16,
    },
}

/// 一个仿真 tick 的完整输入帧。
///
/// 联机锁步要求：**每个 tick 都必须有一帧**；本 tick 无操作时 `commands` 为空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputFrame {
    /// 本帧对应的逻辑 tick。
    pub tick: u64,
    /// 本 tick 消费的命令列表（可为空）。
    pub commands: Vec<GameCommand>,
}

impl InputFrame {
    /// 构造仅含 tick、无命令的空输入帧。
    pub fn empty(tick: u64) -> Self {
        Self { tick, commands: Vec::new() }
    }

    /// 本帧是否没有任何命令。
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// 编码单条命令为网络 `payload` 字节。
pub fn encode_command(cmd: &GameCommand) -> Vec<u8> {
    let mut b = Vec::new();
    match *cmd {
        GameCommand::MoveTo { entity_index, x, y } => {
            b.push(1);
            b.extend_from_slice(&(entity_index as u32).to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Attack { attacker_index, target_index } => {
            b.push(2);
            b.extend_from_slice(&(attacker_index as u32).to_be_bytes());
            b.extend_from_slice(&(target_index as u32).to_be_bytes());
        }
        GameCommand::Deploy { entity_index } => {
            b.push(3);
            b.extend_from_slice(&(entity_index as u32).to_be_bytes());
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
        GameCommand::SetRallyPoint { factory_index, x, y } => {
            b.push(6);
            b.extend_from_slice(&(factory_index as u32).to_be_bytes());
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
            if bytes.len() < 1 + 4 + 2 + 2 {
                return None;
            }
            let entity_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let x = u16::from_be_bytes(bytes[5..7].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[7..9].try_into().ok()?);
            Some(GameCommand::MoveTo { entity_index, x, y })
        }
        2 => {
            if bytes.len() < 1 + 4 + 4 {
                return None;
            }
            let attacker_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let target_index = u32::from_be_bytes(bytes[5..9].try_into().ok()?) as usize;
            Some(GameCommand::Attack { attacker_index, target_index })
        }
        3 => {
            if bytes.len() < 1 + 4 {
                return None;
            }
            let entity_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            Some(GameCommand::Deploy { entity_index })
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
            if bytes.len() < 1 + 4 + 2 + 2 {
                return None;
            }
            let factory_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let x = u16::from_be_bytes(bytes[5..7].try_into().ok()?);
            let y = u16::from_be_bytes(bytes[7..9].try_into().ok()?);
            Some(GameCommand::SetRallyPoint { factory_index, x, y })
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

/// 解码命令列表。
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
