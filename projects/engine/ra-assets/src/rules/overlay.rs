//! 从 `rules.ini` 的 `[OverlayTypes]` 建立 id → 名称表。

use crate::ini::IniDocument;

/// Overlay 类型注册表。
///
/// 内部 id 按 `[OverlayTypes]` **声明顺序**（值序列）编号，不按数字键留空洞。
/// 零售 `rules.ini` 常缺 `0=` / `40=` 等键；若按键号建表，矿/宝石会错位到桥/墙。
#[derive(Debug, Clone, Default)]
pub struct OverlayTypeRegistry {
    /// `overlay_id` → 类型名（大写）；下标即 OverlayPack 字节。
    names: Vec<String>,
}

impl OverlayTypeRegistry {
    /// 解析 `[OverlayTypes]`：按节内条目顺序赋 id `0..n`，忽略键的数字字面量。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let Some(section) = rules.section("OverlayTypes")
        else {
            return Self::default();
        };
        let mut names = Vec::new();
        for (_key, value) in section.pairs() {
            let name = value.trim();
            if name.is_empty() {
                continue;
            }
            names.push(name.to_ascii_uppercase());
        }
        Self { names }
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
}
