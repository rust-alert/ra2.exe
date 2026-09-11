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
}

impl OverlayTypeRegistry {
    /// 由已解析的名称与可采标记构造（装载层填充）。
    pub fn from_entries(names: Vec<String>, harvestable: Vec<bool>) -> Self {
        debug_assert_eq!(names.len(), harvestable.len());
        Self { names, harvestable }
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
}
