//! 将 IsoCell + 砖块 RGBA 合成到一张图。

use crate::iso_math::{iso_to_screen, TILE_HEIGHT, TILE_WIDTH};
use crate::IsoCell;

/// 一块已解码的地形砖（相对钻石原点的像素缓冲）。
#[derive(Debug, Clone)]
pub struct TileBlit {
    pub width: u32,
    pub height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    /// RGBA，长度 = width * height * 4。
    pub rgba: Vec<u8>,
}

/// 合成结果。
#[derive(Debug, Clone)]
pub struct TerrainImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub drawn: usize,
}

/// 按等距顺序把单元画到画布上。
///
/// `resolve` 返回 `(tile_num, sub_tile)` 对应砖块；缺砖则跳过该单元。
pub fn compose_terrain_rgba(
    cells: &[IsoCell],
    mut resolve: impl FnMut(i32, u8) -> Option<TileBlit>,
) -> Option<TerrainImage> {
    if cells.is_empty() {
        return None;
    }

    let mut prepared: Vec<(i32, i32, TileBlit)> = Vec::new();
    for cell in cells {
        let Some(blit) = resolve(cell.tile_num, cell.sub_tile) else {
            continue;
        };
        let (sx, sy) = iso_to_screen(i32::from(cell.x), i32::from(cell.y), cell.z);
        prepared.push((sx + blit.offset_x, sy + blit.offset_y, blit));
    }
    if prepared.is_empty() {
        return None;
    }

    // 画家算法：先画靠上（小 y）的。
    prepared.sort_by_key(|(x, y, _)| (*y, *x));

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for (x, y, blit) in &prepared {
        min_x = min_x.min(*x);
        min_y = min_y.min(*y);
        max_x = max_x.max(*x + blit.width as i32);
        max_y = max_y.max(*y + blit.height as i32);
    }
    // 至少覆盖一颗钻石。
    max_x = max_x.max(min_x + TILE_WIDTH);
    max_y = max_y.max(min_y + TILE_HEIGHT);

    let width = (max_x - min_x).clamp(1, 8192) as u32;
    let height = (max_y - min_y).clamp(1, 8192) as u32;
    let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
    let mut drawn = 0usize;

    for (x, y, blit) in &prepared {
        let dx = *x - min_x;
        let dy = *y - min_y;
        if blit_over(
            &mut pixels,
            width,
            height,
            dx,
            dy,
            blit.width,
            blit.height,
            &blit.rgba,
        ) {
            drawn += 1;
        }
    }

    Some(TerrainImage {
        width,
        height,
        pixels,
        drawn,
    })
}

fn blit_over(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    dx: i32,
    dy: i32,
    src_w: u32,
    src_h: u32,
    src: &[u8],
) -> bool {
    let mut any = false;
    for row in 0..src_h as i32 {
        let y = dy + row;
        if y < 0 || y >= dst_h as i32 {
            continue;
        }
        for col in 0..src_w as i32 {
            let x = dx + col;
            if x < 0 || x >= dst_w as i32 {
                continue;
            }
            let si = ((row as u32 * src_w + col as u32) * 4) as usize;
            if si + 3 >= src.len() {
                continue;
            }
            let a = src[si + 3];
            if a == 0 {
                continue;
            }
            let di = ((y as u32 * dst_w + x as u32) * 4) as usize;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
            any = true;
        }
    }
    any
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_one_opaque_tile() {
        let mut rgba = vec![0u8; 60 * 30 * 4];
        for px in rgba.chunks_exact_mut(4) {
            px.copy_from_slice(&[10, 20, 30, 255]);
        }
        let cells = [IsoCell {
            x: 2,
            y: 3,
            tile_num: 0,
            sub_tile: 0,
            z: 0,
            flags: 0,
        }];
        let img = compose_terrain_rgba(&cells, |_, _| {
            Some(TileBlit {
                width: 60,
                height: 30,
                offset_x: 0,
                offset_y: 0,
                rgba: rgba.clone(),
            })
        })
        .unwrap();
        assert_eq!(img.drawn, 1);
        assert!(img.width >= 60);
        assert!(img.height >= 30);
        assert!(img.pixels.chunks(4).any(|c| c[3] == 255));
    }
}
