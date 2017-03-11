use super::session::BattleSession;

impl BattleSession {
    /// 为所有非本地、非氛围阵营下发本 tick 的 AI 命令（经 `push_command`）。
    ///
    /// `Neutral` / `Civilian` 地图装饰房主不参与遭遇战 AI。
    /// 若该房主已由 `[AITriggerTypes]` 驱动产队：只做 MCV / 供电 / 矿场等经济基建，
    /// **不下发**兵营、车厂、工厂量产与全图追打（部队由 AITrigger → Create Team / Script 负责）。
    /// 否则走启发式基建，并对工厂量产做节流，避免空闲工厂每 tick 乱刷最便宜单位。
    /// `Easy`：奇数 tick 跳过生产与自动进攻，仅保留部署/建造节奏。
    /// `Normal`：每 4 个 tick 跳过一拍进攻/生产（略弱于 Hard）。
    /// `Hard`：每 tick 完整下发，并追加一轮生产尝试（仍受上述节流约束）。
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
            // 战役未触发「Production Begins」的房主不产、不自动进攻。
            if !self.world.house_production_begun(house) {
                continue;
            }
            let mut cmds = Vec::new();
            cmds.extend(crate::gameplay::ai::deploy_mcv_commands(&self.world, house));
            cmds.extend(crate::gameplay::ai::place_power_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_refinery_commands(&self.world, house, *player));

            let ai_trigger_army = crate::gameplay::ai::house_army_driven_by_ai_triggers(&self.world, house);
            if ai_trigger_army {
                // 产队 / 脚本路径负责作战单位，启发式不再造兵营车厂与量产。
                for cmd in cmds {
                    self.world.push_player_command(*player, cmd);
                }
                continue;
            }

            cmds.extend(crate::gameplay::ai::place_barracks_commands(&self.world, house, *player));
            cmds.extend(crate::gameplay::ai::place_war_factory_commands(&self.world, house, *player));
            if !skip_offensive {
                if crate::gameplay::ai::heuristic_should_produce_army(&self.world, house) {
                    cmds.extend(crate::gameplay::ai::produce_infantry_commands(&self.world, house, *player));
                    cmds.extend(crate::gameplay::ai::produce_vehicle_commands(&self.world, house, *player));
                    if hard_extra_produce {
                        cmds.extend(crate::gameplay::ai::produce_infantry_commands(&self.world, house, *player));
                        cmds.extend(crate::gameplay::ai::produce_vehicle_commands(&self.world, house, *player));
                    }
                }
                cmds.extend(crate::gameplay::ai::auto_attack_commands(&self.world, house));
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
    }
    else if difficulty.eq_ignore_ascii_case("Hard") {
        false
    }
    else {
        tick % 4 == 3
    }
}

/// Hard 是否追加一轮生产尝试。
pub fn difficulty_extra_produce(difficulty: &str) -> bool {
    difficulty.eq_ignore_ascii_case("Hard")
}
