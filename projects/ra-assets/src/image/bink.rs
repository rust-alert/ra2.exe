//! Bink 1 容器：固定头解析（BIKi / BIKk）。
//!
//! 布局对照公开的 FFmpeg `libavformat/bink.c` 解复用说明（LGPL-2.1-or-later）。
//! 本模块**只**读尺寸 / 帧率 / 帧数等元数据；帧索引与视频解码后续另加。

/// 支持的 Bink 修订（零售过场常见）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinkVersion {
    /// `BIKi`。
    BikI,
    /// `BIKk`（头后多 4 字节）。
    BikK,
}

impl BinkVersion {
    fn from_tag(tag: u32) -> Result<Self, String> {
        match tag {
            0x694B_4942 => Ok(Self::BikI), // "BIKi" LE
            0x6B4B_4942 => Ok(Self::BikK), // "BIKk" LE
            other => Err(format!("不支持的 Bink 签名 0x{other:08X}（非 BIKi/BIKk）")),
        }
    }
}

/// 视频标志位（头字段 `video_flags`）。
pub const BINK_FLAG_ALPHA: u32 = 0x0010_0000;
/// 灰度标志。
pub const BINK_FLAG_GRAY: u32 = 0x0002_0000;

const MAX_WIDTH: u32 = 7680;
const MAX_HEIGHT: u32 = 4800;
const MAX_FRAMES: u32 = 1_000_000;
const MAX_AUDIO_TRACKS: u32 = 256;

/// Bink 固定头（不含完整音轨描述与帧索引表）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinkHeader {
    /// 修订。
    pub version: BinkVersion,
    /// 真实文件字节数（盘上字段为 `size - 8`）。
    pub file_size: u64,
    /// 帧数。
    pub num_frames: u32,
    /// 最大单帧负载（字节）。
    pub largest_frame: u32,
    /// 宽。
    pub width: u32,
    /// 高。
    pub height: u32,
    /// 帧率分子。
    pub fps_num: u32,
    /// 帧率分母。
    pub fps_den: u32,
    /// 视频标志。
    pub video_flags: u32,
    /// 音轨数。
    pub num_audio_tracks: u32,
    /// 音轨描述起始偏移（若无音轨则为帧索引起点）。
    pub audio_section_offset: usize,
}

impl BinkHeader {
    /// 是否声明 alpha 平面。
    pub fn has_alpha(&self) -> bool {
        self.video_flags & BINK_FLAG_ALPHA != 0
    }

    /// 是否灰度。
    pub fn is_gray(&self) -> bool {
        self.video_flags & BINK_FLAG_GRAY != 0
    }

    /// 帧率（fps）；分母为 0 时返回 0。
    pub fn fps(&self) -> f64 {
        if self.fps_den == 0 {
            0.0
        } else {
            f64::from(self.fps_num) / f64::from(self.fps_den)
        }
    }
}

fn read_u32_le(data: &[u8], off: usize) -> Result<u32, String> {
    let end = off.checked_add(4).ok_or_else(|| "偏移溢出".to_string())?;
    let slice = data
        .get(off..end)
        .ok_or_else(|| format!("Bink 头截断：需读 offset {off}"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// 解析 Bink 固定头（至少 `0x2C` 字节；BIKk 再多 4）。
pub fn parse_bink_header(data: &[u8]) -> Result<BinkHeader, String> {
    if data.len() < 0x2C {
        return Err(format!(
            "Bink 头截断：{} 字节，至少需要 0x2C",
            data.len()
        ));
    }

    let version = BinkVersion::from_tag(read_u32_le(data, 0x00)?)?;
    let file_size = u64::from(read_u32_le(data, 0x04)?) + 8;
    let num_frames = read_u32_le(data, 0x08)?;
    let largest_frame = read_u32_le(data, 0x0C)?;
    let width = read_u32_le(data, 0x14)?;
    let height = read_u32_le(data, 0x18)?;
    let fps_num = read_u32_le(data, 0x1C)?;
    let fps_den = read_u32_le(data, 0x20)?;
    let video_flags = read_u32_le(data, 0x24)?;
    let num_audio_tracks = read_u32_le(data, 0x28)?;

    if num_frames == 0 || num_frames > MAX_FRAMES {
        return Err(format!("无效帧数 {num_frames}"));
    }
    if width == 0 || width > MAX_WIDTH {
        return Err(format!("无效宽度 {width}"));
    }
    if height == 0 || height > MAX_HEIGHT {
        return Err(format!("无效高度 {height}"));
    }
    if fps_num == 0 || fps_den == 0 {
        return Err(format!("无效帧率 {fps_num}/{fps_den}"));
    }
    if num_audio_tracks > MAX_AUDIO_TRACKS {
        return Err(format!("音轨过多：{num_audio_tracks}"));
    }
    if u64::from(largest_frame) > file_size {
        return Err("largest_frame 大于 file_size".into());
    }

    let mut audio_section_offset = 0x2C;
    if version == BinkVersion::BikK {
        if data.len() < audio_section_offset + 4 {
            return Err("BIKk 头截断：缺少额外 4 字节字段".into());
        }
        audio_section_offset += 4;
    }

    Ok(BinkHeader {
        version,
        file_size,
        num_frames,
        largest_frame,
        width,
        height,
        fps_num,
        fps_den,
        video_flags,
        num_audio_tracks,
        audio_section_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_biki_header() -> Vec<u8> {
        let mut data = vec![0u8; 0x2C];
        data[0..4].copy_from_slice(&0x694B_4942u32.to_le_bytes()); // BIKi
        data[4..8].copy_from_slice(&(1000u32 - 8).to_le_bytes());
        data[8..12].copy_from_slice(&30u32.to_le_bytes());
        data[12..16].copy_from_slice(&200u32.to_le_bytes());
        data[0x14..0x18].copy_from_slice(&640u32.to_le_bytes());
        data[0x18..0x1C].copy_from_slice(&480u32.to_le_bytes());
        data[0x1C..0x20].copy_from_slice(&15u32.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
        data[0x24..0x28].copy_from_slice(&0u32.to_le_bytes());
        data[0x28..0x2C].copy_from_slice(&0u32.to_le_bytes());
        data
    }

    #[test]
    fn parses_synthetic_biki_header() {
        let h = parse_bink_header(&synth_biki_header()).unwrap();
        assert_eq!(h.version, BinkVersion::BikI);
        assert_eq!(h.file_size, 1000);
        assert_eq!(h.num_frames, 30);
        assert_eq!((h.width, h.height), (640, 480));
        assert!((h.fps() - 15.0).abs() < f64::EPSILON);
        assert_eq!(h.audio_section_offset, 0x2C);
    }

    #[test]
    fn rejects_unknown_signature() {
        let mut data = synth_biki_header();
        data[0..4].copy_from_slice(b"BIKb");
        assert!(parse_bink_header(&data).is_err());
    }
}
