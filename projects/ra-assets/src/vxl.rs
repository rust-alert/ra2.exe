//! VXL 体素模型：头 + 肢节 + 稀疏柱解码。
//!
//! 公开布局：`Voxel Animation` 魔数 → 调色板页 → 肢节头 → body → 肢节尾。

use ra_types::{RaError, RaResult};

const MAGIC: &[u8; 16] = b"Voxel Animation\0";
const FILE_HEADER: usize = 32;
const PALETTE_PAGE: usize = 770;
const SECTION_HEADER: usize = 28;
const SECTION_TAILER: usize = 92;

/// 单个体素。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VxlVoxel {
    pub x: u8,
    pub y: u8,
    pub z: u8,
    pub color_index: u8,
    pub normal_index: u8,
}

/// 一个肢节（车身 / 炮塔等）。
#[derive(Debug, Clone)]
pub struct VxlLimb {
    pub name: String,
    pub scale: f32,
    pub size_x: u8,
    pub size_y: u8,
    pub size_z: u8,
    pub normals_mode: u8,
    pub voxels: Vec<VxlVoxel>,
}

/// 解析后的 VXL。
#[derive(Debug, Clone)]
pub struct VxlFile {
    pub limb_count: u32,
    pub body_size: u32,
    pub limbs: Vec<VxlLimb>,
}

impl VxlFile {
    pub fn parse(data: &[u8]) -> RaResult<Self> {
        if data.len() < FILE_HEADER {
            return Err(RaError::Parse("vxl 头过小".into()));
        }
        if &data[0..16] != MAGIC.as_slice() {
            return Err(RaError::Parse("不是 VXL（缺 Voxel Animation 魔数）".into()));
        }
        let palette_count = read_u32(data, 16);
        let limb_count = read_u32(data, 20);
        let tailer_count = read_u32(data, 24);
        let body_size = read_u32(data, 28);
        if limb_count == 0 {
            return Err(RaError::Parse("vxl 肢节数为 0".into()));
        }

        let palette_section = palette_count as usize * PALETTE_PAGE;
        let sections_start = FILE_HEADER + palette_section;
        let headers_end = sections_start + SECTION_HEADER * limb_count as usize;
        let tailers_start = headers_end + body_size as usize;
        let tailers_end = tailers_start + SECTION_TAILER * tailer_count as usize;
        if data.len() < tailers_end {
            return Err(RaError::Parse(format!(
                "vxl 截断：需 {tailers_end} 字节，实有 {}",
                data.len()
            )));
        }

        let body_start = headers_end;
        let mut limbs = Vec::with_capacity(limb_count as usize);
        for i in 0..limb_count as usize {
            limbs.push(parse_limb(
                data,
                i,
                tailer_count,
                sections_start,
                body_start,
                tailers_start,
            )?);
        }

        Ok(Self {
            limb_count,
            body_size,
            limbs,
        })
    }

    pub fn total_voxels(&self) -> usize {
        self.limbs.iter().map(|l| l.voxels.len()).sum()
    }
}

fn parse_limb(
    data: &[u8],
    index: usize,
    tailer_count: u32,
    sections_start: usize,
    body_start: usize,
    tailers_start: usize,
) -> RaResult<VxlLimb> {
    let hdr = sections_start + index * SECTION_HEADER;
    let name = read_c_string(&data[hdr..hdr + 16]);
    let limb_number = read_u32(data, hdr + 16);
    if limb_number >= tailer_count {
        return Err(RaError::Parse(format!(
            "vxl 肢节 {index} 尾索引 {limb_number} 越界（tailer_count={tailer_count}）"
        )));
    }

    let tail = tailers_start + limb_number as usize * SECTION_TAILER;
    let span_start = read_u32(data, tail);
    let span_end = read_u32(data, tail + 4);
    let data_span = read_u32(data, tail + 8);
    let scale = read_f32(data, tail + 12);
    let size_x = data[tail + 88];
    let size_y = data[tail + 89];
    let size_z = data[tail + 90];
    let normals_mode = data[tail + 91];

    let voxels = decode_limb_voxels(
        data, body_start, span_start, span_end, data_span, size_x, size_y, size_z,
    )?;

    Ok(VxlLimb {
        name,
        scale,
        size_x,
        size_y,
        size_z,
        normals_mode,
        voxels,
    })
}

fn decode_limb_voxels(
    data: &[u8],
    body_start: usize,
    span_start_off: u32,
    span_end_off: u32,
    data_span_off: u32,
    size_x: u8,
    size_y: u8,
    size_z: u8,
) -> RaResult<Vec<VxlVoxel>> {
    let col_count = size_x as usize * size_y as usize;
    let start_table = body_start + span_start_off as usize;
    let end_table = body_start + span_end_off as usize;
    let data_base = body_start + data_span_off as usize;
    let table_bytes = col_count * 4;
    if start_table + table_bytes > data.len() || end_table + table_bytes > data.len() {
        return Err(RaError::Parse("vxl 柱偏移表越界".into()));
    }

    let mut voxels = Vec::new();
    for col_idx in 0..col_count {
        let x = (col_idx % size_x as usize) as u8;
        let y = (col_idx / size_x as usize) as u8;
        let col_start = i32::from_le_bytes([
            data[start_table + col_idx * 4],
            data[start_table + col_idx * 4 + 1],
            data[start_table + col_idx * 4 + 2],
            data[start_table + col_idx * 4 + 3],
        ]);
        if col_start < 0 {
            continue;
        }
        decode_column(
            data,
            data_base,
            col_start as usize,
            x,
            y,
            size_z,
            &mut voxels,
        )?;
    }
    Ok(voxels)
}

fn decode_column(
    data: &[u8],
    data_base: usize,
    col_offset: usize,
    x: u8,
    y: u8,
    size_z: u8,
    voxels: &mut Vec<VxlVoxel>,
) -> RaResult<()> {
    let mut pos = data_base + col_offset;
    let mut z: u8 = 0;
    while z < size_z {
        if pos + 3 > data.len() {
            break;
        }
        let z_skip = data[pos];
        let count = data[pos + 1];
        pos += 2;
        z = z.saturating_add(z_skip);
        for _ in 0..count {
            if pos + 2 > data.len() || z >= size_z {
                return Ok(());
            }
            let color_index = data[pos];
            let normal_index = data[pos + 1];
            pos += 2;
            if color_index != 0 {
                voxels.push(VxlVoxel {
                    x,
                    y,
                    z,
                    color_index,
                    normal_index,
                });
            }
            z = z.saturating_add(1);
        }
        // dup_count 字节（校验用，可跳过）
        if pos < data.len() {
            pos += 1;
        }
    }
    Ok(())
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

fn read_f32(data: &[u8], off: usize) -> f32 {
    f32::from_le_bytes(data[off..off + 4].try_into().unwrap())
}

fn read_c_string(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_vxl() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(MAGIC);
        data.extend_from_slice(&1u32.to_le_bytes()); // palette_count
        data.extend_from_slice(&1u32.to_le_bytes()); // limb_count
        data.extend_from_slice(&1u32.to_le_bytes()); // tailer_count
        let body_size_at = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // body_size patch later
        data.push(0);
        data.extend_from_slice(&[128u8; 768]);
        data.push(0);
        // section header
        data.extend_from_slice(b"body\0\0\0\0\0\0\0\0\0\0\0\0");
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        let body_start = data.len();
        // 2x2 columns: one voxel in col0
        // span_start table
        let span_start_rel = 0u32;
        data.extend_from_slice(&0i32.to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        data.extend_from_slice(&(-1i32).to_le_bytes());
        // span_end table (unused by our decoder beyond size check)
        let span_end_rel = (data.len() - body_start) as u32;
        data.extend_from_slice(&[0u8; 16]);
        // data spans
        let data_span_rel = (data.len() - body_start) as u32;
        // column 0: z_skip=0, count=1, color=5, normal=1, dup=0
        data.extend_from_slice(&[0, 1, 5, 1, 0]);

        let body_size = (data.len() - body_start) as u32;
        data[body_size_at..body_size_at + 4].copy_from_slice(&body_size.to_le_bytes());

        // tailer 92 bytes
        let mut tail = vec![0u8; SECTION_TAILER];
        tail[0..4].copy_from_slice(&span_start_rel.to_le_bytes());
        tail[4..8].copy_from_slice(&span_end_rel.to_le_bytes());
        tail[8..12].copy_from_slice(&data_span_rel.to_le_bytes());
        tail[12..16].copy_from_slice(&1.0f32.to_le_bytes());
        // identity-ish transform left zero
        tail[88] = 2; // size_x
        tail[89] = 2; // size_y
        tail[90] = 4; // size_z
        tail[91] = 4; // normals RA2
        data.extend_from_slice(&tail);
        data
    }

    #[test]
    fn parse_minimal_one_voxel() {
        let vxl = VxlFile::parse(&minimal_vxl()).unwrap();
        assert_eq!(vxl.limb_count, 1);
        assert_eq!(vxl.limbs[0].name, "body");
        assert_eq!(vxl.limbs[0].size_x, 2);
        assert_eq!(vxl.total_voxels(), 1);
        assert_eq!(vxl.limbs[0].voxels[0].color_index, 5);
    }

    #[test]
    fn reject_bad_magic() {
        assert!(VxlFile::parse(b"not a vxl file!!!!!!!!!!!").is_err());
    }
}
