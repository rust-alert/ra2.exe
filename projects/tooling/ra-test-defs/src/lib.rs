//! 测试夹具：内联 rules INI → [`RuntimeDefinitions`]。
//!
//! 装载仍走 adaptor 投影，供 `ra-engine` / `ra-testing` 等测试消费。
//! 不依赖 `ra-engine`，避免与引擎测试形成依赖环。

#![deny(missing_docs)]

use std::sync::Arc;

use ra_adaptor::runtime_definitions_from_ini_bytes;
use ra_types::{GameEdition, RuntimeDefinitions};

/// 内联 rules INI 字节投影为冻结定义（装载在 adaptor）。
///
/// 仅用于测试夹具；产品路径仍经资源链 / `RulesSystem` 装载。
pub fn defs_from_rules_ini(rules_ini: &[u8]) -> Arc<RuntimeDefinitions> {
    Arc::new(
        runtime_definitions_from_ini_bytes(GameEdition::Ra2, rules_ini, None).expect("测试 rules INI 必须可投影"),
    )
}

/// 带可选 art 层的内联投影（测试夹具）。
pub fn defs_from_rules_and_art_ini(rules_ini: &[u8], art_ini: Option<&[u8]>) -> Arc<RuntimeDefinitions> {
    Arc::new(
        runtime_definitions_from_ini_bytes(GameEdition::Ra2, rules_ini, art_ini).expect("测试 rules/art INI 必须可投影"),
    )
}
