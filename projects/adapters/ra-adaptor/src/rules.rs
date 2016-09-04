//! 按资源链装载 rules/art 与派生注册表。

use ra_assets::{
    ColorSchemes, CountryRegistry, IniDocument, IniMergePolicy, LayeredIniView, RulesGlobals, SuperWeaponTypeRegistry, TechnoTypeRegistry,
    WarheadRegistry, overlay_types_from_layered, terrain_spawners_from_layered,
};
use ra_types::{AssetSource, GameEdition, OverlayTypeRegistry, RaResult, TerrainSpawnerDefinitions};

use crate::ResourceChain;
use crate::rules_schema::techno_section_field_overrides;

/// 一局装载用的规则快照（装载期内容模型；不再长期持有 `IniDocument`）。
#[derive(Debug, Clone)]
pub struct RulesSystem {
    /// 规则来源对应的 `GameEdition`。
    pub edition: GameEdition,
    /// `[General]` / 对话 / 语音等装载期全局字段。
    pub globals: RulesGlobals,
    /// 从 rules 派生的 overlay 类型注册表。
    pub overlay_types: OverlayTypeRegistry,
    /// 从 rules 派生的动画产矿地形表。
    pub terrain_spawners: TerrainSpawnerDefinitions,
    /// 从 rules 派生的配色方案表。
    pub color_schemes: ColorSchemes,
    /// 从 rules 派生的国家 / 势力表。
    pub countries: CountryRegistry,
    /// 从 rules 派生的 techno 类型注册表。
    pub techno_types: TechnoTypeRegistry,
    /// 从 techno 主武器引用的弹头表。
    pub warheads: WarheadRegistry,
    /// 从 `[SuperWeaponTypes]` 派生的超武类型表。
    pub super_weapons: SuperWeaponTypeRegistry,
}

fn build_rules_system(edition: GameEdition, rules: &IniDocument, art: &IniDocument) -> RulesSystem {
    // 当前链仍可能是单份 rules/art；经 LayeredIniView 统一入口，便于后续叠 MP / mod 层。
    let policy = IniMergePolicy::last_wins();
    let rules_view = LayeredIniView::new(std::slice::from_ref(rules), &policy);
    let art_view = LayeredIniView::new(std::slice::from_ref(art), &policy);
    let techno_overrides = techno_section_field_overrides();

    let globals = RulesGlobals::from_layered(rules_view);
    let overlay_types = overlay_types_from_layered(rules_view);
    let terrain_spawners = terrain_spawners_from_layered(rules_view);
    let mut color_schemes = ColorSchemes::from_layered(rules_view);
    let countries = CountryRegistry::from_layered(rules_view);
    {
        let mut house_ids: Vec<&str> = countries.countries().iter().map(|c| c.id.as_str()).collect();
        house_ids.extend(["Neutral", "Special", "Civilian"]);
        color_schemes.bind_houses_from_layered(rules_view, house_ids);
        color_schemes.bind_tiberium_display_from_layered(rules_view);
    }
    let techno_types = {
        let mut techno_types = TechnoTypeRegistry::from_layered_with_overrides(rules_view, Some(&techno_overrides));
        techno_types.apply_art_geometry_layered(art_view);
        techno_types
    };
    let warheads = WarheadRegistry::from_names_layered(
        rules_view,
        techno_types
            .iter()
            .flat_map(|t| [t.warhead.as_str(), t.secondary_warhead.as_str()]),
    );
    let super_weapons = SuperWeaponTypeRegistry::from_layered(rules_view);
    RulesSystem {
        edition,
        globals,
        overlay_types,
        terrain_spawners,
        color_schemes,
        countries,
        techno_types,
        warheads,
        super_weapons,
    }
}

/// 用显式 `ResourceChain` 加载（适配组合装配后的入口）。
pub fn load_rules_chain(source: &dyn AssetSource, chain: &ResourceChain) -> RaResult<RulesSystem> {
    let rules_bytes = source.read(chain.rules_ini)?;
    let rules = IniDocument::parse(&rules_bytes)
        .map_err(|e| ra_types::RaError::Parse(format!("{} ({} bytes): {e}", chain.rules_ini, rules_bytes.len())))?;
    let art_bytes = source.read(chain.art_ini)?;
    let art =
        IniDocument::parse(&art_bytes).map_err(|e| ra_types::RaError::Parse(format!("{} ({} bytes): {e}", chain.art_ini, art_bytes.len())))?;
    Ok(build_rules_system(chain.edition, &rules, &art))
}

/// 按互斥 `GameEdition` 取默认资源表再加载（兼容旧调用）。
pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesSystem> {
    load_rules_chain(source, &ResourceChain::for_edition(edition))
}

/// 从内联 rules/art 字节构造装载期快照（无资源树夹具 / 测试）。
///
/// 与 [`load_rules_chain`] 同一套 `LayeredIniView` 注册表路径；`art_ini` 缺省时用空文档。
pub fn rules_system_from_ini_bytes(
    edition: GameEdition,
    rules_ini: &[u8],
    art_ini: Option<&[u8]>,
) -> RaResult<RulesSystem> {
    let rules = IniDocument::parse(rules_ini).map_err(|e| ra_types::RaError::Parse(format!("rules.ini: {e}")))?;
    let art = match art_ini {
        Some(bytes) => IniDocument::parse(bytes).map_err(|e| ra_types::RaError::Parse(format!("art.ini: {e}")))?,
        None => IniDocument::default(),
    };
    Ok(build_rules_system(edition, &rules, &art))
}
