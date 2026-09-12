//! 装载期 INI 键字符串辅助（仅函数）。
//!
//! 各子系统的 `*Name` 类型内聚在各自模块；此处不存放任何 Name 类型。

use std::fmt;

use serde::de::{self, Deserializer, Visitor};

/// trim + ASCII 大写。
pub(super) fn parse_upper(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

/// 从标量字符串反序列化为大写键；缺省 / unit → 空串。
pub(super) fn deserialize_upper<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct UpperVisitor;

    impl<'de> Visitor<'de> for UpperVisitor {
        type Value = String;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("INI name string")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
            Ok(parse_upper(v))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<String, E> {
            Ok(parse_upper(&v))
        }

        fn visit_none<E: de::Error>(self) -> Result<String, E> {
            Ok(String::new())
        }

        fn visit_unit<E: de::Error>(self) -> Result<String, E> {
            Ok(String::new())
        }
    }

    deserializer.deserialize_any(UpperVisitor)
}

/// trim only（保留大小写；地图文件名等）。
pub(super) fn parse_trim(raw: &str) -> String {
    raw.trim().to_string()
}

/// 从标量字符串反序列化为 trim 文本；缺省 / unit → 空串。
pub(super) fn deserialize_trim<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct TrimVisitor;

    impl<'de> Visitor<'de> for TrimVisitor {
        type Value = String;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("trimmed file name string")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<String, E> {
            Ok(parse_trim(v))
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<String, E> {
            Ok(parse_trim(&v))
        }

        fn visit_none<E: de::Error>(self) -> Result<String, E> {
            Ok(String::new())
        }

        fn visit_unit<E: de::Error>(self) -> Result<String, E> {
            Ok(String::new())
        }
    }

    deserializer.deserialize_any(TrimVisitor)
}
