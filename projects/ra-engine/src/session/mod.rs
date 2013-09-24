//! 运行会话。

mod boot;
mod clock;
mod phase;
mod session;

pub use boot::{SkirmishOpenResult, open_skirmish_session};
pub use session::{Session, SessionPhase, SessionSpec};
