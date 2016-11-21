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

use std::fmt;
use std::ops::Deref;

use serde::Deserialize;

use super::ini_string::{deserialize_trim, deserialize_upper, parse_trim, parse_upper};
use super::{ColorName, HouseName, MapEdge, TechnoName, UiName};

/// 地图 `[Terrain]` 物件类型名（装载期大写，对齐 rules 地形节）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TerrainName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl TerrainName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for TerrainName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for TerrainName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TerrainName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for TerrainName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for TerrainName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for TerrainName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for TerrainName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<TerrainName> for str {
    fn eq(&self, other: &TerrainName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<TerrainName> for &str {
    fn eq(&self, other: &TerrainName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for TerrainName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `[Smudge]` 污迹类型名（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct SmudgeName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl SmudgeName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for SmudgeName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for SmudgeName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for SmudgeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for SmudgeName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for SmudgeName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for SmudgeName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for SmudgeName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<SmudgeName> for str {
    fn eq(&self, other: &SmudgeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<SmudgeName> for &str {
    fn eq(&self, other: &SmudgeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for SmudgeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `Script=` / `[ScriptTypes]` 引用名（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ScriptTypeName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl ScriptTypeName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for ScriptTypeName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for ScriptTypeName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for ScriptTypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for ScriptTypeName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for ScriptTypeName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for ScriptTypeName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for ScriptTypeName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<ScriptTypeName> for str {
    fn eq(&self, other: &ScriptTypeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<ScriptTypeName> for &str {
    fn eq(&self, other: &ScriptTypeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for ScriptTypeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `TaskForce=` / `[TaskForces]` 引用名（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TaskForceName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl TaskForceName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for TaskForceName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for TaskForceName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TaskForceName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for TaskForceName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for TaskForceName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for TaskForceName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for TaskForceName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<TaskForceName> for str {
    fn eq(&self, other: &TaskForceName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<TaskForceName> for &str {
    fn eq(&self, other: &TaskForceName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for TaskForceName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `Team=` / `[TeamTypes]` 引用名（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TeamTypeName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl TeamTypeName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for TeamTypeName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for TeamTypeName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TeamTypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for TeamTypeName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for TeamTypeName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for TeamTypeName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for TeamTypeName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<TeamTypeName> for str {
    fn eq(&self, other: &TeamTypeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<TeamTypeName> for &str {
    fn eq(&self, other: &TeamTypeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for TeamTypeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `[Triggers]` / Tag 关联 Trigger id（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TriggerName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl TriggerName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for TriggerName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for TriggerName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TriggerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for TriggerName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for TriggerName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for TriggerName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for TriggerName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<TriggerName> for str {
    fn eq(&self, other: &TriggerName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<TriggerName> for &str {
    fn eq(&self, other: &TriggerName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for TriggerName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `[Tags]` / CellTag / 实体 Tag id（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct TagName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl TagName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for TagName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for TagName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for TagName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for TagName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for TagName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for TagName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for TagName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<TagName> for str {
    fn eq(&self, other: &TagName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<TagName> for &str {
    fn eq(&self, other: &TagName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for TagName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `[AITriggerTypes]` 触发 id（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct AiTriggerName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl AiTriggerName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for AiTriggerName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for AiTriggerName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for AiTriggerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for AiTriggerName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for AiTriggerName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for AiTriggerName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for AiTriggerName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<AiTriggerName> for str {
    fn eq(&self, other: &AiTriggerName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<AiTriggerName> for &str {
    fn eq(&self, other: &AiTriggerName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for AiTriggerName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}


/// 地图 `[Basic] GameModes` / `missions.pkt` `GameMode` 标签（装载期大写）；空 = 未写。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct GameModeName {
    /// 规范化键（装载期大写）。
    pub name: String,
}

impl GameModeName {
    /// 修剪并规范为大写；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_upper(raw) }
    }

    /// 底层键文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for GameModeName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for GameModeName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for GameModeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for GameModeName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for GameModeName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for GameModeName {
    fn eq(&self, other: &str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<&str> for GameModeName {
    fn eq(&self, other: &&str) -> bool {
        self.name.eq_ignore_ascii_case(other.trim())
    }
}

impl PartialEq<GameModeName> for str {
    fn eq(&self, other: &GameModeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl PartialEq<GameModeName> for &str {
    fn eq(&self, other: &GameModeName) -> bool {
        other.name.eq_ignore_ascii_case(self.trim())
    }
}

impl<'de> Deserialize<'de> for GameModeName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_upper(deserializer)?,
        })
    }
}



/// 地图 / 战役 scenario 文件名（装载期只修剪，保留盘上大小写）。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MapFileName {
    /// 文件名文本。
    pub name: String,
}

impl MapFileName {
    /// 修剪空白；空串表示未配置。
    pub fn parse(raw: &str) -> Self {
        Self { name: parse_trim(raw) }
    }

    /// 底层文件名文本。
    pub fn as_str(&self) -> &str {
        &self.name
    }

    /// 是否未配置。
    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
    }
}

impl Deref for MapFileName {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.name
    }
}

impl AsRef<str> for MapFileName {
    fn as_ref(&self) -> &str {
        &self.name
    }
}

impl fmt::Display for MapFileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)
    }
}

impl From<&str> for MapFileName {
    fn from(value: &str) -> Self {
        Self::parse(value)
    }
}

impl From<String> for MapFileName {
    fn from(value: String) -> Self {
        Self::parse(&value)
    }
}

impl PartialEq<str> for MapFileName {
    fn eq(&self, other: &str) -> bool {
        self.name == other
    }
}

impl PartialEq<&str> for MapFileName {
    fn eq(&self, other: &&str) -> bool {
        self.name == *other
    }
}

impl PartialEq<MapFileName> for str {
    fn eq(&self, other: &MapFileName) -> bool {
        self == other.name.as_str()
    }
}

impl PartialEq<MapFileName> for &str {
    fn eq(&self, other: &MapFileName) -> bool {
        *self == other.name.as_str()
    }
}

impl<'de> Deserialize<'de> for MapFileName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        Ok(Self {
            name: deserialize_trim(deserializer)?,
        })
    }
}

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
    /// 剧院（装载期一次解码为枚举）。
    pub theater: crate::Theater,
    /// `[Basic] Description` CSF 键（装载期一次解码为大写；可空）。
    pub description_csf: UiName,
    /// `[Basic] GameModes` 标签（装载期一次解码为大写）。
    pub game_modes: Vec<GameModeName>,
    /// `[Basic] NextMission`（装载期只修剪，保留盘上大小写；可空）。
    pub next_mission: MapFileName,
    /// `[Basic] AlternateNextMission`（装载期只修剪，保留盘上大小写；可空）。
    pub alternate_next_mission: MapFileName,
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
            theater: crate::Theater::Temperate,
            description_csf: UiName::default(),
            game_modes: Vec::new(),
            next_mission: MapFileName::default(),
            alternate_next_mission: MapFileName::default(),
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
    /// 物件类型名（装载期一次解码为大写地形键）。
    pub name: TerrainName,
}

/// `[Smudge]` 污迹占位（运行契约；装载侧见 `ra-map::MapSmudge`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapSmudge {
    /// 格子 X。
    pub x: u16,
    /// 格子 Y。
    pub y: u16,
    /// 污迹类型名（装载期一次解码为大写污迹键）。
    pub name: SmudgeName,
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
    pub owner: HouseName,
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
    /// 绑定的 Tag id（装载期一次解码为大写 Tags 键；空表示无）。
    pub tag: TagName,
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
    pub country: HouseName,
    /// `TechLevel=`。
    pub tech_level: i32,
    /// `Credits=`（地图单位常为百计资金）。
    pub credits: i32,
    /// `IQ=`。
    pub iq: i32,
    /// `Edge=`（装载期一次解码）。
    pub edge: MapEdge,
    /// `PlayerControl=`。
    pub player_control: bool,
    /// `Color=`（装载期一次解码为大写方案名）。
    pub color: ColorName,
    /// `Allies=` 逗号列表（装载期一次解码为大写房屋键）。
    pub allies: Vec<HouseName>,
}

/// Tag 绑定（运行契约；来自地图 `[Tags]` 语义，非 INI 行镜像）。
///
/// 可改为指向 trigger 的稳定 id / 索引；装载侧同名类型见 `ra-map` 解析层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTag {
    /// Tag id（装载期一次解码为大写 Tags 键）。
    pub id: TagName,
    /// 持久性：0 volatile / 1 semi / 2 persistent。
    pub persistence: u8,
    /// 编辑器名。
    pub name: String,
    /// 关联 Trigger id（装载期一次解码为大写 Triggers 键）。
    pub trigger_id: TriggerName,
}

/// Trigger 定义（运行契约；来自地图 `[Triggers]` 语义，非 INI 行镜像）。
///
/// 可改为稠密表或稳定 id；装载侧同名类型见 `ra-map` 解析层。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTrigger {
    /// Trigger id（装载期一次解码为大写 Triggers 键）。
    pub id: TriggerName,
    /// 所属 house（装载期一次解码为大写）。
    pub house: HouseName,
    /// 链接的另一 trigger（装载期一次解码为大写；`<none>` / 空表示无）。
    pub linked: TriggerName,
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
    /// Trigger id（装载期一次解码为大写 Triggers 键）。
    pub id: TriggerName,
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
    /// Trigger id（装载期一次解码为大写 Triggers 键）。
    pub id: TriggerName,
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
    /// Tag id（装载期一次解码为大写 Tags 键）。
    pub tag_id: TagName,
}

/// TaskForce 成员槽（运行契约；规则绑定前仍用类型名字符串）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForceEntry {
    /// 数量。
    pub count: u16,
    /// 类型 id（装载期一次解码为大写 techno 键）。
    pub type_id: TechnoName,
}

/// TaskForce 编队（运行契约；来自 `[TaskForces]` 语义）。
///
/// 可改为稳定 type id / 稠密成员表；装载侧见 `ra-map::MapTaskForce`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapTaskForce {
    /// id（装载期一次解码为大写 TaskForces 键）。
    pub id: TaskForceName,
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
    /// id（装载期一次解码为大写 ScriptTypes 键）。
    pub id: ScriptTypeName,
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
    /// id（装载期一次解码为大写 TeamTypes 键）。
    pub id: TeamTypeName,
    /// 名称。
    pub name: String,
    /// `House=`（装载期一次解码为大写）。
    pub house: HouseName,
    /// `Script=`（装载期一次解码为大写 ScriptTypes 键）。
    pub script: ScriptTypeName,
    /// `TaskForce=`（装载期一次解码为大写 TaskForces 键）。
    pub task_force: TaskForceName,
    /// `Tag=`（装载期一次解码为大写 Tags 键；可空）。
    pub tag: TagName,
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
    /// 触发 id（装载期一次解码为大写 AITriggerTypes 键）。
    pub id: AiTriggerName,
    /// 显示名。
    pub name: String,
    /// 关联 TeamType（装载期一次解码为大写 TeamTypes 键）。
    pub team: TeamTypeName,
    /// 所属 House（装载期一次解码为大写）。
    pub owner_house: HouseName,
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
