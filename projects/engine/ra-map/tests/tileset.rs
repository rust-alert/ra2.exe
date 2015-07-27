use ra_map::{CLEAR_TILE_SENTINEL, normalize_tile_ref, parse_tileset_ini};

#[test]
fn maps_clear_and_cliff() {
    let text = b"\
[TileSet0000]\nFileName=clear\nTilesInSet=1\n\n\
[TileSet0001]\nFileName=\nTilesInSet=1\n\n\
[TileSet0002]\nFileName=cliff\nTilesInSet=3\n";
    let lookup = parse_tileset_ini(text, "tem").unwrap();
    assert_eq!(lookup.filename(0), Some("clear01.tem"));
    assert_eq!(lookup.filename(1), None);
    assert_eq!(lookup.filename(2), Some("cliff01.tem"));
    assert_eq!(lookup.filename(4), Some("cliff03.tem"));
    assert_eq!(lookup.filename(5), None);
}

#[test]
fn clear_sentinel_maps_to_clear_tile() {
    let text = b"[TileSet0000]\nFileName=clear\nTilesInSet=1\n";
    let lookup = parse_tileset_ini(text, "tem").unwrap();
    assert_eq!(normalize_tile_ref(CLEAR_TILE_SENTINEL, 7), (0, 0));
    assert_eq!(lookup.filename(CLEAR_TILE_SENTINEL), Some("clear01.tem"));
}

#[test]
fn blank_filename_token_reserves_slot_without_tmp() {
    let text = b"\
[TileSet0000]\nFileName=clear\nTilesInSet=1\n\n\
[TileSet0001]\nFileName=blank\nTilesInSet=1\n\n\
[TileSet0002]\nFileName=Cliff\nTilesInSet=1\n";
    let lookup = parse_tileset_ini(text, "tem").unwrap();
    assert_eq!(lookup.filename(0), Some("clear01.tem"));
    assert_eq!(lookup.filename(1), None);
    assert_eq!(lookup.filename(2), Some("Cliff01.tem"));
}
