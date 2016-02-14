//! 零售地图 `[Events]` / `[Actions]` 类型码（竖切已接线子集）。
//!
//! 编号与原版 RA2/YR 触发表一致。未接线码落在 `Unknown`，由能力缺口与运行时诊断消费。

/// 地图 `[Events]` 条件类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapEventKind {
    /// 进入绑定 CellTag 的格子。
    EnteredBy,
    /// 绑定 Tag 的对象被摧毁（任一）。
    DestroyedByAnybody,
    /// 指定 house 全部陆上机动单位已摧毁。
    DestroyedUnitsAll,
    /// 指定 house 全部建筑已摧毁。
    DestroyedBuildingsAll,
    /// 指定 house 全部实体已摧毁。
    DestroyedAll,
    /// 触发所属 house 资金不少于阈值。
    CreditsExceed,
    /// 计时结束。
    TimeElapse,
    /// 指定 house 处于低电。
    LowPower,
    /// 绑定 Tag 的对象被摧毁（与 `DestroyedByAnybody` 同类，YR 常用）。
    DestroyedByAnything,
    /// 触发所属 house 资金少于阈值。
    CreditsBelow,
    /// 未接线的原版事件码。
    Unknown(i32),
}

impl MapEventKind {
    /// 由原版事件表整数码解析；未知码为 `Unknown`。
    pub fn from_code(value: i32) -> Self {
        match value {
            1 => Self::EnteredBy,
            7 => Self::DestroyedByAnybody,
            9 => Self::DestroyedUnitsAll,
            10 => Self::DestroyedBuildingsAll,
            11 => Self::DestroyedAll,
            12 => Self::CreditsExceed,
            13 => Self::TimeElapse,
            30 => Self::LowPower,
            48 => Self::DestroyedByAnything,
            52 => Self::CreditsBelow,
            other => Self::Unknown(other),
        }
    }

    /// 原版事件表整数码。
    pub const fn code(self) -> i32 {
        match self {
            Self::EnteredBy => 1,
            Self::DestroyedByAnybody => 7,
            Self::DestroyedUnitsAll => 9,
            Self::DestroyedBuildingsAll => 10,
            Self::DestroyedAll => 11,
            Self::CreditsExceed => 12,
            Self::TimeElapse => 13,
            Self::LowPower => 30,
            Self::DestroyedByAnything => 48,
            Self::CreditsBelow => 52,
            Self::Unknown(code) => code,
        }
    }

    /// 是否为竖切已接线事件。
    pub const fn is_supported(self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}

/// 地图 `[Actions]` 动作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapActionKind {
    /// 无操作。
    None,
    /// 指定 house 胜利。
    Win,
    /// 失败。
    Lose,
    /// 创建 TeamType。
    CreateTeam,
    /// 销毁指定 TeamType。
    DestroyTeam,
    /// 指定 house 全部机动单位攻击最近敌方。
    AllToHunt,
    /// 增援 TeamType。
    Reinforcement,
    /// 投放区照明弹（竖切 no-op）。
    DropZoneFlare,
    /// 播放全屏影片（竖切 no-op）。
    PlayMovie,
    /// 屏幕文字（竖切 no-op）。
    TextTrigger,
    /// 销毁触发器。
    DestroyTrigger,
    /// 绑定 Tag 实体改属。
    ChangeHouse,
    /// 解除一层胜利阻塞。
    AllowWin,
    /// 全图揭雾（竖切 no-op）。
    RevealAllMap,
    /// 航点附近揭雾（竖切 no-op）。
    RevealAroundWaypoint,
    /// 航点区域揭雾（竖切 no-op）。
    RevealWaypointZone,
    /// 播放音效（竖切 no-op）。
    PlaySound,
    /// 播放主题音乐（竖切 no-op）。
    PlayTheme,
    /// 播放语音（竖切 no-op）。
    PlaySpeech,
    /// 强制执行另一触发器。
    ForceTrigger,
    /// 恢复计时器。
    TimerStart,
    /// 暂停计时器。
    TimerStop,
    /// 延长计时器。
    TimerExtend,
    /// 缩短计时器。
    TimerShorten,
    /// 设置计时器。
    TimerSet,
    /// 迷雾生长（竖切 no-op）。
    GrowShroud,
    /// 摧毁绑定本触发 Tag 的存活实体。
    DestroyAttachedObjects,
    /// 本触发所属 house 全员改属。
    AllChangeHouse,
    /// 结盟。
    MakeAlly,
    /// 解盟。
    MakeEnemy,
    /// 重新笼罩全图（竖切 no-op）。
    ReshroudMap,
    /// 启用触发器。
    EnableTrigger,
    /// 禁用触发器。
    DisableTrigger,
    /// 摧毁指定 Tag 实体。
    DestroyTag,
    /// 启用 AITrigger。
    AiTriggersBegin,
    /// 停用 AITrigger。
    AiTriggersStop,
    /// 增援 TeamType（可带航点）。
    ReinforcementAtWaypoint,
    /// 播放音效（竖切 no-op）。
    PlaySoundEffect,
    /// 航点处重新笼罩（竖切 no-op）。
    ReshroudMapAt,
    /// 计时器文字（竖切 no-op）。
    TimerText,
    /// 摧毁指定 house 全部实体。
    DestroyAllOf,
    /// 摧毁指定 house 全部建筑。
    DestroyAllBuildingsOf,
    /// 摧毁指定 house 全部陆上机动单位。
    DestroyAllLandUnitsOf,
    /// 未接线的原版动作码。
    Unknown(i32),
}

impl MapActionKind {
    /// 由原版动作表整数码解析；未知码为 `Unknown`。
    pub fn from_code(value: i32) -> Self {
        match value {
            0 => Self::None,
            1 => Self::Win,
            2 => Self::Lose,
            4 => Self::CreateTeam,
            5 => Self::DestroyTeam,
            6 => Self::AllToHunt,
            7 => Self::Reinforcement,
            8 => Self::DropZoneFlare,
            10 => Self::PlayMovie,
            11 => Self::TextTrigger,
            12 => Self::DestroyTrigger,
            14 => Self::ChangeHouse,
            15 => Self::AllowWin,
            16 => Self::RevealAllMap,
            17 => Self::RevealAroundWaypoint,
            18 => Self::RevealWaypointZone,
            19 => Self::PlaySound,
            20 => Self::PlayTheme,
            21 => Self::PlaySpeech,
            22 => Self::ForceTrigger,
            23 => Self::TimerStart,
            24 => Self::TimerStop,
            25 => Self::TimerExtend,
            26 => Self::TimerShorten,
            27 => Self::TimerSet,
            31 => Self::GrowShroud,
            32 => Self::DestroyAttachedObjects,
            36 => Self::AllChangeHouse,
            37 => Self::MakeAlly,
            38 => Self::MakeEnemy,
            51 => Self::ReshroudMap,
            53 => Self::EnableTrigger,
            54 => Self::DisableTrigger,
            70 => Self::DestroyTag,
            74 => Self::AiTriggersBegin,
            75 => Self::AiTriggersStop,
            80 => Self::ReinforcementAtWaypoint,
            98 => Self::PlaySoundEffect,
            101 => Self::ReshroudMapAt,
            103 => Self::TimerText,
            119 => Self::DestroyAllOf,
            120 => Self::DestroyAllBuildingsOf,
            121 => Self::DestroyAllLandUnitsOf,
            other => Self::Unknown(other),
        }
    }

    /// 原版动作表整数码。
    pub const fn code(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Win => 1,
            Self::Lose => 2,
            Self::CreateTeam => 4,
            Self::DestroyTeam => 5,
            Self::AllToHunt => 6,
            Self::Reinforcement => 7,
            Self::DropZoneFlare => 8,
            Self::PlayMovie => 10,
            Self::TextTrigger => 11,
            Self::DestroyTrigger => 12,
            Self::ChangeHouse => 14,
            Self::AllowWin => 15,
            Self::RevealAllMap => 16,
            Self::RevealAroundWaypoint => 17,
            Self::RevealWaypointZone => 18,
            Self::PlaySound => 19,
            Self::PlayTheme => 20,
            Self::PlaySpeech => 21,
            Self::ForceTrigger => 22,
            Self::TimerStart => 23,
            Self::TimerStop => 24,
            Self::TimerExtend => 25,
            Self::TimerShorten => 26,
            Self::TimerSet => 27,
            Self::GrowShroud => 31,
            Self::DestroyAttachedObjects => 32,
            Self::AllChangeHouse => 36,
            Self::MakeAlly => 37,
            Self::MakeEnemy => 38,
            Self::ReshroudMap => 51,
            Self::EnableTrigger => 53,
            Self::DisableTrigger => 54,
            Self::DestroyTag => 70,
            Self::AiTriggersBegin => 74,
            Self::AiTriggersStop => 75,
            Self::ReinforcementAtWaypoint => 80,
            Self::PlaySoundEffect => 98,
            Self::ReshroudMapAt => 101,
            Self::TimerText => 103,
            Self::DestroyAllOf => 119,
            Self::DestroyAllBuildingsOf => 120,
            Self::DestroyAllLandUnitsOf => 121,
            Self::Unknown(code) => code,
        }
    }

    /// 竖切已接线（含呈现类 no-op）的全部动作，供能力缺口诊断。
    pub const SUPPORTED: &'static [Self] = &[
        Self::None,
        Self::Win,
        Self::Lose,
        Self::CreateTeam,
        Self::DestroyTeam,
        Self::AllToHunt,
        Self::Reinforcement,
        Self::DropZoneFlare,
        Self::PlayMovie,
        Self::TextTrigger,
        Self::DestroyTrigger,
        Self::ChangeHouse,
        Self::AllowWin,
        Self::RevealAllMap,
        Self::RevealAroundWaypoint,
        Self::RevealWaypointZone,
        Self::PlaySound,
        Self::PlayTheme,
        Self::PlaySpeech,
        Self::ForceTrigger,
        Self::TimerStart,
        Self::TimerStop,
        Self::TimerExtend,
        Self::TimerShorten,
        Self::TimerSet,
        Self::GrowShroud,
        Self::DestroyAttachedObjects,
        Self::AllChangeHouse,
        Self::MakeAlly,
        Self::MakeEnemy,
        Self::ReshroudMap,
        Self::EnableTrigger,
        Self::DisableTrigger,
        Self::DestroyTag,
        Self::AiTriggersBegin,
        Self::AiTriggersStop,
        Self::ReinforcementAtWaypoint,
        Self::PlaySoundEffect,
        Self::ReshroudMapAt,
        Self::TimerText,
        Self::DestroyAllOf,
        Self::DestroyAllBuildingsOf,
        Self::DestroyAllLandUnitsOf,
    ];

    /// 是否为竖切已接线动作。
    pub const fn is_supported(self) -> bool {
        !matches!(self, Self::Unknown(_))
    }
}
