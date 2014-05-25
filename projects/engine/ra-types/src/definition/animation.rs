//! 动画定义表（呈现语义 ID，不含纹理句柄）。

/// 动画定义集合（骨架）。
#[derive(Debug, Clone, Default)]
pub struct AnimationDefinitions {
    /// 条目数占位。
    pub count: u32,
}
