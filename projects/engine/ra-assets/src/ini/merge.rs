//! 多层 `IniDocument` 的字段级有效值视图（无 edition 语义）。

use std::borrow::Cow;
use std::collections::HashSet;

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
    /// 列表语义：自底向顶追加各层同键值（逗号拼接后供一次类型解码）。
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

/// 带层来源的有效字段值（诊断用）。
#[derive(Debug, Clone, Copy)]
pub struct ResolvedIniValue<'a> {
    /// 字段值。
    pub value: IniValue<'a>,
    /// 来自 `LayeredIniView::documents` 的层下标（0 = 最底）。
    pub layer: usize,
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
        let mut layers: Vec<(usize, &'a IniSection)> = Vec::new();
        for (layer, doc) in self.documents.iter().enumerate() {
            if let Some(sec) = doc.section(&name_key) {
                layers.push((layer, sec));
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

    /// 各层出现过的节比较名（底层先出现者在前，顶层新节追加）。
    pub fn section_keys(&self) -> Vec<&'a str> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for doc in self.documents {
            for sec in &doc.sections {
                if seen.insert(sec.name_key.as_str()) {
                    out.push(sec.name_key.as_str());
                }
            }
        }
        out
    }
}

/// 同一逻辑节在多层中的叠合视图。
#[derive(Debug, Clone)]
pub struct LayeredSectionView<'a> {
    /// 从底到顶出现过该节名的各层：`(layer_index, section)`。
    layers: Vec<(usize, &'a IniSection)>,
    policy: EntryMergePolicy,
}

impl<'a> LayeredSectionView<'a> {
    /// 节名（取最顶层原始拼写；若只要比较名可用 `name_key`）。
    pub fn name_raw(&self) -> &'a str {
        self.layers.last().map(|(_, s)| s.name_raw.as_str()).unwrap_or("")
    }

    /// 比较用节名。
    pub fn name_key(&self) -> &'a str {
        self.layers.last().map(|(_, s)| s.name_key.as_str()).unwrap_or("")
    }

    /// 当前节合并策略。
    pub fn policy(&self) -> EntryMergePolicy {
        self.policy
    }

    /// 自底向顶收集同键在各层的取值（缺层跳过）。
    pub fn all_resolved(&self, key: &str) -> Vec<ResolvedIniValue<'a>> {
        let key_up = key.to_ascii_uppercase();
        let mut out = Vec::new();
        for &(layer, sec) in &self.layers {
            if let Some(value) = sec.value(&key_up) {
                out.push(ResolvedIniValue { value, layer });
            }
        }
        out
    }

    /// 按策略解析带来源层的有效值（`AppendValues` 取顶层出现值，完整列表见 [`Self::all_resolved`]）。
    pub fn resolved(&self, key: &str) -> Option<ResolvedIniValue<'a>> {
        let key_up = key.to_ascii_uppercase();
        match self.policy {
            EntryMergePolicy::FirstValue => {
                for &(layer, sec) in &self.layers {
                    if let Some(value) = sec.value(&key_up) {
                        return Some(ResolvedIniValue { value, layer });
                    }
                }
                None
            }
            EntryMergePolicy::ReplaceSection => {
                let &(layer, sec) = self.layers.last()?;
                sec.value(&key_up).map(|value| ResolvedIniValue { value, layer })
            }
            EntryMergePolicy::MergeSection
            | EntryMergePolicy::LastValue
            | EntryMergePolicy::AppendValues
            | EntryMergePolicy::IndexedValues
            | EntryMergePolicy::NumberedPack => {
                for &(layer, sec) in self.layers.iter().rev() {
                    if let Some(value) = sec.value(&key_up) {
                        return Some(ResolvedIniValue { value, layer });
                    }
                }
                None
            }
        }
    }

    /// 按策略取有效键值（借用视图；`AppendValues` 请用 [`Self::effective_raw`]）。
    pub fn get(&self, key: &str) -> Option<IniValue<'a>> {
        self.resolved(key).map(|r| r.value)
    }

    /// 供 Serde / 列表解码使用的有效原文：`AppendValues` 自底向顶逗号拼接。
    pub fn effective_raw(&self, key: &str) -> Option<Cow<'a, str>> {
        match self.policy {
            EntryMergePolicy::AppendValues => {
                let parts: Vec<&str> = self
                    .all_resolved(key)
                    .into_iter()
                    .map(|r| r.value.raw.trim())
                    .filter(|s| !s.is_empty())
                    .collect();
                if parts.is_empty() {
                    None
                } else if parts.len() == 1 {
                    Some(Cow::Borrowed(parts[0]))
                } else {
                    Some(Cow::Owned(parts.join(",")))
                }
            }
            _ => self.get(key).map(|v| Cow::Borrowed(v.raw)),
        }
    }

    /// 合并后可见的比较键集合（保序：底层先出现的键在前，顶层新键追加）。
    pub fn keys(&self) -> Vec<&'a str> {
        match self.policy {
            EntryMergePolicy::ReplaceSection => {
                let Some((_, top)) = self.layers.last()
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
                for (_, sec) in &self.layers {
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
                for (_, sec) in &self.layers {
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

    /// 将本层叠节一次性反序列化为强类型。
    pub fn deserialize<'de, T>(&'de self) -> Result<T, super::de::IniDeError>
    where
        T: serde::Deserialize<'de>,
    {
        super::de::from_layered_section(self)
    }
}
