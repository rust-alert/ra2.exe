//! 封闭引擎能力（定义期声明；tick 期靠组件存在与否，不扫描本枚举）。

/// 内容可声明的内置能力集合（adaptor 校验 / 生成模板用）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinCapability {
    /// 可移动。
    Mobile,
    /// 武器。
    Weapon,
    /// 炮塔。
    Turret,
    /// 建筑。
    Structure,
    /// 供电。
    PowerProducer,
    /// 耗电。
    PowerConsumer,
    /// 建造场。
    ConstructionYard,
    /// 生产设施。
    Producer,
    /// 采矿车。
    Harvester,
    /// 矿场。
    Refinery,
    /// 可部署。
    Deployable,
    /// 运输。
    Transport,
    /// 隐形。
    Cloakable,
    /// 可被心灵控制。
    MindControllable,
    /// 可修理。
    Repairable,
    /// 可出售。
    Sellable,
    /// 可占领。
    Capturable,
    /// 雷达。
    Radar,
    /// 超级武器。
    SuperWeapon,
}

/// 本份定义启用的能力开关集合。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CapabilitySet {
    /// 定义期能力标签（由条目汇总；非 tick 扫描表）。
    pub builtins: Vec<BuiltinCapability>,
}
