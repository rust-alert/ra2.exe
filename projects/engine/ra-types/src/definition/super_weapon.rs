//! 超级武器定义表（`[SuperWeaponTypes]`）。

use std::collections::BTreeMap;

use crate::id::TypeId;

/// 单条超级武器静态定义（adaptor 冻结；引擎只读查询）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperWeaponDefinition {
    /// 稳定类型编号。
    pub id: TypeId,
    /// 外部类型键（INI 节名，大写）。
    pub type_key: String,
    /// `UIName=` CSF 键（可空）。
    pub ui_name: String,
    /// `Type=` 玩法类型字面（如 `LightningStorm` / `Nuke`；大写；可空）。
    pub kind: String,
    /// `Action=` 动作字面（可空）。
    pub action: String,
    /// `RechargeTime=` 原版充能档（整数；`0` 表示缺省/未写）。
    pub recharge_time: i32,
    /// `SidebarImage=` 侧栏图标名（可空）。
    pub sidebar_image: String,
    /// `Weapon=` 关联武器键（可空）。
    pub weapon: String,
}

/// 超级武器定义表（按外部 type_key 查询）。
#[derive(Debug, Clone, Default)]
pub struct SuperWeaponDefinitions {
    by_key: BTreeMap<String, SuperWeaponDefinition>,
}

impl SuperWeaponDefinitions {
    /// 插入一条定义。
    pub fn insert(&mut self, def: SuperWeaponDefinition) {
        self.by_key.insert(def.type_key.clone(), def);
    }

    /// 按外部类型键查找（大小写不敏感）。
    pub fn get(&self, type_key: &str) -> Option<&SuperWeaponDefinition> {
        self.by_key.get(&type_key.to_ascii_uppercase())
    }

    /// 条目数。
    pub fn len(&self) -> usize {
        self.by_key.len()
    }

    /// 是否空表。
    pub fn is_empty(&self) -> bool {
        self.by_key.is_empty()
    }

    /// 遍历。
    pub fn iter(&self) -> impl Iterator<Item = &SuperWeaponDefinition> {
        self.by_key.values()
    }
}
