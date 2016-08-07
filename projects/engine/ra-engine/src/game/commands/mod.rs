//! 玩家/AI 注入的确定性命令（按 tick 排序消费）。
//!
//! 对外调度形状为 [`ScheduledCommand`]；[`GameCommand`] 是其中的可执行载荷（与 `ra_types::CommandBody` 同一类型）。

mod codec;
mod scheduled;
mod types;
mod validation;

pub use codec::{decode_command, decode_commands, encode_command, encode_commands};
pub use scheduled::{decode_scheduled, encode_scheduled};
pub use types::{GameCommand, InputFrame};
