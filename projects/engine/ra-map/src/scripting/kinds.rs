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
    /// 任意事件（原版码 8）：条件恒真，常与其它条件 AND，或单独表示「启用即触发」。
    AnyEvent,
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
            8 => Self::AnyEvent,
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
            Self::AnyEvent => 8,
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
    /// 指定 house 开始 AI 生产。
    ProductionBegins,
    /// 锁定玩家对局输入（暂停/Esc 仍可用）。
    LockInput,
    /// 解锁玩家对局输入。
    UnlockInput,
    /// 航点处造成 100 点伤害（HE 竖切）。
    Apply100Damage,
    /// 在航点创建可拾取箱。
    CreateCrate,
    /// 创建 TeamType。
    CreateTeam,
    /// 销毁指定 TeamType。
    DestroyTeam,
    /// 指定 house 全部机动单位攻击最近敌方。
    AllToHunt,
    /// 增援 TeamType。
    Reinforcement,
    /// 投放区照明弹（呈现占位）。
    DropZoneFlare,
    /// 播放全屏影片（呈现占位）。
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
    /// 播放音效（呈现占位）。
    PlaySound,
    /// 播放主题音乐（呈现占位）。
    PlayTheme,
    /// 播放语音（呈现占位）。
    PlaySpeech,
    /// 航点播放动画（呈现占位）。
    PlayAnimAt,
    /// 镜头移向航点（呈现占位）。
    CenterCameraAtWaypoint,
    /// 创建雷达事件（呈现占位）。
    CreateRadarEvent,
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
    /// 迷雾生长（呈现占位）。
    GrowShroud,
    /// 摧毁绑定本触发 Tag 的存活实体。
    DestroyAttachedObjects,
    /// 本触发所属 house 全员改属。
    AllChangeHouse,
    /// 结盟。
    MakeAlly,
    /// 解盟。
    MakeEnemy,
    /// 重新笼罩全图（呈现占位）。
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
    /// 播放音效（呈现占位）。
    PlaySoundEffect,
    /// 航点处播放音效（呈现占位）。
    PlaySoundEffectAt,
    /// 播放局内影片（呈现占位）。
    PlayIngameMovie,
    /// 航点处重新笼罩（呈现占位）。
    ReshroudMapAt,
    /// 计时器文字（呈现占位）。
    TimerText,
    /// 闪烁 TeamType 成员（呈现占位）。
    FlashTeam,
    /// 指定 house 步兵欢呼（呈现占位）。
    MakeHouseCheer,
    /// 切换侧栏页签（呈现占位）。
    SetSidebarTab,
    /// 闪烁建造栏图标（呈现占位）。
    FlashCameo,
    /// 停止航点处音效（呈现占位）。
    StopSoundsAt,
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
            3 => Self::ProductionBegins,
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
            41 => Self::PlayAnimAt,
            46 => Self::LockInput,
            47 => Self::UnlockInput,
            48 => Self::CenterCameraAtWaypoint,
            51 => Self::ReshroudMap,
            53 => Self::EnableTrigger,
            54 => Self::DisableTrigger,
            55 => Self::CreateRadarEvent,
            63 => Self::Apply100Damage,
            70 => Self::DestroyTag,
            74 => Self::AiTriggersBegin,
            75 => Self::AiTriggersStop,
            80 => Self::ReinforcementAtWaypoint,
            98 => Self::PlaySoundEffect,
            99 => Self::PlaySoundEffectAt,
            100 => Self::PlayIngameMovie,
            101 => Self::ReshroudMapAt,
            103 => Self::TimerText,
            104 => Self::FlashTeam,
            108 => Self::CreateCrate,
            113 => Self::MakeHouseCheer,
            114 => Self::SetSidebarTab,
            115 => Self::FlashCameo,
            116 => Self::StopSoundsAt,
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
            Self::ProductionBegins => 3,
            Self::LockInput => 46,
            Self::UnlockInput => 47,
            Self::Apply100Damage => 63,
            Self::CreateCrate => 108,
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
            Self::PlayAnimAt => 41,
            Self::CenterCameraAtWaypoint => 48,
            Self::ReshroudMap => 51,
            Self::EnableTrigger => 53,
            Self::DisableTrigger => 54,
            Self::CreateRadarEvent => 55,
            Self::DestroyTag => 70,
            Self::AiTriggersBegin => 74,
            Self::AiTriggersStop => 75,
            Self::ReinforcementAtWaypoint => 80,
            Self::PlaySoundEffect => 98,
            Self::PlaySoundEffectAt => 99,
            Self::PlayIngameMovie => 100,
            Self::ReshroudMapAt => 101,
            Self::TimerText => 103,
            Self::FlashTeam => 104,
            Self::MakeHouseCheer => 113,
            Self::SetSidebarTab => 114,
            Self::FlashCameo => 115,
            Self::StopSoundsAt => 116,
            Self::DestroyAllOf => 119,
            Self::DestroyAllBuildingsOf => 120,
            Self::DestroyAllLandUnitsOf => 121,
            Self::Unknown(code) => code,
        }
    }

    /// 已识别动作（含呈现占位）；未识别码为 `Unknown`。
    pub const SUPPORTED: &'static [Self] = &[
        Self::None,
        Self::Win,
        Self::Lose,
        Self::ProductionBegins,
        Self::LockInput,
        Self::UnlockInput,
        Self::Apply100Damage,
        Self::CreateCrate,
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
        Self::PlayAnimAt,
        Self::CenterCameraAtWaypoint,
        Self::ReshroudMap,
        Self::EnableTrigger,
        Self::DisableTrigger,
        Self::CreateRadarEvent,
        Self::DestroyTag,
        Self::AiTriggersBegin,
        Self::AiTriggersStop,
        Self::ReinforcementAtWaypoint,
        Self::PlaySoundEffect,
        Self::PlaySoundEffectAt,
        Self::PlayIngameMovie,
        Self::ReshroudMapAt,
        Self::TimerText,
        Self::FlashTeam,
        Self::MakeHouseCheer,
        Self::SetSidebarTab,
        Self::FlashCameo,
        Self::StopSoundsAt,
        Self::DestroyAllOf,
        Self::DestroyAllBuildingsOf,
        Self::DestroyAllLandUnitsOf,
    ];

    /// 是否已识别（非 `Unknown`）。呈现占位也算已识别，但仍会报 stub 缺口。
    pub const fn is_supported(self) -> bool {
        !matches!(self, Self::Unknown(_))
    }

    /// 呈现/镜头/音效等占位：开局不拒，装载须 WARN，执行不改变玩法状态。
    pub const fn is_presentation_stub(self) -> bool {
        matches!(
            self,
            Self::DropZoneFlare
                | Self::PlayMovie
                | Self::TextTrigger
                | Self::RevealAllMap
                | Self::RevealAroundWaypoint
                | Self::RevealWaypointZone
                | Self::PlaySound
                | Self::PlayTheme
                | Self::PlaySpeech
                | Self::PlayAnimAt
                | Self::CenterCameraAtWaypoint
                | Self::CreateRadarEvent
                | Self::GrowShroud
                | Self::ReshroudMap
                | Self::PlaySoundEffect
                | Self::PlaySoundEffectAt
                | Self::PlayIngameMovie
                | Self::ReshroudMapAt
                | Self::TimerText
                | Self::FlashTeam
                | Self::MakeHouseCheer
                | Self::SetSidebarTab
                | Self::FlashCameo
                | Self::StopSoundsAt
        )
    }
}
