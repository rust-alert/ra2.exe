use std::collections::HashMap;

use ra_map::{MapEntity, MapEntityKind, MapInfo, TerrainImage, paint_map_structures};
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

struct EmptySource;
impl AssetSource for EmptySource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

struct MapSource {
    files: HashMap<String, Vec<u8>>,
}

impl AssetSource for MapSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        self.files.get(&relative.to_ascii_lowercase()).cloned().ok_or_else(|| RaError::MissingFile(relative.to_string()))
    }
}

fn raw_one_pixel_shp(index: u8) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.push(0);
    data.extend_from_slice(&[0, 0, 0]);
    data.extend_from_slice(&[0, 0, 0, 0]);
    data.extend_from_slice(&0u32.to_le_bytes());
    data.extend_from_slice(&(32u32).to_le_bytes());
    data.push(index);
    data
}

fn solid_index_pal(index: usize, r6: u8, g6: u8, b6: u8) -> Vec<u8> {
    let mut data = vec![0u8; 768];
    let o = index * 3;
    data[o] = r6;
    data[o + 1] = g6;
    data[o + 2] = b6;
    data
}

#[test]
fn empty_structures_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(paint_map_structures(&EmptySource, &map, &mut image, "art.ini", &|p, _| p.clone()), 0);
}

#[test]
fn oil_derrick_paints_active_anim_two_flag() {
    let art = b"\
[CAOILD]\n\
Remapable=no\n\
NewTheater=yes\n\
ActiveAnim=CAOILD_A\n\
ActiveAnimTwo=CAOILD_F\n\
ActiveAnimTwoZAdjust=-50\n\
\n\
[CAOILD_A]\n\
Image=CAOILD_A\n\
NewTheater=yes\n\
Start=0\n\
\n\
[CAOILD_F]\n\
Image=CAOILD_F\n\
NewTheater=yes\n\
Start=0\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 63, 0));
    // NewTheater temperate：第二字母 → t
    files.insert("ctoild.shp".into(), raw_one_pixel_shp(5));
    files.insert("ctoild_a.shp".into(), raw_one_pixel_shp(5));
    files.insert("ctoild_f.shp".into(), raw_one_pixel_shp(5));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.entities.push(MapEntity {
        kind: MapEntityKind::Structure,
        owner: "Neutral".into(),
        type_id: "CAOILD".into(),
        health: 256,
        x: 5,
        y: 0,
        facing: 0,
        sub_cell: 0,
    });

    let source = MapSource { files };
    let mut image = TerrainImage::blank(256, 256);
    let painted = paint_map_structures(&source, &map, &mut image, "art.ini", &|p, _| p.clone());
    assert_eq!(painted, 3, "body + pump ActiveAnim + flag ActiveAnimTwo");
}
