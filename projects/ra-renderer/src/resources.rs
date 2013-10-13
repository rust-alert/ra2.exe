//! GPU 资源缓存：atlas / pipeline / buffer 句柄（R1 骨架）。
//!
//! 解码后的逻辑资源在 `ra-assets`；此处只持有已上传的 GPU 对象。
//! 禁止每个可视对象持有独立整图纹理作为长期模型（见 `SpriteGpu::replace_image` 过渡用途）。

/// 渲染资源缓存（纹理、管线、缓冲）。
#[derive(Debug, Default)]
pub struct RenderResourceCache {
    /// 已登记的逻辑资源句柄数（占位诊断）。
    pub handle_count: u32,
}
