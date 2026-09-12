//! 标量字段反序列化（文本 → bool / 整数 / 浮点 / 字符串 / 逗号序列）。

use std::borrow::Cow;

use serde::de::{self, IntoDeserializer, SeqAccess, Visitor};

use super::IniDeError;

/// 标量反序列化器：字段原文以 [`Cow`] 持有，借阅路径不额外 `String` 拷贝。
pub(super) struct ScalarDeserializer<'a> {
    pub raw: Cow<'a, str>,
    pub key: Option<&'a str>,
    pub section: Option<&'a str>,
    pub span: Option<crate::ini::SourceSpan>,
}

impl<'a> ScalarDeserializer<'a> {
    fn err(&self, msg: impl std::fmt::Display) -> IniDeError {
        let mut e = IniDeError::custom(msg);
        if let Some(sec) = self.section {
            e = e.with_section(sec);
        }
        if let Some(k) = self.key {
            e = e.with_key(k);
        }
        if let Some(span) = self.span {
            e = e.with_span(span);
        }
        e
    }

    fn trimmed(&self) -> &str {
        self.raw.trim()
    }

    fn parse_i64(&self) -> Result<i64, IniDeError> {
        let t = self.trimmed();
        t.parse().map_err(|_| self.err(format!("无法解析为整数: {t}")))
    }

    fn parse_u64(&self) -> Result<u64, IniDeError> {
        let t = self.trimmed();
        t.parse().map_err(|_| self.err(format!("无法解析为无符号整数: {t}")))
    }
}

impl<'de> de::Deserializer<'de> for ScalarDeserializer<'de> {
    type Error = IniDeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.raw {
            Cow::Borrowed(s) => visitor.visit_borrowed_str(s.trim()),
            Cow::Owned(s) => visitor.visit_str(s.trim()),
        }
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let t = self.trimmed();
        let v = match t.to_ascii_lowercase().as_str() {
            "yes" | "true" | "1" => true,
            "no" | "false" | "0" => false,
            _ => return Err(self.err(format!("无法解析为布尔: {t}"))),
        };
        visitor.visit_bool(v)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_i64()?)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_i64()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_i64()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_i64()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_u64()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let t = self.trimmed();
        let v: f32 = t.parse().map_err(|_| self.err(format!("无法解析为 f32: {t}")))?;
        visitor.visit_f32(v)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let t = self.trimmed();
        let v: f64 = t.parse().map_err(|_| self.err(format!("无法解析为 f64: {t}")))?;
        visitor.visit_f64(v)
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        let t = self.trimmed();
        let mut chars = t.chars();
        match (chars.next(), chars.next()) {
            (Some(c), None) => visitor.visit_char(c),
            _ => Err(self.err(format!("无法解析为 char: {t}"))),
        }
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.raw {
            Cow::Borrowed(s) => visitor.visit_borrowed_str(s.trim()),
            Cow::Owned(s) => visitor.visit_str(s.trim()),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_string(self.trimmed().to_string())
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.raw {
            Cow::Borrowed(s) => visitor.visit_borrowed_bytes(s.trim().as_bytes()),
            Cow::Owned(s) => visitor.visit_bytes(s.trim().as_bytes()),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_byte_buf(self.trimmed().as_bytes().to_vec())
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        if self.trimmed().is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.raw {
            Cow::Borrowed(s) => {
                let parts: Vec<&'de str> = s
                    .split(|c| c == ',' || c == ';')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .collect();
                visitor.visit_seq(CommaSepBorrowed {
                    parts: parts.into_iter(),
                    key: self.key,
                    section: self.section,
                    span: self.span,
                })
            }
            Cow::Owned(s) => {
                // `AppendValues` 等拥有串：子切片不能安全借出，按项拷贝（稀有路径）。
                let parts: Vec<String> = s
                    .split(|c| c == ',' || c == ';')
                    .map(str::trim)
                    .filter(|p| !p.is_empty())
                    .map(str::to_string)
                    .collect();
                visitor.visit_seq(CommaSepOwned {
                    parts: parts.into_iter(),
                    key: self.key,
                    section: self.section,
                    span: self.span,
                })
            }
        }
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.err("标量不能反序列化为 map"))
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        Err(self.err("标量不能反序列化为 struct"))
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_enum(self.trimmed().into_deserializer())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        match self.raw {
            Cow::Borrowed(s) => visitor.visit_borrowed_str(s.trim()),
            Cow::Owned(s) => visitor.visit_str(s.trim()),
        }
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }
}

/// 借阅路径逗号序列：只收集 `&str` 指针，不物化字段内容。
struct CommaSepBorrowed<'a> {
    parts: std::vec::IntoIter<&'a str>,
    key: Option<&'a str>,
    section: Option<&'a str>,
    span: Option<crate::ini::SourceSpan>,
}

impl<'de> SeqAccess<'de> for CommaSepBorrowed<'de> {
    type Error = IniDeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.parts.next() {
            Some(part) => seed
                .deserialize(ScalarDeserializer {
                    raw: Cow::Borrowed(part),
                    key: self.key,
                    section: self.section,
                    span: self.span,
                })
                .map(Some),
            None => Ok(None),
        }
    }
}

/// 拥有串上的逗号序列（`AppendValues` 拼接等稀有路径）。
struct CommaSepOwned<'a> {
    parts: std::vec::IntoIter<String>,
    key: Option<&'a str>,
    section: Option<&'a str>,
    span: Option<crate::ini::SourceSpan>,
}

impl<'de> SeqAccess<'de> for CommaSepOwned<'de> {
    type Error = IniDeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        match self.parts.next() {
            Some(part) => seed
                .deserialize(ScalarDeserializer {
                    raw: Cow::Owned(part),
                    key: self.key,
                    section: self.section,
                    span: self.span,
                })
                .map(Some),
            None => Ok(None),
        }
    }
}
