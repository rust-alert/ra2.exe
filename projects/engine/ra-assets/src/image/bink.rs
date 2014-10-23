//! Bink 1 容器：固定头与帧索引解析（BIKi / BIKk）。
//!
//! 本模块只解析尺寸、帧率、帧数、音轨描述与帧包偏移；视频码流解码另模块处理。

/// 支持的 Bink 修订（零售过场常见）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinkVersion {
    /// `BIKi`。
    BikI,
    /// `BIKk`（头后多 4 字节）。
    BikK,
}

/// YUV → RGB 时的亮度色域。
///
/// `BIKi` 等常见为 studio/MPEG（黑≈Y16）；`BIKk` 为 full/JPEG（黑=Y0）。
/// 误用 full 解 MPEG 会使暗部抬成深灰（主菜单 CRT 内发灰）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinkColorRange {
    /// Studio：Y∈[16,235] → 黑白。
    Mpeg,
    /// Full：Y∈[0,255] → 黑白。
    Jpeg,
}

impl BinkVersion {
    fn from_tag(tag: u32) -> Result<Self, String> {
        match tag {
            0x694B_4942 => Ok(Self::BikI), // "BIKi" LE
            0x6B4B_4942 => Ok(Self::BikK), // "BIKk" LE
            other => Err(format!("不支持的 Bink 签名 0x{other:08X}（非 BIKi/BIKk）")),
        }
    }

    /// 该修订默认的 YUV 色域。
    pub const fn color_range(self) -> BinkColorRange {
        match self {
            Self::BikK => BinkColorRange::Jpeg,
            Self::BikI => BinkColorRange::Mpeg,
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
        if self.fps_den == 0 { 0.0 } else { f64::from(self.fps_num) / f64::from(self.fps_den) }
    }
}

fn read_u32_le(data: &[u8], off: usize) -> Result<u32, String> {
    let end = off.checked_add(4).ok_or_else(|| "偏移溢出".to_string())?;
    let slice = data.get(off..end).ok_or_else(|| format!("Bink 头截断：需读 offset {off}"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

/// 解析 Bink 固定头（至少 `0x2C` 字节；BIKk 再多 4）。
pub fn parse_bink_header(data: &[u8]) -> Result<BinkHeader, String> {
    if data.len() < 0x2C {
        return Err(format!("Bink 头截断：{} 字节，至少需要 0x2C", data.len()));
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

/// 一条音轨描述（头内）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinkAudioTrack {
    /// 采样率。
    pub sample_rate: u16,
    /// 标志位。
    pub flags: u16,
    /// 轨 id。
    pub track_id: u32,
}

/// 帧索引表一项。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinkFrameIndexEntry {
    /// 帧包起始偏移（已清除 keyframe 低位）。
    pub offset: u32,
    /// 帧包字节数。
    pub size: u32,
    /// 是否关键帧。
    pub is_keyframe: bool,
}

/// 已解析容器（头 + 音轨 + 帧索引；不含视频解码）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinkFile {
    /// 固定头。
    pub header: BinkHeader,
    /// 音轨。
    pub audio_tracks: Vec<BinkAudioTrack>,
    /// 每帧包偏移/长度。
    pub frames: Vec<BinkFrameIndexEntry>,
    /// 帧索引表在文件中的起始偏移。
    pub frame_index_offset: usize,
}

/// 一帧从容器中切出的音/视频载荷（借用源字节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BinkFramePacket<'a> {
    /// 帧下标。
    pub index: usize,
    /// 是否关键帧。
    pub is_keyframe: bool,
    /// 音频载荷（已去掉长度前缀）；无音轨时为 `None`。
    pub audio: Option<&'a [u8]>,
    /// 视频码流字节。
    pub video: &'a [u8],
}

impl BinkFile {
    /// 单帧显示时长（微秒）。
    pub fn frame_duration_us(&self) -> u32 {
        let num = self.header.fps_num.max(1);
        let den = self.header.fps_den.max(1);
        ((u64::from(den) * 1_000_000) / u64::from(num)) as u32
    }

    /// 按索引切出一帧包（音轨前缀拆开，视频码流单独给出）。
    pub fn frame_packet<'a>(&self, data: &'a [u8], index: usize) -> Result<BinkFramePacket<'a>, String> {
        let entry = self.frames.get(index).ok_or_else(|| format!("帧下标越界：{index} / {}", self.frames.len()))?;
        let start = entry.offset as usize;
        let end = start.checked_add(entry.size as usize).ok_or_else(|| format!("帧 #{index} 长度溢出"))?;
        let packet = data.get(start..end).ok_or_else(|| format!("帧 #{index} 数据越界：[{start},{end}) / {}", data.len()))?;

        if self.audio_tracks.is_empty() {
            return Ok(BinkFramePacket { index, is_keyframe: entry.is_keyframe, audio: None, video: packet });
        }
        if packet.len() < 4 {
            return Err(format!("帧 #{index} 音轨前缀截断"));
        }
        let aud_len = u32::from_le_bytes([packet[0], packet[1], packet[2], packet[3]]) as usize;
        let audio_end = 4usize.checked_add(aud_len).ok_or_else(|| format!("帧 #{index} 音频长度溢出"))?;
        if audio_end > packet.len() {
            return Err(format!("帧 #{index} 音频越界：声明 {aud_len} · 包长 {}", packet.len()));
        }
        Ok(BinkFramePacket { index, is_keyframe: entry.is_keyframe, audio: Some(&packet[4..audio_end]), video: &packet[audio_end..] })
    }
}

fn read_u16_le(data: &[u8], off: usize) -> Result<u16, String> {
    let end = off.checked_add(2).ok_or_else(|| "偏移溢出".to_string())?;
    let slice = data.get(off..end).ok_or_else(|| format!("Bink 截断：需读 offset {off}"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn parse_audio_tracks(data: &[u8], header: &BinkHeader, start: usize) -> Result<(Vec<BinkAudioTrack>, usize), String> {
    if header.num_audio_tracks == 0 {
        return Ok((Vec::new(), start));
    }
    let n = header.num_audio_tracks as usize;
    let needed = 12 * n;
    if data.len() < start + needed {
        return Err("音轨描述截断".into());
    }
    // 跳过 max_decoded_bytes[n]
    let mut off = start + 4 * n;
    let mut partial = Vec::with_capacity(n);
    for _ in 0..n {
        let sample_rate = read_u16_le(data, off)?;
        let flags = read_u16_le(data, off + 2)?;
        partial.push((sample_rate, flags));
        off += 4;
    }
    let mut tracks = Vec::with_capacity(n);
    for &(sample_rate, flags) in &partial {
        let track_id = read_u32_le(data, off)?;
        off += 4;
        tracks.push(BinkAudioTrack { sample_rate, flags, track_id });
    }
    Ok((tracks, off))
}

fn parse_frame_index(data: &[u8], header: &BinkHeader, start: usize) -> Result<Vec<BinkFrameIndexEntry>, String> {
    let n = header.num_frames as usize;
    let needed = 4 * n;
    if data.len() < start + needed {
        return Err("帧索引表截断".into());
    }
    let mut raw = Vec::with_capacity(n + 1);
    for i in 0..n {
        raw.push(read_u32_le(data, start + 4 * i)?);
    }
    raw.push(header.file_size as u32);

    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let cur = raw[i];
        let next = raw[i + 1] & !1;
        let offset = cur & !1;
        let is_keyframe = i == 0 || (raw[i] & 1) != 0;
        if next <= offset {
            return Err(format!("无效帧索引 #{i}：next <= current"));
        }
        entries.push(BinkFrameIndexEntry { offset, size: next - offset, is_keyframe });
    }
    Ok(entries)
}

/// 解析 Bink 容器：固定头 + 音轨描述 + 帧索引（不解码视频）。
pub fn parse_bink_file(data: &[u8]) -> Result<BinkFile, String> {
    let header = parse_bink_header(data)?;
    let (audio_tracks, frame_index_offset) = parse_audio_tracks(data, &header, header.audio_section_offset)?;
    let frames = parse_frame_index(data, &header, frame_index_offset)?;
    Ok(BinkFile { header, audio_tracks, frames, frame_index_offset })
}
