//! Bink 视频码流解码与平面 → RGBA 转换。
//!
//! 容器抽包见 [`super::bink`]；本模块产出可上传 GPU / `set_ui_page` 的像素。

use super::bink::{BinkHeader, BinkVersion};
use super::bink_bits::{BitReader, VlcTable, build_fixed_vlc_tables};
use super::bink_bundle::{
    BinkBundle, BinkSrc, NB_SRC, alloc_bundles, init_bundle_lengths, read_bundle,
};
use super::bink_huff::HuffmanTree;

/// 视频解码错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinkVideoError {
    /// 尚不支持的容器修订。
    UnsupportedVersion(BinkVersion),
    /// 非法尺寸。
    BadSize {
        /// 宽。
        width: u32,
        /// 高。
        height: u32,
    },
    /// 码流解码尚未接完（骨架占位）。
    NotImplemented(&'static str),
    /// 码流或状态错误说明。
    Msg(String),
}

impl std::fmt::Display for BinkVideoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedVersion(v) => write!(f, "不支持的 Bink 修订：{v:?}"),
            Self::BadSize { width, height } => write!(f, "非法画面尺寸 {width}×{height}"),
            Self::NotImplemented(what) => write!(f, "未实现：{what}"),
            Self::Msg(s) => write!(f, "{s}"),
        }
    }
}

impl std::error::Error for BinkVideoError {}

/// 一帧 YUV420 平面（可带 alpha）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinkYuvFrame {
    /// 宽。
    pub width: u32,
    /// 高。
    pub height: u32,
    /// Y 平面（`width * height`）。
    pub y: Vec<u8>,
    /// U 平面（约半宽半高）。
    pub u: Vec<u8>,
    /// V 平面。
    pub v: Vec<u8>,
    /// 可选 A 平面（与 Y 同尺寸）。
    pub a: Option<Vec<u8>>,
}

impl BinkYuvFrame {
    /// 分配空平面（Y=0，UV=128，A=255）。
    pub fn blank(width: u32, height: u32, with_alpha: bool) -> Result<Self, BinkVideoError> {
        if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
            return Err(BinkVideoError::BadSize { width, height });
        }
        let y_len = (width as usize).saturating_mul(height as usize);
        let uv_w = (width / 2) as usize;
        let uv_h = (height / 2) as usize;
        let uv_len = uv_w.saturating_mul(uv_h);
        Ok(Self {
            width,
            height,
            y: vec![0u8; y_len],
            u: vec![128u8; uv_len],
            v: vec![128u8; uv_len],
            a: with_alpha.then(|| vec![255u8; y_len]),
        })
    }

    /// 转为紧密 RGBA8（BT.601，色度最近邻上采样）。
    pub fn to_rgba8(&self) -> Vec<u8> {
        yuv420_planes_to_rgba8(
            self.width,
            self.height,
            &self.y,
            &self.u,
            &self.v,
            self.a.as_deref(),
        )
    }
}

/// YUV420 平面 → RGBA8。
pub fn yuv420_planes_to_rgba8(
    width: u32,
    height: u32,
    y: &[u8],
    u: &[u8],
    v: &[u8],
    a: Option<&[u8]>,
) -> Vec<u8> {
    let w = width as usize;
    let h = height as usize;
    let uv_w = w / 2;
    let mut out = vec![0u8; w.saturating_mul(h).saturating_mul(4)];
    for row in 0..h {
        let y_row = row * w;
        let uv_row = (row / 2) * uv_w;
        for col in 0..w {
            let yi = y.get(y_row + col).copied().unwrap_or(0) as f32;
            let ui = u.get(uv_row + col / 2).copied().unwrap_or(128) as f32 - 128.0;
            let vi = v.get(uv_row + col / 2).copied().unwrap_or(128) as f32 - 128.0;
            let r = (yi + 1.402 * vi).clamp(0.0, 255.0) as u8;
            let g = (yi - 0.344_136 * ui - 0.714_136 * vi).clamp(0.0, 255.0) as u8;
            let b = (yi + 1.772 * ui).clamp(0.0, 255.0) as u8;
            let alpha = a
                .and_then(|plane| plane.get(y_row + col).copied())
                .unwrap_or(255);
            let o = (y_row + col) * 4;
            out[o] = r;
            out[o + 1] = g;
            out[o + 2] = b;
            out[o + 3] = alpha;
        }
    }
    out
}

/// 自有 Bink 视频解码器（双缓冲 + 固定 VLC；平面块逐步填入）。
#[derive(Debug)]
pub struct BinkVideoDecoder {
    width: u32,
    height: u32,
    has_alpha: bool,
    version: BinkVersion,
    /// 固定 Huffman VLC 表。
    vlc: [VlcTable; 16],
    bundles: [BinkBundle; NB_SRC],
    bundle_data: Vec<u8>,
    col_high: [HuffmanTree; 16],
    col_lastval: u8,
    cur: BinkYuvFrame,
    prev: BinkYuvFrame,
    has_prev: bool,
}

impl BinkVideoDecoder {
    /// 按容器头构造。
    pub fn new(header: &BinkHeader) -> Result<Self, BinkVideoError> {
        match header.version {
            BinkVersion::BikI | BinkVersion::BikK => {}
        }
        if header.is_gray() {
            return Err(BinkVideoError::Msg("不支持灰度 Bink".into()));
        }
        let cur = BinkYuvFrame::blank(header.width, header.height, header.has_alpha())?;
        let prev = BinkYuvFrame::blank(header.width, header.height, header.has_alpha())?;
        let (bundles, bundle_data) = alloc_bundles(header.width, header.height);
        Ok(Self {
            width: header.width,
            height: header.height,
            has_alpha: header.has_alpha(),
            version: header.version,
            vlc: build_fixed_vlc_tables()?,
            bundles,
            bundle_data,
            col_high: std::array::from_fn(|_| HuffmanTree::default()),
            col_lastval: 0,
            cur,
            prev,
            has_prev: false,
        })
    }

    /// 画面宽。
    pub fn width(&self) -> u32 {
        self.width
    }

    /// 画面高。
    pub fn height(&self) -> u32 {
        self.height
    }

    /// 是否有 alpha 平面。
    pub fn has_alpha(&self) -> bool {
        self.has_alpha
    }

    /// 容器修订。
    pub fn version(&self) -> BinkVersion {
        self.version
    }

    /// 固定 VLC 表（供 bundle 解码使用）。
    pub fn vlc_tables(&self) -> &[VlcTable; 16] {
        &self.vlc
    }

    /// 是否已有上一帧（运动补偿参考）。
    pub fn has_prev(&self) -> bool {
        self.has_prev
    }

    /// 解码一帧视频码流（`BinkFramePacket::video`）。
    pub fn decode_packet(
        &mut self,
        video: &[u8],
        _is_keyframe: bool,
    ) -> Result<&BinkYuvFrame, BinkVideoError> {
        if self.has_alpha {
            return Err(BinkVideoError::Msg("暂不支持带 alpha 的 Bink".into()));
        }
        let mut r = BitReader::from_bytes(video);
        if r.bits_left() < 32 {
            return Err(BinkVideoError::Msg(format!(
                "视频包过短：{} 位",
                r.bits_left()
            )));
        }
        // BIKi/BIKk：视频包开头跳过 32 位对齐槽。
        r.skip(32);

        std::mem::swap(&mut self.cur, &mut self.prev);

        for plane in 0..3usize {
            self.decode_plane_preamble(&mut r, plane)?;
        }

        self.has_prev = true;
        // 块类型循环（SKIP/FILL/DCT…）下一步提交。
        let _ = (&self.bundle_data, &self.vlc, BinkSrc::BlockTypes);
        Err(BinkVideoError::NotImplemented("平面块类型循环"))
    }

    /// 读满一平面的 9 路树（BIKk 整平面填充捷径单独处理）。
    fn decode_plane_preamble(
        &mut self,
        r: &mut BitReader<'_>,
        plane_idx: usize,
    ) -> Result<(), BinkVideoError> {
        let is_chroma = plane_idx != 0;
        let shift = if is_chroma { 1u32 } else { 0 };
        let width = self.width >> shift;
        let bw = if is_chroma {
            (self.width + 15) >> 4
        } else {
            (self.width + 7) >> 3
        };

        if self.version == BinkVersion::BikK && r.read_bit()? {
            let fill = r.read_bits(8)? as u8;
            match plane_idx {
                0 => self.cur.y.fill(fill),
                1 => self.cur.u.fill(fill),
                _ => self.cur.v.fill(fill),
            }
            r.align_to_dword();
            return Ok(());
        }

        init_bundle_lengths(&mut self.bundles, width.max(8), bw);
        for i in 0..NB_SRC {
            read_bundle(
                r,
                &mut self.bundles,
                &mut self.col_high,
                &mut self.col_lastval,
                i,
            )?;
        }
        // 块循环结束后再 `align_to_dword`；树读完后码流紧接块类型。
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::bink::{parse_bink_header, BinkVersion};

    fn tiny_header() -> crate::image::bink::BinkHeader {
        let mut data = vec![0u8; 0x2C];
        data[0..4].copy_from_slice(&0x694B_4942u32.to_le_bytes());
        data[4..8].copy_from_slice(&(100u32 - 8).to_le_bytes());
        data[8..12].copy_from_slice(&1u32.to_le_bytes());
        data[12..16].copy_from_slice(&10u32.to_le_bytes());
        data[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
        data[0x18..0x1C].copy_from_slice(&8u32.to_le_bytes());
        data[0x1C..0x20].copy_from_slice(&10u32.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
        parse_bink_header(&data).unwrap()
    }

    #[test]
    fn decoder_new_builds_vlc_and_rejects_short_packet() {
        let mut d = BinkVideoDecoder::new(&tiny_header()).unwrap();
        assert_eq!((d.width(), d.height()), (8, 8));
        assert_eq!(d.version(), BinkVersion::BikI);
        assert_eq!(d.vlc_tables().len(), 16);
        assert!(matches!(
            d.decode_packet(&[], true).unwrap_err(),
            BinkVideoError::Msg(_)
        ));
        // 4 字节 = 32 位，跳过对齐槽后读树时码流耗尽。
        assert!(matches!(
            d.decode_packet(&[0, 0, 0, 0], true).unwrap_err(),
            BinkVideoError::Msg(_)
        ));
    }

    #[test]
    fn blank_yuv_to_rgba_is_opaque_black() {
        let frame = BinkYuvFrame::blank(4, 4, false).unwrap();
        let rgba = frame.to_rgba8();
        assert_eq!(rgba.len(), 4 * 4 * 4);
        assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
    }
}
