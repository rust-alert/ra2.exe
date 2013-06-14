//! Alpha 遭遇战启动用地图探测。

use ra_types::GameEdition;

use crate::theater::theater_mix_names;
use crate::{MapInfo, Theater};

/// Alpha 单机优先尝试的遭遇图文件名（按序）。
pub const BOOT_MAP_CANDIDATES: &[&str] = &[
    "mp03t4.map",
    "mp01t4.map",
    "mp01t2.map",
    "mp02t4.map",
];

/// 尝试解析一张启动地图；失败返回 `None`。
pub fn try_parse_boot_map(
    edition: GameEdition,
    name: &str,
    bytes: &[u8],
) -> Result<MapInfo, String> {
    MapInfo::parse_ini(edition, name, bytes).map_err(|e| e.to_string())
}

/// 为剧院挂载标准剧院 MIX 名；`mount_nested` 返回是否新挂载。
pub fn mount_theater_mixes(
    theater: Theater,
    mount_nested: &mut dyn FnMut(&str) -> bool,
) -> usize {
    let mut n = 0usize;
    for mix_name in theater_mix_names(theater) {
        if mount_nested(mix_name) {
            n += 1;
        }
    }
    n
}
