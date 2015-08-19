//! TMP 等距地形砖：模板头 + 钻石像素展开。

use ra_types::{RaError, RaResult};

use super::pal::Palette;

const TMP_HEADER_SIZE: usize = 16;
/// TMP 单格头大小（字节）。
pub const TILE_HEADER_SIZE: usize = 52;
const FLAG_HAS_EXTRA_DATA: u32 = 0x01;
const FLAG_HAS_Z_DATA: u32 = 0x02;
const DIAMOND_INITIAL_WIDTH: u32 = 4;
const DIAMOND_WIDTH_STEP: u32 = 4;

/// 一份 TMP 模板（若干 60×30 钻石单元）。
#[derive(Debug, Clone)]
pub struct TmpFile {
    /// 模板横向单元数。
    pub template_width: u32,
    /// 模板纵向单元数。
    pub template_height: u32,
    /// 单格像素宽（常见 60）。
    pub tile_width: u32,
    /// 单格像素高（常见 30）。
    pub tile_height: u32,
    /// 按行主序的单元；`None` 表示空槽。
    pub tiles: Vec<Option<TmpTile>>,
}

/// 单个地形单元的矩形缓冲（钻石外为 0）。
#[derive(Debug, Clone)]
pub struct TmpTile {
    /// 高度档。
    pub height: u8,
    /// 地形类型字节。
    pub terrain_type: u8,
    /// 坡道类型字节。
    pub ramp_type: u8,
    /// 行优先调色板索引。
    pub pixels: Vec<u8>,
    /// 缓冲宽。
    pub pixel_width: u32,
    /// 缓冲高。
    pub pixel_height: u32,
    /// 钻石原点相对缓冲的 X 偏移。
    pub offset_x: i32,
    /// 钻石原点相对缓冲的 Y 偏移。
    pub offset_y: i32,
}

impl TmpFile {
    /// 解析 TMP 字节。
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() < TMP_HEADER_SIZE {
            return Err(RaError::Parse("tmp 头过小".into()));
        }

        let raw_tw = read_u32(data, 0);
        let raw_th = read_u32(data, 4);
        let template_width = u32::from(data[0]);
        let template_height = u32::from(data[4]);
        if raw_tw != template_width || raw_th != template_height {
            return Err(RaError::Parse(format!("tmp 模板尺寸高位非零: {raw_tw}x{raw_th}")));
        }
        if template_width == 0 || template_height == 0 {
            return Err(RaError::Parse("tmp 模板尺寸为 0".into()));
        }

        let tile_width = read_u32(data, 8);
        let tile_height = read_u32(data, 12);
        if tile_width < DIAMOND_INITIAL_WIDTH || tile_height < 2 {
            return Err(RaError::Parse(format!("tmp 单元过小: {tile_width}x{tile_height}")));
        }

        let cell_count =
            (template_width as usize).checked_mul(template_height as usize).ok_or_else(|| RaError::Parse("tmp 单元数溢出".into()))?;
        let offsets_end = TMP_HEADER_SIZE
            .checked_add(cell_count.checked_mul(4).ok_or_else(|| RaError::Parse("tmp 偏移表过大".into()))?)
            .ok_or_else(|| RaError::Parse("tmp 偏移表溢出".into()))?;
        if data.len() < offsets_end {
            return Err(RaError::Parse("tmp 偏移表截断".into()));
        }

        let mut tiles = Vec::with_capacity(cell_count);
        for i in 0..cell_count {
            let offset = read_u32(data, TMP_HEADER_SIZE + i * 4);
            if offset == 0 {
                tiles.push(None);
                continue;
            }
            tiles.push(Some(parse_tile_cell(data, offset as usize, tile_width, tile_height)?));
        }

        Ok(Self { template_width, template_height, tile_width, tile_height, tiles })
    }

    /// 将指定单元转为 RGBA。
    ///
    /// 索引 0（及调色板里标透明的槽）始终全透明；钻石内把索引 0 画成
    /// `temperat.pal[0]` 实心色会在悬崖/空洞格上冒出深蓝三角碎片。
    pub fn tile_to_rgba(&self, tile_index: usize, palette: &Palette) -> RaResult<Vec<u8>> {
        let tile =
            self.tiles.get(tile_index).and_then(|t| t.as_ref()).ok_or_else(|| RaError::Parse(format!("tmp 单元 {tile_index} 为空或不存在")))?;

        let mut rgba = Vec::with_capacity(tile.pixels.len() * 4);
        for &idx in &tile.pixels {
            let color = palette.colors[idx as usize];
            rgba.push(color.r);
            rgba.push(color.g);
            rgba.push(color.b);
            rgba.push(if color.a == 0 { 0 } else { 255 });
        }
        Ok(rgba)
    }

    /// 模板单元槽位数（含空槽）。
    pub fn cell_count(&self) -> usize {
        self.tiles.len()
    }
}

fn parse_tile_cell(data: &[u8], offset: usize, tile_width: u32, tile_height: u32) -> RaResult<TmpTile> {
    let header_end = offset.checked_add(TILE_HEADER_SIZE).ok_or_else(|| RaError::Parse("tmp 单元头溢出".into()))?;
    if header_end > data.len() {
        return Err(RaError::Parse("tmp 单元头截断".into()));
    }

    let extra_data_offset = read_u32(data, offset + 8);
    let z_data_offset = read_u32(data, offset + 12);
    let extra_z_data_offset = read_u32(data, offset + 16);
    let raw_extra_x = read_i32(data, offset + 20);
    let raw_extra_y = read_i32(data, offset + 24);
    let stored_x = read_i32(data, offset);
    let stored_y = read_i32(data, offset + 4);
    let extra_width = read_u32(data, offset + 28);
    let extra_height = read_u32(data, offset + 32);
    let flags = read_u32(data, offset + 36);
    let height = data[offset + 40];
    let terrain_type = data[offset + 41];
    let ramp_type = data[offset + 42];

    let has_extra = (flags & FLAG_HAS_EXTRA_DATA) != 0;
    let has_z = (flags & FLAG_HAS_Z_DATA) != 0;
    let (extra_x, extra_y) = if has_extra { (raw_extra_x - stored_x, raw_extra_y - stored_y) } else { (0, 0) };

    let (pixel_width, pixel_height, offset_x, offset_y) = if has_extra {
        let min_x = 0i64.min(i64::from(extra_x));
        let min_y = 0i64.min(i64::from(extra_y));
        let max_x = i64::from(tile_width).max(i64::from(extra_x) + i64::from(extra_width));
        let max_y = i64::from(tile_height).max(i64::from(extra_y) + i64::from(extra_height));
        (
            u32::try_from(max_x - min_x).map_err(|_| RaError::Parse("tmp 宽溢出".into()))?,
            u32::try_from(max_y - min_y).map_err(|_| RaError::Parse("tmp 高溢出".into()))?,
            i32::try_from(min_x).map_err(|_| RaError::Parse("tmp 原点溢出".into()))?,
            i32::try_from(min_y).map_err(|_| RaError::Parse("tmp 原点溢出".into()))?,
        )
    }
    else {
        (tile_width, tile_height, 0, 0)
    };

    let buf_size = (pixel_width as usize).checked_mul(pixel_height as usize).ok_or_else(|| RaError::Parse("tmp 像素缓冲溢出".into()))?;
    let mut pixels = vec![0u8; buf_size];
    let mut depth = vec![0u8; buf_size];

    let diamond_bytes = diamond_byte_count(tile_width, tile_height)?;
    let diamond = slice_at(data, offset, TILE_HEADER_SIZE as u32, diamond_bytes)?;
    unpack_diamond(diamond, tile_width, tile_height, &mut pixels, pixel_width, offset_x, offset_y)?;

    if has_z {
        let z = slice_at(data, offset, z_data_offset, diamond_bytes)?;
        unpack_diamond(z, tile_width, tile_height, &mut depth, pixel_width, offset_x, offset_y)?;
    }

    if has_extra {
        let extra_count = (extra_width as usize).checked_mul(extra_height as usize).ok_or_else(|| RaError::Parse("tmp 附加面过大".into()))?;
        let extra = slice_at(data, offset, extra_data_offset, extra_count)?;
        let extra_z = if has_z { Some(slice_at(data, offset, extra_z_data_offset, extra_count)?) } else { None };
        overlay_extra(extra, extra_z, extra_x, extra_y, extra_width, extra_height, &mut pixels, &mut depth, pixel_width, offset_x, offset_y)?;
    }

    let _ = depth;
    Ok(TmpTile { height, terrain_type, ramp_type, pixels, pixel_width, pixel_height, offset_x, offset_y })
}

/// 计算 TMP 钻石像素区字节数（供解析与测试）。
pub fn diamond_byte_count(tile_width: u32, tile_height: u32) -> RaResult<usize> {
    let mut total = 0usize;
    let mut row_width = DIAMOND_INITIAL_WIDTH;
    let half_minus_one = tile_height / 2 - 1;
    for j in 0..tile_height {
        if row_width > tile_width {
            return Err(RaError::Parse(format!("tmp 钻石行 {j} 宽 {row_width} 超过 {tile_width}")));
        }
        total = total.checked_add(row_width as usize).ok_or_else(|| RaError::Parse("tmp 钻石字节溢出".into()))?;
        if j < half_minus_one {
            row_width += DIAMOND_WIDTH_STEP;
        }
        else {
            row_width = row_width.saturating_sub(DIAMOND_WIDTH_STEP);
        }
    }
    Ok(total)
}

fn unpack_diamond(
    data: &[u8],
    tile_width: u32,
    tile_height: u32,
    buf: &mut [u8],
    buf_width: u32,
    buf_origin_x: i32,
    buf_origin_y: i32,
) -> RaResult<()> {
    let buf_width = buf_width as usize;
    if buf_width == 0 || buf.len() % buf_width != 0 {
        return Err(RaError::Parse("tmp 缓冲尺寸无效".into()));
    }
    let buf_height = buf.len() / buf_width;
    let mut read_pos = 0usize;
    let mut row_width = DIAMOND_INITIAL_WIDTH;
    let half_minus_one = tile_height / 2 - 1;

    for j in 0..tile_height {
        if row_width > 0 {
            let end = read_pos + row_width as usize;
            if end > data.len() {
                return Err(RaError::Parse(format!("tmp 钻石行 {j} 数据不足")));
            }
            let x_start = i64::from((tile_width - row_width) / 2) - i64::from(buf_origin_x);
            let y = i64::from(j) - i64::from(buf_origin_y);
            if x_start < 0 || y < 0 || x_start + i64::from(row_width) > buf_width as i64 || y >= buf_height as i64 {
                return Err(RaError::Parse(format!("tmp 钻石行 {j} 越界")));
            }
            let dest = y as usize * buf_width + x_start as usize;
            buf[dest..dest + row_width as usize].copy_from_slice(&data[read_pos..end]);
            read_pos = end;
        }
        if j < half_minus_one {
            row_width += DIAMOND_WIDTH_STEP;
        }
        else {
            row_width = row_width.saturating_sub(DIAMOND_WIDTH_STEP);
        }
    }
    Ok(())
}

fn overlay_extra(
    extra_data: &[u8],
    extra_z_data: Option<&[u8]>,
    extra_x: i32,
    extra_y: i32,
    extra_width: u32,
    extra_height: u32,
    pixels: &mut [u8],
    depth: &mut [u8],
    buf_width: u32,
    buf_origin_x: i32,
    buf_origin_y: i32,
) -> RaResult<()> {
    let buf_width = buf_width as usize;
    let buf_height = pixels.len() / buf_width;
    for ey in 0..extra_height {
        for ex in 0..extra_width {
            let src = ey as usize * extra_width as usize + ex as usize;
            let val = extra_data[src];
            if val == 0 {
                continue;
            }
            let bx = i64::from(extra_x) + i64::from(ex) - i64::from(buf_origin_x);
            let by = i64::from(extra_y) + i64::from(ey) - i64::from(buf_origin_y);
            if bx < 0 || by < 0 || bx >= buf_width as i64 || by >= buf_height as i64 {
                return Err(RaError::Parse("tmp 附加像素越界".into()));
            }
            let dest = by as usize * buf_width + bx as usize;
            pixels[dest] = val;
            if let Some(z) = extra_z_data {
                depth[dest] = z[src];
            }
        }
    }
    Ok(())
}

fn slice_at<'a>(data: &'a [u8], cell: usize, rel: u32, len: usize) -> RaResult<&'a [u8]> {
    let start = cell.checked_add(rel as usize).ok_or_else(|| RaError::Parse("tmp 平面偏移溢出".into()))?;
    let end = start.checked_add(len).ok_or_else(|| RaError::Parse("tmp 平面长度溢出".into()))?;
    data.get(start..end).ok_or_else(|| RaError::Parse("tmp 平面截断".into()))
}

fn read_u32(data: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(data[o..o + 4].try_into().unwrap())
}

fn read_i32(data: &[u8], o: usize) -> i32 {
    i32::from_le_bytes(data[o..o + 4].try_into().unwrap())
}
