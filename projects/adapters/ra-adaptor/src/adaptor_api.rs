//! 适配器契约：探测安装并构造 [`ra_types::RuntimeDefinitions`]。

use ra_types::{AssetSource, RaError, RuntimeDefinitions};

/// 定义构建请求（地图 / 能力选择等，骨架）。
#[derive(Debug, Clone, Default)]
pub struct DefinitionRequest {
    /// 可选：逻辑地图名或标识（不含磁盘绝对路径要求）。
    pub map_key: Option<String>,
}

/// 安装探测报告（骨架）。
#[derive(Debug, Clone, Default)]
pub struct DetectionReport {
    /// 人类可读摘要。
    pub summary: String,
    /// 是否足以尝试 `build_definitions`。
    pub usable: bool,
}

/// 适配错误（当前复用 [`RaError`]，后续可细化）。
pub type AdaptorError = RaError;

/// 主动适配入口：解释现有安装，只交付冻结定义。
pub trait Adaptor {
    /// 探测资源源是否可识别、具备哪些能力迹象。
    fn detect(&self, source: &dyn AssetSource) -> DetectionReport;

    /// 构造对局创建后不可变的 [`RuntimeDefinitions`]。
    fn build_definitions(&self, request: DefinitionRequest) -> Result<RuntimeDefinitions, AdaptorError>;
}
