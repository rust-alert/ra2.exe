//! 自顶层 `placements.rs`。

use ra_assets::IniDocument;
use ra_map::{MapEntityKind, parse_map_entities};

#[test]
fn parse_structure_and_infantry() {
    let text = b"\
[Structures]\n\
1=Neutral,GACNST,256,10,20,0,None\n\
[Infantry]\n\
2=Americans,E1,256,11,21,2,Guard,32\n\
";
    let doc = IniDocument::parse(text).unwrap();
    let ents = parse_map_entities(&doc);
    assert_eq!(ents.len(), 2);
    let structure = ents.iter().find(|e| e.kind == MapEntityKind::Structure).unwrap();
    let infantry = ents.iter().find(|e| e.kind == MapEntityKind::Infantry).unwrap();
    assert_eq!(structure.type_id, "GACNST");
    assert_eq!(structure.x, 10);
    assert_eq!(structure.tag, "None");
    assert_eq!(infantry.sub_cell, 2);
    assert_eq!(infantry.facing, 32);
    assert_eq!(infantry.mission, "Guard");
}

#[test]
fn parse_unit_via_westwood_csv_row() {
    let text = b"\
[Units]\n\
0=Americans,MTNK,bad,5,6,999,Hunt,TagA\n\
";
    let doc = IniDocument::parse(text).unwrap();
    let ents = parse_map_entities(&doc);
    assert_eq!(ents.len(), 1);
    let unit = &ents[0];
    assert_eq!(unit.kind, MapEntityKind::Unit);
    assert_eq!(unit.type_id, "MTNK");
    assert_eq!(unit.health, 256);
    assert_eq!(unit.x, 5);
    assert_eq!(unit.y, 6);
    assert_eq!(unit.facing, 255);
    assert_eq!(unit.mission, "Hunt");
    assert_eq!(unit.tag, "TagA");
}

#[test]
fn reject_placement_when_cell_is_not_numeric() {
    let text = b"[Structures]\n0=Neutral,GAPOWR,256,xx,20,0\n";
    let doc = IniDocument::parse(text).unwrap();
    assert!(parse_map_entities(&doc).is_empty());
}
