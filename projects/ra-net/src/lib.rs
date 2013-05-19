//! 联机对战的协议无关基础类型。
//!
//! 本 crate 不打开 socket，也不依赖具体 Web 或桌面传输层。平台壳负责传输，
//! `ra-world` 负责消费命令并产生状态哈希。

use ra_types::PlayerId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchId(pub u128);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchFingerprint {
    pub edition: String,
    pub map: String,
    pub rules_hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputCommand {
    pub player: PlayerId,
    pub sequence: u32,
    pub tick: u64,
    pub payload: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDigest {
    pub tick: u64,
    pub hash: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionMessage {
    Hello { protocol: u16, fingerprint: MatchFingerprint },
    Command(InputCommand),
    Digest(StateDigest),
    ResyncRequest { tick: u64 },
    ResyncSnapshot { tick: u64, bytes: Vec<u8> },
}
