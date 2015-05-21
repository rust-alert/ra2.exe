use std::collections::HashMap;

use ra_map::{MapInfo, OverlayCell, TerrainImage, paint_map_overlays};
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

fn overlay_map(id: u8, data: u8) -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "t");
    map.overlays = vec![OverlayCell { x: 5, y: 0, overlay_id: id, data }];
    map
}

#[test]
fn empty_overlays_noop() {
    let map = MapInfo::empty(GameEdition::Ra2, "t");
    let mut image = TerrainImage::blank(1, 1);
    assert_eq!(paint_map_overlays(&EmptySource, &map, &mut image, "art.ini", &|_| None), (0, 0));
}

#[test]
fn theater_overlay_uses_theater_palette() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[LOBRDG26]\nTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("lobrdg26.tem".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = overlay_map(102, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(&source, &map, &mut image, "art.ini", &|id| (id == 102).then(|| "LOBRDG26".into()));
    assert_eq!((shp, mark), (1, 0));
    let px = image.image.as_raw();
    let green = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted");
    assert!(green[1] > green[0] && green[1] > green[2], "expected theater green, got {green:?}");
}

#[test]
fn new_theater_wall_uses_unittem_palette() {
    let mut files = HashMap::new();
    files.insert("art.ini".into(), b"[NAWALL]\nNewTheater=yes\n".to_vec());
    files.insert("isotem.pal".into(), solid_index_pal(5, 0, 63, 0));
    files.insert("unittem.pal".into(), solid_index_pal(5, 63, 0, 0));
    files.insert("ntwall.shp".into(), raw_one_pixel_shp(5));
    let source = MapSource { files };
    let map = overlay_map(27, 0);
    let mut image = TerrainImage::blank(256, 256);
    let (shp, mark) = paint_map_overlays(&source, &map, &mut image, "art.ini", &|id| (id == 27).then(|| "NAWALL".into()));
    assert_eq!((shp, mark), (1, 0));
    let px = image.image.as_raw();
    let red = px.chunks_exact(4).find(|c| c[3] > 0).expect("painted");
    assert!(red[0] > red[1] && red[0] > red[2], "expected unittem red for wall, got {red:?}");
}
