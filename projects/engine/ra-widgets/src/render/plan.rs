//! 与后端无关的渲染计划（几何来自 `LayoutSnapshot`）。

use ra_layout::{
    solve_battle_hud, solve_campaign, solve_choose_map, solve_exit_confirm, solve_load_screen,
    solve_network_page, solve_options_page, solve_shell_page, solve_skirmish_lobby, LayoutId,
    LayoutSnapshot, Rect,
};

/// 单条可绘制命令。
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommand {
    /// 纯色矩形（占位 / 调试 / 未绑资源时）。
    SolidRect {
        /// 与 snapshot 对齐的控件 id。
        id: LayoutId,
        /// 视口矩形（与 snapshot 同源）。
        rect: Rect,
        /// RGBA。
        color: [u8; 4],
    },
}

/// 一帧的有序绘制计划。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderPlan {
    /// 按绘制序排列的命令。
    pub commands: Vec<RenderCommand>,
}

impl RenderPlan {
    /// 由快照生成占位色块计划（跳过根容器 id）。
    pub fn solid_placeholders_from_snapshot(snapshot: &LayoutSnapshot, root_id: &str) -> Self {
        let commands = snapshot
            .elements
            .iter()
            .filter(|e| e.id.0 != root_id)
            .map(|e| RenderCommand::SolidRect {
                id: e.id.clone(),
                rect: e.layout.rect,
                color: [80, 80, 80, 255],
            })
            .collect();
        Self { commands }
    }

    /// 按控件 id 查找命令矩形（与 snapshot 同源校验用）。
    pub fn rect_of(&self, id: &str) -> Option<Rect> {
        self.commands.iter().find_map(|cmd| match cmd {
            RenderCommand::SolidRect { id: cid, rect, .. } if cid.0 == id => Some(*rect),
            RenderCommand::SolidRect { .. } => None,
        })
    }

    /// 对局 HUD：`solve_battle_hud` → snapshot → 占位 `RenderPlan`。
    pub fn battle_hud_placeholders(viewport_w: u32, viewport_h: u32) -> Self {
        let snap = solve_battle_hud(viewport_w, viewport_h);
        Self::solid_placeholders_from_snapshot(&snap, "battle_hud")
    }

    /// 壳层页：`solve_shell_page` → snapshot → 占位 `RenderPlan`。
    pub fn shell_page_placeholders(
        root_id: &str,
        stacked_ids: &[&str],
        bottom_id: Option<&str>,
    ) -> Self {
        let snap = solve_shell_page(root_id, stacked_ids, bottom_id);
        Self::solid_placeholders_from_snapshot(&snap, root_id)
    }

    /// 退出确认：`solve_exit_confirm` → snapshot → 占位 `RenderPlan`。
    pub fn exit_confirm_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_exit_confirm(), "exit_confirm")
    }

    /// 壳层对话框模板：已知 id 走 `solve_choose_map` / `solve_skirmish_lobby`。
    pub fn shell_dialog_placeholders(dialog_id: u16, root_id: &str) -> Self {
        let snap = match dialog_id {
            0x6B => solve_choose_map(),
            0x102 => solve_skirmish_lobby(),
            other => panic!("unsupported shell dialog {other:#x}"),
        };
        Self::solid_placeholders_from_snapshot(&snap, root_id)
    }

    /// 战役页：`solve_campaign` → snapshot → 占位 `RenderPlan`。
    pub fn campaign_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_campaign(), "campaign")
    }

    /// 装载页：`solve_load_screen` → snapshot → 占位 `RenderPlan`。
    pub fn load_screen_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_load_screen(), "load_screen")
    }

    /// 网络占位页：`solve_network_page` → snapshot → 占位 `RenderPlan`。
    pub fn network_page_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_network_page(), "network")
    }

    /// 选项整页：`solve_options_page` → snapshot → 占位 `RenderPlan`。
    pub fn options_page_placeholders() -> Self {
        Self::solid_placeholders_from_snapshot(&solve_options_page(), "options")
    }
}
