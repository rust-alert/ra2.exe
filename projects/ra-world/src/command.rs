//! 玩家/AI 注入的确定性命令（按 tick 排序消费）。

/// 单条命令。后续扩展攻击、生产、部署等。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameCommand {
    /// 将实体移动到目标格（会重算路径）。
    MoveTo {
        entity_index: usize,
        x: u16,
        y: u16,
    },
}
