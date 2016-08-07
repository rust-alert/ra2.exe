use super::types::BattleState;


impl BattleState {
    /// 按 house 名称设置资金（启动与测试播种用）。
    pub fn set_house_funds(&mut self, house: &str, funds: i32) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house) {
            player.funds = funds;
            self.rehash();
            true
        } else {
            false
        }
    }

    /// 按 house 名称设置科技上限（战役地图 `[Houses]` 播种）。
    pub fn set_house_tech_level(&mut self, house: &str, tech_level: i32) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house) {
            player.tech_level = tech_level.max(0);
            self.rehash();
            true
        } else {
            false
        }
    }

    /// 按 house 名称设置同盟列表（战役地图 `[Houses]` `Allies=`）。
    pub fn set_house_allies(&mut self, house: &str, allies: Vec<String>) -> bool {
        if let Some(player) = self.players.iter_mut().find(|p| p.house.as_ref() == house) {
            player.allies = allies;
            self.rehash();
            true
        } else {
            false
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
        self.players.iter().find(|p| p.house.as_ref() == house).map(|p| p.funds)
    }

    /// 查询规则造价；未知类型为 `None`。
    pub fn techno_cost(&self, type_id: &str) -> Option<u32> {
        self.definitions.techno.get(type_id).map(|t| t.cost.max(0) as u32)
    }
}
