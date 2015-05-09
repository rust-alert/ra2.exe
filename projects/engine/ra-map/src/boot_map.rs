//! Alpha 遭遇战启动用地图探测。

use ra_types::{AssetSource, GameEdition};

use crate::{MapInfo, Theater, theater::theater_mix_names};

/// Alpha 单机优先尝试的遭遇图文件名（按序）。
pub const BOOT_MAP_CANDIDATES: &[&str] = &["mp03t4.map", "mp01t4.map", "mp01t2.map", "mp02t4.map"];

/// 成功解析的启动地图（尚未挂载剧院 MIX）。
#[derive(Debug)]
pub struct BootMapResult {
    /// 解析得到的地图。
    pub map: MapInfo,
    /// 相对本步的注记片段（不含前缀分隔符）。
    pub note: String,
}

/// 可解析的启动候选地图摘要（供遭遇战大厅列表）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootMapCandidate {
    /// 文件名（如 `mp03t4.map`）。
    pub file_name: String,
    /// 地图宽（格）。
    pub width: u32,
    /// 地图高（格）。
    pub height: u32,
    /// 剧院。
    pub theater: Theater,
    /// 遭遇战开局席位数（2..=8；来自航点 0..7 或文件名 `tN`）。
    pub start_slots: u8,
}

/// 统计遭遇战开局席位：优先航点编号 `< 8`，否则从文件名 `tN` 推断，再否则 4。
pub fn count_skirmish_start_slots(waypoints: &[crate::Waypoint], file_name: &str) -> u8 {
    let from_wp = waypoints.iter().filter(|w| w.index < 8).count();
    if from_wp >= 2 {
        return (from_wp as u8).min(8);
    }
    infer_start_slots_from_file_name(file_name).unwrap_or(4)
}

/// 本地玩家占 1 席后，大厅应显示的 AI 行数（0..=7）。
pub fn skirmish_ai_row_count(start_slots: u8) -> usize {
    start_slots.saturating_sub(1).min(7) as usize
}

fn infer_start_slots_from_file_name(file_name: &str) -> Option<u8> {
    let stem = file_name.rsplit_once('.').map(|(s, _)| s).unwrap_or(file_name);
    let bytes = stem.as_bytes();
    let mut i = 0usize;
    while i + 1 < bytes.len() {
        if matches!(bytes[i], b't' | b'T') && bytes[i + 1].is_ascii_digit() {
            let n = (bytes[i + 1] - b'0') as u8;
            if (2..=8).contains(&n) {
                return Some(n);
            }
        }
        i += 1;
    }
    None
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

/// 列出候选表中当前资源源可解析的遭遇图（保序）。
pub fn list_parseable_boot_maps(edition: GameEdition, source: &dyn AssetSource) -> Vec<BootMapCandidate> {
    let mut out = Vec::new();
    for name in BOOT_MAP_CANDIDATES {
        let Ok(bytes) = source.read(name)
        else {
            continue;
        };
        let Ok(map) = try_parse_boot_map(edition, name, &bytes)
        else {
            continue;
        };
        out.push(BootMapCandidate {
            file_name: (*name).to_string(),
            width: map.size_width,
            height: map.size_height,
            theater: map.theater,
            start_slots: count_skirmish_start_slots(&map.waypoints, name),
        });
    }
    out
}

/// 按文件名解析一张启动地图；找不到或解析失败返回 `None`。
pub fn find_boot_map_named(edition: GameEdition, source: &dyn AssetSource, name: &str) -> Option<BootMapResult> {
    let bytes = source.read(name).ok()?;
    let map = try_parse_boot_map(edition, name, &bytes).ok()?;
    let mut note = format!(
        "map:{name} size={}x{} grid={}x{} {}",
        map.size_width,
        map.size_height,
        map.width,
        map.height,
        map.theater.as_str()
    );
    note.push_str(&map_content_note(&map));
    Some(BootMapResult { map, note })
}

/// 按候选顺序解析第一张可加载遭遇图（不挂载 MIX）。
///
/// 全部失败返回 `None`（不返回空占位图）。供探测 / 无指定地图的自动选图。
pub fn find_first_boot_map(edition: GameEdition, source: &dyn AssetSource) -> Option<BootMapResult> {
    for name in BOOT_MAP_CANDIDATES {
        let Ok(bytes) = source.read(name)
        else {
            continue;
        };
        match try_parse_boot_map(edition, name, &bytes) {
            Ok(map) => {
                let mut note = format!(
        "map:{name} size={}x{} grid={}x{} {}",
        map.size_width,
        map.size_height,
        map.width,
        map.height,
        map.theater.as_str()
    );
                note.push_str(&map_content_note(&map));
                return Some(BootMapResult { map, note });
            }
            Err(_) => continue,
        }
    }
    None
}

/// 按请求装载启动地图。
///
/// - `preferred` 有值：必须命中该文件，失败**不**换候选、**不**返回空图。
/// - `preferred` 为 `None`：按候选表自动选首张可解析图；全部失败返回错误。
pub fn find_boot_map(edition: GameEdition, source: &dyn AssetSource, preferred: Option<&str>) -> Result<BootMapResult, String> {
    if let Some(name) = preferred {
        return find_boot_map_named(edition, source, name).ok_or_else(|| format!("指定地图不可用: {name}（不换图）"));
    }
    find_first_boot_map(edition, source).ok_or_else(|| "无可用启动地图（候选均不可读或解析失败）".to_string())
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
