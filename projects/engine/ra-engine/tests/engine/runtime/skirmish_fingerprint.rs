//! 遭遇战开局指纹：规则字节必须真实可读，禁止空字节污染身份。
use std::sync::Arc;

use ra_adaptor::{ResourceChain, RulesSystem, build_runtime_definitions};
use ra_assets::{ColorSchemes, CountryRegistry, IniDocument, OverlayTypeRegistry, SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry};
use ra_engine::open_skirmish_session;
use ra_map::MapInfo;
use ra_types::{AssetSource, GameEdition, RaError, RaResult};

fn minimal_rules() -> RulesSystem {
    let rules = IniDocument::parse(b"[BuildingTypes]\n0=GACNST\n[GACNST]\nConstructionYard=yes\nStrength=1000\n").expect("测试 INI 必须有效");
    RulesSystem {
        edition: GameEdition::Ra2,
        rules: rules.clone(),
        art: IniDocument::default(),
        overlay_types: OverlayTypeRegistry::default(),
        color_schemes: ColorSchemes::default(),
        countries: CountryRegistry::default(),
        techno_types: TechnoTypeRegistry::from_rules(&rules),
        warheads: WarheadRegistry::default(),
        super_weapons: SuperWeaponTypeRegistry::default(),
    }
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
        let chain = ResourceChain::for_edition(GameEdition::Ra2);
        if relative.eq_ignore_ascii_case(chain.rules_ini) { Ok(Vec::new()) } else { Err(RaError::MissingFile(relative.to_string())) }
    }
}

#[test]
fn open_skirmish_rejects_missing_rules_for_fingerprint() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let err = open_skirmish_session(&MissingRulesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&minimal_rules())), tiny_map(), "t".into(), (0, 0), None, &[], 0).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(chain.rules_ini), "{msg}");
    assert!(msg.contains("指纹") || msg.contains("规则"), "{msg}");
}

#[test]
fn open_skirmish_rejects_empty_rules_bytes_for_fingerprint() {
    let chain = ResourceChain::for_edition(GameEdition::Ra2);
    let err = open_skirmish_session(&EmptyRulesSource, chain.edition, chain.rules_ini, Arc::new(build_runtime_definitions(&minimal_rules())), tiny_map(), "t".into(), (0, 0), None, &[], 0).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("空"), "{msg}");
}
