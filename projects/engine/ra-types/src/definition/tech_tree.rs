//! 科技树相关冻结定义（前置组、偷取科技与默认科技上限）。

use std::collections::BTreeMap;

/// 渗透作战实验室后可获得的偷取科技类别（对齐 `RequiresStolen*Tech`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StolenTechKind {
    /// 盟军（`Side=GDI`）→ `RequiresStolenAlliedTech`。
    Allied,
    /// 苏军（`Side=Nod`）→ `RequiresStolenSovietTech`。
    Soviet,
    /// 第三势力（`Side=ThirdSide`）→ `RequiresStolenThirdTech`。
    Third,
}

impl StolenTechKind {
    /// 从国家 `Side=` 字符串映射；未知则 `None`。
    pub fn from_side(side: &str) -> Option<Self> {
        match side.trim().to_ascii_uppercase().as_str() {
            "GDI" => Some(Self::Allied),
            "NOD" => Some(Self::Soviet),
            "THIRDSIDE" | "THIRD" => Some(Self::Third),
            _ => None,
        }
    }
}

/// `[General]` 通用前置组：组内任一存活建筑即可满足对应 token。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PrerequisiteGroups {
    /// `PrerequisitePower` → token `POWER`。
    pub power: Vec<String>,
    /// `PrerequisiteFactory` → token `FACTORY`。
    pub factory: Vec<String>,
    /// `PrerequisiteBarracks` → token `BARRACKS`。
    pub barracks: Vec<String>,
    /// `PrerequisiteRadar` → token `RADAR`。
    pub radar: Vec<String>,
    /// `PrerequisiteTech` → token `TECH`。
    pub tech: Vec<String>,
    /// `PrerequisiteProc` → token `PROC`。
    pub proc: Vec<String>,
    /// `PrerequisiteProcAlternate`（并入 `PROC` 判定）。
    pub proc_alternate: Vec<String>,
}

impl PrerequisiteGroups {
    /// 按通用 token 名取类型键列表（大小写不敏感）。未知 token 返回空切片。
    pub fn types_for_token(&self, token: &str) -> &[String] {
        match token.trim().to_ascii_uppercase().as_str() {
            "POWER" => self.power.as_slice(),
            "FACTORY" => self.factory.as_slice(),
            "BARRACKS" => self.barracks.as_slice(),
            "RADAR" => self.radar.as_slice(),
            "TECH" => self.tech.as_slice(),
            "PROC" => self.proc.as_slice(),
            _ => &[],
        }
    }

    /// `PROC` 判定用的全部类型键（主列表 + alternate）。
    pub fn proc_all(&self) -> impl Iterator<Item = &str> {
        self.proc
            .iter()
            .chain(self.proc_alternate.iter())
            .map(String::as_str)
    }

    /// 类型键是否属于 `TECH` 通用组（作战实验室等）。
    pub fn is_tech_building(&self, type_key: &str) -> bool {
        let want = type_key.to_ascii_uppercase();
        self.tech.iter().any(|t| t.eq_ignore_ascii_case(&want))
    }
}

/// house id（大写）→ 渗透其科技建筑时授予的偷取科技类别。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HouseStolenTechMap {
    by_house: BTreeMap<String, StolenTechKind>,
}

impl HouseStolenTechMap {
    /// 插入一条映射。
    pub fn insert(&mut self, house: impl AsRef<str>, kind: StolenTechKind) {
        self.by_house.insert(house.as_ref().to_ascii_uppercase(), kind);
    }

    /// 按 house 查找。
    pub fn get(&self, house: &str) -> Option<StolenTechKind> {
        self.by_house.get(&house.to_ascii_uppercase()).copied()
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_house.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.by_house.is_empty()
    }
}
