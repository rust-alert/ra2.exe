//! 将 IsoCell + 砖块 RGBA 合成到一张图。

use image::RgbaImage;

use crate::{
    IsoCell,
    iso_math::{TILE_HEIGHT, TILE_WIDTH, iso_to_screen},
};

/// 一块已解码的地形砖（相对钻石原点的像素缓冲）。
#[derive(Debug, Clone)]
pub struct TileBlit {
    /// 像素宽。
    pub width: u32,
    /// 像素高。
    pub height: u32,
    /// 相对格子钻石原点的 X 偏移。
    pub offset_x: i32,
    /// 相对格子钻石原点的 Y 偏移。
    pub offset_y: i32,
    /// RGBA，长度 = width * height * 4。
    pub rgba: Vec<u8>,
}

/// 合成结果。
#[derive(Debug, Clone)]
pub struct TerrainImage {
    /// RGBA 画布。
    pub image: RgbaImage,
    /// 实际画上的砖块数。
    pub drawn: usize,
    /// 画布左上角对应的世界屏幕坐标。
    pub origin_x: i32,
    /// 画布左上角对应的世界屏幕 Y。
    pub origin_y: i32,
}

impl TerrainImage {
    /// 空白画布（测试与占位合成用）。
    pub fn blank(width: u32, height: u32) -> Self {
        Self { image: RgbaImage::new(width.max(1), height.max(1)), drawn: 0, origin_x: 0, origin_y: 0 }
    }
}

/// 按等距顺序把单元画到画布上。
///
/// `resolve` 返回 `(tile_num, sub_tile)` 对应砖块；缺砖则跳过该单元。
///
/// 排序按**格子**深度（`x+y`、`x`、高度），不用 blit 顶边。悬崖 TMP 的 extra
/// 常 `offset_y < 0`（朝屏幕上方伸出）；若按 blit 顶边排序，会被更北的水面钻石盖住，
/// 北向峡谷边就会出现锯齿缺口。
pub fn compose_terrain_rgba(cells: &[IsoCell], mut resolve: impl FnMut(i32, u8) -> Option<TileBlit>) -> Option<TerrainImage> {
    if cells.is_empty() {
        return None;
    }

    // (blit_x, blit_y, depth=x+y, cell_x, z, blit)
    let mut prepared: Vec<(i32, i32, i32, i16, u8, TileBlit)> = Vec::new();
    for cell in cells {
        let Some(blit) = resolve(cell.tile_num, cell.sub_tile)
        else {
            continue;
        };
        let (sx, sy) = iso_to_screen(i32::from(cell.x), i32::from(cell.y), cell.z);
        let depth = i32::from(cell.x) + i32::from(cell.y);
        prepared.push((sx + blit.offset_x, sy + blit.offset_y, depth, cell.x, cell.z, blit));
    }
    if prepared.is_empty() {
        return None;
    }

    // 先远后近；同列先低后高，让高地悬崖面盖住低处水面。
    prepared.sort_by_key(|(_, _, depth, x, z, _)| (*depth, *x, *z));

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    for (x, y, _, _, _, blit) in &prepared {
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
    let mut image = RgbaImage::new(width, height);
    let mut drawn = 0usize;

    for (x, y, _, _, _, blit) in &prepared {
        let dx = *x - min_x;
        let dy = *y - min_y;
        if blit_over(image.as_mut(), width, height, dx, dy, blit.width, blit.height, &blit.rgba) {
            drawn += 1;
        }
    }

    Some(TerrainImage { image, drawn, origin_x: min_x, origin_y: min_y })
}

/// 在已合成地形上绘制覆盖层标记（占位色块，尚未接 SHP）。
///
/// `cell_z` 提供格子高度；未知时按 0。返回画上的格数。
pub fn paint_overlay_markers(image: &mut TerrainImage, overlays: &[crate::OverlayCell], mut cell_z: impl FnMut(u16, u16) -> u8) -> usize {
    let mut painted = 0usize;
    let (width, height) = (image.image.width(), image.image.height());
    for cell in overlays {
        let z = cell_z(cell.x, cell.y);
        let (sx, sy) = iso_to_screen(i32::from(cell.x), i32::from(cell.y), z);
        // 钻石中心附近 6×6 色块。
        let cx = sx + TILE_WIDTH / 2 - image.origin_x - 3;
        let cy = sy + TILE_HEIGHT / 2 - image.origin_y - 3;
        let rgba = overlay_marker_rgba(cell.overlay_id);
        if fill_rect(image.image.as_mut(), width, height, cx, cy, 6, 6, rgba) {
            painted += 1;
        }
    }
    painted
}

/// 主体 SHP 缺失时的建筑占位色块（品红，区别于 overlay 青绿/金）。
///
/// `cells` 为 `(x, y)`；色块约 12×12，落在钻石中心。
pub fn paint_structure_missing_markers(
    image: &mut TerrainImage,
    cells: &[(u16, u16)],
    mut cell_z: impl FnMut(u16, u16) -> u8,
) -> usize {
    const MARK: [u8; 4] = [240, 80, 200, 230];
    const SIZE: i32 = 12;
    let mut painted = 0usize;
    let (width, height) = (image.image.width(), image.image.height());
    for &(x, y) in cells {
        let z = cell_z(x, y);
        let (sx, sy) = iso_to_screen(i32::from(x), i32::from(y), z);
        let cx = sx + TILE_WIDTH / 2 - image.origin_x - SIZE / 2;
        let cy = sy + TILE_HEIGHT / 2 - image.origin_y - SIZE / 2;
        if fill_rect(image.image.as_mut(), width, height, cx, cy, SIZE, SIZE, MARK) {
            painted += 1;
        }
    }
    painted
}

/// 在已合成地形上按格子绘制精灵（树 / 建筑等）。
///
/// `items` 为 `(cell_x, cell_y, blit)`；返回实际画上的数量。
pub fn paint_cell_sprites(image: &mut TerrainImage, items: &[(u16, u16, TileBlit)], mut cell_z: impl FnMut(u16, u16) -> u8) -> usize {
    if items.is_empty() {
        return 0;
    }
    let mut prepared: Vec<(i32, i32, &TileBlit)> = Vec::with_capacity(items.len());
    for (x, y, blit) in items {
        let z = cell_z(*x, *y);
        let (sx, sy) = iso_to_screen(i32::from(*x), i32::from(*y), z);
        prepared.push((sx + blit.offset_x - image.origin_x, sy + blit.offset_y - image.origin_y, blit));
    }
    prepared.sort_by_key(|(x, y, _)| (*y, *x));
    let mut painted = 0usize;
    let (width, height) = (image.image.width(), image.image.height());
    for (dx, dy, blit) in prepared {
        if blit_over(image.image.as_mut(), width, height, dx, dy, blit.width, blit.height, &blit.rgba) {
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

fn fill_rect(dst: &mut [u8], dst_w: u32, dst_h: u32, dx: i32, dy: i32, w: i32, h: i32, rgba: [u8; 4]) -> bool {
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

fn blit_over(dst: &mut [u8], dst_w: u32, dst_h: u32, dx: i32, dy: i32, src_w: u32, src_h: u32, src: &[u8]) -> bool {
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
