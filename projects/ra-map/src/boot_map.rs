//! Alpha 遭遇战启动用地图探测。

use ra_types::{AssetSource, GameEdition};

use crate::{MapInfo, Theater, theater::theater_mix_names};

/// Alpha 单机优先尝试的遭遇图文件名（按序）。
pub const BOOT_MAP_CANDIDATES: &[&str] = &["mp03t4.map", "mp01t4.map", "mp01t2.map", "mp02t4.map"];

/// `find_first_boot_map` 的结果（尚未挂载剧院 MIX）。
#[derive(Debug)]
pub struct BootMapResult {
    /// 解析得到的地图（失败时为空占位图）。
    pub map: MapInfo,
    /// 相对本步的注记片段（不含前缀分隔符）。
    pub note: String,
}

/// 尝试解析一张启动地图；失败返回错误文案。
pub fn try_parse_boot_map(edition: GameEdition, name: &str, bytes: &[u8]) -> Result<MapInfo, String> {
    MapInfo::parse_ini(edition, name, bytes).map_err(|e| e.to_string())
}

/// 为剧院挂载标准剧院 MIX 名；`mount_nested` 返回是否新挂载。
pub fn mount_theater_mixes(theater: Theater, mount_nested: &mut dyn FnMut(&str) -> bool) -> usize {
    let mut n = 0usize;
    for mix_name in theater_mix_names(theater) {
        if mount_nested(mix_name) {
            n += 1;
        }
    }
    n
}

/// 按候选顺序解析第一张可加载遭遇图（不挂载 MIX）。
pub fn find_first_boot_map(edition: GameEdition, source: &dyn AssetSource) -> BootMapResult {
    let mut fail_note = String::new();
    for name in BOOT_MAP_CANDIDATES {
        let Ok(bytes) = source.read(name)
        else {
            continue;
        };
        match try_parse_boot_map(edition, name, &bytes) {
            Ok(map) => {
                let mut note = format!("map:{name} {}x{} {}", map.width, map.height, map.theater.as_str());
                note.push_str(&map_content_note(&map));
                return BootMapResult { map, note };
            }
            Err(e) => {
                if fail_note.is_empty() {
                    fail_note = format!("map:{name} 解析失败（{e}）");
                }
                else {
                    fail_note = format!("{fail_note} · map:{name} 解析失败（{e}）");
                }
            }
        }
    }
    let note = if fail_note.is_empty() { "map:无".to_string() } else { format!("{fail_note} · map:无") };
    BootMapResult { map: MapInfo::empty(edition, "boot"), note }
}

fn map_content_note(map: &MapInfo) -> String {
    let mut parts = String::new();
    if !map.cells.is_empty() {
        parts = format!("{parts} · iso#{}", map.cells.len());
    }
    if !map.overlays.is_empty() {
        parts = format!("{parts} · overlay#{}", map.overlays.len());
    }
    if !map.terrain_objects.is_empty() {
        parts = format!("{parts} · terrain#{}", map.terrain_objects.len());
    }
    if !map.entities.is_empty() {
        parts = format!("{parts} · entities#{}", map.entities.len());
    }
    if !map.waypoints.is_empty() {
        parts = format!("{parts} · wp#{}", map.waypoints.len());
    }
    parts
}
