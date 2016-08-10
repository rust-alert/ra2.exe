//! 瞬时弹出层几何（下拉列表等，锚在已求解控件面下方）。

use super::RectPx;

/// 锚点控件正下方第 `index` 行。
pub fn popup_row_below(anchor: RectPx, row_h: i32, index: usize) -> RectPx {
    RectPx::new(anchor.x, anchor.y + anchor.h + (index as i32) * row_h, anchor.w, row_h)
}

/// 锚点控件正下方共 `rows` 行的整块列表（至少一行高）。
pub fn popup_list_below(anchor: RectPx, row_h: i32, rows: usize) -> RectPx {
    let n = rows.max(1) as i32;
    RectPx::new(anchor.x, anchor.y + anchor.h, anchor.w, row_h * n)
}

/// 同 [`popup_list_below`]，但列表宽不小于 `min_w`。
pub fn popup_list_below_min_w(anchor: RectPx, row_h: i32, rows: usize, min_w: i32) -> RectPx {
    let mut list = popup_list_below(anchor, row_h, rows);
    list.w = list.w.max(min_w);
    list
}
