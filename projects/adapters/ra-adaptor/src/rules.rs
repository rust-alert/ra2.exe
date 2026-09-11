//! 按资源链装载 rules/art 与派生注册表。

use ra_assets::{
    ColorSchemes, CountryRegistry, IniDocument, IniMergePolicy, LayeredIniView, RulesGlobals, SuperWeaponTypeRegistry, TechnoTypeRegistry,
    WarheadRegistry, overlay_types_from_layered, terrain_spawners_from_layered,
};
use ra_types::{AssetSource, GameEdition, OverlayTypeRegistry, RaResult, TerrainSpawnerDefinitions};

use crate::ResourceChain;

/// 一局装载用的规则快照。
#[derive(Debug, Clone)]
pub struct RulesSystem {
    /// 规则来源对应的 `GameEdition`。
    pub edition: GameEdition,
    /// 解析后的 `rules` INI 文档。
    pub rules: IniDocument,
    /// 解析后的 `art` INI 文档。
    pub art: IniDocument,
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

/// 用显式 `ResourceChain` 加载（适配组合装配后的入口）。
pub fn load_rules_chain(source: &dyn AssetSource, chain: &ResourceChain) -> RaResult<RulesSystem> {
    let rules_bytes = source.read(chain.rules_ini)?;
    let rules = IniDocument::parse(&rules_bytes)
        .map_err(|e| ra_types::RaError::Parse(format!("{} ({} bytes): {e}", chain.rules_ini, rules_bytes.len())))?;
    let art_bytes = source.read(chain.art_ini)?;
    let art =
        IniDocument::parse(&art_bytes).map_err(|e| ra_types::RaError::Parse(format!("{} ({} bytes): {e}", chain.art_ini, art_bytes.len())))?;

    // 当前链仍是单份 rules/art；经 LayeredIniView 统一入口，便于后续叠 MP / mod 层。
    let policy = IniMergePolicy::last_wins();
    let rules_view = LayeredIniView::new(std::slice::from_ref(&rules), &policy);
    let art_view = LayeredIniView::new(std::slice::from_ref(&art), &policy);

    let globals = RulesGlobals::from_layered(rules_view);
    let overlay_types = overlay_types_from_layered(rules_view);
    let terrain_spawners = terrain_spawners_from_layered(rules_view);
    let mut color_schemes = ColorSchemes::from_layered(rules_view);
    let countries = CountryRegistry::from_layered(rules_view);
    {
        let mut house_ids: Vec<&str> = countries.countries().iter().map(|c| c.id.as_str()).collect();
        house_ids.extend(["Neutral", "Special", "Civilian"]);
        color_schemes.bind_houses_from_layered(rules_view, house_ids);
    }
    let techno_types = {
        let mut techno_types = TechnoTypeRegistry::from_layered(rules_view);
        techno_types.apply_art_geometry_layered(art_view);
        techno_types
    };
    let warheads = WarheadRegistry::from_names_layered(rules_view, techno_types.iter().map(|t| t.warhead.as_str()));
    let super_weapons = SuperWeaponTypeRegistry::from_layered(rules_view);
    Ok(RulesSystem {
        edition: chain.edition,
        rules,
        art,
        globals,
        overlay_types,
        terrain_spawners,
        color_schemes,
        countries,
        techno_types,
        warheads,
        super_weapons,
    })
}

/// 按互斥 `GameEdition` 取默认资源表再加载（兼容旧调用）。
pub fn load_rules(source: &dyn AssetSource, edition: GameEdition) -> RaResult<RulesSystem> {
    load_rules_chain(source, &ResourceChain::for_edition(edition))
}
