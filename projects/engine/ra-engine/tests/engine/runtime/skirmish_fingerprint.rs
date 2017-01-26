//! 遭遇战开局指纹：规则字节必须真实可读，禁止空字节污染身份。

use crate::common::defs_from_rules_ini;
use ra_engine::open_skirmish_session;
use ra_map::MapInfo;
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

/// 引擎测试夹具用的 RA2 规则逻辑名（不经 adaptor `ResourceChain`）。
const RULES_INI: &str = "rules.ini";

fn minimal_defs() -> std::sync::Arc<ra_types::RuntimeDefinitions> {
    defs_from_rules_ini(b"[BuildingTypes]\n0=GACNST\n[GACNST]\nConstructionYard=yes\nStrength=1000\n")
}

fn tiny_map() -> MapInfo {
    let mut map = MapInfo::empty(GameEdition::Ra2, "fp");
    map.width = 8;
    map.height = 8;
    map
}

struct MissingRulesSource;
impl AssetSource for MissingRulesSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        Err(RaError::MissingFile(relative.to_string()))
    }
}

struct EmptyRulesSource;
impl AssetSource for EmptyRulesSource {
    fn read(&self, relative: &str) -> RaResult<Vec<u8>> {
        if relative.eq_ignore_ascii_case(RULES_INI) { Ok(Vec::new()) } else { Err(RaError::MissingFile(relative.to_string())) }
    }
}

#[test]
fn open_skirmish_rejects_missing_rules_for_fingerprint() {
    let err =
        open_skirmish_session(&MissingRulesSource, GameEdition::Ra2, RULES_INI, minimal_defs(), tiny_map(), "t".into(), (0, 0), None, &[], 0)
            .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(RULES_INI), "{msg}");
    assert!(msg.contains("指纹") || msg.contains("规则"), "{msg}");
}

#[test]
fn open_skirmish_rejects_empty_rules_bytes_for_fingerprint() {
    let err =
        open_skirmish_session(&EmptyRulesSource, GameEdition::Ra2, RULES_INI, minimal_defs(), tiny_map(), "t".into(), (0, 0), None, &[], 0)
            .unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("空"), "{msg}");
}
