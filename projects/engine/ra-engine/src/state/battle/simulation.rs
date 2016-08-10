use super::types::BattleState;
use crate::game::InputFrame;

impl BattleState {
    /// 推进一个逻辑 tick（使用默认 [`SystemSchedule`]）。
    pub fn advance_tick(&mut self) {
        self.advance_scheduled_tick(&crate::engine::SystemSchedule::standard());
    }

    /// 按调度表推进一个逻辑 tick（阶段顺序的唯一来源）。
    ///
    /// 只消费 `tick <= 当前 tick` 的待执行命令。更晚的调度命令留在队列中，禁止提前执行。
    pub fn advance_scheduled_tick(&mut self, schedule: &crate::engine::SystemSchedule) {
        use crate::engine::SystemPhase;

        self.tick = self.tick.wrapping_add(1);
        let mut due = Vec::new();
        let mut deferred = Vec::new();
        for cmd in std::mem::take(&mut self.pending_commands) {
            if cmd.tick.0 <= self.tick {
                due.push(cmd);
            }
            else {
                deferred.push(cmd);
            }
        }
        self.pending_commands = deferred;
        self.last_input_frame = InputFrame { tick: self.tick, commands: due.clone() };
        self.last_rejects.clear();
        for phase in &schedule.phases {
            match *phase {
                SystemPhase::ApplyCommands => self.apply_commands(&due),
                SystemPhase::Movement => self.advance_movement(),
                SystemPhase::HitFlash => self.tick_hit_flash(),
                SystemPhase::Combat => {
                    self.resolve_combat();
                    self.resolve_infiltrate();
                    self.resolve_capture_building();
                    self.tick_power_blackouts();
                }
                SystemPhase::Turrets => self.advance_turrets(),
                SystemPhase::RefineryIncome => self.advance_refinery_income(),
                SystemPhase::Production => {
                    self.advance_production();
                    crate::gameplay::tick_repairs(self);
                    self.tick_eva_funds_nag();
                }
                SystemPhase::Powers => {
                    crate::gameplay::tick_super_weapon_charges(self);
                    crate::gameplay::tick_lightning_storm(self);
                }
                SystemPhase::TerrainSpawn => self.advance_terrain_spawners(),
                SystemPhase::Triggers => {
                    crate::gameplay::tick_triggers(self);
                    crate::gameplay::tick_ai_triggers(self);
                    crate::gameplay::flush_pending_team_spawns(self);
                    crate::gameplay::tick_script_teams(self);
                }
                SystemPhase::Rehash => {
                    self.sync_ecs_components();
                    self.rehash();
                }
            }
        }
    }
}
