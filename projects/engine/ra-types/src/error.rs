//! 统一错误类型。

use thiserror::Error;

/// 本工作区共享的 `Result` 别名。
pub type RaResult<T> = Result<T, RaError>;

/// 可跨 crate 传递的错误。
#[derive(Debug, Error)]
pub enum RaError {
    /// 配置里的版本字符串无法识别。
    #[error("未知版本 `{0}`（请用 ra2 或 yr）")]
    UnknownEdition(String),
    /// 配置里的显示分辨率档无法识别。
    #[error("未知显示模式 `{0}`（支持 640x480 / 800x600 / 1024x768）")]
    UnknownDisplayMode(String),
    /// 安装目录同时具备原版与 YR 特征且未显式指定版本。
    #[error("目录 `{0}` 同时有原版与 YR 特征；请在配置里写明 edition")]
    AmbiguousEdition(String),
    /// 自动探测失败。
    #[error("无法在 `{0}` 探测版本")]
    CannotDetectEdition(String),
    /// 期望的资源文件不存在。
    #[error("缺少必要文件 `{0}`")]
    MissingFile(String),
    /// 字节 / INI / 地图等解析失败。
    #[error("解析错误: {0}")]
    Parse(String),
    /// 规则引用了不存在的名称（非空可选引用不得静默成零 ID）。
    #[error("未知{kind}引用 `{name}`（来自 `{owner}`）")]
    UnknownReference {
        /// 引用种类（如 `weapon` / `warhead` / `projectile`）。
        kind: &'static str,
        /// 未命中的名称。
        name: String,
        /// 引用方（类型键或装载步骤）。
        owner: String,
    },
    /// 文件系统或其它 I/O 失败。
    #[error("读写错误: {0}")]
    Io(String),
    /// 其它短消息。
    #[error("{0}")]
    Msg(String),
}
