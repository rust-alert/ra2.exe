use crate::{
    game::commands::GameCommand,
    state::components::{Health, Identity, Owner, ProductionQueue},
};
use ra_map::MapEntityKind;
use ra_types::EntityId;

use super::session::BattleSession;

impl BattleSession {
    /// 指定实体移动到目标格。
    pub fn order_move(&mut self, selected: &[EntityId], x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::MoveTo { entity: id, x, y });
            }
        }
    }

    /// 指定实体沿航点序列移动（路径点规划；首点为当前目的地）。
    pub fn order_move_path(&mut self, selected: &[EntityId], points: &[(u16, u16)]) {
        if self.outcome.is_some() || points.is_empty() {
            return;
        }
        let points = points.to_vec();
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::MovePath { entity: id, points: points.clone() });
            }
        }
    }

    /// 指定实体攻击目标。
    pub fn order_attack(&mut self, selected: &[EntityId], target: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        if self.world.entity_index(target).is_none() {
            return;
        }
        for &id in selected {
            if id != target && self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Attack { attacker: id, target });
            }
        }
    }

    /// 间谍渗透敌方建筑（选中中的 Agent 单位）。
    pub fn order_infiltrate(&mut self, selected: &[EntityId], building: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        if self.world.entity_index(building).is_none() {
            return;
        }
        for &id in selected {
            if id != building && self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Infiltrate { agent: id, building });
            }
        }
    }

    /// 选中是否含可渗透的间谍（`Agent=yes`）。
    pub fn selection_has_agent(&self, selected: &[EntityId]) -> bool {
        selected.iter().any(|&id| {
            self.world
                .ecs_get::<crate::state::components::Identity>(id)
                .is_some_and(|i| crate::gameplay::is_agent(&self.world.definitions, i.type_id))
        })
    }

    /// 工程师占领敌方可俘建筑（选中中的 Engineer 单位）。
    pub fn order_capture_building(&mut self, selected: &[EntityId], building: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        if self.world.entity_index(building).is_none() {
            return;
        }
        for &id in selected {
            if id != building && self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::CaptureBuilding { engineer: id, building });
            }
        }
    }

    /// 选中是否含工程师（`Engineer=yes`）。
    pub fn selection_has_engineer(&self, selected: &[EntityId]) -> bool {
        selected.iter().any(|&id| {
            self.world
                .ecs_get::<crate::state::components::Identity>(id)
                .is_some_and(|i| crate::gameplay::is_engineer(&self.world.definitions, i.type_id))
        })
    }

    /// 建筑类型是否可被工程师占领（`Capturable=yes`）。
    pub fn is_capturable_structure(&self, id: EntityId) -> bool {
        self.world.ecs_get::<crate::state::components::Identity>(id).is_some_and(|i| {
            i.kind == MapEntityKind::Structure && crate::gameplay::is_capturable(&self.world.definitions, i.type_id)
        })
    }

    /// 部署指定可展开单位（如 MCV）。
    pub fn order_deploy(&mut self, selected: &[EntityId]) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Deploy { entity: id });
            }
        }
    }

    /// 对选中机动单位下发就地警戒（清移动/攻击，写入 `mission=Guard`）。
    pub fn order_guard(&mut self, selected: &[EntityId]) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Guard { entity: id });
            }
        }
    }

    /// 对选中机动单位下发停止（清移动/攻击，清空 `mission`）。
    pub fn order_stop(&mut self, selected: &[EntityId]) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Stop { entity: id });
            }
        }
    }

    /// 对选中机动单位下发攻击移动（途中自动接敌）。
    pub fn order_attack_move(&mut self, selected: &[EntityId], x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::AttackMove { entity: id, x, y });
            }
        }
    }

    /// 对选中机动单位下发跟随（持续追目标当前格）。
    pub fn order_follow(&mut self, selected: &[EntityId], target: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if id == target {
                continue;
            }
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Follow { entity: id, target });
            }
        }
    }

    /// 对选中机动单位下发散开（邻近空闲格短距移动）。
    pub fn order_scatter(&mut self, selected: &[EntityId]) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Scatter { entity: id });
            }
        }
    }

    /// 删除选中己方实体（无退款；对应 `Delete` 热键）。
    pub fn order_delete(&mut self, selected: &[EntityId]) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::Delete { entity: id });
            }
        }
    }

    /// 若实体可部署，返回目标建筑类型键（如 `GACNST` / `NACNST`）。
    pub fn deploy_target_of(&self, id: EntityId) -> Option<&str> {
        let identity = self.world.ecs_get::<crate::state::components::Identity>(id)?;
        let target = crate::gameplay::deploy_into_type(&self.world.definitions, identity.type_id)?;
        Some(crate::gameplay::type_key_of(&self.world.definitions, target))
    }

    /// 本地玩家在目标格放置建筑。
    pub fn order_place_building(&mut self, type_id: impl AsRef<str>, x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        let Some(type_id) = crate::gameplay::type_id_of(&self.world.definitions, type_id.as_ref())
        else {
            return;
        };
        self.push_command(GameCommand::PlaceBuilding { player: self.world.local_player, type_id, x, y });
    }

    /// 本地玩家排队生产单位。
    pub fn order_produce(&mut self, type_id: impl AsRef<str>) {
        if self.outcome.is_some() {
            return;
        }
        let Some(type_id) = crate::gameplay::type_id_of(&self.world.definitions, type_id.as_ref())
        else {
            return;
        };
        self.push_command(GameCommand::Produce { player: self.world.local_player, type_id });
    }

    /// 本地玩家出售己方建筑（侧栏出售工具）。
    pub fn order_sell_building(&mut self, building: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        self.push_command(GameCommand::SellBuilding { player: self.world.local_player, building });
    }

    /// 本地玩家切换己方建筑的持续修理（侧栏修理工具）。
    pub fn order_repair_building(&mut self, building: EntityId) {
        if self.outcome.is_some() {
            return;
        }
        self.push_command(GameCommand::RepairBuilding { player: self.world.local_player, building });
    }

    /// 本地玩家取消指定类型的在产项（退款并由引擎排队 `EVA_Canceled`）。
    pub fn order_cancel_produce(&mut self, type_id: impl AsRef<str>) {
        if self.outcome.is_some() {
            return;
        }
        let Some(type_id) = crate::gameplay::type_id_of(&self.world.definitions, type_id.as_ref())
        else {
            return;
        };
        self.push_command(GameCommand::CancelProduce { player: self.world.local_player, type_id });
    }

    /// 本地玩家释放超级武器到目标格（须充能就绪且有挂接建筑）。
    pub fn order_fire_super_weapon(&mut self, type_id: impl AsRef<str>, x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        let Some(type_id) = crate::gameplay::super_weapon_id_of(&self.world.definitions, type_id.as_ref())
        else {
            return;
        };
        self.push_command(GameCommand::FireSuperWeapon { player: self.world.local_player, type_id, x, y });
    }

    /// 本机阵营是否正在生产指定类型。
    pub fn is_local_producing(&self, type_id: &str) -> bool {
        let Some(local_house) = self.world.players.iter().find(|p| p.id == self.world.local_player).map(|p| p.house.clone())
        else {
            return false;
        };
        let Some(wanted_id) = crate::gameplay::type_id_of(&self.world.definitions, type_id)
        else {
            return false;
        };
        self.world.entities.iter().any(|e| {
            let id = e.id;
            if self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true) {
                return false;
            }
            if !self
                .world
                .ecs_get::<Owner>(id)
                .map(|o| crate::gameplay::house_id_of(&self.world.definitions, local_house.as_ref()) == Some(o.house))
                .unwrap_or(false)
            {
                return false;
            }
            self.world.ecs_get::<ProductionQueue>(id).and_then(|q| q.item).is_some_and(|(queued, _)| queued == wanted_id)
        })
    }

    /// 本机建造场是否持有指定类型的待放置完工件。
    pub fn is_local_ready_to_place(&self, type_id: &str) -> bool {
        let Some(local_house) = self.world.players.iter().find(|p| p.id == self.world.local_player).map(|p| p.house.clone())
        else {
            return false;
        };
        let Some(wanted_id) = crate::gameplay::type_id_of(&self.world.definitions, type_id)
        else {
            return false;
        };
        self.world.house_ready_building(local_house.as_ref()).is_some_and(|r| r == wanted_id)
    }

    /// 本机建造场当前待放置的完工件类型（若有）。
    pub fn local_ready_building(&self) -> Option<std::sync::Arc<str>> {
        let local_house = self.world.players.iter().find(|p| p.id == self.world.local_player).map(|p| p.house.clone())?;
        let ready = self.world.house_ready_building(local_house.as_ref())?;
        Some(std::sync::Arc::<str>::from(crate::gameplay::type_key_of(&self.world.definitions, ready)))
    }

    /// 为指定工厂设置集结点（非工厂由世界拒绝）。
    pub fn order_rally(&mut self, selected: &[EntityId], x: u16, y: u16) {
        if self.outcome.is_some() {
            return;
        }
        for &id in selected {
            if self.world.entity_index(id).is_some() {
                self.push_command(GameCommand::SetRallyPoint { factory: id, x, y });
            }
        }
    }

    /// 选中集合中是否包含建筑。
    pub fn selection_has_structure(&self, selected: &[EntityId]) -> bool {
        selected.iter().any(|&id| {
            !self.world.ecs_get::<Health>(id).map(|h| h.dead).unwrap_or(true)
                && self.world.ecs_get::<Identity>(id).map(|identity| identity.kind == MapEntityKind::Structure).unwrap_or(false)
        })
    }
}
