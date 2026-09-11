//! 多层 `IniDocument` 的字段级有效值视图（无 edition 语义）。

use crate::ini::document::{IniDocument, IniSection};
use crate::ini::value::IniValue;

/// 单键 / 单节合并策略（由 adaptor schema 选定，本模块只执行）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryMergePolicy {
    /// 后层覆盖前层（同键取最后一份文档中的最后一次写入）。
    LastValue,
    /// 前层优先（同键取最先一份文档中的最后一次写入）。
    FirstValue,
    /// 后层整节替换前层（后层若出现该节名则完全取代）。
    ReplaceSection,
    /// 节内按键合并：未写的键保留下层，已写的键按 `LastValue`。
    MergeSection,
    /// 列表语义：追加各层同键值（骨架：暂与 `LastValue` 相同，待 schema 声明后启用）。
    AppendValues,
    /// 编号索引列表：按索引替换（骨架：暂与 `LastValue` 相同）。
    IndexedValues,
    /// 编号 pack 串接（骨架：暂与 `LastValue` 相同）。
    NumberedPack,
}

/// 整份层叠视图的默认策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IniMergePolicy {
    /// 未单独声明时的键策略。
    pub default_entry: EntryMergePolicy,
}

impl Default for IniMergePolicy {
    fn default() -> Self {
        Self {
            default_entry: EntryMergePolicy::LastValue,
        }
    }
}

impl IniMergePolicy {
    /// 默认后层覆盖。
    pub const fn last_wins() -> Self {
        Self {
            default_entry: EntryMergePolicy::LastValue,
        }
    }
}

/// 多层文档的只读视图：`documents[0]` 为最底层，末元素为最顶层。
#[derive(Debug, Clone, Copy)]
pub struct LayeredIniView<'a> {
    /// 参与合并的文档，从底到顶。
    pub documents: &'a [IniDocument],
    /// 合并策略。
    pub policy: &'a IniMergePolicy,
}

impl<'a> LayeredIniView<'a> {
    /// 构造。
    pub fn new(documents: &'a [IniDocument], policy: &'a IniMergePolicy) -> Self {
        Self { documents, policy }
    }

    /// 按节名取层叠节视图；各层都无该节则 `None`。
    pub fn section(&self, name: &str) -> Option<LayeredSectionView<'a>> {
        let name_key = name.to_ascii_uppercase();
        let mut layers: Vec<&'a IniSection> = Vec::new();
        for doc in self.documents {
            if let Some(sec) = doc.section(&name_key) {
                layers.push(sec);
            }
        }
        if layers.is_empty() {
            return None;
        }
        Some(LayeredSectionView {
            layers,
            policy: self.policy.default_entry,
        })
    }

    /// 直接取有效字段值。
    pub fn get(&self, section: &str, key: &str) -> Option<IniValue<'a>> {
        self.section(section)?.get(key)
    }
}

/// 同一逻辑节在多层中的叠合视图。
#[derive(Debug, Clone)]
pub struct LayeredSectionView<'a> {
    /// 从底到顶出现过该节名的各层节。
    layers: Vec<&'a IniSection>,
    policy: EntryMergePolicy,
}

impl<'a> LayeredSectionView<'a> {
    /// 节名（取最顶层原始拼写；若只要比较名可用 `name_key`）。
    pub fn name_raw(&self) -> &'a str {
        self.layers.last().map(|s| s.name_raw.as_str()).unwrap_or("")
    }

    /// 比较用节名。
    pub fn name_key(&self) -> &'a str {
        self.layers.last().map(|s| s.name_key.as_str()).unwrap_or("")
    }

    /// 按策略取有效键值。
    pub fn get(&self, key: &str) -> Option<IniValue<'a>> {
        let key_up = key.to_ascii_uppercase();
        match self.policy {
            EntryMergePolicy::FirstValue => {
                for sec in &self.layers {
                    if let Some(v) = sec.value(&key_up) {
                        return Some(v);
                    }
                }
                None
            }
            EntryMergePolicy::ReplaceSection => {
                // 整节替换：只看最顶层有该节的那一份（layers 末元素）。
                self.layers.last().and_then(|sec| sec.value(&key_up))
            }
            EntryMergePolicy::MergeSection
            | EntryMergePolicy::LastValue
            | EntryMergePolicy::AppendValues
            | EntryMergePolicy::IndexedValues
            | EntryMergePolicy::NumberedPack => {
                for sec in self.layers.iter().rev() {
                    if let Some(v) = sec.value(&key_up) {
                        return Some(v);
                    }
                }
                None
            }
        }
    }

    /// 合并后可见的比较键集合（保序：底层先出现的键在前，顶层新键追加）。
    pub fn keys(&self) -> Vec<&'a str> {
        match self.policy {
            EntryMergePolicy::ReplaceSection => {
                let Some(top) = self.layers.last()
                else {
                    return Vec::new();
                };
                let mut out = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for e in &top.entries {
                    if seen.insert(e.key_key.as_str()) {
                        out.push(e.key_raw.as_str());
                    }
                }
                out
            }
            EntryMergePolicy::FirstValue => {
                let mut out = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for sec in &self.layers {
                    for e in &sec.entries {
                        if seen.insert(e.key_key.as_str()) {
                            out.push(e.key_raw.as_str());
                        }
                    }
                }
                out
            }
            EntryMergePolicy::MergeSection
            | EntryMergePolicy::LastValue
            | EntryMergePolicy::AppendValues
            | EntryMergePolicy::IndexedValues
            | EntryMergePolicy::NumberedPack => {
                let mut out = Vec::new();
                let mut seen = std::collections::HashSet::new();
                // 先扫底层定序，再让顶层新键追加；取值仍由 get 做 LastValue。
                for sec in &self.layers {
                    for e in &sec.entries {
                        if seen.insert(e.key_key.as_str()) {
                            out.push(e.key_raw.as_str());
                        }
                    }
                }
                out
            }
        }
    }
}
