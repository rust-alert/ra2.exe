//! 与后端无关的渲染计划（几何来自 `LayoutSnapshot`）。

use ra_layout::{LayoutId, LayoutSnapshot, Rect};

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
}
