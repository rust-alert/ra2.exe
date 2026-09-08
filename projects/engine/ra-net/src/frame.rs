//! 长度前缀成帧与消息编解码。

use crate::{InputCommand, MAX_PAYLOAD_BYTES, MatchFingerprint, PROTOCOL_VERSION, SessionMessage, StateDigest, fnv1a64};
use ra_types::PlayerId;

/// 成帧 / 编解码错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetCodecError {
    /// 缓冲区不足以完成读取。
    Truncated,
    /// 载荷长度超过 [`MAX_PAYLOAD_BYTES`]。
    PayloadTooLarge {
        /// 实际长度（字节）。
        len: usize,
    },
    /// 消息体首字节为未知标签。
    UnknownTag(
        /// 未知标签值。
        u8,
    ),
    /// 内嵌字符串非合法 UTF-8。
    BadUtf8,
    /// Hello 中的协议版本与 [`PROTOCOL_VERSION`] 不一致。
    ProtocolMismatch {
        /// 对端声明的版本。
        got: u16,
    },
}

impl std::fmt::Display for NetCodecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetCodecError::Truncated => write!(f, "消息截断"),
            NetCodecError::PayloadTooLarge { len } => {
                write!(f, "载荷过大: {len} > {MAX_PAYLOAD_BYTES}")
            }
            NetCodecError::UnknownTag(t) => write!(f, "未知消息标签: {t}"),
            NetCodecError::BadUtf8 => write!(f, "字符串非 UTF-8"),
            NetCodecError::ProtocolMismatch { got } => {
                write!(f, "协议版本不匹配: got={got} expect={PROTOCOL_VERSION}")
            }
        }
    }
}

const TAG_HELLO: u8 = 1;
const TAG_COMMAND: u8 = 2;
const TAG_DIGEST: u8 = 3;
const TAG_RESYNC_REQ: u8 = 4;
const TAG_RESYNC_SNAP: u8 = 5;

/// 将 [`SessionMessage`] 编码为 `[u32 BE 长度][body]` 帧。
pub fn encode_frame(msg: &SessionMessage) -> Result<Vec<u8>, NetCodecError> {
    let body = encode_body(msg)?;
    if body.len() > MAX_PAYLOAD_BYTES {
        return Err(NetCodecError::PayloadTooLarge { len: body.len() });
    }
    let mut out = Vec::with_capacity(4 + body.len());
    out.extend_from_slice(&(body.len() as u32).to_be_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// 从缓冲区解码一帧；返回消息与消耗字节数（含 4 字节长度头）。
pub fn decode_frame(buf: &[u8]) -> Result<(SessionMessage, usize), NetCodecError> {
    if buf.len() < 4 {
        return Err(NetCodecError::Truncated);
    }
    let len = u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
    if len > MAX_PAYLOAD_BYTES {
        return Err(NetCodecError::PayloadTooLarge { len });
    }
    if buf.len() < 4 + len {
        return Err(NetCodecError::Truncated);
    }
    let msg = decode_body(&buf[4..4 + len])?;
    Ok((msg, 4 + len))
}

fn encode_body(msg: &SessionMessage) -> Result<Vec<u8>, NetCodecError> {
    let mut b = Vec::new();
    match msg {
        SessionMessage::Hello { protocol, fingerprint } => {
            b.push(TAG_HELLO);
            b.extend_from_slice(&protocol.to_be_bytes());
            write_str(&mut b, &fingerprint.edition)?;
            write_str(&mut b, &fingerprint.map)?;
            b.extend_from_slice(&fingerprint.rules_hash.to_be_bytes());
        }
        SessionMessage::Command(cmd) => {
            b.push(TAG_COMMAND);
            b.push(cmd.player.0);
            b.extend_from_slice(&cmd.sequence.to_be_bytes());
            b.extend_from_slice(&cmd.tick.to_be_bytes());
            if cmd.payload.len() > MAX_PAYLOAD_BYTES {
                return Err(NetCodecError::PayloadTooLarge { len: cmd.payload.len() });
            }
            b.extend_from_slice(&(cmd.payload.len() as u32).to_be_bytes());
            b.extend_from_slice(&cmd.payload);
        }
        SessionMessage::Digest(d) => {
            b.push(TAG_DIGEST);
            b.extend_from_slice(&d.tick.to_be_bytes());
            b.extend_from_slice(&d.hash.to_be_bytes());
        }
        SessionMessage::ResyncRequest { tick } => {
            b.push(TAG_RESYNC_REQ);
            b.extend_from_slice(&tick.to_be_bytes());
        }
        SessionMessage::ResyncSnapshot { tick, bytes } => {
            b.push(TAG_RESYNC_SNAP);
            b.extend_from_slice(&tick.to_be_bytes());
            if bytes.len() > MAX_PAYLOAD_BYTES {
                return Err(NetCodecError::PayloadTooLarge { len: bytes.len() });
            }
            b.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
            b.extend_from_slice(bytes);
        }
    }
    Ok(b)
}

fn decode_body(body: &[u8]) -> Result<SessionMessage, NetCodecError> {
    if body.is_empty() {
        return Err(NetCodecError::Truncated);
    }
    let tag = body[0];
    let mut i = 1usize;
    match tag {
        TAG_HELLO => {
            let protocol = read_u16(body, &mut i)?;
            if protocol != PROTOCOL_VERSION {
                return Err(NetCodecError::ProtocolMismatch { got: protocol });
            }
            let edition = read_str(body, &mut i)?;
            let map = read_str(body, &mut i)?;
            let rules_hash = read_u64(body, &mut i)?;
            Ok(SessionMessage::Hello { protocol, fingerprint: MatchFingerprint { edition, map, rules_hash } })
        }
        TAG_COMMAND => {
            if i >= body.len() {
                return Err(NetCodecError::Truncated);
            }
            let player = PlayerId(body[i]);
            i += 1;
            let sequence = read_u32(body, &mut i)?;
            let tick = read_u64(body, &mut i)?;
            let plen = read_u32(body, &mut i)? as usize;
            if plen > MAX_PAYLOAD_BYTES {
                return Err(NetCodecError::PayloadTooLarge { len: plen });
            }
            let payload = read_bytes(body, &mut i, plen)?;
            Ok(SessionMessage::Command(InputCommand { player, sequence, tick, payload }))
        }
        TAG_DIGEST => {
            let tick = read_u64(body, &mut i)?;
            let hash = read_u64(body, &mut i)?;
            Ok(SessionMessage::Digest(StateDigest { tick, hash }))
        }
        TAG_RESYNC_REQ => {
            let tick = read_u64(body, &mut i)?;
            Ok(SessionMessage::ResyncRequest { tick })
        }
        TAG_RESYNC_SNAP => {
            let tick = read_u64(body, &mut i)?;
            let plen = read_u32(body, &mut i)? as usize;
            if plen > MAX_PAYLOAD_BYTES {
                return Err(NetCodecError::PayloadTooLarge { len: plen });
            }
            let bytes = read_bytes(body, &mut i, plen)?;
            Ok(SessionMessage::ResyncSnapshot { tick, bytes })
        }
        other => Err(NetCodecError::UnknownTag(other)),
    }
}

fn write_str(buf: &mut Vec<u8>, s: &str) -> Result<(), NetCodecError> {
    let bytes = s.as_bytes();
    if bytes.len() > u16::MAX as usize {
        return Err(NetCodecError::PayloadTooLarge { len: bytes.len() });
    }
    buf.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    buf.extend_from_slice(bytes);
    Ok(())
}

fn read_str(buf: &[u8], i: &mut usize) -> Result<String, NetCodecError> {
    let len = read_u16(buf, i)? as usize;
    let bytes = read_bytes(buf, i, len)?;
    String::from_utf8(bytes).map_err(|_| NetCodecError::BadUtf8)
}

fn read_bytes(buf: &[u8], i: &mut usize, n: usize) -> Result<Vec<u8>, NetCodecError> {
    if *i + n > buf.len() {
        return Err(NetCodecError::Truncated);
    }
    let out = buf[*i..*i + n].to_vec();
    *i += n;
    Ok(out)
}

fn read_u16(buf: &[u8], i: &mut usize) -> Result<u16, NetCodecError> {
    if *i + 2 > buf.len() {
        return Err(NetCodecError::Truncated);
    }
    let v = u16::from_be_bytes([buf[*i], buf[*i + 1]]);
    *i += 2;
    Ok(v)
}

fn read_u32(buf: &[u8], i: &mut usize) -> Result<u32, NetCodecError> {
    if *i + 4 > buf.len() {
        return Err(NetCodecError::Truncated);
    }
    let v = u32::from_be_bytes([buf[*i], buf[*i + 1], buf[*i + 2], buf[*i + 3]]);
    *i += 4;
    Ok(v)
}

fn read_u64(buf: &[u8], i: &mut usize) -> Result<u64, NetCodecError> {
    if *i + 8 > buf.len() {
        return Err(NetCodecError::Truncated);
    }
    let mut a = [0u8; 8];
    a.copy_from_slice(&buf[*i..*i + 8]);
    *i += 8;
    Ok(u64::from_be_bytes(a))
}

/// 快速校验：指纹 `rules_hash` 是否等于给定规则字节的 FNV-1a 哈希。
pub fn fingerprint_matches_rules(fp: &MatchFingerprint, rules_bytes: &[u8]) -> bool {
    fp.rules_hash == fnv1a64(rules_bytes)
}
