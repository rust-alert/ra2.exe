//! 渲染阶段图：地形 / 对象 / 特效 / UI / 小地图（R1 骨架）。

/// 一帧内的渲染阶段顺序（后续扩展为真实 pass 录制）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderPassKind {
    /// 清屏与相机。
    Clear,
    /// TMP 地形与静态层（尚未接通）。
    Terrain,
    /// Overlay / 阴影等（尚未接通）。
    Overlay,
    /// 单位、建筑、动画实例。
    Objects,
    /// 爆炸、弹道等特效。
    Effects,
    /// 原版 HUD / 主 UI。
    Ui,
    /// 小地图。
    Minimap,
}

/// 阶段图：规定提交顺序，避免把所有绘制堆进单一 `Renderer` 方法。
#[derive(Debug, Default)]
pub struct PassGraph {
    /// 启用的阶段（默认含 Clear + Objects，与当前原型一致）。
    pub passes: Vec<RenderPassKind>,
}

impl PassGraph {
    /// 原型默认：清屏后画对象标记（预览底图仍由过渡 `SpriteGpu` 处理）。
    pub fn prototype_default() -> Self {
        Self {
            passes: vec![RenderPassKind::Clear, RenderPassKind::Objects],
        }
    }
}
