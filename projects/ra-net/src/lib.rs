//! 联机对战的协议无关基础类型。
//!
//! 本 crate 不打开 socket，也不依赖具体 Web 或桌面传输层。平台壳负责传输，
//! `ra-world` 负责消费命令并产生状态哈希。

#![deny(missing_docs)]

mod frame;
mod seq;

use ra_types::PlayerId;

pub use frame::{NetCodecError, decode_frame, encode_frame, fingerprint_matches_rules};
pub use seq::SequenceWindow;

/// 线协议主版本（握手 Hello 使用）。不兼容变更时递增。
pub const PROTOCOL_VERSION: u16 = 1;

/// 单条消息载荷上限（字节，不含外层成帧）。
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024;

/// 对局唯一标识（128 位）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchId(
    /// 原始 128 位对局 ID。
    pub u128,
);

/// 握手阶段用于校验规则与地图一致性的指纹。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchFingerprint {
    /// 游戏版本标识（如 `ra2`）。
    pub edition: String,
    /// 地图文件名。
    pub map: String,
    /// 规则字节的 FNV-1a 64 位哈希。
    pub rules_hash: u64,
}

impl MatchFingerprint {
    /// 由版本、地图名与规则字节构建握手指纹。
    pub fn build(edition: &str, map: &str, rules_bytes: &[u8]) -> Self {
        Self { edition: edition.to_string(), map: map.to_string(), rules_hash: fnv1a64(rules_bytes) }
    }

    /// 把额外材料混入 `rules_hash`（地图尺寸、实体数等）。
    pub fn mix_bytes(mut self, extra: &[u8]) -> Self {
        self.rules_hash = fnv1a64_continue(self.rules_hash, extra);
        self
    }
}

/// 对字节序列计算 FNV-1a 64 位哈希。
pub fn fnv1a64(data: &[u8]) -> u64 {
    fnv1a64_continue(0xcbf29ce484222325, data)
}

fn fnv1a64_continue(mut hash: u64, data: &[u8]) -> u64 {
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// 玩家输入命令（序号、逻辑 tick 与载荷）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCommand {
    /// 发送方玩家 ID。
    pub player: PlayerId,
    /// 该玩家命令流中的严格递增序号。
    pub sequence: u32,
    /// 命令所属逻辑 tick。
    pub tick: u64,
    /// 命令载荷（由上层解释）。
    pub payload: Vec<u8>,
}

/// 某一逻辑 tick 的世界状态摘要。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDigest {
    /// 摘要对应的逻辑 tick。
    pub tick: u64,
    /// 状态哈希值。
    pub hash: u64,
}

/// 会话层消息（握手、命令、摘要与重同步）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMessage {
    /// 握手：协议版本与对局指纹。
    Hello {
        /// 发送方声明的协议版本。
        protocol: u16,
        /// 对局指纹。
        fingerprint: MatchFingerprint,
    },
    /// 玩家输入命令。
    Command(InputCommand),
    /// 状态摘要。
    Digest(StateDigest),
    /// 请求从指定 tick 起重同步。
    ResyncRequest {
        /// 希望对齐到的逻辑 tick。
        tick: u64,
    },
    /// 重同步快照数据。
    ResyncSnapshot {
        /// 快照对应的逻辑 tick。
        tick: u64,
        /// 序列化快照字节。
        bytes: Vec<u8>,
    },
}
