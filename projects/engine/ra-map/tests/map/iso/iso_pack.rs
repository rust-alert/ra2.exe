//! 自顶层 `iso_pack.rs`。

use ra_map::parse_iso_cells;

#[test]
fn parse_one_cell_record() {
    let mut raw = Vec::new();
    raw.extend_from_slice(&3i16.to_le_bytes());
    raw.extend_from_slice(&4i16.to_le_bytes());
    raw.extend_from_slice(&7i32.to_le_bytes());
    raw.push(1);
    raw.push(2);
    raw.push(0);
    // padding empty
    raw.extend_from_slice(&[0u8; 11]);
    let cells = parse_iso_cells(&raw);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].x, 3);
    assert_eq!(cells[0].y, 4);
    assert_eq!(cells[0].tile_num, 7);
    assert_eq!(cells[0].sub_tile, 1);
    assert_eq!(cells[0].z, 2);
}
