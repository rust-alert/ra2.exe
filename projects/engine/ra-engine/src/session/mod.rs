//! 运行会话。

mod boot;
mod clock;
mod phase;
mod session;

pub use boot::{
    SkirmishOpenResult, open_campaign_session, open_campaign_session_prepared, open_skirmish_session, validate_map_for_battle,
};
pub use session::{Session, SessionPhase, SessionSpec};
