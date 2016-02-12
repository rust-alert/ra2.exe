//! 零售地图 `[Events]` / `[Actions]` 类型码（竖切已接线子集）。
//!
//! 编号与原版 RA2/YR 触发表一致。未列出的码由引擎记入 unsupported 或视为条件未满足。

/// 地图 `[Events]` 条件类型。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapEventKind {
    /// 进入绑定 CellTag 的格子。
    EnteredBy = 1,
    /// 绑定 Tag 的对象被摧毁（任一）。
    DestroyedByAnybody = 7,
    /// 指定 house 全部陆上机动单位已摧毁。
    DestroyedUnitsAll = 9,
    /// 指定 house 全部建筑已摧毁。
    DestroyedBuildingsAll = 10,
    /// 指定 house 全部实体已摧毁。
    DestroyedAll = 11,
    /// 触发所属 house 资金不少于阈值。
    CreditsExceed = 12,
    /// 计时结束。
    TimeElapse = 13,
    /// 指定 house 处于低电。
    LowPower = 30,
    /// 绑定 Tag 的对象被摧毁（与 `DestroyedByAnybody` 同类，YR 常用）。
    DestroyedByAnything = 48,
    /// 触发所属 house 资金少于阈值。
    CreditsBelow = 52,
}

impl MapEventKind {
    /// 由原始整数码解析；未知码返回 `None`。
    pub fn from_i32(value: i32) -> Option<Self> {
        Some(match value {
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
            _ => return None,
        })
    }

    /// 原版事件表整数码。
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

/// 地图 `[Actions]` 动作类型。
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapActionKind {
    /// 无操作。
    None = 0,
    /// 指定 house 胜利。
    Win = 1,
    /// 失败。
    Lose = 2,
    /// 创建 TeamType。
    CreateTeam = 4,
    /// 销毁指定 TeamType。
    DestroyTeam = 5,
    /// 指定 house 全部机动单位攻击最近敌方。
    AllToHunt = 6,
    /// 增援 TeamType。
    Reinforcement = 7,
    /// 投放区照明弹（竖切 no-op）。
    DropZoneFlare = 8,
    /// 播放全屏影片（竖切 no-op）。
    PlayMovie = 10,
    /// 屏幕文字（竖切 no-op）。
    TextTrigger = 11,
    /// 销毁触发器。
    DestroyTrigger = 12,
    /// 绑定 Tag 实体改属。
    ChangeHouse = 14,
    /// 解除一层胜利阻塞。
    AllowWin = 15,
    /// 全图揭雾（竖切 no-op）。
    RevealAllMap = 16,
    /// 航点附近揭雾（竖切 no-op）。
    RevealAroundWaypoint = 17,
    /// 航点区域揭雾（竖切 no-op）。
    RevealWaypointZone = 18,
    /// 播放音效（竖切 no-op）。
    PlaySound = 19,
    /// 播放主题音乐（竖切 no-op）。
    PlayTheme = 20,
    /// 播放语音（竖切 no-op）。
    PlaySpeech = 21,
    /// 强制执行另一触发器。
    ForceTrigger = 22,
    /// 恢复计时器。
    TimerStart = 23,
    /// 暂停计时器。
    TimerStop = 24,
    /// 延长计时器。
    TimerExtend = 25,
    /// 缩短计时器。
    TimerShorten = 26,
    /// 设置计时器。
    TimerSet = 27,
    /// 迷雾生长（竖切 no-op）。
    GrowShroud = 31,
    /// 摧毁绑定本触发 Tag 的存活实体。
    DestroyAttachedObjects = 32,
    /// 本触发所属 house 全员改属。
    AllChangeHouse = 36,
    /// 结盟。
    MakeAlly = 37,
    /// 解盟。
    MakeEnemy = 38,
    /// 重新笼罩全图（竖切 no-op）。
    ReshroudMap = 51,
    /// 启用触发器。
    EnableTrigger = 53,
    /// 禁用触发器。
    DisableTrigger = 54,
    /// 摧毁指定 Tag 实体。
    DestroyTag = 70,
    /// 启用 AITrigger。
    AiTriggersBegin = 74,
    /// 停用 AITrigger。
    AiTriggersStop = 75,
    /// 增援 TeamType（可带航点）。
    ReinforcementAtWaypoint = 80,
    /// 播放音效（竖切 no-op）。
    PlaySoundEffect = 98,
    /// 航点处重新笼罩（竖切 no-op）。
    ReshroudMapAt = 101,
    /// 计时器文字（竖切 no-op）。
    TimerText = 103,
    /// 摧毁指定 house 全部实体。
    DestroyAllOf = 119,
    /// 摧毁指定 house 全部建筑。
    DestroyAllBuildingsOf = 120,
    /// 摧毁指定 house 全部陆上机动单位。
    DestroyAllLandUnitsOf = 121,
}

impl MapActionKind {
    /// 由原始整数码解析；未知码返回 `None`。
    pub fn from_i32(value: i32) -> Option<Self> {
        Some(match value {
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
            _ => return None,
        })
    }

    /// 原版动作表整数码。
    pub const fn as_i32(self) -> i32 {
        self as i32
    }

    /// 竖切已接线（含呈现类 no-op）的全部动作码，供能力缺口诊断。
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
    pub fn is_supported(kind: i32) -> bool {
        Self::from_i32(kind).is_some()
    }
}
