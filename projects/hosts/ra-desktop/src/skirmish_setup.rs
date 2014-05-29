//! 遭遇战大厅配置：选边 / 难度 → 装载请求（占位枚举，非原版大厅控件）。

/// 大厅可选阵营短名（需与地图实体 `owner` 对得上才会成为本地玩家）。
pub const LOBBY_SIDES: &[&str] = &["Americans", "Russians"];

/// 大厅可选难度标签（写入装载请求；引擎按 Easy/Normal/Hard 调节 AI 节奏）。
pub const LOBBY_DIFFICULTIES: &[&str] = &["Easy", "Normal", "Hard"];

/// 遭遇战装载请求（大厅选项的可序列化快照）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkirmishBootRequest {
    /// 优选地图文件名。
    pub preferred_map: Option<String>,
    /// 期望本地阵营（规则/地图 house 名）。
    pub side: String,
    /// 难度标签。
    pub difficulty: String,
}

impl SkirmishBootRequest {
    /// 默认：无指定图、盟军、普通难度。
    pub fn default_lobby() -> Self {
        Self { preferred_map: None, side: LOBBY_SIDES[0].to_string(), difficulty: LOBBY_DIFFICULTIES[1].to_string() }
    }

    /// 循环下一阵营。
    pub fn cycle_side(&mut self) {
        let i = LOBBY_SIDES.iter().position(|s| *s == self.side.as_str()).unwrap_or(0);
        self.side = LOBBY_SIDES[(i + 1) % LOBBY_SIDES.len()].to_string();
    }

    /// 循环下一难度。
    pub fn cycle_difficulty(&mut self) {
        let i = LOBBY_DIFFICULTIES.iter().position(|s| *s == self.difficulty.as_str()).unwrap_or(1);
        self.difficulty = LOBBY_DIFFICULTIES[(i + 1) % LOBBY_DIFFICULTIES.len()].to_string();
    }

    /// 装载笔记片段。
    pub fn note_fragment(&self) -> String {
        format!("side={} diff={} map={}", self.side, self.difficulty, self.preferred_map.as_deref().unwrap_or("(auto)"))
    }
}
