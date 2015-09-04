//! 与后端无关的渲染计划（几何来自 `LayoutSnapshot`）。

use ra_adaptor::shell_runtime_ui_profile;
use ra_layout::{
    battle_hud_layout_tree, campaign_content_layout_tree, dialog_layout_tree,
    exit_confirm_content_layout_tree, load_screen_layout_tree, options_page_layout_tree,
    shell_design_size, solve_shell_page, LayoutEngine, LayoutId, LayoutSnapshot, Rect,
    RightPanelChrome, Size2, Viewport,
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

    /// 对局 HUD：`battle_hud_layout_tree` → snapshot → 占位 `RenderPlan`。
    pub fn battle_hud_placeholders(viewport_w: u32, viewport_h: u32) -> Self {
        let snap = LayoutEngine.solve(
            Viewport {
                size: Size2 {
                    width: viewport_w.max(1) as f32,
                    height: viewport_h.max(1) as f32,
                },
                ..Viewport::default()
            },
            &battle_hud_layout_tree(viewport_w, viewport_h),
        );
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

    /// 退出确认：`exit_confirm_content_layout_tree` → snapshot → 占位 `RenderPlan`。
    pub fn exit_confirm_placeholders() -> Self {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &exit_confirm_content_layout_tree(chrome),
        );
        Self::solid_placeholders_from_snapshot(&snap, "exit_confirm")
    }

    /// 壳层对话框模板：`RuntimeUiProfile` → `dialog_layout_tree` → 占位 `RenderPlan`。
    pub fn shell_dialog_placeholders(dialog_id: u16, root_id: &str) -> Self {
        let chrome = RightPanelChrome::shell_defaults();
        let profile = shell_runtime_ui_profile();
        let template = profile
            .dialog(dialog_id)
            .unwrap_or_else(|| panic!("shell profile missing dialog {dialog_id:#x}"));
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &dialog_layout_tree(root_id, template, chrome),
        );
        Self::solid_placeholders_from_snapshot(&snap, root_id)
    }

    /// 战役页：`campaign_content_layout_tree` → snapshot → 占位 `RenderPlan`。
    pub fn campaign_placeholders() -> Self {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &campaign_content_layout_tree(chrome),
        );
        Self::solid_placeholders_from_snapshot(&snap, "campaign")
    }

    /// 装载页：`load_screen_layout_tree` → snapshot → 占位 `RenderPlan`。
    pub fn load_screen_placeholders() -> Self {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &load_screen_layout_tree(chrome),
        );
        Self::solid_placeholders_from_snapshot(&snap, "load_screen")
    }

    /// 选项整页：`options_page_layout_tree` → snapshot → 占位 `RenderPlan`。
    pub fn options_page_placeholders() -> Self {
        let chrome = RightPanelChrome::shell_defaults();
        let snap = LayoutEngine.solve(
            Viewport {
                size: shell_design_size(chrome),
                ..Viewport::default()
            },
            &options_page_layout_tree(chrome),
        );
        Self::solid_placeholders_from_snapshot(&snap, "options")
    }
}
