//! INI 名称 newtype：装载期一次规范化，绑定阶段再换成稳定 ID。

use std::fmt;
use std::ops::Deref;

use serde::de::{self, Deserializer, Visitor};
use serde::Deserialize;

macro_rules! ini_name {
    ($(#[$meta:meta])* $name:ident, $expect:expr) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
        pub struct $name(String);

        impl $name {
            /// 修剪并规范为大写；空串表示未配置。
            pub fn parse(raw: &str) -> Self {
                Self(raw.trim().to_ascii_uppercase())
            }

            /// 底层键文本。
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// 是否未配置。
            pub fn is_empty(&self) -> bool {
                self.0.is_empty()
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::parse(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::parse(&value)
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for str {
            fn eq(&self, other: &$name) -> bool {
                self == other.0.as_str()
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0.as_str()
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct NameVisitor;

                impl<'de> Visitor<'de> for NameVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                        f.write_str($expect)
                    }

                    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::parse(v))
                    }

                    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::parse(&v))
                    }

                    fn visit_none<E>(self) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::default())
                    }

                    fn visit_unit<E>(self) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        Ok($name::default())
                    }
                }

                deserializer.deserialize_any(NameVisitor)
            }
        }
    };
}

ini_name!(
    /// 武器节名（`Primary` / `Secondary` / `Weapon=` 等）；空 = 未配置。
    WeaponName,
    "weapon section name"
);

ini_name!(
    /// 弹头节名（`Warhead=`）；空 = 未配置。
    WarheadName,
    "warhead section name"
);

ini_name!(
    /// 房屋 / 阵营名（`Owner=` / `RequiredHouses=` / `ForbiddenHouses=` 等）；空 = 未配置。
    HouseName,
    "house / country name"
);
