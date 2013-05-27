//! 联机对战的协议无关基础类型。
//!
//! 本 crate 不打开 socket，也不依赖具体 Web 或桌面传输层。平台壳负责传输，
//! `ra-world` 负责消费命令并产生状态哈希。

mod frame;
mod seq;

use ra_types::PlayerId;

pub use frame::{
    decode_frame, encode_frame, fingerprint_matches_rules, NetCodecError,
};
pub use seq::SequenceWindow;

/// 线协议主版本（握手 Hello 使用）。不兼容变更时递增。
pub const PROTOCOL_VERSION: u16 = 1;

/// 单条消息载荷上限（字节，不含外层成帧）。
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MatchId(pub u128);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchFingerprint {
    pub edition: String,
    pub map: String,
    pub rules_hash: u64,
}

impl MatchFingerprint {
    /// 由版本、地图名与规则字节构建握手指纹。
    pub fn build(edition: &str, map: &str, rules_bytes: &[u8]) -> Self {
        Self {
            edition: edition.to_string(),
            map: map.to_string(),
            rules_hash: fnv1a64(rules_bytes),
        }
    }

    /// 把额外材料混入 `rules_hash`（地图尺寸、实体数等）。
    pub fn mix_bytes(mut self, extra: &[u8]) -> Self {
        self.rules_hash = fnv1a64_continue(self.rules_hash, extra);
        self
    }
}

/// FNV-1a 64 位。
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
    Hello {
        protocol: u16,
        fingerprint: MatchFingerprint,
    },
    Command(InputCommand),
    Digest(StateDigest),
    ResyncRequest { tick: u64 },
    ResyncSnapshot { tick: u64, bytes: Vec<u8> },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_nonzero() {
        assert!(PROTOCOL_VERSION >= 1);
        assert!(MAX_PAYLOAD_BYTES >= 1024);
    }

    #[test]
    fn fingerprint_stable_for_same_bytes() {
        let a = MatchFingerprint::build("ra2", "mp01t4.map", b"[General]\n");
        let b = MatchFingerprint::build("ra2", "mp01t4.map", b"[General]\n");
        assert_eq!(a, b);
        let c = MatchFingerprint::build("ra2", "mp01t4.map", b"[General]\nX=1\n");
        assert_ne!(a.rules_hash, c.rules_hash);
    }
}

