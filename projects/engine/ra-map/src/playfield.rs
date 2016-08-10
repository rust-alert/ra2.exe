//! `[Map] LocalSize` 可见区：镜头应夹在此内缘，而非整张 `Size` 外缘。

use crate::iso_math::{TILE_HEIGHT, TILE_WIDTH, iso_to_screen};

/// `[Map] LocalSize=left,top,width,height`（格）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[doc(hidden)]
pub struct LocalSize {
    /// 左缘偏移。
    pub left: i32,
    /// 上缘偏移。
    pub top: i32,
    /// 可见宽。
    pub width: i32,
    /// 可见高。
    pub height: i32,
}

impl LocalSize {
    /// 由 `Size` 宽高得到缺省可见区（与整图同大；有 `LocalSize` 键时应覆盖）。
    pub fn from_full_size(size_width: u32, size_height: u32) -> Self {
        Self { left: 0, top: 0, width: size_width.max(1) as i32, height: size_height.max(1) as i32 }
    }
}

/// 格子是否落在 `LocalSize` 可见钻石内（平面、高度 0）。
///
/// `size_width` 为 `[Map] Size` 的宽。可见带由 `Size` 宽与 `LocalSize` 四元组共同界定。
pub fn cell_in_local_playfield(size_width: i32, local: LocalSize, cell_x: i32, cell_y: i32) -> bool {
    if local.width <= 0 || local.height <= 0 || size_width <= 0 {
        return false;
    }
    let sw = size_width;
    let lx = local.left;
    let ly = local.top;
    let lw = local.width;
    let lh = local.height;
    let sum = cell_x + cell_y;
    let diff = cell_x - cell_y;
    let sum_ok = sum > sw + 2 * ly && sum <= sw + 2 + 2 * (ly + lh);
    let diff_ok = diff > 2 * lx - sw && diff < 2 * (lx + lw) - sw;
    sum_ok && diff_ok
}

/// 将 `LocalSize` 可见区投影为预览图像素轴对齐矩形 `(min_x, min_y, max_x, max_y)`。
///
/// `origin_*` 为地形合成画布原点（与 `TerrainImage::origin_*` / `preview_origin` 同口径）。
pub fn local_size_preview_rect(size_width: u32, local: LocalSize, origin_x: i32, origin_y: i32) -> Option<(i32, i32, i32, i32)> {
    if size_width == 0 || local.width <= 0 || local.height <= 0 {
        return None;
    }
    let sw = size_width as i32;
    let sum_lo = sw + 2 * local.top + 1;
    let sum_hi = sw + 2 + 2 * (local.top + local.height);
    let diff_lo = 2 * local.left - sw + 1;
    let diff_hi = 2 * (local.left + local.width) - sw - 1;
    if sum_lo > sum_hi || diff_lo > diff_hi {
        return None;
    }

    let corners = [
        (sum_lo, diff_lo),
        (sum_lo, diff_hi),
        (sum_hi, diff_lo),
        (sum_hi, diff_hi),
        ((sum_lo + sum_hi) / 2, diff_lo),
        ((sum_lo + sum_hi) / 2, diff_hi),
        (sum_lo, (diff_lo + diff_hi) / 2),
        (sum_hi, (diff_lo + diff_hi) / 2),
    ];

    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut any = false;
    for (sum, diff) in corners {
        // 保持与格子同奇偶，否则 (sum+diff) 非偶。
        let mut sum = sum;
        let diff = if (sum + diff) & 1 != 0 {
            // 与格子同奇偶：优先动 sum。
            sum += 1;
            diff
        }
        else {
            diff
        };
        let cell_x = (sum + diff) / 2;
        let cell_y = (sum - diff) / 2;
        if !cell_in_local_playfield(sw, local, cell_x, cell_y) {
            // 角点校正后可能越界；仍用邻近合法格。
            let mut found = false;
            for dy in -2..=2 {
                for dx in -2..=2 {
                    let cx = cell_x + dx;
                    let cy = cell_y + dy;
                    if cell_in_local_playfield(sw, local, cx, cy) {
                        expand_cell_image_bounds(cx, cy, origin_x, origin_y, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
                        any = true;
                        found = true;
                        break;
                    }
                }
                if found {
                    break;
                }
            }
            continue;
        }
        expand_cell_image_bounds(cell_x, cell_y, origin_x, origin_y, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
        any = true;
    }
    if !any || min_x >= max_x || min_y >= max_y {
        return None;
    }
    Some((min_x, min_y, max_x, max_y))
}

#[doc(hidden)]
pub fn expand_cell_image_bounds(
    cell_x: i32,
    cell_y: i32,
    origin_x: i32,
    origin_y: i32,
    min_x: &mut i32,
    min_y: &mut i32,
    max_x: &mut i32,
    max_y: &mut i32,
) {
    let (sx, sy) = iso_to_screen(cell_x, cell_y, 0);
    let x0 = sx - origin_x;
    let y0 = sy - origin_y;
    let x1 = x0 + TILE_WIDTH;
    let y1 = y0 + TILE_HEIGHT;
    *min_x = (*min_x).min(x0);
    *min_y = (*min_y).min(y0);
    *max_x = (*max_x).max(x1);
    *max_y = (*max_y).max(y1);
}
