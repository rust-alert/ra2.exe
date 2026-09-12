//! 冻结地图定义契约：装载完成后的静态地图真相。
//!
//! `ra-map` loader 产出语义结构后迁入本契约；adaptor 绑定规则得到 [`PreparedMap`]。
//! 不含 `IniDocument`、文件路径、MIX/GPU 句柄或对局可变状态。
//!
//! # 与磁盘 / INI 的关系
//!
//! 本模块类型是**运行期最优形状**，不是地图文件或 Westwood INI 的存储镜像。
//! 字段布局、命名与嵌套可随时改为更利于引擎执行的形式（稳定 ID、稠密表、拆分索引等）。
//! 兼容原版内容的职责在 loader / adaptor：把文件格式**投影**进本契约，而不是把本契约钉死成文件 schema。

/// 冻结的完整静态地图（装载期产出，对局与绘制只读）。
///
/// 当前为骨架：字段随地图语义层收口逐步迁入，禁止在运行路径回查地图 INI。
/// 结构可演进；同名于 `ra-map::scripting` 的解析类型只是装载侧中间态，不必与本契约字段一一同构。
#[derive(Debug, Clone, PartialEq)]
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
    /// `[Lighting]` 普通环境光。
    pub lighting: MapLighting,
    /// `[Lighting]` Ion / 闪电风暴档。
    pub ion_lighting: MapLighting,
    /// `[Waypoints]` 格子锚点（编号已排序）。
    pub waypoints: Vec<MapWaypoint>,
    /// `[Terrain]` 静态地形物件。
    pub terrain_objects: Vec<MapTerrainObject>,
    /// `[Smudge]` 污迹占位。
    pub smudges: Vec<MapSmudge>,
    /// 预放实体（Structures / Units / Infantry / Aircraft）。
    pub entities: Vec<MapPlacedEntity>,
    /// `[IsoMapPack5]` 等距地形单元。
    pub cells: Vec<MapIsoCell>,
    /// `[OverlayPack]` / `[OverlayDataPack]` 覆盖层格。
    pub overlays: Vec<MapOverlayCell>,
    /// `[Houses]` 地图各方。
    pub houses: Vec<MapHouse>,
    /// `[Tags]` 触发器绑定标签。
    pub tags: Vec<MapTag>,
    /// `[Triggers]` 触发器定义。
    pub triggers: Vec<MapTrigger>,
    /// `[Events]`（与 trigger id 对齐）。
    pub events: Vec<MapEvent>,
    /// `[Actions]`（与 trigger id 对齐）。
    pub actions: Vec<MapAction>,
    /// `[CellTags]` 格子绑定的 Tag。
    pub cell_tags: Vec<MapCellTag>,
    /// `[TaskForces]` 编队成员表。
    pub task_forces: Vec<MapTaskForce>,
    /// `[ScriptTypes]` 脚本步骤表。
    pub script_types: Vec<MapScriptType>,
    /// `[TeamTypes]` 产队定义。
    pub team_types: Vec<MapTeamType>,
    /// `[AITriggerTypes]` AI 产队触发（字段子集）。
    pub ai_triggers: Vec<MapAiTrigger>,
    /// `[Preview] Size` 宽（缺节或无效为 0）。
    pub preview_width: u32,
    /// `[Preview] Size` 高（缺节或无效为 0）。
    pub preview_height: u32,
    /// `[Digest]` 编号键按序拼接的校验摘要（缺节为空）。
    pub digest: String,
    /// 装载期天气种类（剧院默认投影；粒子场由呈现层按此实例化）。
    pub weather: MapWeatherKind,
}

impl Default for MapDefinition {
    fn default() -> Self {
        Self {
            name: String::new(),
            size_width: 0,
            size_height: 0,
            local_size: MapLocalSize::default(),
            cell_side: 0,
            theater: String::new(),
            description_csf: String::new(),
            game_modes: Vec::new(),
            next_mission: String::new(),
            alternate_next_mission: String::new(),
            starting_credits: 0,
            lighting: MapLighting::default(),
            ion_lighting: MapLighting::ion_default(),
            waypoints: Vec::new(),
            terrain_objects: Vec::new(),
            smudges: Vec::new(),
            entities: Vec::new(),
            cells: Vec::new(),
            overlays: Vec::new(),
            houses: Vec::new(),
            tags: Vec::new(),
            triggers: Vec::new(),
            events: Vec::new(),
            actions: Vec::new(),
            cell_tags: Vec::new(),
            task_forces: Vec::new(),
            script_types: Vec::new(),
            team_types: Vec::new(),
            ai_triggers: Vec::new(),
            preview_width: 0,
            preview_height: 0,
            digest: String::new(),
            weather: MapWeatherKind::None,
        }
    }
}

/// 装载期天气种类（运行契约；不是粒子仿真状态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MapWeatherKind {
    /// 无氛围粒子。
    #[default]
    None,
    /// 飘雪。
    Snow,
    /// 薄雾。
    Fog,
    /// 雨丝。
    Rain,
}

/// 地图环境光档（`[Lighting]` / Ion 键）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapLighting {
    /// 环境亮度。
    pub ambient: f32,
    /// 红通道倍率。
    pub red: f32,
    /// 绿通道倍率。
    pub green: f32,
    /// 蓝通道倍率。
    pub blue: f32,
    /// 地面压暗项。
    pub ground: f32,
    /// 每级高度对 ambient 的增量。
    pub level: f32,
}

impl Default for MapLighting {
    fn default() -> Self {
        Self { ambient: 1.0, red: 1.0, green: 1.0, blue: 1.0, ground: 0.20, level: 0.032 }
    }
}

impl MapLighting {
    /// Ion / 闪电风暴零售缺省。
    pub const fn ion_default() -> Self {
        Self { ambient: 0.87, red: 0.30, green: 0.40, blue: 0.75, ground: 0.0, level: 0.0 }
    }
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

/// `[Smudge]` 污迹占位（运行契约；装载侧见 `ra-map::MapSmudge`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSmudge {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 污迹类型名（通常已大写）。
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
    /// 所属方名称（装载期一次解码为大写）。
    pub owner: crate::HouseName,
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

/// 地图一方（运行契约；规则绑定前仍可用名称字符串）。
///
/// 形状可改为稳定 house id 等；不保证与地图 INI 节字段同构。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapHouse {
    /// 节名（常为 `Player House` 等）。
    pub name: String,
    /// `Country=`（装载期一次解码为大写国家键）。
    pub country: crate::HouseName,
    /// `TechLevel=`。
    pub tech_level: i32,
    /// `Credits=`（地图单位常为百计资金）。
    pub credits: i32,
    /// `IQ=`。
    pub iq: i32,
    /// `Edge=`（装载期一次解码）。
    pub edge: crate::MapEdge,
    /// `PlayerControl=`。
    pub player_control: bool,
    /// `Color=`。
    pub color: String,
    /// `Allies=` 逗号列表。
    pub allies: Vec<String>,
}

/// Tag 绑定（运行契约；来自地图 `[Tags]` 语义，非 INI 行镜像）。
///
/// 可改为指向 trigger 的稳定 id / 索引；装载侧同名类型见 `ra-map` 解析层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTag {
    /// Tag id。
    pub id: String,
    /// 持久性：0 volatile / 1 semi / 2 persistent。
    pub persistence: u8,
    /// 编辑器名。
    pub name: String,
    /// 关联 Trigger id。
    pub trigger_id: String,
}

/// Trigger 定义（运行契约；来自地图 `[Triggers]` 语义，非 INI 行镜像）。
///
/// 可改为稠密表或稳定 id；装载侧同名类型见 `ra-map` 解析层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTrigger {
    /// Trigger id。
    pub id: String,
    /// 所属 house（装载期一次解码为大写）。
    pub house: crate::HouseName,
    /// 链接的另一 trigger（`<none>` 表示无）。
    pub linked: String,
    /// 编辑器名。
    pub name: String,
    /// `1` = 初始禁用。
    pub disabled: bool,
    /// Easy 难度启用。
    pub easy: bool,
    /// Normal 难度启用。
    pub normal: bool,
    /// Hard 难度启用。
    pub hard: bool,
}

/// 单条事件条件（运行契约；`kind_code` 为原版事件表整数码）。
///
/// 可改为枚举或稳定条件 id；装载侧见 `ra-map::MapEventCondition`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEventCondition {
    /// 原版事件类型码。
    pub kind_code: i32,
    /// 参数（通常 2 个；变长事件保留原文）。
    pub params: Vec<String>,
}

/// 与某 trigger 对齐的事件表（运行契约）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapEvent {
    /// Trigger id。
    pub id: String,
    /// 条件列表。
    pub conditions: Vec<MapEventCondition>,
}

/// 单条动作（运行契约；`kind_code` 为原版动作表整数码）。
///
/// 可改为枚举或稳定动作 id；装载侧见 `ra-map::MapActionCommand`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapActionCommand {
    /// 原版动作类型码。
    pub kind_code: i32,
    /// 七个参数槽（含航点字母等）。
    pub params: [String; 7],
}

/// 与某 trigger 对齐的动作表（运行契约）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAction {
    /// Trigger id。
    pub id: String,
    /// 动作列表。
    pub commands: Vec<MapActionCommand>,
}

/// 格子上的 Tag 绑定（运行契约；来自 `[CellTags]` 语义）。
///
/// 可改为按格索引的稠密表；装载侧同名类型见 `ra-map` 解析层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapCellTag {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// Tag id。
    pub tag_id: String,
}

/// TaskForce 成员槽（运行契约；规则绑定前仍用类型名字符串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForceEntry {
    /// 数量。
    pub count: u16,
    /// 类型 id。
    pub type_id: String,
}

/// TaskForce 编队（运行契约；来自 `[TaskForces]` 语义）。
///
/// 可改为稳定 type id / 稠密成员表；装载侧见 `ra-map::MapTaskForce`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForce {
    /// id。
    pub id: String,
    /// 名称。
    pub name: String,
    /// 成员（最多 6）。
    pub entries: Vec<MapTaskForceEntry>,
    /// `Group=`。
    pub group: i32,
}

/// Script 一步（运行契约；`action` / `argument` 为原版 ScriptTypes 整数码）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapScriptStep {
    /// 动作码（例如 `1` = 攻击航点，`3` = 移动）。
    pub action: i32,
    /// 参数（含义随 `action`）。
    pub argument: i32,
}

/// ScriptTypes 脚本（运行契约；来自 `[ScriptTypes]` 语义）。
///
/// 可改为稳定动作枚举；装载侧见 `ra-map::MapScriptType`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapScriptType {
    /// id。
    pub id: String,
    /// 名称。
    pub name: String,
    /// 步骤。
    pub steps: Vec<MapScriptStep>,
}

/// TeamType 产队（运行契约；来自 `[TeamTypes]` 语义子集）。
///
/// 可改为稳定 house / script / task_force id；装载侧见 `ra-map::MapTeamType`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTeamType {
    /// id。
    pub id: String,
    /// 名称。
    pub name: String,
    /// `House=`（装载期一次解码为大写）。
    pub house: crate::HouseName,
    /// `Script=`。
    pub script: String,
    /// `TaskForce=`。
    pub task_force: String,
    /// `Tag=`（可空）。
    pub tag: String,
    /// `Waypoint=`：产队航点编号；`<0` 表示未指定。
    pub waypoint: i32,
    /// `Max=`。
    pub max: i32,
    /// `Priority=`。
    pub priority: i32,
    /// `VeteranLevel=`。
    pub veteran_level: i32,
}

/// AI 触发（运行契约；来自 `[AITriggerTypes]` 语义子集）。
///
/// 可改为稳定 team / house id；装载侧见 `ra-map::MapAiTrigger`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapAiTrigger {
    /// 触发 id。
    pub id: String,
    /// 显示名。
    pub name: String,
    /// 关联 TeamType。
    pub team: String,
    /// 所属 House（装载期一次解码为大写）。
    pub owner_house: crate::HouseName,
    /// 科技等级门槛。
    pub tech_level: i32,
}

/// 行优先粗占格码（与 [`PreparedMap::occupancy`] 元素语义对齐）。
pub mod occupancy_kind {
    /// 空格。
    pub const EMPTY: u8 = 0;
    /// 建筑占地（锚点或 `Foundation` 展开）。
    pub const STRUCTURE: u8 = 1;
    /// 地形物件。
    pub const TERRAIN: u8 = 2;
    /// 污迹。
    pub const SMUDGE: u8 = 3;
}

/// 与 [`crate::RuntimeDefinitions`] 绑定后的可开战 / 可预览地图。
///
/// 当前为骨架：通行格与粗占格已可由装载侧灌入；渲染资源清单等仍待准备层收口。
/// 与 [`MapDefinition`] 一样，本类型是运行最优形状，可随时改，不绑定磁盘格式。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PreparedMap {
    /// 已冻结的静态地图。
    pub definition: MapDefinition,
    /// 通行表宽（格）。`passable` / `cell_heights` / `occupancy` 为空时为 0。
    pub pass_width: u32,
    /// 通行表高（格）。
    pub pass_height: u32,
    /// 行优先通行位：`1` 可走，`0` 封死。空表示尚未填充。
    pub passable: Vec<u8>,
    /// 行优先格高度档，与 [`Self::passable`] 同长或空。
    pub cell_heights: Vec<u8>,
    /// 行优先粗占格：见 [`occupancy_kind`]。空表示尚未填充。
    ///
    /// 绑定 `StructureDefinitions` 时可按 `Foundation=` 多格展开；裸骨架仅锚点 `1x1`。
    pub occupancy: Vec<u8>,
}
