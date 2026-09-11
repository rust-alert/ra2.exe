//! Overlay 类型表（装载期冻结；id 按声明顺序）。

/// Overlay 类型注册表。
///
/// 内部 id 按 `[OverlayTypes]` **声明顺序**（值序列）编号，不按数字键留空洞。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OverlayTypeRegistry {
    /// `overlay_id` → 类型名（大写）；下标即 OverlayPack 字节。
    names: Vec<String>,
    /// 与 `names` 对齐：该 id 是否可采（矿/宝石）。
    harvestable: Vec<bool>,
    /// 与 `names` 对齐：`NoUseTileLandType` 时按 `Land=` 得到的通行覆盖；`None` 表示不改 TMP 封格。
    land_pass_override: Vec<Option<bool>>,
}

impl OverlayTypeRegistry {
    /// 由已解析的名称、可采标记与通行覆盖构造（装载层填充）。
    pub fn from_entries(names: Vec<String>, harvestable: Vec<bool>, land_pass_override: Vec<Option<bool>>) -> Self {
        debug_assert_eq!(names.len(), harvestable.len());
        debug_assert_eq!(names.len(), land_pass_override.len());
        Self { names, harvestable, land_pass_override }
    }

    /// 已登记的类型数量。
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// 是否没有任何类型。
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// 按 overlay id 取类型名。
    pub fn name(&self, id: u8) -> Option<&str> {
        self.names.get(usize::from(id)).map(String::as_str)
    }

    /// 该 overlay id 是否可采矿/宝石。
    pub fn is_harvestable(&self, id: u8) -> bool {
        self.harvestable.get(usize::from(id)).copied().unwrap_or(false)
    }

    /// `NoUseTileLandType` 覆盖下的目标通行性；无覆盖则 `None`。
    pub fn land_pass_override(&self, id: u8) -> Option<bool> {
        self.land_pass_override.get(usize::from(id)).copied().flatten()
    }
}
