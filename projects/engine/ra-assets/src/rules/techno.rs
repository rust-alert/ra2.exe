//! 从 rules 列表节解析 TechnoType 注册表（节字段经 Serde 一次解码）。

use std::collections::HashMap;

use serde::Deserialize;

use crate::ini::{FieldMergeOverrides, IniDocument, IniMergePolicy, LayeredIniView};
use ra_types::{
    BuildCat, Foundation, HouseAllowList, ImageName, PrerequisiteList, ProductionCategory, ProjectileName, SuperWeaponName, TechnoCategory,
    TechnoName, WarheadName, WeaponName, deserialize_optional_factory,
};

/// 步兵 / 载具 / 飞行器 / 建筑的共用类型字段。
#[derive(Debug, Clone, PartialEq)]
pub struct TechnoType {
    /// 类型 id（大写）。
    pub id: String,
    /// 所属大类。
    pub kind: TechnoKind,
    /// `Strength` 生命值。
    pub strength: u32,
    /// `Armor` 护甲种类（装载期一次解码）。
    pub armor: ra_types::ArmorKind,
    /// `Speed` 移动速度。
    pub speed: u32,
    /// `Sight` 视野。
    pub sight: u32,
    /// `Cost` 造价。
    pub cost: u32,
    /// `TechLevel`；缺省为 -1。
    pub tech_level: i32,
    /// `Owner` 所属阵营名单（装载期一次解码；空 = 不限）。
    pub owner: HouseAllowList,
    /// `Image` 资源名（装载期一次解码；缺省等于类型 id）。
    pub image: ImageName,
    /// `Category=`（装载期一次解码；空表示未写）。
    pub category: TechnoCategory,
    /// `Naval=yes`。
    pub naval: bool,
    /// `Agent=yes`（可渗透敌方建筑的间谍类单位）。
    pub agent: bool,
    /// `Engineer=yes`（可占领敌方可俘建筑）。
    pub engineer: bool,
    /// `Harvester=yes`（采矿车）。
    pub harvester: bool,
    /// 主武器名（`Primary`）；空表示未配置。
    pub primary: WeaponName,
    /// 主武器伤害（来自武器节 `Damage`）；0 表示未配置。
    pub damage: u32,
    /// 主武器射程（来自武器节 `Range`，格）；0 表示未配置。
    pub range: u32,
    /// 射速间隔（tick）；优先武器节 `ROF`，否则类型节；0 表示未配置。
    pub rof: u32,
    /// 主武器弹头名（武器节 `Warhead`）；空表示未配置。
    pub warhead: WarheadName,
    /// 主武器抛射体名（武器节 `Projectile`）；空表示未配置。
    pub projectile: ProjectileName,
    /// 副武器名（`Secondary`）；空表示未配置。
    pub secondary: WeaponName,
    /// 副武器伤害（来自武器节 `Damage`）；0 表示未配置。
    pub secondary_damage: u32,
    /// 副武器射程（来自武器节 `Range`，格）；0 表示未配置。
    pub secondary_range: u32,
    /// 副武器射速间隔（tick）；0 表示未配置。
    pub secondary_rof: u32,
    /// 副武器弹头名；空表示未配置。
    pub secondary_warhead: WarheadName,
    /// 副武器抛射体名；空表示未配置。
    pub secondary_projectile: ProjectileName,
    /// `Prerequisite`（装载期一次解码）。
    pub prerequisite: PrerequisiteList,
    /// `PrerequisiteOverride`（装载期一次解码）。
    pub prerequisite_override: PrerequisiteList,
    /// `RequiredHouses`（装载期一次解码；空 = 不限制）。
    pub required_houses: HouseAllowList,
    /// `ForbiddenHouses`（装载期一次解码；空 = 不禁止）。
    pub forbidden_houses: HouseAllowList,
    /// `BuildLimit`；`0` 表示不限。
    pub build_limit: i32,
    /// `BuildTime`；`0` 表示缺省。
    pub build_time: u32,
    /// `RequiresStolenAlliedTech`。
    pub requires_stolen_allied_tech: bool,
    /// `RequiresStolenSovietTech`。
    pub requires_stolen_soviet_tech: bool,
    /// `RequiresStolenThirdTech`。
    pub requires_stolen_third_tech: bool,
    /// `PixelSelectionBracketDelta`。
    pub pixel_selection_bracket_delta: i32,
    /// `DeploysInto` 目标类型名；空表示无。
    pub deploys_into: TechnoName,
    /// `Power` 原始值（正产电、负耗电）。
    pub power: i32,
    /// `Powered`；缺省时由耗电推导。
    pub powered: Option<bool>,
    /// `ConstructionYard`。
    pub construction_yard: bool,
    /// `Refinery`。
    pub refinery: bool,
    /// `Radar`。
    pub radar: bool,
    /// `BuildCat`（装载期一次解码）。
    pub build_cat: BuildCat,
    /// `Capturable`。
    pub capturable: bool,
    /// `Factory` 生产类别（装载期一次解码；`None` = 非工厂）。
    pub factory: Option<ProductionCategory>,
    /// `SuperWeapon` 名；空表示无。
    pub super_weapon: SuperWeaponName,
    /// `Foundation`（优先 art，否则 rules；装载期一次解码）。
    pub foundation: Foundation,
    /// `Height`（优先 art，否则 rules）；`None` 表示未写。
    pub height: Option<u16>,
    /// `LightIntensity`；缺省 0。
    pub light_intensity: f32,
    /// `LightVisibility`；缺省 5000。
    pub light_visibility: i32,
    /// `LightRedTint`；缺省 1。
    pub light_red: f32,
    /// `LightGreenTint`；缺省 1。
    pub light_green: f32,
    /// `LightBlueTint`；缺省 1。
    pub light_blue: f32,
}

/// Techno 大类，对应 rules 列表节。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TechnoKind {
    /// `[InfantryTypes]`
    Infantry,
    /// `[VehicleTypes]`
    Vehicle,
    /// `[AircraftTypes]`
    Aircraft,
    /// `[BuildingTypes]`
    Building,
}

/// `type_id` → 解析后的 TechnoType。
#[derive(Debug, Clone, Default)]
pub struct TechnoTypeRegistry {
    by_id: HashMap<String, TechnoType>,
}

impl TechnoTypeRegistry {
    /// 扫描 `[InfantryTypes]` / `[VehicleTypes]` / `[AircraftTypes]` / `[BuildingTypes]`。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        Self::from_layered(LayeredIniView::new(docs, &policy))
    }

    /// 从层叠 rules 视图扫描列表节并解码各类型（字段按视图默认策略合并）。
    pub fn from_layered(view: LayeredIniView<'_>) -> Self {
        Self::from_layered_with_overrides(view, None)
    }

    /// 从层叠 rules 视图扫描并解码；`overrides` 由 adaptor schema 声明列表等字段的合并策略。
    pub fn from_layered_with_overrides(view: LayeredIniView<'_>, overrides: Option<&FieldMergeOverrides>) -> Self {
        let mut by_id = HashMap::new();
        for (section, kind) in [
            ("InfantryTypes", TechnoKind::Infantry),
            ("VehicleTypes", TechnoKind::Vehicle),
            ("AircraftTypes", TechnoKind::Aircraft),
            ("BuildingTypes", TechnoKind::Building),
        ] {
            let Some(list) = view.section(section)
            else {
                continue;
            };
            for key in list.keys() {
                let Some(name_val) = list.get(key)
                else {
                    continue;
                };
                let id = name_val.trimmed().raw;
                if id.is_empty() {
                    continue;
                }
                let id_up = id.to_ascii_uppercase();
                if by_id.contains_key(&id_up) {
                    continue;
                }
                if let Some(tt) = parse_techno(view, &id_up, kind, overrides) {
                    by_id.insert(id_up, tt);
                }
            }
        }
        Self { by_id }
    }

    /// 按 id 查找（大小写不敏感）。
    pub fn get(&self, id: &str) -> Option<&TechnoType> {
        self.by_id.get(&id.to_ascii_uppercase())
    }

    /// 已解析类型总数。
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// 是否为空表。
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }

    /// 统计某一大类的数量。
    pub fn count_kind(&self, kind: TechnoKind) -> usize {
        self.by_id.values().filter(|t| t.kind == kind).count()
    }

    /// 遍历已解析类型。
    pub fn iter(&self) -> impl Iterator<Item = &TechnoType> {
        self.by_id.values()
    }

    /// 用 art（含 `Image=` 跳转）覆盖建筑的 `Foundation` / `Height`，缺键保留 rules。
    pub fn apply_art_geometry(&mut self, art: &IniDocument) {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(art);
        self.apply_art_geometry_layered(LayeredIniView::new(docs, &policy));
    }

    /// 用层叠 art 覆盖建筑几何字段。
    pub fn apply_art_geometry_layered(&mut self, art: LayeredIniView<'_>) {
        for tt in self.by_id.values_mut() {
            if tt.kind != TechnoKind::Building {
                continue;
            }
            if let Some(v) = art_geometry_string(art, &tt.id, "Foundation") {
                tt.foundation = Foundation::parse(&v);
            }
            if let Some(v) = art_geometry_string(art, &tt.id, "Height") {
                if let Ok(h) = v.parse::<i32>() {
                    tt.height = Some(h.max(1) as u16);
                }
            }
        }
    }
}

/// 类型节字段（一次 Serde 解码；缺省与归一化在组装 `TechnoType` 时完成）。
#[derive(Debug, Deserialize)]
struct TechnoSectionFields {
    #[serde(rename = "Strength")]
    strength: Option<u32>,
    #[serde(rename = "Armor", default)]
    armor: ra_types::ArmorKind,
    #[serde(rename = "Speed")]
    speed: Option<u32>,
    #[serde(rename = "Sight")]
    sight: Option<u32>,
    #[serde(rename = "Cost")]
    cost: Option<u32>,
    #[serde(rename = "TechLevel")]
    tech_level: Option<i32>,
    #[serde(rename = "Owner", default)]
    owner: HouseAllowList,
    #[serde(rename = "Image", default)]
    image: ImageName,
    #[serde(rename = "Category", default)]
    category: TechnoCategory,
    #[serde(rename = "Naval")]
    naval: Option<bool>,
    #[serde(rename = "Agent")]
    agent: Option<bool>,
    #[serde(rename = "Engineer")]
    engineer: Option<bool>,
    #[serde(rename = "Harvester")]
    harvester: Option<bool>,
    #[serde(rename = "Primary", default)]
    primary: WeaponName,
    #[serde(rename = "Secondary", default)]
    secondary: WeaponName,
    #[serde(rename = "ROF")]
    rof: Option<u32>,
    #[serde(rename = "Prerequisite", default)]
    prerequisite: PrerequisiteList,
    #[serde(rename = "PrerequisiteOverride", default)]
    prerequisite_override: PrerequisiteList,
    #[serde(rename = "RequiredHouses", default)]
    required_houses: HouseAllowList,
    #[serde(rename = "ForbiddenHouses", default)]
    forbidden_houses: HouseAllowList,
    #[serde(rename = "BuildLimit")]
    build_limit: Option<i32>,
    #[serde(rename = "BuildTime")]
    build_time: Option<i32>,
    #[serde(rename = "RequiresStolenAlliedTech")]
    requires_stolen_allied_tech: Option<bool>,
    #[serde(rename = "RequiresStolenSovietTech")]
    requires_stolen_soviet_tech: Option<bool>,
    #[serde(rename = "RequiresStolenThirdTech")]
    requires_stolen_third_tech: Option<bool>,
    #[serde(rename = "PixelSelectionBracketDelta")]
    pixel_selection_bracket_delta: Option<i32>,
    #[serde(rename = "DeploysInto", default)]
    deploys_into: TechnoName,
    #[serde(rename = "Power")]
    power: Option<i32>,
    #[serde(rename = "Powered")]
    powered: Option<bool>,
    #[serde(rename = "ConstructionYard")]
    construction_yard: Option<bool>,
    #[serde(rename = "Refinery")]
    refinery: Option<bool>,
    #[serde(rename = "Radar")]
    radar: Option<bool>,
    #[serde(rename = "BuildCat", default)]
    build_cat: BuildCat,
    #[serde(rename = "Capturable")]
    capturable: Option<bool>,
    #[serde(rename = "Factory", default, deserialize_with = "deserialize_optional_factory")]
    factory: Option<ProductionCategory>,
    #[serde(rename = "SuperWeapon", default)]
    super_weapon: SuperWeaponName,
    #[serde(rename = "Foundation", default)]
    foundation: Foundation,
    #[serde(rename = "Height")]
    height: Option<i32>,
    #[serde(rename = "LightIntensity")]
    light_intensity: Option<f32>,
    #[serde(rename = "LightVisibility")]
    light_visibility: Option<i32>,
    #[serde(rename = "LightRedTint")]
    light_red: Option<f32>,
    #[serde(rename = "LightGreenTint")]
    light_green: Option<f32>,
    #[serde(rename = "LightBlueTint")]
    light_blue: Option<f32>,
}

/// 武器节字段。
#[derive(Debug, Deserialize)]
struct WeaponSectionFields {
    #[serde(rename = "Damage")]
    damage: Option<u32>,
    #[serde(rename = "Range")]
    range: Option<u32>,
    #[serde(rename = "ROF")]
    rof: Option<u32>,
    #[serde(rename = "Warhead", default)]
    warhead: WarheadName,
    #[serde(rename = "Projectile", default)]
    projectile: ProjectileName,
}

/// 装载期解析出的武器节字段包。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct ResolvedWeaponFields {
    damage: u32,
    range: u32,
    rof: u32,
    warhead: WarheadName,
    projectile: ProjectileName,
}

fn parse_techno(
    view: LayeredIniView<'_>,
    id: &str,
    kind: TechnoKind,
    overrides: Option<&FieldMergeOverrides>,
) -> Option<TechnoType> {
    let section = view.section_with_overrides(id, overrides)?;
    let fields: TechnoSectionFields = section.deserialize().ok()?;
    let primary = fields.primary;
    let secondary = fields.secondary;
    let techno_rof = fields.rof.unwrap_or(0);
    let primary_w = resolve_weapon(view, &primary, techno_rof);
    let secondary_w = resolve_weapon(view, &secondary, 0);
    let image = if fields.image.is_empty() {
        ImageName::parse(id)
    } else {
        fields.image
    };
    Some(TechnoType {
        id: id.to_string(),
        kind,
        strength: fields.strength.unwrap_or(1),
        armor: fields.armor,
        speed: fields.speed.unwrap_or(0),
        sight: fields.sight.unwrap_or(0),
        cost: fields.cost.unwrap_or(0),
        tech_level: fields.tech_level.unwrap_or(-1),
        owner: fields.owner,
        image,
        category: fields.category,
        naval: fields.naval.unwrap_or(false),
        agent: fields.agent.unwrap_or(false),
        engineer: fields.engineer.unwrap_or(false),
        harvester: fields.harvester.unwrap_or(false),
        primary,
        damage: primary_w.damage,
        range: primary_w.range,
        rof: primary_w.rof,
        warhead: primary_w.warhead,
        projectile: primary_w.projectile,
        secondary,
        secondary_damage: secondary_w.damage,
        secondary_range: secondary_w.range,
        secondary_rof: secondary_w.rof,
        secondary_warhead: secondary_w.warhead,
        secondary_projectile: secondary_w.projectile,
        prerequisite: fields.prerequisite,
        prerequisite_override: fields.prerequisite_override,
        required_houses: fields.required_houses,
        forbidden_houses: fields.forbidden_houses,
        build_limit: fields.build_limit.unwrap_or(0).max(0),
        build_time: fields.build_time.unwrap_or(0).max(0) as u32,
        requires_stolen_allied_tech: fields.requires_stolen_allied_tech.unwrap_or(false),
        requires_stolen_soviet_tech: fields.requires_stolen_soviet_tech.unwrap_or(false),
        requires_stolen_third_tech: fields.requires_stolen_third_tech.unwrap_or(false),
        pixel_selection_bracket_delta: fields.pixel_selection_bracket_delta.unwrap_or(0),
        deploys_into: fields.deploys_into,
        power: fields.power.unwrap_or(0),
        powered: fields.powered,
        construction_yard: fields.construction_yard.unwrap_or(false),
        refinery: fields.refinery.unwrap_or(false),
        radar: fields.radar.unwrap_or(false),
        build_cat: fields.build_cat,
        capturable: fields.capturable.unwrap_or(false),
        factory: fields.factory,
        super_weapon: fields.super_weapon,
        foundation: fields.foundation,
        height: fields.height.map(|h| h.max(1) as u16),
        light_intensity: fields.light_intensity.unwrap_or(0.0),
        light_visibility: fields.light_visibility.unwrap_or(5000),
        light_red: fields.light_red.unwrap_or(1.0),
        light_green: fields.light_green.unwrap_or(1.0),
        light_blue: fields.light_blue.unwrap_or(1.0),
    })
}

/// 先读 art 本节，再跟 `Image=` 指向的 art 节。
fn art_geometry_string(art: LayeredIniView<'_>, type_key: &str, key: &str) -> Option<String> {
    if let Some(v) = art.get(type_key, key) {
        let t = v.trimmed();
        if !t.raw.is_empty() {
            return Some(t.raw.to_string());
        }
    }
    let image = art.get(type_key, "Image")?.trimmed().raw.to_ascii_uppercase();
    if image.eq_ignore_ascii_case(type_key) {
        return None;
    }
    let v = art.get(&image, key)?.trimmed();
    if v.raw.is_empty() {
        None
    } else {
        Some(v.raw.to_string())
    }
}

/// 从武器节读取伤害 / 射程 / ROF / 弹头 / 抛射体；缺省时可用 `fallback_rof`（主武器可回退类型节 ROF）。
fn resolve_weapon(view: LayeredIniView<'_>, weapon: &WeaponName, fallback_rof: u32) -> ResolvedWeaponFields {
    if weapon.is_empty() {
        return ResolvedWeaponFields {
            rof: fallback_rof,
            ..ResolvedWeaponFields::default()
        };
    }
    let Some(section) = view.section(weapon.as_str())
    else {
        return ResolvedWeaponFields {
            rof: fallback_rof,
            ..ResolvedWeaponFields::default()
        };
    };
    let Ok(w) = section.deserialize::<WeaponSectionFields>()
    else {
        return ResolvedWeaponFields {
            rof: fallback_rof,
            ..ResolvedWeaponFields::default()
        };
    };
    let weapon_rof = w.rof.unwrap_or(0);
    let rof = if weapon_rof > 0 { weapon_rof } else { fallback_rof };
    ResolvedWeaponFields {
        damage: w.damage.unwrap_or(0),
        range: w.range.unwrap_or(0),
        rof,
        warhead: w.warhead,
        projectile: w.projectile,
    }
}
