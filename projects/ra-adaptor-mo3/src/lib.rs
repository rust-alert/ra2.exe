//! MO3 音乐载荷适配。
//!
//! 零售主题曲常见为 MIX 内嵌音频；部分发行或模组会使用 MO3。
//! 本 crate 先暴露探测与占位解码接口，不阻塞引擎启动。

use ra_types::{RaError, RaResult};

/// 判断字节是否像 MO3 容器（`MO3` 魔数）。
pub fn looks_like_mo3(data: &[u8]) -> bool {
    data.len() >= 3 && data[0] == b'M' && data[1] == b'O' && data[2] == b'3'
}

/// 解码结果占位：后续可换成 PCM 样本或交给音频后端。
#[derive(Debug, Clone)]
pub struct Mo3Track {
    pub byte_len: usize,
}

/// 占位解码：校验魔数后返回元数据，暂不展开样本。
pub fn probe(data: &[u8]) -> RaResult<Mo3Track> {
    if !looks_like_mo3(data) {
        return Err(RaError::Parse("不是 MO3 载荷".into()));
    }
    Ok(Mo3Track {
        byte_len: data.len(),
    })
}
