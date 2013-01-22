use thiserror::Error;

pub type RaResult<T> = Result<T, RaError>;

#[derive(Debug, Error)]
pub enum RaError {
    #[error("未知版本 `{0}`（请用 ra2 或 yr）")]
    UnknownEdition(String),
    #[error("目录 `{0}` 同时有原版与 YR 特征；请在配置里写明 edition")]
    AmbiguousEdition(String),
    #[error("无法在 `{0}` 探测版本")]
    CannotDetectEdition(String),
    #[error("缺少必要文件 `{0}`")]
    MissingFile(String),
    #[error("解析错误: {0}")]
    Parse(String),
    #[error("读写错误: {0}")]
    Io(String),
    #[error("{0}")]
    Msg(String),
}
