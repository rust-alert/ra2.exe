//! 自顶层 `overlay.rs`。

use ra_assets::IniDocument;
use ra_map::{NO_OVERLAY, OverlayCell, base64_encode, decode_overlay_packs};

fn pack_section(name: &str, grid: &[u8]) -> String {
    // 单块：字面量 + 结束，再包分块帧。
    assert!(grid.len() <= 63);
    let mut lcw = Vec::new();
    lcw.push(0x80 | (grid.len() as u8));
    lcw.extend_from_slice(grid);
    lcw.push(0x80);
    let mut framed = Vec::new();
    framed.extend_from_slice(&(lcw.len() as u16).to_le_bytes());
    framed.extend_from_slice(&(grid.len() as u16).to_le_bytes());
    framed.extend_from_slice(&lcw);
    let b64 = base64_encode(&framed);
    format!("[{name}]\n1={b64}\n")
}

#[test]
fn decode_two_cells() {
    // 仅前 4 字节有意义：id, FF, id, FF → 两格有效。
    let grid = [10u8, NO_OVERLAY, 20, NO_OVERLAY];
    let data = [3u8, 0, 7, 0];
    let text = format!("{}{}", pack_section("OverlayPack", &grid), pack_section("OverlayDataPack", &data));
    let doc = IniDocument::parse(text.as_bytes()).unwrap();
    let cells = decode_overlay_packs(&doc).unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0], OverlayCell { x: 0, y: 0, overlay_id: 10, data: 3 });
    assert_eq!(cells[1], OverlayCell { x: 2, y: 0, overlay_id: 20, data: 7 });
}
