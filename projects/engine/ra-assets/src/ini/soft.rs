//! 软解析标量：非法文本回落 `None`，避免拖垮整节其它键。

use std::fmt;

use serde::{
    Deserializer,
    de::{self, Visitor},
};

/// 可选 `u32`（允许尾随 `%`）。
pub fn deserialize_opt_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptU32;
    impl<'de> Visitor<'de> for OptU32 {
        type Value = Option<u32>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("optional u32, optional % suffix")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(u32::try_from(v).ok())
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(u32::try_from(v).ok())
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            if !v.is_finite() || v < 0.0 {
                return Ok(None);
            }
            Ok(Some(v as u32))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let t = v.trim().trim_end_matches('%').trim();
            if t.is_empty() {
                return Ok(None);
            }
            Ok(t.parse::<u32>().ok().or_else(|| t.parse::<f64>().ok().filter(|f| f.is_finite() && *f >= 0.0).map(|f| f as u32)))
        }
    }
    deserializer.deserialize_any(OptU32)
}

/// 可选 `i32`（允许尾随 `%`）。
pub fn deserialize_opt_i32<'de, D>(deserializer: D) -> Result<Option<i32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptI32;
    impl<'de> Visitor<'de> for OptI32 {
        type Value = Option<i32>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("optional i32, optional % suffix")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(i32::try_from(v).ok())
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(i32::try_from(v).ok())
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            if !v.is_finite() {
                return Ok(None);
            }
            Ok(Some(v as i32))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let t = v.trim().trim_end_matches('%').trim();
            if t.is_empty() {
                return Ok(None);
            }
            Ok(t.parse::<i32>().ok().or_else(|| t.parse::<f64>().ok().filter(|f| f.is_finite()).map(|f| f as i32)))
        }
    }
    deserializer.deserialize_any(OptI32)
}

/// 可选 `f32`。
pub fn deserialize_opt_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptF32;
    impl<'de> Visitor<'de> for OptF32 {
        type Value = Option<f32>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("optional f32")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            if v.is_finite() { Ok(Some(v as f32)) } else { Ok(None) }
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as f32))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as f32))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let t = v.trim();
            if t.is_empty() {
                return Ok(None);
            }
            Ok(t.parse::<f32>().ok().filter(|f| f.is_finite()))
        }
    }
    deserializer.deserialize_any(OptF32)
}

/// 可选布尔（非法文本 → `None`）。
pub fn deserialize_opt_bool<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptBool;
    impl<'de> Visitor<'de> for OptBool {
        type Value = Option<bool>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("optional bool")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
            Ok(Some(v))
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let t = v.trim();
            if t.is_empty() {
                return Ok(None);
            }
            match t.to_ascii_lowercase().as_str() {
                "yes" | "true" | "1" | "on" => Ok(Some(true)),
                "no" | "false" | "0" | "off" => Ok(Some(false)),
                _ => Ok(None),
            }
        }
    }
    deserializer.deserialize_any(OptBool)
}

/// 可选 `f64`。
pub fn deserialize_opt_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptF64;
    impl<'de> Visitor<'de> for OptF64 {
        type Value = Option<f64>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("optional f64")
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }

        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(Some(v as f64))
        }

        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            if v.is_finite() { Ok(Some(v)) } else { Ok(None) }
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            let t = v.trim();
            if t.is_empty() {
                return Ok(None);
            }
            Ok(t.parse::<f64>().ok().filter(|f| f.is_finite()))
        }
    }
    deserializer.deserialize_any(OptF64)
}
