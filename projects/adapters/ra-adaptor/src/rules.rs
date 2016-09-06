//! 按资源链装载 rules/art 与派生注册表。

use ra_assets::{
    ColorSchemes, CountryRegistry, EntryMergePolicy, IniDocument, IniMergePolicy, LayeredIniView, RulesGlobals,
    SuperWeaponTypeRegistry, TechnoTypeRegistry, WarheadRegistry, overlay_types_from_layered, terrain_spawners_from_layered,
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

/// 层叠 rules/art 文档装载（`documents[0]` 最底，末元素最顶；便于叠 MD / MP / mod）。
fn build_rules_system_layered(edition: GameEdition, rules_docs: &[IniDocument], art_docs: &[IniDocument]) -> RulesSystem {
    let policy = IniMergePolicy {
        default_entry: EntryMergePolicy::MergeSection,
    };
    let rules_view = LayeredIniView::new(rules_docs, &policy);
    let art_view = LayeredIniView::new(art_docs, &policy);
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

fn build_rules_system(edition: GameEdition, rules: &IniDocument, art: &IniDocument) -> RulesSystem {
    build_rules_system_layered(edition, std::slice::from_ref(rules), std::slice::from_ref(art))
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
    let art = art_ini.unwrap_or(b"");
    rules_system_from_layered_ini_bytes(edition, &[rules_ini], &[art])
}

/// 从自底向顶的多份 rules/art 字节构造装载期快照（MP / mod 覆盖入口）。
///
/// - `rules_layers` / `art_layers`：索引 0 为最底层，末元素覆盖其上
/// - 空 `art_layers` 视为无 art 文档
/// - techno 列表键合并策略见 [`crate::techno_section_field_overrides`]
pub fn rules_system_from_layered_ini_bytes(
    edition: GameEdition,
    rules_layers: &[&[u8]],
    art_layers: &[&[u8]],
) -> RaResult<RulesSystem> {
    if rules_layers.is_empty() {
        return Err(ra_types::RaError::Parse("rules layers must not be empty".into()));
    }
    let mut rules_docs = Vec::with_capacity(rules_layers.len());
    for (i, bytes) in rules_layers.iter().enumerate() {
        let doc = IniDocument::parse(bytes).map_err(|e| ra_types::RaError::Parse(format!("rules layer {i}: {e}")))?;
        rules_docs.push(doc);
    }
    let mut art_docs = Vec::with_capacity(art_layers.len().max(1));
    if art_layers.is_empty() {
        art_docs.push(IniDocument::default());
    } else {
        for (i, bytes) in art_layers.iter().enumerate() {
            let doc = if bytes.is_empty() {
                IniDocument::default()
            } else {
                IniDocument::parse(bytes).map_err(|e| ra_types::RaError::Parse(format!("art layer {i}: {e}")))?
            };
            art_docs.push(doc);
        }
    }
    Ok(build_rules_system_layered(edition, &rules_docs, &art_docs))
}
