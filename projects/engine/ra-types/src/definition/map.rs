//! 冻结地图定义契约：装载完成后的静态地图真相。
//!
//! `ra-map` loader 产出语义结构后迁入本契约；adaptor 绑定规则得到 [`PreparedMap`]。
//! 不含 `IniDocument`、文件路径、MIX/GPU 句柄或对局可变状态。

/// 冻结的完整静态地图（装载期产出，对局与绘制只读）。
///
/// 当前为骨架：字段随地图语义层收口逐步迁入，禁止在运行路径回查地图 INI。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MapDefinition {
    /// 地图逻辑名（场景名 / 文件 stem）。
    pub name: String,
    /// `[Map] Size` 宽。
    pub size_width: u32,
    /// `[Map] Size` 高。
    pub size_height: u32,
    /// `[Map] LocalSize` 可见区（格）。
    pub local_size: MapLocalSize,
    /// 游戏格网边长（与 iso / 航点 / 覆盖层同一坐标系）。
    pub cell_side: u32,
    /// 剧院名（大写，如 `TEMPERATE`）。
    pub theater: String,
    /// `[Basic] Description` CSF 键（可空）。
    pub description_csf: String,
    /// `[Basic] GameModes` 标签。
    pub game_modes: Vec<String>,
    /// `[Basic] NextMission`（可空）。
    pub next_mission: String,
    /// `[Basic] AlternateNextMission`（可空）。
    pub alternate_next_mission: String,
    /// `[Basic] StartingCredits`。
    pub starting_credits: i32,
    /// `[Waypoints]` 格子锚点（编号已排序）。
    pub waypoints: Vec<MapWaypoint>,
    /// `[Terrain]` 静态地形物件。
    pub terrain_objects: Vec<MapTerrainObject>,
}

/// 冻结地图航点（任务 / 出生点等格子锚点）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapWaypoint {
    /// 航点编号。
    pub index: u32,
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
}

/// `[Map] LocalSize=left,top,width,height` 可见区（格）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapLocalSize {
    /// 左缘偏移。
    pub left: i32,
    /// 上缘偏移。
    pub top: i32,
    /// 可见宽。
    pub width: i32,
    /// 可见高。
    pub height: i32,
}

/// `[Terrain]` 静态地形物件占位。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTerrainObject {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 物件类型名（通常已大写）。
    pub name: String,
}

/// 与 [`crate::RuntimeDefinitions`] 绑定后的可开战 / 可预览地图。
///
/// 当前为骨架：通行网格、占格、渲染资源清单等在准备层收口后填入。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedMap {
    /// 已冻结的静态地图。
    pub definition: MapDefinition,
}
