use super::types::BattleState;

impl BattleState {
    /// 按 house 名称设置资金（启动与测试播种用）。
    pub fn set_house_funds(&mut self, house: &str, funds: i32) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(house)) {
            player.funds = funds;
            self.rehash();
            true
        }
        else {
            false
        }
    }

    /// 按 house 名称设置科技上限（战役地图 `[Houses]` 播种）。
    pub fn set_house_tech_level(&mut self, house: &str, tech_level: i32) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(house)) {
            player.tech_level = tech_level.max(0);
            self.rehash();
            true
        }
        else {
            false
        }
    }

    /// 按 house 名称设置同盟列表（战役地图 `[Houses]` `Allies=`）。
    pub fn set_house_allies(&mut self, house: &str, allies: Vec<String>) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(house)) {
            player.allies = allies;
            self.rehash();
            true
        }
        else {
            false
        }
    }

    /// 按遭遇战大厅队伍号写入各方同盟（同队互列；`0` = 无队 / 各自为战）。
    ///
    /// `houses[i]` 与 `teams[i]` 对齐。与 `GameEdition` 无关，RA2 / YR / Mo3 共用。
    pub fn apply_skirmish_lobby_teams(&mut self, houses: &[impl AsRef<str>], teams: &[u8]) {
        let n = houses.len().min(teams.len());
        if n == 0 {
            return;
        }
        let mut changed = false;
        for i in 0..n {
            let team = teams[i];
            let house = houses[i].as_ref();
            let allies: Vec<String> = if team == 0 {
                Vec::new()
            } else {
                (0..n)
                    .filter(|&j| j != i && teams[j] == team)
                    .map(|j| houses[j].as_ref().to_string())
                    .collect()
            };
            if let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(house)) {
                if player.allies != allies {
                    player.allies = allies;
                    changed = true;
                }
            }
        }
        if changed {
            self.rehash();
        }
    }

    /// 将所有玩家资金设为同一起始值（遭遇战大厅资金滑条）。
    pub fn set_all_players_funds(&mut self, funds: i32) {
        if self.players.is_empty() {
            return;
        }
        for player in &mut self.players {
            player.funds = funds;
        }
        self.rehash();
    }

    /// 将所有玩家科技上限设为同一值（遭遇战大厅 / rules 默认）。
    pub fn set_all_players_tech_level(&mut self, tech_level: i32) {
        if self.players.is_empty() {
            return;
        }
        let tech_level = tech_level.max(0);
        for player in &mut self.players {
            player.tech_level = tech_level;
        }
        self.rehash();
    }

    /// 按 house 名称读取资金。
    pub fn house_funds(&self, house: &str) -> Option<i32> {
        self.players.iter().find(|p| p.house.eq_ignore_ascii_case(house)).map(|p| p.funds)
    }

    /// 该 house 是否已允许 AI 生产。
    pub fn house_production_begun(&self, house: &str) -> bool {
        self.players.iter().find(|p| p.house.eq_ignore_ascii_case(house)).map(|p| p.production_begun).unwrap_or(false)
    }

    /// 是否存在任一 house 已允许 AI 生产。
    pub fn any_house_production_begun(&self) -> bool {
        self.players.iter().any(|p| p.production_begun)
    }

    /// 打开指定 house 的 AI 生产（地图动作 Production Begins）。无该 house 时先 `ensure_house`。
    pub fn begin_house_production(&mut self, house: &str) -> bool {
        let house = house.trim();
        if house.is_empty() {
            return false;
        }
        self.ensure_house(house);
        let Some(player) = self.players.iter_mut().find(|p| p.house.eq_ignore_ascii_case(house))
        else {
            return false;
        };
        player.production_begun = true;
        self.rehash();
        true
    }

    /// 查询规则造价；未知类型为 `None`。
    pub fn techno_cost(&self, type_id: ra_types::TypeId) -> Option<u32> {
        self.definitions.techno.get_by_id(type_id).map(|t| t.cost.max(0) as u32)
    }
}
