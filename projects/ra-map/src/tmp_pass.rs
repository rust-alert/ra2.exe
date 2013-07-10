//! 用剧院 TMP 的陆地类型封死不可走格。

use std::collections::HashMap;

use ra_assets::TmpFile;
use ra_types::AssetSource;

use crate::{
    MapInfo, ground_passable,
    pass_grid::PassGrid,
    theater::{theater_ini_name, theater_tmp_extension},
    tileset::parse_tileset_ini,
};

/// 读取剧院 TMP 的 `terrain_type`，对水/岩/墙等不可走格调用 `PassGrid::seal_land_types`。
///
/// 返回新封格数量。
pub fn seal_pass_grid_from_tmp(source: &dyn AssetSource, map: &MapInfo, grid: &mut PassGrid) -> usize {
    if map.cells.is_empty() {
        return 0;
    }
    let Ok(ini_bytes) = source.read(theater_ini_name(map.theater))
    else {
        return 0;
    };
    let Ok(lookup) = parse_tileset_ini(&ini_bytes, theater_tmp_extension(map.theater))
    else {
        return 0;
    };
    let mut file_cache: HashMap<String, TmpFile> = HashMap::new();
    let mut sealed: Vec<(u16, u16, u8)> = Vec::new();
    for cell in &map.cells {
        if cell.x < 0 || cell.y < 0 {
            continue;
        }
        let x = cell.x as u16;
        let y = cell.y as u16;
        let Some(name) = lookup.filename(cell.tile_num).map(str::to_string)
        else {
            continue;
        };
        if !file_cache.contains_key(&name) {
            let Ok(data) = source.read(&name)
            else {
                continue;
            };
            let Ok(tmp) = TmpFile::parse(&data)
            else {
                continue;
            };
            file_cache.insert(name.clone(), tmp);
        }
        let Some(tmp) = file_cache.get(&name)
        else {
            continue;
        };
        let Some(tile) = tmp.tiles.get(usize::from(cell.sub_tile)).and_then(|t| t.as_ref())
        else {
            continue;
        };
        if !ground_passable(tile.terrain_type) {
            sealed.push((x, y, tile.terrain_type));
        }
    }
    grid.seal_land_types(&sealed)
}
