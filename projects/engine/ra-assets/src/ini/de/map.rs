//! 节 → map / struct 字段访问。

use std::borrow::Cow;
use std::collections::HashMap;

use serde::de::{self, MapAccess, Visitor};

use super::IniDeError;
use super::scalar::ScalarDeserializer;
use crate::ini::document::IniSection;
use crate::ini::merge::LayeredSectionView;
use crate::ini::SourceSpan;

pub(super) struct SectionMapAccess<'a> {
    /// 节名原始拼写（诊断用；可空）。
    section: Option<&'a str>,
    /// 比较键 → 有效原文值（层叠 `AppendValues` 可能为拥有拼接串）。
    values: HashMap<String, Cow<'a, str>>,
    /// 比较键 → 原始键拼写（供 `serde(rename)` 对齐）。
    key_raw: HashMap<String, &'a str>,
    /// 比较键 → 值源位置（`AppendValues` 拼接时可能为空）。
    spans: HashMap<String, Option<SourceSpan>>,
    /// 仍待消费的比较键（策略决定的顺序）。
    keys: Vec<String>,
    index: usize,
}

impl<'a> SectionMapAccess<'a> {
    pub(super) fn new(section: &'a IniSection) -> Self {
        let mut values: HashMap<String, Cow<'a, str>> = HashMap::new();
        let mut key_raw: HashMap<String, &'a str> = HashMap::new();
        let mut spans: HashMap<String, Option<SourceSpan>> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for e in &section.entries {
            if !values.contains_key(&e.key_key) {
                order.push(e.key_key.clone());
            }
            values.insert(e.key_key.clone(), Cow::Borrowed(e.value_raw.as_str()));
            key_raw.insert(e.key_key.clone(), e.key_raw.as_str());
            spans.insert(e.key_key.clone(), e.span);
        }
        Self {
            section: Some(section.name_raw.as_str()),
            values,
            key_raw,
            spans,
            keys: order,
            index: 0,
        }
    }

    /// 从层叠节构造：键序与有效值由 [`LayeredSectionView`] 策略决定。
    pub(super) fn from_layered(section: &'a LayeredSectionView<'a>) -> Self {
        let mut values: HashMap<String, Cow<'a, str>> = HashMap::new();
        let mut key_raw: HashMap<String, &'a str> = HashMap::new();
        let mut spans: HashMap<String, Option<SourceSpan>> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for raw_key in section.keys() {
            let cmp = raw_key.to_ascii_uppercase();
            if values.contains_key(&cmp) {
                continue;
            }
            let Some(raw) = section.effective_raw(raw_key)
            else {
                continue;
            };
            let span = section.get(raw_key).and_then(|v| v.span);
            order.push(cmp.clone());
            values.insert(cmp.clone(), raw);
            key_raw.insert(cmp.clone(), raw_key);
            spans.insert(cmp, span);
        }
        let name = section.name_raw();
        Self {
            section: if name.is_empty() { None } else { Some(name) },
            values,
            key_raw,
            spans,
            keys: order,
            index: 0,
        }
    }

    pub(super) fn empty() -> Self {
        Self {
            section: None,
            values: HashMap::new(),
            key_raw: HashMap::new(),
            spans: HashMap::new(),
            keys: Vec::new(),
            index: 0,
        }
    }

    fn attach_section(&self, err: IniDeError) -> IniDeError {
        match self.section {
            Some(sec) => err.with_section(sec),
            None => err,
        }
    }
}

impl<'de> MapAccess<'de> for SectionMapAccess<'de> {
    type Error = IniDeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.index >= self.keys.len() {
            return Ok(None);
        }
        let cmp = self.keys[self.index].clone();
        self.index += 1;
        match self.key_raw.get(&cmp).copied() {
            Some(raw) => seed.deserialize(KeyDeserializer { key: raw }).map(Some),
            None => seed.deserialize(KeyDeserializerOwned { key: cmp }).map(Some),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let cmp = self.keys[self.index - 1].clone();
        let key = self.key_raw.get(&cmp).copied();
        let section = self.section;
        let span = self.spans.remove(&cmp).flatten();
        let raw = self
            .values
            .remove(&cmp)
            .ok_or_else(|| self.attach_section(IniDeError::custom(format!("内部错误：缺少键 {cmp}"))))?;
        seed.deserialize(ScalarDeserializer {
            raw,
            key,
            section,
            span,
        })
        .map_err(|e| self.attach_section(e))
    }
}

struct KeyDeserializer<'a> {
    key: &'a str,
}

impl<'de> de::Deserializer<'de> for KeyDeserializer<'de> {
    type Error = IniDeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.key)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.key)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key.to_string())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.key)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum ignored_any
    }
}

struct KeyDeserializerOwned {
    key: String,
}

impl<'de> de::Deserializer<'de> for KeyDeserializerOwned {
    type Error = IniDeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.key)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char bytes byte_buf
        option unit unit_struct newtype_struct seq tuple tuple_struct map struct enum ignored_any
    }
}
