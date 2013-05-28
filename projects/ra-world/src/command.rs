//! 玩家/AI 注入的确定性命令（按 tick 排序消费）。

/// 单条命令。后续扩展生产、部署等。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    /// 将实体移动到目标格（会重算路径）。
    MoveTo {
        entity_index: usize,
        x: u16,
        y: u16,
    },
    /// 指定攻击目标（进入射程后造成伤害）。
    Attack {
        attacker_index: usize,
        target_index: usize,
    },
}

/// 一个仿真 tick 的完整输入帧。
///
/// 联机锁步要求：**每个 tick 都必须有一帧**；本 tick 无操作时 `commands` 为空。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InputFrame {
    pub tick: u64,
    pub commands: Vec<GameCommand>,
}

impl InputFrame {
    pub fn empty(tick: u64) -> Self {
        Self {
            tick,
            commands: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

/// 编码单条命令为网络 `payload` 字节。
pub fn encode_command(cmd: &GameCommand) -> Vec<u8> {
    let mut b = Vec::new();
    match *cmd {
        GameCommand::MoveTo {
            entity_index,
            x,
            y,
        } => {
            b.push(1);
            b.extend_from_slice(&(entity_index as u32).to_be_bytes());
            b.extend_from_slice(&x.to_be_bytes());
            b.extend_from_slice(&y.to_be_bytes());
        }
        GameCommand::Attack {
            attacker_index,
            target_index,
        } => {
            b.push(2);
            b.extend_from_slice(&(attacker_index as u32).to_be_bytes());
            b.extend_from_slice(&(target_index as u32).to_be_bytes());
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
            Some(GameCommand::MoveTo {
                entity_index,
                x,
                y,
            })
        }
        2 => {
            if bytes.len() < 1 + 4 + 4 {
                return None;
            }
            let attacker_index = u32::from_be_bytes(bytes[1..5].try_into().ok()?) as usize;
            let target_index = u32::from_be_bytes(bytes[5..9].try_into().ok()?) as usize;
            Some(GameCommand::Attack {
                attacker_index,
                target_index,
            })
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

#[cfg(test)]
mod codec_tests {
    use super::*;

    #[test]
    fn command_codec_roundtrip() {
        let cmds = vec![
            GameCommand::MoveTo {
                entity_index: 3,
                x: 10,
                y: 20,
            },
            GameCommand::Attack {
                attacker_index: 3,
                target_index: 7,
            },
        ];
        let bytes = encode_commands(&cmds);
        assert_eq!(decode_commands(&bytes), Some(cmds));
    }
}
