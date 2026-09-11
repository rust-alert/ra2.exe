//! 节 → map / struct 字段访问。

use std::collections::HashMap;

use serde::de::{self, MapAccess, Visitor};

use super::IniDeError;
use super::scalar::ScalarDeserializer;
use crate::ini::document::IniSection;

pub(super) struct SectionMapAccess<'a> {
    /// 比较键 → 最后一次出现的原文值。
    values: HashMap<String, &'a str>,
    /// 比较键 → 最后一次出现的原始键拼写（供 `serde(rename)` 对齐）。
    key_raw: HashMap<String, &'a str>,
    /// 仍待消费的比较键（首次出现顺序）。
    keys: Vec<String>,
    index: usize,
}

impl<'a> SectionMapAccess<'a> {
    pub(super) fn new(section: &'a IniSection) -> Self {
        let mut values: HashMap<String, &'a str> = HashMap::new();
        let mut key_raw: HashMap<String, &'a str> = HashMap::new();
        let mut order: Vec<String> = Vec::new();
        for e in &section.entries {
            if !values.contains_key(&e.key_key) {
                order.push(e.key_key.clone());
            }
            values.insert(e.key_key.clone(), e.value_raw.as_str());
            key_raw.insert(e.key_key.clone(), e.key_raw.as_str());
        }
        Self {
            values,
            key_raw,
            keys: order,
            index: 0,
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
        let raw = self
            .key_raw
            .get(&cmp)
            .copied()
            .unwrap_or(cmp.as_str())
            .to_string();
        seed.deserialize(KeyDeserializer { key: raw }).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let cmp = &self.keys[self.index - 1];
        let raw = self
            .values
            .get(cmp)
            .copied()
            .ok_or_else(|| IniDeError::custom(format!("内部错误：缺少键 {cmp}")))?;
        seed.deserialize(ScalarDeserializer {
            raw,
            key: Some(cmp.clone()),
        })
    }
}

struct KeyDeserializer {
    key: String,
}

impl<'de> de::Deserializer<'de> for KeyDeserializer {
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
