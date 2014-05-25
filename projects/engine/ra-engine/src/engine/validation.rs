//! 会话 / 开局规格校验。

/// 会话规格校验错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionValidationError {
    /// 说明。
    pub message: String,
}

impl std::fmt::Display for SessionValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for SessionValidationError {}
