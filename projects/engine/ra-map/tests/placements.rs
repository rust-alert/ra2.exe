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
