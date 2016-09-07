//! Westwood CSV → Serde。

mod error;
mod scalar;

pub use error::CsvDeError;

use serde::Deserialize;

use super::parse::parse_westwood_csv_line;
use super::row::CsvRow;
use scalar::ScalarDeserializer;

/// 将一行 Westwood CSV 反序列化为强类型（结构体按字段声明顺序对列）。
pub fn from_row<T>(raw: &str) -> Result<T, CsvDeError>
where
    T: for<'de> Deserialize<'de>,
{
    from_csv_row(&parse_westwood_csv_line(raw))
}

/// 将已切分的 [`CsvRow`] 反序列化为强类型。
pub fn from_csv_row<T>(row: &CsvRow) -> Result<T, CsvDeError>
where
    T: for<'de> Deserialize<'de>,
{
    let mut de = RowDeserializer { row };
    T::deserialize(&mut de)
}

struct RowDeserializer<'a> {
    row: &'a CsvRow,
}

impl<'de> serde::Deserializer<'de> for &mut RowDeserializer<'_> {
    type Error = CsvDeError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_seq(RowSeqAccess {
            fields: &self.row.fields,
            index: 0,
        })
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(self, _name: &'static str, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(RowStructAccess {
            schema: fields,
            values: &self.row.fields,
            index: 0,
            pending_value: None,
        })
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

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf map enum identifier
    }
}

struct RowSeqAccess<'a> {
    fields: &'a [super::row::CsvField],
    index: usize,
}

impl<'de> serde::de::SeqAccess<'de> for RowSeqAccess<'_> {
    type Error = CsvDeError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        match self.fields.get(self.index) {
            Some(field) => {
                let col = self.index;
                self.index += 1;
                seed.deserialize(ScalarDeserializer {
                    raw: field.value.clone(),
                    column: Some(col),
                    field: None,
                })
                .map(Some)
            }
            None => Ok(None),
        }
    }
}

struct RowStructAccess<'a> {
    schema: &'static [&'static str],
    values: &'a [super::row::CsvField],
    index: usize,
    pending_value: Option<&'a str>,
}

impl<'de> serde::de::MapAccess<'de> for RowStructAccess<'_> {
    type Error = CsvDeError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.index >= self.schema.len() {
            return Ok(None);
        }
        let name = self.schema[self.index];
        self.pending_value = self.values.get(self.index).map(|f| f.as_str());
        self.index += 1;
        seed.deserialize(serde::de::value::StrDeserializer::new(name)).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let col = self.index.saturating_sub(1);
        let field_name = self.schema.get(col).copied();
        let raw = self.pending_value.take().unwrap_or("");
        seed.deserialize(ScalarDeserializer {
            raw: raw.to_string(),
            column: Some(col),
            field: field_name.map(str::to_string),
        })
    }
}
