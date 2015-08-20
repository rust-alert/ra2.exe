use std::collections::HashMap;

use ra_map::{
    MapEntity, MapEntityKind, MapInfo, StructureAnimMode, TerrainImage, buildup_frame_index, paint_map_structures,
    structure_anim_frame,
};
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

fn multi_frame_shp(indices: &[u8]) -> Vec<u8> {
    let frame_count = indices.len() as u16;
    let header_size = 8 + frame_count as usize * 24;
    let mut data = Vec::new();
    data.extend_from_slice(&0u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&frame_count.to_le_bytes());
    for i in 0..indices.len() {
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&0u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.extend_from_slice(&1u16.to_le_bytes());
        data.push(0);
        data.extend_from_slice(&[0, 0, 0]);
        data.extend_from_slice(&[0, 0, 0, 0]);
        data.extend_from_slice(&0u32.to_le_bytes());
        let offset = (header_size + i) as u32;
        data.extend_from_slice(&offset.to_le_bytes());
    }
    data.extend_from_slice(indices);
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
    assert_eq!(
        paint_map_structures(&EmptySource, &map, &mut image, "art.ini", &|p, _| p.clone(), StructureAnimMode::BodyOnly),
        0
    );
}

#[test]
fn structure_anim_frame_loops_by_rate() {
    assert_eq!(structure_anim_frame(0, 300, 0, 16), 0);
    assert_eq!(structure_anim_frame(299, 300, 0, 16), 0);
    assert_eq!(structure_anim_frame(300, 300, 0, 16), 1);
    assert_eq!(structure_anim_frame(300 * 16, 300, 0, 16), 0);
    assert_eq!(structure_anim_frame(1000, 220, 33, 64), 33 + ((1000 / 220) % 31) as u16);
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
LoopStart=0\n\
LoopEnd=1\n\
Rate=220\n\
\n\
[CAOILD_F]\n\
Image=CAOILD_F\n\
NewTheater=yes\n\
Start=0\n\
LoopStart=0\n\
LoopEnd=2\n\
Rate=300\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 63, 0));
    files.insert("ctoild.shp".into(), raw_one_pixel_shp(5));
    files.insert("ctoild_a.shp".into(), raw_one_pixel_shp(5));
    files.insert("ctoild_f.shp".into(), multi_frame_shp(&[5, 5]));

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
    let painted = paint_map_structures(
        &source,
        &map,
        &mut image,
        "art.ini",
        &|p, _| p.clone(),
        StructureAnimMode::BodyAndAnims { clock_ms: 300 },
    );
    assert_eq!(painted, 3, "body + pump ActiveAnim + flag ActiveAnimTwo");
}

#[test]
fn buildup_frame_index_one_shot() {
    assert_eq!(buildup_frame_index(0, 100, 3), Some(0));
    assert_eq!(buildup_frame_index(99, 100, 3), Some(0));
    assert_eq!(buildup_frame_index(100, 100, 3), Some(1));
    assert_eq!(buildup_frame_index(299, 100, 3), Some(2));
    assert_eq!(buildup_frame_index(300, 100, 3), None);
    assert_eq!(buildup_frame_index(0, 100, 0), None);
}

#[test]
fn load_structure_buildup_clip_decodes_frames() {
    use ra_map::load_structure_buildup_clip;

    let art = b"\
[GACNST]\n\
Remapable=yes\n\
NewTheater=yes\n\
Buildup=GACNSTMK\n\
\n\
[GACNSTMK]\n\
Rate=50\n\
";
    let mut files = HashMap::new();
    files.insert("art.ini".into(), art.to_vec());
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 63, 0));
    files.insert("gtcnstmk.shp".into(), multi_frame_shp(&[5, 5, 5]));

    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.theater = ra_map::Theater::Temperate;
    let source = MapSource { files };
    let clip = load_structure_buildup_clip(&source, &map, "art.ini", "GACNST", "Americans", 3, 4, &|p, _| p.clone())
        .expect("buildup clip");
    assert_eq!(clip.frames.len(), 3);
    assert_eq!(clip.rate_ms, 50);
    assert_eq!((clip.x, clip.y), (3, 4));
    assert_eq!(clip.frame_at(0), Some(0));
    assert_eq!(clip.frame_at(50), Some(1));
    assert_eq!(clip.frame_at(150), None);
}
