//! 从 `[SuperWeaponTypes]` 解析超武类型注册表（节字段经 Serde 一次解码）。

use std::collections::HashMap;

use serde::Deserialize;

use crate::ini::{IniDocument, IniMergePolicy, LayeredIniView};
use ra_types::{ImageName, ProjectileName, SuperWeaponActionName, SuperWeaponKindName, UiName, WarheadName, WeaponName};

/// 超武类型（装载期资源侧记录）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperWeaponType {
    /// 类型键（大写）。
    pub id: String,
    /// `UIName=`（装载期一次解码）；空表示未写。
    pub ui_name: UiName,
    /// `Type=` 玩法类型名。
    pub kind: SuperWeaponKindName,
    /// `Action=` 动作名。
    pub action: SuperWeaponActionName,
    /// `RechargeTime`（分钟量级整型，缺省 0）。
    pub recharge_time: i32,
    /// `SidebarImage=`（装载期一次解码）；空表示未写。
    pub sidebar_image: ImageName,
    /// `Weapon` 名；空表示未写。
    pub weapon: WeaponName,
    /// `Weapon=` 节 `Damage`；未写或节缺失为 0。
    pub weapon_damage: u32,
    /// `Weapon=` 节 `Range`（格）；未写或节缺失为 0。
    pub weapon_range: u32,
    /// `Weapon=` 节 `ROF`（tick）；未写或节缺失为 0。
    pub weapon_rof: u32,
    /// `Weapon=` 节 `Warhead` 名；空表示未写。
    pub weapon_warhead: WarheadName,
    /// `Weapon=` 节 `Projectile` 名；空表示未写。
    pub weapon_projectile: ProjectileName,
}

/// 保序的超武类型表。
#[derive(Debug, Clone, Default)]
pub struct SuperWeaponTypeRegistry {
    items: Vec<SuperWeaponType>,
    by_id: HashMap<String, usize>,
}

impl SuperWeaponTypeRegistry {
    /// 扫描 `[SuperWeaponTypes]` 列表并解码各类型节。
    pub fn from_rules(rules: &IniDocument) -> Self {
        let policy = IniMergePolicy::last_wins();
        let docs = std::slice::from_ref(rules);
        Self::from_layered(LayeredIniView::new(docs, &policy))
    }

    /// 从层叠 rules 视图扫描列表并解码各类型节。
    pub fn from_layered(view: LayeredIniView<'_>) -> Self {
        let mut items = Vec::new();
        let mut by_id = HashMap::new();
        let Some(list) = view.section("SuperWeaponTypes")
        else {
            return Self { items, by_id };
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
            let Some(sw) = parse_super_weapon(view, &id_up)
            else {
                continue;
            };
            let idx = items.len();
            by_id.insert(id_up, idx);
            items.push(sw);
        }
        Self { items, by_id }
    }

    /// 按 id 查找（大小写不敏感）。
    pub fn get(&self, id: &str) -> Option<&SuperWeaponType> {
        let idx = *self.by_id.get(&id.to_ascii_uppercase())?;
        self.items.get(idx)
    }

    /// 列表顺序遍历。
    pub fn iter(&self) -> impl Iterator<Item = &SuperWeaponType> {
        self.items.iter()
    }

    /// 已解析类型数。
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// 是否为空表。
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[derive(Debug, Deserialize)]
struct SuperWeaponSectionFields {
    #[serde(rename = "UIName", default)]
    ui_name: UiName,
    #[serde(rename = "Type", default)]
    kind: SuperWeaponKindName,
    #[serde(rename = "Action", default)]
    action: SuperWeaponActionName,
    #[serde(rename = "RechargeTime")]
    recharge_time: Option<i32>,
    #[serde(rename = "SidebarImage", default)]
    sidebar_image: ImageName,
    #[serde(rename = "Weapon", default)]
    weapon: WeaponName,
}

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

fn resolve_weapon(view: LayeredIniView<'_>, weapon: &WeaponName) -> (u32, u32, u32, WarheadName, ProjectileName) {
    if weapon.is_empty() {
        return (0, 0, 0, WarheadName::default(), ProjectileName::default());
    }
    let Some(section) = view.section(weapon.as_str())
    else {
        return (0, 0, 0, WarheadName::default(), ProjectileName::default());
    };
    let Ok(w) = section.deserialize::<WeaponSectionFields>()
    else {
        return (0, 0, 0, WarheadName::default(), ProjectileName::default());
    };
    (
        w.damage.unwrap_or(0),
        w.range.unwrap_or(0),
        w.rof.unwrap_or(0),
        w.warhead,
        w.projectile,
    )
}

fn parse_super_weapon(view: LayeredIniView<'_>, id: &str) -> Option<SuperWeaponType> {
    let section = view.section(id)?;
    let fields: SuperWeaponSectionFields = section.deserialize().ok()?;
    let weapon = fields.weapon;
    let (weapon_damage, weapon_range, weapon_rof, weapon_warhead, weapon_projectile) = resolve_weapon(view, &weapon);
    Some(SuperWeaponType {
        id: id.to_string(),
        ui_name: fields.ui_name,
        kind: fields.kind,
        action: fields.action,
        recharge_time: fields.recharge_time.unwrap_or(0).max(0),
        sidebar_image: fields.sidebar_image,
        weapon,
        weapon_damage,
        weapon_range,
        weapon_rof,
        weapon_warhead,
        weapon_projectile,
    })
}
