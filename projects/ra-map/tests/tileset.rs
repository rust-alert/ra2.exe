use ra_map::parse_tileset_ini;

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
