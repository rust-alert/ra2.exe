//! 对局页控制器：输入意图、命令、tick、快照；不含窗口与页面导航外壳。

use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use ra_engine::{BattleOutcome, SessionPhase};
use ra_map::MapEntityKind;
use ra_types::EntityId;

use super::{BattleController, BattleNav};

impl BattleController {
    /// 推进仿真（仅对局页调用）并检测是否应进入结算。返回导航与本段耗时。
    pub fn pump(&mut self, dt: f64) -> (BattleNav, std::time::Duration) {
        let started = Instant::now();
        if let (Some(engine), Some(session)) = (self.engine.as_ref(), self.session.as_mut()) {
            let _ = session.pump(&engine.runtime(), dt);
            if let Some(game) = session.battle() {
                self.local.prune_dead(game);
            }
        }
        self.poll_in_battle_eva();
        self.drain_engine_eva_cues();
        let nav = self.poll_outcome_nav();
        self.resolve_deploy_watch();
        (nav, started.elapsed())
    }

    /// 消费引擎 `EvaCue`：仅本机 house 入播报队列。
    pub(super) fn drain_engine_eva_cues(&mut self) {
        let Some(game) = self.session.as_mut().and_then(|s| s.battle_mut())
        else {
            return;
        };
        let Some(local_house) = game.world.players.iter().find(|p| p.id == game.world.local_player).map(|p| p.house.to_string())
        else {
            return;
        };
        let cues = game.world.take_eva_cues();
        for cue in cues {
            if cue.house.eq_ignore_ascii_case(local_house.as_str()) {
                self.queue_battle_sfx_once(cue.event);
            }
        }
    }

    /// 排队对局音效 / EVA（同 id 未播前不重复入队）。
    pub(super) fn queue_battle_sfx_once(&mut self, event_id: &str) {
        if event_id.is_empty() {
            return;
        }
        if self.pending_battle_sfx.iter().any(|e| e.eq_ignore_ascii_case(event_id)) {
            return;
        }
        self.pending_battle_sfx.push(event_id.to_string());
    }

    /// 局内 EVA：低电 / 资金不足 / 单位出厂 / 新建造选项。
    ///
    /// `EVA_UnitLost` / `EVA_OurBaseIsUnderAttack` 由引擎经 [`Self::drain_engine_eva_cues`] 入队。
    /// 结束播报仍由 [`Self::note_outcome_once`] 排队；本函数在已有胜负时跳过。
    /// 建造完成由 [`Self::settle_deployed_structure`] 另行排队。
    pub(super) fn poll_in_battle_eva(&mut self) {
        let mut to_queue: Vec<&'static str> = Vec::new();
        let mut next_alive: Option<HashSet<EntityId>> = None;
        let mut seed_alive = false;
        let mut set_low_latch: Option<bool> = None;
        let mut next_producing: Option<HashSet<EntityId>> = None;
        let mut seed_producing = false;
        let mut unit_ready = false;
        let mut next_options: Option<HashSet<String>> = None;
        let mut seed_options = false;
        let mut new_options = false;

        {
            let Some(game) = self.session.as_ref().and_then(|s| s.battle())
            else {
                return;
            };
            if game.outcome.is_some() {
                return;
            }

            let local_id = game.world.local_player;
            let Some(local) = game.world.players.iter().find(|p| p.id == local_id)
            else {
                return;
            };
            let local_house = local.house.as_ref();
            let low_power = local.low_power();

            if game.world.last_rejects().iter().any(|r| matches!(r.reason, ra_engine::CommandRejectReason::InsufficientFunds)) {
                to_queue.push("EVA_InsufficientFunds");
            }

            if !low_power {
                set_low_latch = Some(false);
            }
            else if !self.eva_low_power_latched {
                set_low_latch = Some(true);
                to_queue.push("EVA_LowPower");
            }

            let mut alive_now: HashSet<EntityId> = HashSet::new();
            let mut producing_now: HashSet<EntityId> = HashSet::new();
            for id in game.world.entity_ids() {
                let Some((_, _, dead)) = game.world.ecs_health(id)
                else {
                    continue;
                };
                if dead {
                    continue;
                }
                let Some(owner) = game.world.ecs_owner(id)
                else {
                    continue;
                };
                if !owner.eq_ignore_ascii_case(local_house) {
                    continue;
                }
                let Some((_, kind)) = game.world.ecs_identity(id)
                else {
                    continue;
                };
                match kind {
                    MapEntityKind::Unit | MapEntityKind::Infantry | MapEntityKind::Aircraft => {
                        alive_now.insert(id);
                    }
                    MapEntityKind::Structure => {
                        if game.world.ecs_produce_item(id).is_some_and(|item| item.is_some()) {
                            producing_now.insert(id);
                        }
                    }
                }
            }

            // 只跟踪「新增机动单位」供 `EVA_UnitReady`。部署 / 变形会让 id 离开机动集，不能当阵亡。
            let gained_mobile = self.eva_alive_seeded && alive_now.iter().any(|id| !self.eva_alive_local_mobiles.contains(id));
            if !self.eva_alive_seeded {
                next_alive = Some(alive_now);
                seed_alive = true;
            }
            else {
                next_alive = Some(alive_now);
            }

            if !self.eva_producing_seeded {
                next_producing = Some(producing_now);
                seed_producing = true;
            }
            else {
                // 仅「出厂」视为就绪：队列清空且本机机动单位集合出现新 ID。
                // 取消生产也会清队列，但不能播 `EVA_UnitReady`。
                let factory_finished = self.eva_producing_factories.iter().any(|id| !producing_now.contains(id));
                unit_ready = factory_finished && gained_mobile;
                next_producing = Some(producing_now);
            }

            // 科技/前置解锁使侧栏条目集合变大 → 新建造选项（资金/电力禁用不计入）。
            let caps = game.snapshot_capabilities(&[]);
            let mut options_now: HashSet<String> = HashSet::new();
            for item in caps
                .build_items
                .iter()
                .chain(caps.defense_items.iter())
                .chain(caps.infantry_items.iter())
                .chain(caps.vehicle_items.iter())
                .chain(caps.aircraft_items.iter())
            {
                options_now.insert(item.type_id.as_ref().to_string());
            }
            if !self.eva_options_seeded {
                next_options = Some(options_now);
                seed_options = true;
            }
            else {
                new_options = options_now.iter().any(|id| !self.eva_known_options.contains(id));
                next_options = Some(options_now);
            }
        }

        if let Some(latch) = set_low_latch {
            self.eva_low_power_latched = latch;
        }
        if let Some(alive) = next_alive {
            self.eva_alive_local_mobiles = alive;
            if seed_alive {
                self.eva_alive_seeded = true;
            }
        }
        if let Some(producing) = next_producing {
            self.eva_producing_factories = producing;
            if seed_producing {
                self.eva_producing_seeded = true;
            }
        }
        if let Some(options) = next_options {
            self.eva_known_options = options;
            if seed_options {
                self.eva_options_seeded = true;
            }
        }
        if unit_ready {
            to_queue.push("EVA_UnitReady");
        }
        if new_options {
            to_queue.push("EVA_NewConstructionOptions");
        }
        for event_id in to_queue {
            self.queue_battle_sfx_once(event_id);
        }
    }

    /// 胜负已定：排队 EVA，留在对局页播报后再 `ToResults`。
    pub(super) fn poll_outcome_nav(&mut self) -> BattleNav {
        let has_outcome = self.session.as_ref().and_then(|s| s.battle()).is_some_and(|g| g.outcome.is_some());
        if !has_outcome {
            return BattleNav::None;
        }
        if let Some(session) = self.session.as_mut() {
            session.phase = SessionPhase::Finished;
        }
        self.leave_armed = false;
        self.begin_outcome_hold();
        match self.outcome_hold_until {
            Some(deadline) if Instant::now() >= deadline => BattleNav::ToResults,
            _ => BattleNav::None,
        }
    }

    /// 首次记录胜负并启动 EVA 播报窗口（幂等）。
    pub(super) fn begin_outcome_hold(&mut self) {
        self.note_outcome_once();
        if self.outcome_hold_until.is_none() {
            // 原版先播 Battle control terminated / Mission Accomplished，再进积分页。
            // 实际时长由壳层按采样长度 `extend_outcome_hold` 校正。
            self.outcome_hold_until = Some(Instant::now() + Duration::from_millis(2500));
            tracing::info!("胜负已定 · 播报 EVA 后进结算");
        }
    }

    /// 按已播放 EVA 采样时长拉长结算延迟（至少覆盖播完）。
    pub fn extend_outcome_hold(&mut self, sample: &ra_assets::PcmAudio) {
        let ch = sample.channels.max(1) as u64;
        let rate = u64::from(sample.sample_rate.max(1));
        let frames = (sample.samples.len() as u64) / ch;
        let ms = frames.saturating_mul(1000) / rate;
        // 尾音留白，避免切页掐断。
        let hold = Duration::from_millis(ms.saturating_add(400).max(1200));
        let deadline = Instant::now() + hold;
        match self.outcome_hold_until {
            Some(prev) if prev >= deadline => {}
            _ => {
                self.outcome_hold_until = Some(deadline);
                tracing::debug!(ms = hold.as_millis(), "已按 EVA 采样延长结算延迟");
            }
        }
    }

    pub(super) fn note_outcome_once(&mut self) {
        let Some(game) = self.session.as_ref().and_then(|s| s.battle())
        else {
            return;
        };
        let Some(outcome) = game.outcome.as_ref()
        else {
            return;
        };
        let label = match outcome {
            BattleOutcome::Victory { owner } => owner.clone(),
            BattleOutcome::Defeat { reason } => {
                if reason.is_empty() {
                    "defeat".into()
                }
                else {
                    format!("defeat:{reason}")
                }
            }
        };
        if self.logged_outcome.as_deref() == Some(label.as_str()) {
            return;
        }
        self.logged_outcome = Some(label.clone());
        let stats = game
            .battle_stats
            .as_ref()
            .map(|s| format!(" · {}tick · 损单位{} · 损建筑{} · 花费{}", s.duration_ticks, s.units_lost, s.buildings_lost, s.funds_spent))
            .unwrap_or_default();
        tracing::info!("对局结束 · {label} · tick={}{stats}", game.world.tick);

        // EVA：放弃/败北播 Battle control terminated；胜利用 Mission Accomplished。
        let eva = match outcome {
            BattleOutcome::Victory { .. } => "EVA_MissionAccomplished",
            BattleOutcome::Defeat { .. } => "EVA_BattleControlTerminated",
        };
        self.queue_battle_sfx_once(eva);
    }
}
