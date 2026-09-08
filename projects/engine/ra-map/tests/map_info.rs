use ra_map::{MapInfo, Theater};
use ra_types::GameEdition;

#[test]
fn parse_basic_map_ini() {
    let text = b"[Map]\nSize=0,0,50,40\nTheater=SNOW\n";
    let info = MapInfo::parse_ini(GameEdition::Ra2, "t", text).unwrap();
    assert_eq!(info.width, 50);
    assert_eq!(info.height, 40);
    assert_eq!(info.theater, Theater::Snow);
    assert!(info.cells.is_empty());
}
