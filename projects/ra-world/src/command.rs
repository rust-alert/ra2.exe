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
