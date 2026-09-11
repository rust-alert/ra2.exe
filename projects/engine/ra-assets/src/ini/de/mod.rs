//! `IniSection` / `LayeredSectionView` → Serde 反序列化。

mod error;
mod map;
mod scalar;

pub use error::IniDeError;

use serde::Deserialize;

use super::document::IniSection;
use super::merge::LayeredSectionView;
use map::SectionMapAccess;

/// 将单节反序列化为强类型结构（字段名与 INI 键拼写一致，可用 `serde(rename)`；重复键取最后一次）。
pub fn from_section<'de, T>(section: &'de IniSection) -> Result<T, IniDeError>
where
    T: Deserialize<'de>,
{
    let mut de = SectionDeserializer {
        access: Some(SectionMapAccess::new(section)),
    };
    T::deserialize(&mut de)
}

/// 将层叠节的有效字段视图反序列化为强类型（不先物化为业务 `HashMap`）。
pub fn from_layered_section<'de, T>(section: &'de LayeredSectionView<'de>) -> Result<T, IniDeError>
where
    T: Deserialize<'de>,
{
    let mut de = SectionDeserializer {
        access: Some(SectionMapAccess::from_layered(section)),
    };
    T::deserialize(&mut de)
}

struct SectionDeserializer<'a> {
    access: Option<SectionMapAccess<'a>>,
}

impl<'de> serde::Deserializer<'de> for &mut SectionDeserializer<'de> {
    type Error = IniDeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        let access = self.access.take().unwrap_or_else(SectionMapAccess::empty);
        visitor.visit_map(access)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf seq tuple tuple_struct enum identifier
    }
}
