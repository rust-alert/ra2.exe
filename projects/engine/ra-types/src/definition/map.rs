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
    /// 预放实体（Structures / Units / Infantry / Aircraft）。
    pub entities: Vec<MapPlacedEntity>,
    /// `[IsoMapPack5]` 等距地形单元。
    pub cells: Vec<MapIsoCell>,
    /// `[OverlayPack]` / `[OverlayDataPack]` 覆盖层格。
    pub overlays: Vec<MapOverlayCell>,
    /// `[Houses]` 地图各方。
    pub houses: Vec<MapHouse>,
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

/// 预放实体类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapPlacedEntityKind {
    /// 建筑。
    Structure,
    /// 载具。
    Unit,
    /// 步兵。
    Infantry,
    /// 飞行器。
    Aircraft,
}

/// 场景预放实体（装载期快照；规则绑定前仍用类型名字符串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapPlacedEntity {
    /// 放置类别。
    pub kind: MapPlacedEntityKind,
    /// 所属方名称。
    pub owner: String,
    /// 类型 id（通常已大写）。
    pub type_id: String,
    /// 0..=256；原版常写 256 表示满血。
    pub health: u16,
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 朝向。
    pub facing: u8,
    /// 步兵子格 0..=4；其它为 0。
    pub sub_cell: u8,
    /// 初始任务（如 `Guard`）；空表示未指定。
    pub mission: String,
    /// 绑定的 Tag id；空表示无。
    pub tag: String,
}

/// 等距地形单元（自 IsoMapPack 解码）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapIsoCell {
    /// 格子 X。
    pub x: i16,
    /// 格子 Y。
    pub y: i16,
    /// 全局砖块号（相对 tileset；`0xFFFF` 表示 Clear 占位，绘制时当 0）。
    pub tile_num: i32,
    /// TMP 子砖下标。
    pub sub_tile: u8,
    /// 高度档。
    pub z: u8,
    /// 原版标志字节。
    pub flags: u8,
}

/// 覆盖层格（自 OverlayPack / OverlayDataPack 解码）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MapOverlayCell {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 覆盖类型索引 id。
    pub overlay_id: u8,
    /// 来自 OverlayDataPack：矿密度 / 墙帧等。
    pub data: u8,
}

/// 地图一方（战役 / 遭遇均可出现；规则绑定前仍用名称字符串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHouse {
    /// 节名（常为 `Player House` 等）。
    pub name: String,
    /// `Country=`。
    pub country: String,
    /// `TechLevel=`。
    pub tech_level: i32,
    /// `Credits=`（地图单位常为百计资金）。
    pub credits: i32,
    /// `IQ=`。
    pub iq: i32,
    /// `Edge=`。
    pub edge: String,
    /// `PlayerControl=`。
    pub player_control: bool,
    /// `Color=`。
    pub color: String,
    /// `Allies=` 逗号列表。
    pub allies: Vec<String>,
}

/// 与 [`crate::RuntimeDefinitions`] 绑定后的可开战 / 可预览地图。
///
/// 当前为骨架：通行网格、占格、渲染资源清单等在准备层收口后填入。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PreparedMap {
    /// 已冻结的静态地图。
    pub definition: MapDefinition,
}
