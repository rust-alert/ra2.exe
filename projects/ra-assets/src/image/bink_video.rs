//! Bink 视频码流解码与平面 → RGBA 转换。
//!
//! 容器抽包见 [`super::bink`]；本模块产出可上传 GPU / `set_ui_page` 的像素。

use super::{
    bink::{BinkHeader, BinkVersion},
    bink_bits::{BitReader, VlcTable, build_fixed_vlc_tables},
    bink_bundle::{
        BinkBundle, BinkSrc, NB_SRC, alloc_bundles, init_bundle_lengths, read_block_types, read_bundle, read_colors, read_dcs,
        read_motion_values, read_patterns, read_runs, take_value, take_value16,
    },
    bink_dct::{decode_inter_dct_block, decode_intra_dct_block},
    bink_huff::HuffmanTree,
    bink_idct::{idct_add, idct_put},
    bink_patterns::BINK_RUN_PATTERNS,
    bink_residue::{add_pixels8, read_residue},
    bink_tables::DC_START_BITS,
};

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
        yuv420_planes_to_rgba8(self.width, self.height, &self.y, &self.u, &self.v, self.a.as_deref())
    }
}

/// YUV420 平面 → RGBA8。
pub fn yuv420_planes_to_rgba8(width: u32, height: u32, y: &[u8], u: &[u8], v: &[u8], a: Option<&[u8]>) -> Vec<u8> {
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
            let alpha = a.and_then(|plane| plane.get(y_row + col).copied()).unwrap_or(255);
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

    /// 清空双缓冲，回到可解关键帧的状态（循环播放回绕时用）。
    pub fn reset(&mut self) {
        let with_alpha = self.has_alpha;
        if let Ok(blank) = BinkYuvFrame::blank(self.width, self.height, with_alpha) {
            self.cur = blank.clone();
            self.prev = blank;
        }
        self.has_prev = false;
        self.col_lastval = 0;
    }

    /// 解码一帧视频码流（`BinkFramePacket::video`）。
    ///
    /// 已接全部 BIKi/BIKk 常用平面块类型（含 SCALED 16×16）。
    pub fn decode_packet(&mut self, video: &[u8], _is_keyframe: bool) -> Result<&BinkYuvFrame, BinkVideoError> {
        if self.has_alpha {
            return Err(BinkVideoError::Msg("暂不支持带 alpha 的 Bink".into()));
        }
        let mut r = BitReader::from_bytes(video);
        if r.bits_left() < 32 {
            return Err(BinkVideoError::Msg(format!("视频包过短：{} 位", r.bits_left())));
        }
        // BIKi/BIKk：视频包开头跳过 32 位对齐槽。
        r.skip(32);

        std::mem::swap(&mut self.cur, &mut self.prev);

        for plane in 0..3usize {
            self.decode_plane(&mut r, plane)?;
        }

        self.has_prev = true;
        Ok(&self.cur)
    }

    /// 解码一个色度/亮度平面。
    fn decode_plane(&mut self, r: &mut BitReader<'_>, plane_idx: usize) -> Result<(), BinkVideoError> {
        let is_chroma = plane_idx != 0;
        let shift = if is_chroma { 1u32 } else { 0 };
        let width = self.width >> shift;
        let height = self.height >> shift;
        let bw = if is_chroma { (self.width + 15) >> 4 } else { (self.width + 7) >> 3 };
        let bh = if is_chroma { (self.height + 15) >> 4 } else { (self.height + 7) >> 3 };

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
            read_bundle(r, &mut self.bundles, &mut self.col_high, &mut self.col_lastval, i)?;
        }

        self.decode_plane_blocks(r, plane_idx, width, height, bw, bh)?;
        r.align_to_dword();
        Ok(())
    }

    fn refill_row_bundles(&mut self, r: &mut BitReader<'_>) -> Result<(), BinkVideoError> {
        let vlc = &self.vlc;
        let version = self.version;
        read_block_types(r, &mut self.bundles, &mut self.bundle_data, vlc, version, BinkSrc::BlockTypes as usize)?;
        read_block_types(r, &mut self.bundles, &mut self.bundle_data, vlc, version, BinkSrc::SubBlockTypes as usize)?;
        read_colors(
            r,
            &mut self.bundles,
            &mut self.bundle_data,
            vlc,
            &self.col_high,
            &mut self.col_lastval,
            BinkSrc::Colors as usize,
        )?;
        read_patterns(r, &mut self.bundles, &mut self.bundle_data, vlc, BinkSrc::Pattern as usize)?;
        read_motion_values(r, &mut self.bundles, &mut self.bundle_data, vlc, BinkSrc::XOff as usize)?;
        read_motion_values(r, &mut self.bundles, &mut self.bundle_data, vlc, BinkSrc::YOff as usize)?;
        read_dcs(r, &mut self.bundles, &mut self.bundle_data, BinkSrc::IntraDc as usize, DC_START_BITS, true)?;
        read_dcs(r, &mut self.bundles, &mut self.bundle_data, BinkSrc::InterDc as usize, DC_START_BITS, true)?;
        read_runs(r, &mut self.bundles, &mut self.bundle_data, vlc, BinkSrc::Run as usize)?;
        Ok(())
    }

    fn decode_plane_blocks(
        &mut self,
        r: &mut BitReader<'_>,
        plane_idx: usize,
        width: u32,
        height: u32,
        bw: u32,
        bh: u32,
    ) -> Result<(), BinkVideoError> {
        for by in 0..bh {
            self.refill_row_bundles(r)?;
            let mut bx = 0u32;
            while bx < bw {
                let blk = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::BlockTypes as usize);
                // 奇数行/列上的 SCALED 标记属于已解的 16×16，跳过并多推进一格。
                if blk == 1 && ((by & 1) != 0 || (bx & 1) != 0) {
                    bx += 2;
                    continue;
                }
                match blk {
                    0 => {
                        self.copy_block(plane_idx, bx, by, width, height, 0, 0);
                    }
                    1 => {
                        self.scaled_block(r, plane_idx, bx, by, width, height)?;
                        bx += 1; // 再加下面公共 +1 → 共跳过右侧 8×8
                    }
                    2 => {
                        let ox = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::XOff as usize) as i8 as i32;
                        let oy = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::YOff as usize) as i8 as i32;
                        self.copy_block(plane_idx, bx, by, width, height, ox, oy);
                    }
                    3 => {
                        self.run_block(r, plane_idx, bx, by, width, height)?;
                    }
                    4 => {
                        let ox = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::XOff as usize) as i8 as i32;
                        let oy = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::YOff as usize) as i8 as i32;
                        self.copy_block(plane_idx, bx, by, width, height, ox, oy);
                        let masks = r.read_bits(7)? as i32;
                        let mut block = [0i16; 64];
                        read_residue(r, &mut block, masks)?;
                        self.add_residue_block(plane_idx, bx, by, width, height, &block);
                    }
                    5 => {
                        let dc = take_value16(&mut self.bundles, &self.bundle_data, BinkSrc::IntraDc as usize) as i32;
                        let mut block = decode_intra_dct_block(r, dc)?;
                        self.idct_put_block(plane_idx, bx, by, width, height, &mut block);
                    }
                    6 => {
                        let v = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                        self.fill_block(plane_idx, bx, by, width, height, v);
                    }
                    7 => {
                        let ox = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::XOff as usize) as i8 as i32;
                        let oy = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::YOff as usize) as i8 as i32;
                        self.copy_block(plane_idx, bx, by, width, height, ox, oy);
                        let dc = take_value16(&mut self.bundles, &self.bundle_data, BinkSrc::InterDc as usize) as i32;
                        let mut block = decode_inter_dct_block(r, dc)?;
                        self.idct_add_block(plane_idx, bx, by, width, height, &mut block);
                    }
                    8 => {
                        let c0 = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                        let c1 = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                        let mut patterns = [0u8; 8];
                        for p in &mut patterns {
                            *p = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Pattern as usize);
                        }
                        self.pattern_block(plane_idx, bx, by, width, height, &patterns, c0, c1);
                    }
                    9 => {
                        self.raw_block(plane_idx, bx, by, width, height);
                    }
                    _ => {
                        self.copy_block(plane_idx, bx, by, width, height, 0, 0);
                    }
                }
                bx += 1;
            }
        }
        Ok(())
    }

    fn scaled_block(
        &mut self,
        r: &mut BitReader<'_>,
        plane_idx: usize,
        bx: u32,
        by: u32,
        width: u32,
        height: u32,
    ) -> Result<(), BinkVideoError> {
        let sub = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::SubBlockTypes as usize);
        match sub {
            6 => {
                // FILL：直接填 16×16。
                let v = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                self.fill_rect(plane_idx, (bx * 8) as i32, (by * 8) as i32, 16, 16, width, height, v);
            }
            3 => {
                let mut ublock = [0u8; 64];
                self.decode_run_into(r, &mut ublock)?;
                self.blit_scaled_ublock(plane_idx, bx, by, width, height, &ublock);
            }
            5 => {
                let dc = take_value16(&mut self.bundles, &self.bundle_data, BinkSrc::IntraDc as usize) as i32;
                let mut block = decode_intra_dct_block(r, dc)?;
                let mut ublock = [0u8; 64];
                idct_put(&mut ublock, 8, &mut block);
                self.blit_scaled_ublock(plane_idx, bx, by, width, height, &ublock);
            }
            8 => {
                let c0 = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                let c1 = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                let mut ublock = [0u8; 64];
                for row in 0..8usize {
                    let mut bits = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Pattern as usize);
                    for col in 0..8usize {
                        ublock[row * 8 + col] = if (bits & 1) != 0 { c1 } else { c0 };
                        bits >>= 1;
                    }
                }
                self.blit_scaled_ublock(plane_idx, bx, by, width, height, &ublock);
            }
            9 => {
                let mut ublock = [0u8; 64];
                for i in 0..64 {
                    ublock[i] = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                }
                self.blit_scaled_ublock(plane_idx, bx, by, width, height, &ublock);
            }
            other => {
                return Err(BinkVideoError::Msg(format!("非法 SCALED 子块类型 {other}")));
            }
        }
        Ok(())
    }

    fn blit_scaled_ublock(&mut self, plane_idx: usize, bx: u32, by: u32, width: u32, height: u32, ublock: &[u8; 64]) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        for row in 0..8usize {
            for col in 0..8usize {
                let v = ublock[row * 8 + col];
                for dy in 0..2usize {
                    for dx in 0..2usize {
                        let x = x0 + col * 2 + dx;
                        let y = y0 + row * 2 + dy;
                        if x < w && y < h {
                            self.put_plane_px(plane_idx, w, x, y, v);
                        }
                    }
                }
            }
        }
    }

    fn fill_rect(
        &mut self,
        plane_idx: usize,
        x0: i32,
        y0: i32,
        rw: i32,
        rh: i32,
        width: u32,
        height: u32,
        value: u8,
    ) {
        let (w, h) = Self::plane_dims(width, height);
        for row in 0..rh {
            let y = y0 + row;
            if y < 0 || y as usize >= h {
                continue;
            }
            for col in 0..rw {
                let x = x0 + col;
                if x < 0 || x as usize >= w {
                    continue;
                }
                self.put_plane_px(plane_idx, w, x as usize, y as usize, value);
            }
        }
    }

    fn decode_run_into(&mut self, r: &mut BitReader<'_>, ublock: &mut [u8; 64]) -> Result<(), BinkVideoError> {
        if r.bits_left() < 4 {
            return Err(BinkVideoError::Msg("RUN 块缺扫描图案索引".into()));
        }
        let pat_idx = r.read_bits(4)? as usize;
        let pattern = &BINK_RUN_PATTERNS[pat_idx];
        let mut scan = 0usize;
        let mut filled = 0i32;
        loop {
            let run = i32::from(take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Run as usize)) + 1;
            filled += run;
            if filled > 64 {
                return Err(BinkVideoError::Msg("RUN 块越界".into()));
            }
            if r.read_bit()? {
                let v = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                for _ in 0..run {
                    ublock[pattern[scan] as usize] = v;
                    scan += 1;
                }
            } else {
                for _ in 0..run {
                    ublock[pattern[scan] as usize] =
                        take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                    scan += 1;
                }
            }
            if filled >= 63 {
                break;
            }
        }
        if filled == 63 {
            ublock[pattern[scan] as usize] = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
        }
        Ok(())
    }

    fn plane_dims(width: u32, height: u32) -> (usize, usize) {
        (width as usize, height as usize)
    }

    fn put_plane_px(&mut self, plane_idx: usize, w: usize, x: usize, y: usize, v: u8) {
        match plane_idx {
            0 => self.cur.y[y * w + x] = v,
            1 => self.cur.u[y * w + x] = v,
            _ => self.cur.v[y * w + x] = v,
        }
    }

    fn run_block(
        &mut self,
        r: &mut BitReader<'_>,
        plane_idx: usize,
        bx: u32,
        by: u32,
        width: u32,
        height: u32,
    ) -> Result<(), BinkVideoError> {
        let mut ublock = [0u8; 64];
        self.decode_run_into(r, &mut ublock)?;
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        for row in 0..8usize {
            let y = y0 + row;
            if y >= h {
                break;
            }
            for col in 0..8usize {
                let x = x0 + col;
                if x >= w {
                    break;
                }
                self.put_plane_px(plane_idx, w, x, y, ublock[row * 8 + col]);
            }
        }
        Ok(())
    }

    fn raw_block(&mut self, plane_idx: usize, bx: u32, by: u32, width: u32, height: u32) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        for row in 0..8usize {
            let y = y0 + row;
            for col in 0..8usize {
                let v = take_value(&mut self.bundles, &self.bundle_data, BinkSrc::Colors as usize);
                let x = x0 + col;
                if y < h && x < w {
                    self.put_plane_px(plane_idx, w, x, y, v);
                }
            }
        }
    }

    fn add_residue_block(
        &mut self,
        plane_idx: usize,
        bx: u32,
        by: u32,
        width: u32,
        height: u32,
        block: &[i16; 64],
    ) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        if x0 + 8 <= w && y0 + 8 <= h {
            let plane = match plane_idx {
                0 => self.cur.y.as_mut_slice(),
                1 => self.cur.u.as_mut_slice(),
                _ => self.cur.v.as_mut_slice(),
            };
            add_pixels8(&mut plane[y0 * w + x0..], w, block);
            return;
        }
        for row in 0..8usize {
            let y = y0 + row;
            if y >= h {
                break;
            }
            for col in 0..8usize {
                let x = x0 + col;
                if x >= w {
                    break;
                }
                let add = i32::from(block[row * 8 + col]);
                match plane_idx {
                    0 => {
                        let i = y * w + x;
                        self.cur.y[i] = (i32::from(self.cur.y[i]) + add).clamp(0, 255) as u8;
                    }
                    1 => {
                        let i = y * w + x;
                        self.cur.u[i] = (i32::from(self.cur.u[i]) + add).clamp(0, 255) as u8;
                    }
                    _ => {
                        let i = y * w + x;
                        self.cur.v[i] = (i32::from(self.cur.v[i]) + add).clamp(0, 255) as u8;
                    }
                }
            }
        }
    }

    fn idct_put_block(&mut self, plane_idx: usize, bx: u32, by: u32, width: u32, height: u32, block: &mut [i32; 64]) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        // 边界外不写；完整 8×8 才走快速路径。
        if x0 + 8 <= w && y0 + 8 <= h {
            let plane = match plane_idx {
                0 => self.cur.y.as_mut_slice(),
                1 => self.cur.u.as_mut_slice(),
                _ => self.cur.v.as_mut_slice(),
            };
            idct_put(&mut plane[y0 * w + x0..], w, block);
            return;
        }
        let mut tmp = [0u8; 64];
        idct_put(&mut tmp, 8, block);
        for row in 0..8usize {
            let y = y0 + row;
            if y >= h {
                break;
            }
            for col in 0..8usize {
                let x = x0 + col;
                if x >= w {
                    break;
                }
                let v = tmp[row * 8 + col];
                match plane_idx {
                    0 => self.cur.y[y * w + x] = v,
                    1 => self.cur.u[y * w + x] = v,
                    _ => self.cur.v[y * w + x] = v,
                }
            }
        }
    }

    fn idct_add_block(&mut self, plane_idx: usize, bx: u32, by: u32, width: u32, height: u32, block: &mut [i32; 64]) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        if x0 + 8 <= w && y0 + 8 <= h {
            let plane = match plane_idx {
                0 => self.cur.y.as_mut_slice(),
                1 => self.cur.u.as_mut_slice(),
                _ => self.cur.v.as_mut_slice(),
            };
            idct_add(&mut plane[y0 * w + x0..], w, block);
            return;
        }
        // 裁剪边界：逐像素加。
        let mut tmp = [0i32; 64];
        tmp.copy_from_slice(block);
        super::bink_idct::bink_idct(&mut tmp);
        for row in 0..8usize {
            let y = y0 + row;
            if y >= h {
                break;
            }
            for col in 0..8usize {
                let x = x0 + col;
                if x >= w {
                    break;
                }
                let add = tmp[row * 8 + col];
                match plane_idx {
                    0 => {
                        let i = y * w + x;
                        self.cur.y[i] = (i32::from(self.cur.y[i]) + add).clamp(0, 255) as u8;
                    }
                    1 => {
                        let i = y * w + x;
                        self.cur.u[i] = (i32::from(self.cur.u[i]) + add).clamp(0, 255) as u8;
                    }
                    _ => {
                        let i = y * w + x;
                        self.cur.v[i] = (i32::from(self.cur.v[i]) + add).clamp(0, 255) as u8;
                    }
                }
            }
        }
    }

    fn copy_block(&mut self, plane_idx: usize, bx: u32, by: u32, width: u32, height: u32, ox: i32, oy: i32) {
        let (w, h) = Self::plane_dims(width, height);
        let dst_x0 = (bx as i32) * 8;
        let dst_y0 = (by as i32) * 8;
        let src_x0 = dst_x0 + ox;
        let src_y0 = dst_y0 + oy;
        for row in 0..8i32 {
            let dy = dst_y0 + row;
            let sy = src_y0 + row;
            if dy < 0 || sy < 0 || dy as usize >= h || sy as usize >= h {
                continue;
            }
            for col in 0..8i32 {
                let dx = dst_x0 + col;
                let sx = src_x0 + col;
                if dx < 0 || sx < 0 || dx as usize >= w || sx as usize >= w {
                    continue;
                }
                let s = match plane_idx {
                    0 => self.prev.y[(sy as usize) * w + (sx as usize)],
                    1 => self.prev.u[(sy as usize) * w + (sx as usize)],
                    _ => self.prev.v[(sy as usize) * w + (sx as usize)],
                };
                match plane_idx {
                    0 => self.cur.y[(dy as usize) * w + (dx as usize)] = s,
                    1 => self.cur.u[(dy as usize) * w + (dx as usize)] = s,
                    _ => self.cur.v[(dy as usize) * w + (dx as usize)] = s,
                }
            }
        }
    }

    fn fill_block(&mut self, plane_idx: usize, bx: u32, by: u32, width: u32, height: u32, value: u8) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        for row in 0..8usize {
            let y = y0 + row;
            if y >= h {
                break;
            }
            for col in 0..8usize {
                let x = x0 + col;
                if x >= w {
                    break;
                }
                match plane_idx {
                    0 => self.cur.y[y * w + x] = value,
                    1 => self.cur.u[y * w + x] = value,
                    _ => self.cur.v[y * w + x] = value,
                }
            }
        }
    }

    fn pattern_block(
        &mut self,
        plane_idx: usize,
        bx: u32,
        by: u32,
        width: u32,
        height: u32,
        patterns: &[u8; 8],
        c0: u8,
        c1: u8,
    ) {
        let (w, h) = Self::plane_dims(width, height);
        let x0 = (bx as usize) * 8;
        let y0 = (by as usize) * 8;
        for row in 0..8usize {
            let y = y0 + row;
            if y >= h {
                break;
            }
            let mut bits = patterns[row];
            for col in 0..8usize {
                let x = x0 + col;
                if x >= w {
                    break;
                }
                let v = if (bits & 1) != 0 { c1 } else { c0 };
                bits >>= 1;
                match plane_idx {
                    0 => self.cur.y[y * w + x] = v,
                    1 => self.cur.u[y * w + x] = v,
                    _ => self.cur.v[y * w + x] = v,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::bink::{BinkVersion, parse_bink_header};

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

    fn tiny_bikk_header() -> crate::image::bink::BinkHeader {
        let mut data = vec![0u8; 0x30];
        data[0..4].copy_from_slice(&0x6B4B_4942u32.to_le_bytes()); // BIKk
        data[4..8].copy_from_slice(&(100u32 - 8).to_le_bytes());
        data[8..12].copy_from_slice(&1u32.to_le_bytes());
        data[12..16].copy_from_slice(&10u32.to_le_bytes());
        data[0x14..0x18].copy_from_slice(&8u32.to_le_bytes());
        data[0x18..0x1C].copy_from_slice(&8u32.to_le_bytes());
        data[0x1C..0x20].copy_from_slice(&10u32.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&1u32.to_le_bytes());
        // BIKk 额外 4 字节；无音轨时帧索引从 0x30 起。
        parse_bink_header(&data).unwrap()
    }

    #[test]
    fn decoder_new_builds_vlc_and_rejects_short_packet() {
        let mut d = BinkVideoDecoder::new(&tiny_header()).unwrap();
        assert_eq!((d.width(), d.height()), (8, 8));
        assert_eq!(d.version(), BinkVersion::BikI);
        assert_eq!(d.vlc_tables().len(), 16);
        assert!(matches!(d.decode_packet(&[], true).unwrap_err(), BinkVideoError::Msg(_)));
        // 4 字节 = 32 位，跳过对齐槽后读树时码流耗尽。
        assert!(matches!(d.decode_packet(&[0, 0, 0, 0], true).unwrap_err(), BinkVideoError::Msg(_)));
    }

    #[test]
    fn bikk_solid_fill_planes_return_frame() {
        let mut d = BinkVideoDecoder::new(&tiny_bikk_header()).unwrap();
        // 位流 LSB 优先拼进字节缓冲。
        let mut bytes = Vec::new();
        let mut bit_buf: u32 = 0;
        let mut bit_n: u32 = 0;
        let push = |val: u32, len: u32, bytes: &mut Vec<u8>, bit_buf: &mut u32, bit_n: &mut u32| {
            *bit_buf |= val << *bit_n;
            *bit_n += len;
            while *bit_n >= 8 {
                bytes.push((*bit_buf & 0xFF) as u8);
                *bit_buf >>= 8;
                *bit_n -= 8;
            }
        };
        push(0, 32, &mut bytes, &mut bit_buf, &mut bit_n);
        let mut n_total = 32u32;
        for color in [16u32, 128u32, 200u32] {
            push(1, 1, &mut bytes, &mut bit_buf, &mut bit_n);
            push(color, 8, &mut bytes, &mut bit_buf, &mut bit_n);
            n_total += 9;
            let pad = (32 - (n_total % 32)) % 32;
            push(0, pad, &mut bytes, &mut bit_buf, &mut bit_n);
            n_total += pad;
        }
        if bit_n > 0 {
            bytes.push((bit_buf & 0xFF) as u8);
        }
        let frame = d.decode_packet(&bytes, true).unwrap();
        assert_eq!(frame.y[0], 16);
        assert_eq!(frame.u[0], 128);
        assert_eq!(frame.v[0], 200);
        assert!(d.has_prev());
    }

    #[test]
    fn blank_yuv_to_rgba_is_opaque_black() {
        let frame = BinkYuvFrame::blank(4, 4, false).unwrap();
        let rgba = frame.to_rgba8();
        assert_eq!(rgba.len(), 4 * 4 * 4);
        assert_eq!(&rgba[0..4], &[0, 0, 0, 255]);
    }
}
