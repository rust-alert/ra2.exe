//! 自顶层 `terrain_objects.rs`。

use ra_assets::IniDocument;
use ra_map::parse_terrain_objects;

#[test]
fn parse_two_trees() {
    let doc = IniDocument::parse(b"[Terrain]\n3002=INTREE01\n4005=CACTUS01\n").unwrap();
    let objs = parse_terrain_objects(&doc);
    assert_eq!(objs.len(), 2);
    assert_eq!(objs[0].y, 3);
    assert_eq!(objs[0].x, 2);
    assert_eq!(objs[0].name, "INTREE01");
    assert_eq!(objs[1].y, 4);
    assert_eq!(objs[1].x, 5);
    assert_eq!(objs[1].name, "CACTUS01");
}
