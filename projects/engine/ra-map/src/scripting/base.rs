//! 地图 `[Base]`：AI 基建节点表（类型 + 落点），不展开为开局已有建筑。

use ra_assets::{IniDocument, numbered_pairs, parse_westwood_csv_line};
use ra_types::{HouseName, TechnoName};

/// 单个基地建造节点（装载中间态）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapBaseNode {
    /// 建筑类型名（装载期大写）。
    pub type_name: TechnoName,
    /// 落点格 X（建筑锚点）。
    pub x: u16,
    /// 落点格 Y（建筑锚点）。
    pub y: u16,
}

/// 地图 `[Base]` 计划（装载中间态）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapBasePlan {
    /// `Player=`（国家 / 房屋键，装载期大写；可空表示未指定）。
    pub player: HouseName,
    /// `Count=`（若有则与节点数交叉核对，不强制截断）。
    pub count: Option<u32>,
    /// 编号行按序排列的节点。
    pub nodes: Vec<MapBaseNode>,
}

/// 解析 `[Base]`；缺节返回 `None`。
pub fn parse_map_base(doc: &IniDocument) -> Option<MapBasePlan> {
    let sec = doc.section("Base")?;
    let mut player = HouseName::default();
    let mut count = None;
    for (key, value) in sec.pairs() {
        if key.eq_ignore_ascii_case("Player") {
            player = HouseName::parse(value.trim());
            continue;
        }
        if key.eq_ignore_ascii_case("Count") {
            count = value.trim().parse::<u32>().ok();
        }
    }
    let mut nodes = Vec::new();
    for (_index, raw) in numbered_pairs(sec) {
        let row = parse_westwood_csv_line(raw);
        let Some(type_raw) = row.fields.first()
        else {
            continue;
        };
        let type_name = TechnoName::parse(type_raw.as_str().trim());
        if type_name.is_empty() {
            continue;
        }
        let x = row.fields.get(1).and_then(|s| s.as_str().trim().parse::<u16>().ok()).unwrap_or(0);
        let y = row.fields.get(2).and_then(|s| s.as_str().trim().parse::<u16>().ok()).unwrap_or(0);
        nodes.push(MapBaseNode { type_name, x, y });
    }
    Some(MapBasePlan { player, count, nodes })
}
