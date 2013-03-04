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
    /// 画布左上角对应的世界屏幕坐标。
    pub origin_x: i32,
    pub origin_y: i32,
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
        origin_x: min_x,
        origin_y: min_y,
    })
}

/// 在已合成地形上绘制覆盖层标记（占位色块，尚未接 SHP）。
///
/// `cell_z` 提供格子高度；未知时按 0。返回画上的格数。
pub fn paint_overlay_markers(
    image: &mut TerrainImage,
    overlays: &[crate::OverlayCell],
    mut cell_z: impl FnMut(u16, u16) -> u8,
) -> usize {
    let mut painted = 0usize;
    for cell in overlays {
        let z = cell_z(cell.x, cell.y);
        let (sx, sy) = iso_to_screen(i32::from(cell.x), i32::from(cell.y), z);
        // 钻石中心附近 6×6 色块。
        let cx = sx + TILE_WIDTH / 2 - image.origin_x - 3;
        let cy = sy + TILE_HEIGHT / 2 - image.origin_y - 3;
        let rgba = overlay_marker_rgba(cell.overlay_id);
        if fill_rect(
            &mut image.pixels,
            image.width,
            image.height,
            cx,
            cy,
            6,
            6,
            rgba,
        ) {
            painted += 1;
        }
    }
    painted
}

/// 在已合成地形上按格子绘制精灵（树 / 建筑等）。
///
/// `items` 为 `(cell_x, cell_y, blit)`；返回实际画上的数量。
pub fn paint_cell_sprites(
    image: &mut TerrainImage,
    items: &[(u16, u16, TileBlit)],
    mut cell_z: impl FnMut(u16, u16) -> u8,
) -> usize {
    if items.is_empty() {
        return 0;
    }
    let mut prepared: Vec<(i32, i32, &TileBlit)> = Vec::with_capacity(items.len());
    for (x, y, blit) in items {
        let z = cell_z(*x, *y);
        let (sx, sy) = iso_to_screen(i32::from(*x), i32::from(*y), z);
        prepared.push((
            sx + blit.offset_x - image.origin_x,
            sy + blit.offset_y - image.origin_y,
            blit,
        ));
    }
    prepared.sort_by_key(|(x, y, _)| (*y, *x));
    let mut painted = 0usize;
    for (dx, dy, blit) in prepared {
        if blit_over(
            &mut image.pixels,
            image.width,
            image.height,
            dx,
            dy,
            blit.width,
            blit.height,
            &blit.rgba,
        ) {
            painted += 1;
        }
    }
    painted
}

fn overlay_marker_rgba(id: u8) -> [u8; 4] {
    // 公开资源类型号段的软着色：宝石 / 矿石偏金，其余偏青绿。
    match id {
        27..=38 | 102..=121 | 127..=166 => [230, 190, 40, 220],
        _ => [50, 210, 120, 200],
    }
}

fn fill_rect(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    dx: i32,
    dy: i32,
    w: i32,
    h: i32,
    rgba: [u8; 4],
) -> bool {
    let mut any = false;
    for row in 0..h {
        let y = dy + row;
        if y < 0 || y >= dst_h as i32 {
            continue;
        }
        for col in 0..w {
            let x = dx + col;
            if x < 0 || x >= dst_w as i32 {
                continue;
            }
            let di = ((y as u32 * dst_w + x as u32) * 4) as usize;
            dst[di..di + 4].copy_from_slice(&rgba);
            any = true;
        }
    }
    any
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

    #[test]
    fn paint_overlay_marks_pixel() {
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
        let mut img = compose_terrain_rgba(&cells, |_, _| {
            Some(TileBlit {
                width: 60,
                height: 30,
                offset_x: 0,
                offset_y: 0,
                rgba: rgba.clone(),
            })
        })
        .unwrap();
        let overlays = [crate::OverlayCell {
            x: 2,
            y: 3,
            overlay_id: 110,
            data: 0,
        }];
        let n = paint_overlay_markers(&mut img, &overlays, |_, _| 0);
        assert_eq!(n, 1);
        assert!(img
            .pixels
            .chunks(4)
            .any(|c| c[0] == 230 && c[1] == 190 && c[2] == 40));
    }

    #[test]
    fn paint_cell_sprite_marks_pixel() {
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
        let mut img = compose_terrain_rgba(&cells, |_, _| {
            Some(TileBlit {
                width: 60,
                height: 30,
                offset_x: 0,
                offset_y: 0,
                rgba: rgba.clone(),
            })
        })
        .unwrap();
        let mut sprite = vec![0u8; 4 * 4 * 4];
        for px in sprite.chunks_exact_mut(4) {
            px.copy_from_slice(&[255, 0, 0, 255]);
        }
        let items = [(
            2u16,
            3u16,
            TileBlit {
                width: 4,
                height: 4,
                offset_x: 28,
                offset_y: 13,
                rgba: sprite,
            },
        )];
        let n = paint_cell_sprites(&mut img, &items, |_, _| 0);
        assert_eq!(n, 1);
        assert!(img.pixels.chunks(4).any(|c| c[0] == 255 && c[1] == 0));
    }
}
