//! 布局约束（关系，不是复制原版像素）。

use crate::geometry::{Insets, Size2};

/// 尺寸策略。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeRule {
    /// 由内容决定（局部辅助；不作原版主结构）。
    Content,
    /// 固定逻辑像素。
    Fixed(f32),
    /// 相对父尺寸比例（0..=1）。
    Relative(f32),
    /// 夹在最小 / 首选 / 最大之间。
    Clamp {
        /// 最小。
        min: f32,
        /// 首选。
        preferred: f32,
        /// 最大。
        max: f32,
    },
}

/// 水平规则。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HorizontalRule {
    /// 贴父左，偏移。
    Start(f32),
    /// 水平居中，额外偏移。
    Center(f32),
    /// 贴父右，偏移。
    End(f32),
    /// 水平拉伸。
    Stretch,
}

/// 垂直规则。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalRule {
    /// 贴父顶，偏移。
    Top(f32),
    /// 垂直居中，额外偏移。
    Center(f32),
    /// 贴父底，偏移。
    Bottom(f32),
    /// 垂直拉伸。
    Stretch,
}

/// 单节点布局规则。
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutRules {
    /// 水平。
    pub horizontal: HorizontalRule,
    /// 垂直。
    pub vertical: VerticalRule,
    /// 宽。
    pub width: SizeRule,
    /// 高。
    pub height: SizeRule,
    /// 外边距。
    pub margin: Insets,
    /// 内边距。
    pub padding: Insets,
    /// 可选宽高比（宽/高）。
    pub aspect_ratio: Option<f32>,
}

impl Default for LayoutRules {
    fn default() -> Self {
        Self {
            horizontal: HorizontalRule::Start(0.0),
            vertical: VerticalRule::Top(0.0),
            width: SizeRule::Content,
            height: SizeRule::Content,
            margin: Insets::default(),
            padding: Insets::default(),
            aspect_ratio: None,
        }
    }
}

impl LayoutRules {
    /// 固定宽高并贴左上。
    pub fn fixed_size(size: Size2) -> Self {
        Self {
            width: SizeRule::Fixed(size.width),
            height: SizeRule::Fixed(size.height),
            ..Self::default()
        }
    }
}
