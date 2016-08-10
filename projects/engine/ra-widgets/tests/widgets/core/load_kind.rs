//! 自 `engine/ra-widgets/src/core/load_kind.rs` 迁出的单元测试（集成测试 crate）。

// 自 engine/ra-widgets/src/core/load_kind.rs :: tests
use ra_widgets::{OriginalScreen, core::load_kind::*};

#[test]
fn cancel_screen_matches_kind() {
    assert_eq!(LoadKind::Skirmish.cancel_screen(), OriginalScreen::SkirmishLobby);
    assert_eq!(LoadKind::Campaign.cancel_screen(), OriginalScreen::Campaign);
}
