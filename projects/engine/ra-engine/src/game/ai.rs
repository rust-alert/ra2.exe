use super::session::BattleSession;

impl BattleSession {
    /// 为所有非本地、非氛围阵营下发本 tick 的 AI 命令（经 `push_command`）。
    ///
    /// `Neutral` / `Civilian` 地图装饰房主不参与遭遇战 AI。
    /// `Easy`：奇数 tick 跳过生产与自动进攻，仅保留部署/建造节奏。
    /// `Normal`：每 4 个 tick 跳过一拍进攻/生产（略弱于 Hard）。
    /// `Hard`：每 tick 完整下发，并追加一轮生产尝试。
    pub(super) fn push_ai_commands(&mut self) {
        let local_house = self.world.players.iter().find(|p| p.id == self.world.local_player).map(|p| p.house.clone());
        let opponents: Vec<(ra_types::PlayerId, std::sync::Arc<str>)> = self
            .world
            .players
            .iter()
            .filter(|p| !crate::gameplay::ai::is_ambient_house(p.house.as_ref()))
            .filter(|p| local_house.as_ref().map(|h| p.house.as_ref() != h.as_ref()).unwrap_or(true))
            .map(|p| (p.id, p.house.clone()))
            .collect();
        let skip_offensive = difficulty_skips_offensive(&self.difficulty, self.world.tick);
        let hard_extra_produce = difficulty_extra_produce(&self.difficulty);
        for (player, house) in &opponents {
            let house = house.as_ref();
            let mut cmds = Vec::new();
            cmds.extend(crate::gameplay::ai::deploy_mcv_commands(&self.world, house));
            cmds.extend(crate::gameplay::ai::place_power_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_barracks_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_war_factory_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_refinery_commands(&self.world, house, *player));
            if !skip_offensive {
                cmds.extend(crate::gameplay::ai::produce_infantry_commands(&self.world, house, *player));
                cmds.extend(crate::gameplay::ai::produce_vehicle_commands(&self.world, house, *player));
                cmds.extend(crate::gameplay::ai::auto_attack_commands(&self.world, house));
                if hard_extra_produce {
                    cmds.extend(crate::gameplay::ai::produce_infantry_commands(&self.world, house, *player));
                    cmds.extend(crate::gameplay::ai::produce_vehicle_commands(&self.world, house, *player));
                }
            }
            for cmd in cmds {
                self.world.push_player_command(*player, cmd);
            }
        }
    }
}

/// 按难度决定本 tick 是否跳过 AI 进攻/生产。
pub fn difficulty_skips_offensive(difficulty: &str, tick: u64) -> bool {
    if difficulty.eq_ignore_ascii_case("Easy") {
        tick % 2 == 1
    } else if difficulty.eq_ignore_ascii_case("Hard") {
        false
    } else {
        tick % 4 == 3
    }
}

/// Hard 是否追加一轮生产尝试。
pub fn difficulty_extra_produce(difficulty: &str) -> bool {
    difficulty.eq_ignore_ascii_case("Hard")
}
