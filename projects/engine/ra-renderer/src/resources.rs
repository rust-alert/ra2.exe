//! GPU 资源缓存：atlas / pipeline / buffer 句柄（骨架）。
//!
//! 当前仅有 `handle_count` 诊断字段，**不能**计作已实现 atlas 缓存或纹理生命周期管理。
//! 解码后的逻辑资源在 `ra-assets`；此处只预留已上传 GPU 对象的登记位。
//! 禁止每个可视对象持有独立整图纹理作为长期模型（见 `SpriteGpu::replace_image` 过渡用途）。

/// 渲染资源缓存（纹理、管线、缓冲）——骨架占位。
#[derive(Debug, Default)]
pub struct RenderResourceCache {
    /// 已登记的逻辑资源句柄数（占位诊断，非真实 GPU 对象表）。
    pub handle_count: u32,
}
